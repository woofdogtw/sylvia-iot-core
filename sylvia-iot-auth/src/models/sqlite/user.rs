use std::{collections::HashMap, error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::TryStreamExt;
use serde_json;
use sql_builder::{SqlBuilder, quote};
use sqlx::SqlitePool;

use super::{
    super::user::{
        Cursor, ListOptions, ListQueryCond, QueryCond, SortKey, Updates, User, UserModel,
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
    user_id: String,
    account: String,
    /// i64 as time tick from Epoch in milliseconds.
    created_at: i64,
    /// i64 as time tick from Epoch in milliseconds.
    modified_at: i64,
    /// i64 as time tick from Epoch in milliseconds.
    verified_at: Option<i64>,
    /// i64 as time tick from Epoch in milliseconds.
    expired_at: Option<i64>,
    /// i64 as time tick from Epoch in milliseconds.
    disabled_at: Option<i64>,
    /// JSON string value such as `{"role1":true,"role2":false}`.
    roles: String,
    password: String,
    salt: String,
    name: String,
    info: String,
}

/// Use "COUNT(*)" instead of "COUNT(fields...)" to simplify the implementation.
#[derive(sqlx::FromRow)]
struct CountSchema {
    #[sqlx(rename = "COUNT(*)")]
    count: i64,
}

const TABLE_NAME: &'static str = "user";
const FIELDS: &'static [&'static str] = &[
    "user_id",
    "account",
    "created_at",
    "modified_at",
    "verified_at",
    "expired_at",
    "disabled_at",
    "roles",
    "password",
    "salt",
    "name",
    "info",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS user (\
    user_id TEXT NOT NULL UNIQUE,\
    account TEXT NOT NULL UNIQUE,\
    created_at INTEGER NOT NULL,\
    modified_at INTEGER NOT NULL,\
    verified_at INTEGER,\
    expired_at INTEGER,\
    disabled_at INTEGER,\
    roles TEXT,\
    password TEXT NOT NULL,\
    salt TEXT NOT NULL,\
    name TEXT NOT NULL,\
    info TEXT,\
    PRIMARY KEY (user_id))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl UserModel for Model {
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
    ) -> Result<(Vec<User>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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
            let roles: HashMap<String, bool> = match serde_json::from_str(row.roles.as_str()) {
                Err(_) => HashMap::new(),
                Ok(roles) => roles,
            };
            list.push(User {
                user_id: row.user_id,
                account: row.account,
                created_at: Utc.timestamp_nanos(row.created_at * 1000000),
                modified_at: Utc.timestamp_nanos(row.modified_at * 1000000),
                verified_at: match row.verified_at {
                    None => None,
                    Some(value) => Some(Utc.timestamp_nanos(value * 1000000)),
                },
                expired_at: match row.expired_at {
                    None => None,
                    Some(value) => Some(Utc.timestamp_nanos(value * 1000000)),
                },
                disabled_at: match row.disabled_at {
                    None => None,
                    Some(value) => Some(Utc.timestamp_nanos(value * 1000000)),
                },
                roles,
                password: row.password,
                salt: row.salt,
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

    async fn get(&self, cond: &QueryCond) -> Result<Option<User>, Box<dyn StdError>> {
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

        let roles: HashMap<String, bool> = match serde_json::from_str(row.roles.as_str()) {
            Err(_) => HashMap::new(),
            Ok(roles) => roles,
        };
        Ok(Some(User {
            user_id: row.user_id,
            account: row.account,
            created_at: Utc.timestamp_nanos(row.created_at * 1000000),
            modified_at: Utc.timestamp_nanos(row.modified_at * 1000000),
            verified_at: match row.verified_at {
                None => None,
                Some(value) => Some(Utc.timestamp_nanos(value * 1000000)),
            },
            expired_at: match row.expired_at {
                None => None,
                Some(value) => Some(Utc.timestamp_nanos(value * 1000000)),
            },
            disabled_at: match row.disabled_at {
                None => None,
                Some(value) => Some(Utc.timestamp_nanos(value * 1000000)),
            },
            roles,
            password: row.password,
            salt: row.salt,
            name: row.name,
            info: serde_json::from_str(row.info.as_str())?,
        }))
    }

    async fn add(&self, user: &User) -> Result<(), Box<dyn StdError>> {
        let roles = match serde_json::to_string(&user.roles) {
            Err(_) => quote("{}"),
            Ok(value) => quote(value.as_str()),
        };
        let info = match serde_json::to_string(&user.info) {
            Err(_) => quote("{}"),
            Ok(value) => quote(value.as_str()),
        };
        let values = vec![
            quote(user.user_id.as_str()),
            quote(user.account.to_lowercase().as_str()),
            user.created_at.timestamp_millis().to_string(),
            user.modified_at.timestamp_millis().to_string(),
            match user.verified_at {
                None => "NULL".to_string(),
                Some(value) => value.timestamp_millis().to_string(),
            },
            match user.expired_at {
                None => "NULL".to_string(),
                Some(value) => value.timestamp_millis().to_string(),
            },
            match user.disabled_at {
                None => "NULL".to_string(),
                Some(value) => value.timestamp_millis().to_string(),
            },
            roles,
            quote(user.password.as_str()),
            quote(user.salt.as_str()),
            quote(user.name.as_str()),
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

    async fn del(&self, user_id: &str) -> Result<(), Box<dyn StdError>> {
        let sql = SqlBuilder::delete_from(TABLE_NAME)
            .and_where_eq("user_id", quote(user_id))
            .sql()?;
        let _ = sqlx::query(sql.as_str())
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn update(&self, user_id: &str, updates: &Updates) -> Result<(), Box<dyn StdError>> {
        let sql =
            match build_update_where(&mut SqlBuilder::update_table(TABLE_NAME), user_id, updates) {
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
    async fn try_next(&mut self) -> Result<Option<User>, Box<dyn StdError>> {
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
    if let Some(value) = cond.account {
        builder.and_where_eq("account", quote(value.to_lowercase().as_str()));
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
    if let Some(value) = cond.account {
        builder.and_where_eq("account", quote(value.to_lowercase().as_str()));
    }
    if let Some(value) = cond.account_contains {
        build_where_like(builder, "account", value.to_lowercase().as_str());
    }
    if let Some(value) = cond.verified_at {
        if value {
            builder.and_where_is_not_null("verified_at");
        } else {
            builder.and_where_is_null("verified_at");
        }
    }
    if let Some(value) = cond.disabled_at {
        if value {
            builder.and_where_is_not_null("disabled_at");
        } else {
            builder.and_where_is_null("disabled_at");
        }
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
                SortKey::Account => "account",
                SortKey::CreatedAt => "created_at",
                SortKey::ModifiedAt => "modified_at",
                SortKey::VerifiedAt => "verified_at",
                SortKey::ExpiredAt => "expired_at",
                SortKey::DisabledAt => "disabled_at",
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
    user_id: &str,
    updates: &Updates,
) -> Option<&'a mut SqlBuilder> {
    let mut count = 0;
    if let Some(value) = updates.modified_at.as_ref() {
        builder.set("modified_at", value.timestamp_millis());
        count += 1;
    }
    if let Some(value) = updates.verified_at.as_ref() {
        builder.set("verified_at", value.timestamp_millis());
        count += 1;
    }
    if let Some(value) = updates.expired_at.as_ref() {
        match value {
            None => {
                builder.set("expired_at", "NULL");
            }
            Some(value) => {
                builder.set("expired_at", value.timestamp_millis());
            }
        }
        count += 1;
    }
    if let Some(value) = updates.disabled_at.as_ref() {
        match value {
            None => {
                builder.set("disabled_at", "NULL");
            }
            Some(value) => {
                builder.set("disabled_at", value.timestamp_millis());
            }
        }
        count += 1;
    }
    if let Some(value) = updates.roles {
        builder.set(
            "roles",
            match serde_json::to_string(value) {
                Err(_) => quote("{}"),
                Ok(value) => quote(value.as_str()),
            },
        );
        count += 1;
    }
    if let Some(value) = updates.password.as_ref() {
        builder.set("password", quote(value));
        count += 1;
    }
    if let Some(value) = updates.salt.as_ref() {
        builder.set("salt", quote(value));
        count += 1;
    }
    if let Some(value) = updates.name {
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

    builder.and_where_eq("user_id", quote(user_id));
    Some(builder)
}
