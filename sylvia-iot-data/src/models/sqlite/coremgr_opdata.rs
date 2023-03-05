use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::TryStreamExt;
use sql_builder::{quote, SqlBuilder};
use sqlx::SqlitePool;

use super::super::coremgr_opdata::{
    CoremgrOpData, CoremgrOpDataModel, Cursor, ListOptions, ListQueryCond, QueryCond, SortKey,
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
    pub data_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    pub req_time: i64,
    /// i64 as time tick from Epoch in milliseconds.
    pub res_time: i64,
    pub latency_ms: i64,
    pub status: i32,
    pub source_ip: String,
    pub method: String,
    pub path: String,
    /// use empty string as NULL.
    pub body: String,
    pub user_id: String,
    pub client_id: String,
    /// use empty string as NULL.
    pub err_code: String,
    /// use empty string as NULL.
    pub err_message: String,
}

/// Use "COUNT(*)" instead of "COUNT(fields...)" to simplify the implementation.
#[derive(sqlx::FromRow)]
struct CountSchema {
    #[sqlx(rename = "COUNT(*)")]
    count: i64,
}

const TABLE_NAME: &'static str = "coremgr_opdata";
const FIELDS: &'static [&'static str] = &[
    "data_id",
    "req_time",
    "res_time",
    "latency_ms",
    "status",
    "source_ip",
    "method",
    "path",
    "body",
    "user_id",
    "client_id",
    "err_code",
    "err_message",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS coremgr_opdata (\
    data_id TEXT NOT NULL UNIQUE,\
    req_time INTEGER NOT NULL,\
    res_time INTEGER NOT NULL,\
    latency_ms INTEGER NOT NULL,\
    status INTEGER NOT NULL,\
    source_ip TEXT NOT NULL,\
    method TEXT NOT NULL,\
    path TEXT NOT NULL,\
    body TEXT NOT NULL,\
    user_id TEXT NOT NULL,\
    client_id TEXT NOT NULL,\
    err_code TEXT NOT NULL,\
    err_message TEXT NOT NULL)";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl CoremgrOpDataModel for Model {
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
    ) -> Result<(Vec<CoremgrOpData>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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
            list.push(CoremgrOpData {
                data_id: row.data_id,
                req_time: Utc.timestamp_nanos(row.req_time * 1000000),
                res_time: Utc.timestamp_nanos(row.res_time * 1000000),
                latency_ms: row.latency_ms,
                status: row.status,
                source_ip: row.source_ip,
                method: row.method,
                path: row.path,
                body: match row.body.len() {
                    0 => None,
                    _ => Some(serde_json::from_str(row.body.as_str())?),
                },
                user_id: row.user_id,
                client_id: row.client_id,
                err_code: match row.err_code.len() {
                    0 => None,
                    _ => Some(row.err_code),
                },
                err_message: match row.err_message.len() {
                    0 => None,
                    _ => Some(row.err_message),
                },
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

    async fn add(&self, data: &CoremgrOpData) -> Result<(), Box<dyn StdError>> {
        let body = match data.body.as_ref() {
            None => quote(""),
            Some(body) => match serde_json::to_string(body) {
                Err(_) => quote("{}"),
                Ok(value) => quote(value.as_str()),
            },
        };
        let err_code = match data.err_code.as_deref() {
            None => quote(""),
            Some(value) => quote(value),
        };
        let err_message = match data.err_message.as_deref() {
            None => quote(""),
            Some(value) => quote(value),
        };
        let values = vec![
            quote(data.data_id.as_str()),
            data.req_time.timestamp_millis().to_string(),
            data.res_time.timestamp_millis().to_string(),
            data.latency_ms.to_string(),
            data.status.to_string(),
            quote(data.source_ip.as_str()),
            quote(data.method.as_str()),
            quote(data.path.as_str()),
            body,
            quote(data.user_id.as_str()),
            quote(data.client_id.as_str()),
            err_code,
            err_message,
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
}

impl DbCursor {
    /// To create the cursor instance.
    pub fn new() -> Self {
        DbCursor { offset: 0 }
    }
}

#[async_trait]
impl Cursor for DbCursor {
    async fn try_next(&mut self) -> Result<Option<CoremgrOpData>, Box<dyn StdError>> {
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
    if let Some(value) = cond.req_gte {
        builder.and_where_ge("req_time", value.timestamp_millis());
    }
    if let Some(value) = cond.req_lte {
        builder.and_where_le("req_time", value.timestamp_millis());
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
    if let Some(value) = cond.req_gte {
        builder.and_where_ge("req_time", value.timestamp_millis());
    }
    if let Some(value) = cond.req_lte {
        builder.and_where_le("req_time", value.timestamp_millis());
    }
    if let Some(value) = cond.res_gte {
        builder.and_where_ge("res_time", value.timestamp_millis());
    }
    if let Some(value) = cond.res_lte {
        builder.and_where_le("res_time", value.timestamp_millis());
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
                SortKey::ReqTime => "req_time",
                SortKey::ResTime => "res_time",
                SortKey::Latency => "latency_ms",
            };
            builder.order_by(key, !cond.asc);
        }
    }
    builder
}
