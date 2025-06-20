use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::TryStreamExt;
use sql_builder::{SqlBuilder, quote};
use sqlx::SqlitePool;

use super::{
    super::application::{
        Application, ApplicationModel, Cursor, ListOptions, ListQueryCond, QueryCond, SortKey,
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
    application_id: String,
    code: String,
    unit_id: String,
    unit_code: String,
    /// i64 as time tick from Epoch in milliseconds.
    created_at: i64,
    /// i64 as time tick from Epoch in milliseconds.
    modified_at: i64,
    host_uri: String,
    name: String,
    info: String,
}

/// Use "COUNT(*)" instead of "COUNT(fields...)" to simplify the implementation.
#[derive(sqlx::FromRow)]
struct CountSchema {
    #[sqlx(rename = "COUNT(*)")]
    count: i64,
}

const TABLE_NAME: &'static str = "application";
const FIELDS: &'static [&'static str] = &[
    "application_id",
    "code",
    "unit_id",
    "unit_code",
    "created_at",
    "modified_at",
    "host_uri",
    "name",
    "info",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS application (\
    application_id TEXT NOT NULL UNIQUE,\
    code TEXT NOT NULL,\
    unit_id TEXT NOT NULL,\
    unit_code TEXT NOT NULL,\
    created_at INTEGER NOT NULL,\
    modified_at INTEGER NOT NULL,\
    host_uri TEXT NOT NULL,\
    name TEXT NOT NULL,\
    info TEXT,\
    UNIQUE (unit_id,code),\
    PRIMARY KEY (application_id))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl ApplicationModel for Model {
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
            Err(e) => return Err(Box::new(e)),
            Ok(row) => row,
        };
        Ok(row.count as u64)
    }

    async fn list(
        &self,
        opts: &ListOptions,
        cursor: Option<Box<dyn Cursor>>,
    ) -> Result<(Vec<Application>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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
            list.push(Application {
                application_id: row.application_id,
                code: row.code,
                unit_id: row.unit_id,
                unit_code: row.unit_code,
                created_at: Utc.timestamp_nanos(row.created_at * 1000000),
                modified_at: Utc.timestamp_nanos(row.modified_at * 1000000),
                host_uri: row.host_uri,
                name: row.name,
                info: serde_json::from_str(row.info.as_str())?,
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

    async fn get(&self, cond: &QueryCond) -> Result<Option<Application>, Box<dyn StdError>> {
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

        Ok(Some(Application {
            application_id: row.application_id,
            code: row.code,
            unit_id: row.unit_id,
            unit_code: row.unit_code,
            created_at: Utc.timestamp_nanos(row.created_at * 1000000),
            modified_at: Utc.timestamp_nanos(row.modified_at * 1000000),
            host_uri: row.host_uri,
            name: row.name,
            info: serde_json::from_str(row.info.as_str())?,
        }))
    }

    async fn add(&self, application: &Application) -> Result<(), Box<dyn StdError>> {
        let info = match serde_json::to_string(&application.info) {
            Err(_) => quote("{}"),
            Ok(value) => quote(value.as_str()),
        };
        let values = vec![
            quote(application.application_id.as_str()),
            quote(application.code.as_str()),
            quote(application.unit_id.as_str()),
            quote(application.unit_code.as_str()),
            application.created_at.timestamp_millis().to_string(),
            application.modified_at.timestamp_millis().to_string(),
            quote(application.host_uri.as_str()),
            quote(application.name.as_str()),
            info,
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
    async fn try_next(&mut self) -> Result<Option<Application>, Box<dyn StdError>> {
        self.offset += 1;
        Ok(None)
    }

    fn offset(&self) -> u64 {
        self.offset
    }
}

/// Transforms query conditions to the SQL builder.
fn build_where<'a>(builder: &'a mut SqlBuilder, cond: &QueryCond<'a>) -> &'a mut SqlBuilder {
    if let Some(value) = cond.unit_id {
        builder.and_where_eq("unit_id", quote(value));
    }
    if let Some(value) = cond.application_id {
        builder.and_where_eq("application_id", quote(value));
    }
    if let Some(value) = cond.code {
        builder.and_where_eq("code", quote(value));
    }
    builder
}

/// Transforms query conditions to the SQL builder.
fn build_list_where<'a>(
    builder: &'a mut SqlBuilder,
    cond: &ListQueryCond<'a>,
) -> &'a mut SqlBuilder {
    if let Some(value) = cond.unit_id {
        builder.and_where_eq("unit_id", quote(value));
    }
    if let Some(value) = cond.application_id {
        builder.and_where_eq("application_id", quote(value));
    }
    if let Some(value) = cond.code {
        builder.and_where_eq("code", quote(value));
    }
    if let Some(value) = cond.code_contains {
        build_where_like(builder, "code", value.to_lowercase().as_str());
    }
    if let Some(value) = cond.name_contains {
        build_where_like(builder, "name", value.to_lowercase().as_str());
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
                SortKey::Code => "code",
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
    if let Some(value) = updates.host_uri.as_ref() {
        builder.set("host_uri", quote(value));
        count += 1;
    }
    if let Some(value) = updates.name.as_ref() {
        builder.set("name", quote(value));
        count += 1;
    }
    if let Some(value) = updates.info {
        match serde_json::to_string(value) {
            Err(_) => {
                builder.set("info", quote("{}"));
            }
            Ok(value) => {
                builder.set("info", quote(value));
            }
        }
        count += 1;
    }
    if count == 0 {
        return None;
    }

    builder.and_where_eq("application_id", quote(cond.application_id));
    Some(builder)
}
