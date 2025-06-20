use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sql_builder::{SqlBuilder, quote};
use sqlx::SqlitePool;

use super::super::authorization_code::{AuthorizationCode, AuthorizationCodeModel, QueryCond};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<SqlitePool>,
}

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    code: String,
    /// i64 as time tick from Epoch in milliseconds.
    expires_at: i64,
    redirect_uri: String,
    scope: Option<String>,
    client_id: String,
    user_id: String,
}

const TABLE_NAME: &'static str = "authorization_code";
const FIELDS: &'static [&'static str] = &[
    "code",
    "expires_at",
    "redirect_uri",
    "scope",
    "client_id",
    "user_id",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS authorization_code (\
    code TEXT NOT NULL UNIQUE,\
    expires_at INTEGER NOT NULL,\
    redirect_uri TEXT NOT NULL,\
    scope TEXT,\
    client_id TEXT NOT NULL,\
    user_id TEXT NOT NULL,\
    PRIMARY KEY (code))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl AuthorizationCodeModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let _ = sqlx::query(TABLE_INIT_SQL)
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn get(&self, code: &str) -> Result<Option<AuthorizationCode>, Box<dyn StdError>> {
        let cond = QueryCond {
            code: Some(code),
            ..Default::default()
        };
        let sql = get_query_sql(SqlBuilder::select_from(TABLE_NAME).fields(FIELDS), &cond).sql()?;

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
        Ok(Some(AuthorizationCode {
            code: row.code,
            expires_at: Utc.timestamp_nanos(row.expires_at * 1000000),
            redirect_uri: row.redirect_uri,
            scope: row.scope,
            client_id: row.client_id,
            user_id: row.user_id,
        }))
    }

    async fn add(&self, code: &AuthorizationCode) -> Result<(), Box<dyn StdError>> {
        let scope = match code.scope.as_deref() {
            None => "NULL".to_string(),
            Some(value) => quote(value),
        };
        let values = vec![
            quote(code.code.as_str()),
            code.expires_at.timestamp_millis().to_string(),
            quote(code.redirect_uri.as_str()),
            scope,
            quote(code.client_id.as_str()),
            quote(code.user_id.as_str()),
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
        let sql = get_query_sql(&mut SqlBuilder::delete_from(TABLE_NAME), cond).sql()?;
        let _ = sqlx::query(sql.as_str())
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }
}

/// Transforms query conditions to the SQL builder.
fn get_query_sql<'a>(builder: &'a mut SqlBuilder, cond: &QueryCond<'a>) -> &'a mut SqlBuilder {
    if let Some(value) = cond.code {
        builder.and_where_eq("code", quote(value));
    }
    if let Some(value) = cond.client_id {
        builder.and_where_eq("client_id", quote(value));
    }
    if let Some(value) = cond.user_id {
        builder.and_where_eq("user_id", quote(value));
    }
    builder
}
