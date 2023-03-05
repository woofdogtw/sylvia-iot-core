use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sql_builder::{quote, SqlBuilder};
use sqlx::SqlitePool;

use super::super::refresh_token::{QueryCond, RefreshToken, RefreshTokenModel};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<SqlitePool>,
}

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    refresh_token: String,
    /// i64 as time tick from Epoch in milliseconds.
    expires_at: i64,
    scope: Option<String>,
    client_id: String,
    redirect_uri: String,
    user_id: String,
}

const TABLE_NAME: &'static str = "refresh_token";
const FIELDS: &'static [&'static str] = &[
    "refresh_token",
    "expires_at",
    "scope",
    "client_id",
    "redirect_uri",
    "user_id",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS refresh_token (\
    refresh_token TEXT NOT NULL UNIQUE,\
    expires_at INTEGER NOT NULL,\
    scope TEXT,\
    client_id TEXT NOT NULL,\
    redirect_uri TEXT NOT NULL,\
    user_id TEXT NOT NULL,\
    PRIMARY KEY (refresh_token))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl RefreshTokenModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let _ = sqlx::query(TABLE_INIT_SQL)
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn get(&self, refresh_token: &str) -> Result<Option<RefreshToken>, Box<dyn StdError>> {
        let cond = QueryCond {
            refresh_token: Some(refresh_token),
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
        Ok(Some(RefreshToken {
            refresh_token: row.refresh_token,
            expires_at: Utc.timestamp_nanos(row.expires_at * 1000000),
            scope: row.scope,
            client_id: row.client_id,
            user_id: row.user_id,
            redirect_uri: row.redirect_uri,
        }))
    }

    async fn add(&self, token: &RefreshToken) -> Result<(), Box<dyn StdError>> {
        let scope = match token.scope.as_deref() {
            None => "NULL".to_string(),
            Some(scope) => quote(scope),
        };
        let values = vec![
            quote(token.refresh_token.as_str()),
            token.expires_at.timestamp_millis().to_string(),
            scope,
            quote(token.client_id.as_str()),
            quote(token.redirect_uri.as_str()),
            quote(token.user_id.as_str()),
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
    if let Some(value) = cond.refresh_token {
        builder.and_where_eq("refresh_token", quote(value));
    }
    if let Some(value) = cond.client_id {
        builder.and_where_eq("client_id", quote(value));
    }
    if let Some(value) = cond.user_id {
        builder.and_where_eq("user_id", quote(value));
    }
    builder
}
