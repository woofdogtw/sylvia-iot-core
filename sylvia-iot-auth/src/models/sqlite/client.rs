use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::TryStreamExt;
use sql_builder::{SqlBuilder, quote};
use sqlx::SqlitePool;

use super::{
    super::client::{
        Client, ClientModel, Cursor, ListOptions, ListQueryCond, QueryCond, SortKey,
        UpdateQueryCond, Updates,
    },
    build_where_like,
};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<SqlitePool>,
}

/// Cursor instance.
///
/// The SQLite implementation uses the original list options and the progress offset.
pub struct DbCursor {
    offset: u64,
}

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    client_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    created_at: i64,
    /// i64 as time tick from Epoch in milliseconds.
    modified_at: i64,
    client_secret: Option<String>,
    redirect_uris: String,
    /// Space-separated value such as `scope1 scope2`.
    scopes: String,
    user_id: String,
    name: String,
    image_url: Option<String>,
}

/// Use "COUNT(*)" instead of "COUNT(fields...)" to simplify the implementation.
#[derive(sqlx::FromRow)]
struct CountSchema {
    #[sqlx(rename = "COUNT(*)")]
    count: i64,
}

const TABLE_NAME: &'static str = "client";
const FIELDS: &'static [&'static str] = &[
    "client_id",
    "created_at",
    "modified_at",
    "client_secret",
    "redirect_uris",
    "scopes",
    "user_id",
    "name",
    "image_url",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS client (\
    client_id TEXT NOT NULL UNIQUE,\
    created_at INTEGER NOT NULL,\
    modified_at INTEGER NOT NULL,\
    client_secret TEXT,\
    redirect_uris TEXT NOT NULL,\
    scopes TEXT NOT NULL,\
    user_id TEXT NOT NULL,\
    name TEXT NOT NULL,\
    image_url TEXT,\
    PRIMARY KEY (client_id))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl ClientModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let _ = sqlx::query(TABLE_INIT_SQL)
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn count(&self, cond: &ListQueryCond) -> Result<u64, Box<dyn StdError>> {
        let sql = build_list_where(SqlBuilder::select_from(TABLE_NAME).count("*"), &cond).sql()?;

        let result: Result<CountSchema, sqlx::Error> = sqlx::query_as(sql.as_str())
            .fetch_one(self.conn.as_ref())
            .await;

        let row = match result {
            Err(e) => {
                return Err(Box::new(e));
            }
            Ok(row) => row,
        };
        Ok(row.count as u64)
    }

    async fn list(
        &self,
        opts: &ListOptions,
        cursor: Option<Box<dyn Cursor>>,
    ) -> Result<(Vec<Client>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
        let mut cursor = match cursor {
            None => Box::new(DbCursor::new()),
            Some(cursor) => cursor,
        };

        let mut opts = ListOptions { ..*opts };
        if let Some(offset) = opts.offset {
            opts.offset = Some(offset + cursor.offset());
        } else {
            opts.offset = Some(cursor.offset());
        }
        let opts_limit = opts.limit;
        if let Some(limit) = opts_limit {
            if limit > 0 {
                if cursor.offset() >= limit {
                    return Ok((vec![], None));
                }
                opts.limit = Some(limit - cursor.offset());
            }
        }
        let mut builder = SqlBuilder::select_from(TABLE_NAME);
        build_limit_offset(&mut builder, &opts);
        build_sort(&mut builder, &opts);
        let sql = build_list_where(&mut builder, opts.cond).sql()?;

        let mut rows = sqlx::query_as::<_, Schema>(sql.as_str()).fetch(self.conn.as_ref());

        let mut count: u64 = 0;
        let mut list = vec![];
        while let Some(row) = rows.try_next().await? {
            let _ = cursor.as_mut().try_next().await?;
            let redirect_uris = row
                .redirect_uris
                .split(" ")
                .filter_map(|x| {
                    if x.len() > 0 {
                        Some(x.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            let scopes = row
                .scopes
                .split(" ")
                .filter_map(|x| {
                    if x.len() > 0 {
                        Some(x.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            list.push(Client {
                client_id: row.client_id,
                created_at: Utc.timestamp_nanos(row.created_at * 1000000),
                modified_at: Utc.timestamp_nanos(row.modified_at * 1000000),
                client_secret: row.client_secret,
                redirect_uris,
                scopes,
                user_id: row.user_id,
                name: row.name,
                image_url: row.image_url,
            });
            if let Some(limit) = opts_limit {
                if limit > 0 && cursor.offset() >= limit {
                    if let Some(cursor_max) = opts.cursor_max {
                        if (count + 1) >= cursor_max {
                            return Ok((list, Some(cursor)));
                        }
                    }
                    return Ok((list, None));
                }
            }
            if let Some(cursor_max) = opts.cursor_max {
                count += 1;
                if count >= cursor_max {
                    return Ok((list, Some(cursor)));
                }
            }
        }
        Ok((list, None))
    }

    async fn get(&self, cond: &QueryCond) -> Result<Option<Client>, Box<dyn StdError>> {
        let sql = build_where(SqlBuilder::select_from(TABLE_NAME).fields(FIELDS), &cond).sql()?;

        let result: Result<Schema, sqlx::Error> = sqlx::query_as(sql.as_str())
            .fetch_one(self.conn.as_ref())
            .await;

        let row = match result {
            Err(e) => match e {
                sqlx::Error::RowNotFound => return Ok(None),
                _ => return Err(Box::new(e)),
            },
            Ok(row) => row,
        };

        let redirect_uris = row
            .redirect_uris
            .split(" ")
            .filter_map(|x| {
                if x.len() > 0 {
                    Some(x.to_string())
                } else {
                    None
                }
            })
            .collect();
        let scopes = row
            .scopes
            .split(" ")
            .filter_map(|x| {
                if x.len() > 0 {
                    Some(x.to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(Some(Client {
            client_id: row.client_id,
            created_at: Utc.timestamp_nanos(row.created_at * 1000000),
            modified_at: Utc.timestamp_nanos(row.modified_at * 1000000),
            client_secret: row.client_secret,
            redirect_uris,
            scopes,
            user_id: row.user_id,
            name: row.name,
            image_url: row.image_url,
        }))
    }

    async fn add(&self, client: &Client) -> Result<(), Box<dyn StdError>> {
        let client_secret = match client.client_secret.as_deref() {
            None => "NULL".to_string(),
            Some(value) => quote(value),
        };
        let image_url = match client.image_url.as_deref() {
            None => "NULL".to_string(),
            Some(value) => quote(value),
        };
        let values = vec![
            quote(client.client_id.as_str()),
            client.created_at.timestamp_millis().to_string(),
            client.modified_at.timestamp_millis().to_string(),
            client_secret,
            quote(client.redirect_uris.join(" ")),
            quote(client.scopes.join(" ")),
            quote(client.user_id.as_str()),
            quote(client.name.as_str()),
            image_url,
        ];
        let sql = SqlBuilder::insert_into(TABLE_NAME)
            .fields(FIELDS)
            .values(&values)
            .sql()?;
        let _ = sqlx::query(sql.as_str())
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>> {
        let sql = build_where(&mut SqlBuilder::delete_from(TABLE_NAME), cond).sql()?;
        let _ = sqlx::query(sql.as_str())
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn update(
        &self,
        cond: &UpdateQueryCond,
        updates: &Updates,
    ) -> Result<(), Box<dyn StdError>> {
        let sql = match build_update_where(&mut SqlBuilder::update_table(TABLE_NAME), cond, updates)
        {
            None => return Ok(()),
            Some(builder) => builder.sql()?,
        };
        let _ = sqlx::query(sql.as_str())
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }
}

impl DbCursor {
    /// To create the cursor instance.
    pub fn new() -> Self {
        DbCursor { offset: 0 }
    }
}

#[async_trait]
impl Cursor for DbCursor {
    async fn try_next(&mut self) -> Result<Option<Client>, Box<dyn StdError>> {
        self.offset += 1;
        Ok(None)
    }

    fn offset(&self) -> u64 {
        self.offset
    }
}

/// Transforms query conditions to the SQL builder.
fn build_where<'a>(builder: &'a mut SqlBuilder, cond: &QueryCond<'a>) -> &'a mut SqlBuilder {
    if let Some(value) = cond.user_id {
        builder.and_where_eq("user_id", quote(value));
    }
    if let Some(value) = cond.client_id {
        builder.and_where_eq("client_id", quote(value));
    }
    builder
}

/// Transforms query conditions to the SQL builder.
fn build_list_where<'a>(
    builder: &'a mut SqlBuilder,
    cond: &ListQueryCond<'a>,
) -> &'a mut SqlBuilder {
    if let Some(value) = cond.user_id {
        builder.and_where_eq("user_id", quote(value));
    }
    if let Some(value) = cond.client_id {
        builder.and_where_eq("client_id", quote(value));
    }
    if let Some(value) = cond.name_contains {
        build_where_like(builder, "name", value);
    }
    builder
}

/// Transforms model options to the SQL builder.
fn build_limit_offset<'a>(builder: &'a mut SqlBuilder, opts: &ListOptions) -> &'a mut SqlBuilder {
    if let Some(value) = opts.limit {
        if value > 0 {
            builder.limit(value);
        }
    }
    if let Some(value) = opts.offset {
        match opts.limit {
            None => builder.limit(-1).offset(value),
            Some(0) => builder.limit(-1).offset(value),
            _ => builder.offset(value),
        };
    }
    builder
}

/// Transforms model options to the SQL builder.
fn build_sort<'a>(builder: &'a mut SqlBuilder, opts: &ListOptions) -> &'a mut SqlBuilder {
    if let Some(sort_cond) = opts.sort.as_ref() {
        for cond in sort_cond.iter() {
            let key = match cond.key {
                SortKey::CreatedAt => "created_at",
                SortKey::ModifiedAt => "modified_at",
                SortKey::Name => "name",
            };
            builder.order_by(key, !cond.asc);
        }
    }
    builder
}

/// Transforms query conditions and the model object to the SQL builder.
fn build_update_where<'a>(
    builder: &'a mut SqlBuilder,
    cond: &UpdateQueryCond<'a>,
    updates: &Updates,
) -> Option<&'a mut SqlBuilder> {
    let mut count = 0;
    if let Some(value) = updates.modified_at.as_ref() {
        builder.set("modified_at", value.timestamp_millis());
        count += 1;
    }
    if let Some(value) = updates.client_secret.as_ref() {
        match value {
            None => {
                builder.set("client_secret", "NULL");
            }
            Some(value) => {
                builder.set("client_secret", quote(value));
            }
        }
        count += 1;
    }
    if let Some(value) = updates.redirect_uris.as_ref() {
        builder.set("redirect_uris", quote(value.join(" ")));
        count += 1;
    }
    if let Some(value) = updates.scopes.as_ref() {
        builder.set("scopes", quote(value.join(" ")));
        count += 1;
    }
    if let Some(value) = updates.name.as_ref() {
        builder.set("name", quote(value));
        count += 1;
    }
    if let Some(value) = updates.image_url.as_ref() {
        match value {
            None => {
                builder.set("image_url", "NULL");
            }
            Some(value) => {
                builder.set("image_url", quote(value));
            }
        }
        count += 1;
    }
    if count == 0 {
        return None;
    }

    builder.and_where_eq("user_id", quote(cond.user_id));
    builder.and_where_eq("client_id", quote(cond.client_id));
    Some(builder)
}
