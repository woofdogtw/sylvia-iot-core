use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sql_builder::{SqlBuilder, quote};
use sqlx::{AssertSqlSafe, SqlitePool};

use super::super::login_session::{LoginSession, LoginSessionModel, QueryCond};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<SqlitePool>,
}

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    session_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    expires_at: i64,
    user_id: String,
    client_id: String,
    redirect_uri: String,
}

const TABLE_NAME: &'static str = "login_session";
const FIELDS: &'static [&'static str] = &[
    "session_id",
    "expires_at",
    "user_id",
    "client_id",
    "redirect_uri",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS login_session (\
    session_id TEXT NOT NULL UNIQUE,\
    expires_at INTEGER NOT NULL,\
    user_id TEXT NOT NULL,\
    client_id TEXT NOT NULL,\
    redirect_uri TEXT NOT NULL,\
    PRIMARY KEY (session_id))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl LoginSessionModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let _ = sqlx::query(TABLE_INIT_SQL)
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn get(&self, session_id: &str) -> Result<Option<LoginSession>, Box<dyn StdError>> {
        let cond = QueryCond {
            session_id: Some(session_id),
            ..Default::default()
        };
        let sql = get_query_sql(SqlBuilder::select_from(TABLE_NAME).fields(FIELDS), &cond).sql()?;

        let result: Result<Schema, sqlx::Error> = sqlx::query_as(AssertSqlSafe(sql))
            .fetch_one(self.conn.as_ref())
            .await;

        let row = match result {
            Err(e) => match e {
                sqlx::Error::RowNotFound => return Ok(None),
                _ => return Err(Box::new(e)),
            },
            Ok(row) => row,
        };
        Ok(Some(LoginSession {
            session_id: row.session_id,
            expires_at: Utc.timestamp_nanos(row.expires_at * 1000000),
            user_id: row.user_id,
            client_id: row.client_id,
            redirect_uri: row.redirect_uri,
        }))
    }

    async fn add(&self, session: &LoginSession) -> Result<(), Box<dyn StdError>> {
        // SQLite has no TTL, so remove expired sessions here.
        let sql = SqlBuilder::delete_from(TABLE_NAME)
            .and_where_lt("expires_at", Utc::now().timestamp_millis())
            .sql()?;
        let _ = sqlx::query(AssertSqlSafe(sql))
            .execute(self.conn.as_ref())
            .await?;

        let values = vec![
            quote(session.session_id.as_str()),
            session.expires_at.timestamp_millis().to_string(),
            quote(session.user_id.as_str()),
            quote(session.client_id.as_str()),
            quote(session.redirect_uri.as_str()),
        ];
        let sql = SqlBuilder::insert_into(TABLE_NAME)
            .fields(FIELDS)
            .values(&values)
            .sql()?;
        let _ = sqlx::query(AssertSqlSafe(sql))
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }

    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>> {
        let sql = get_query_sql(&mut SqlBuilder::delete_from(TABLE_NAME), cond).sql()?;
        let _ = sqlx::query(AssertSqlSafe(sql))
            .execute(self.conn.as_ref())
            .await?;
        Ok(())
    }
}

/// Transforms query conditions to the SQL builder.
fn get_query_sql<'a>(builder: &'a mut SqlBuilder, cond: &QueryCond<'a>) -> &'a mut SqlBuilder {
    if let Some(value) = cond.session_id {
        builder.and_where_eq("session_id", quote(value));
    }
    if let Some(value) = cond.user_id {
        builder.and_where_eq("user_id", quote(value));
    }
    builder
}
