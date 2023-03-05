use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::TryStreamExt;
use sql_builder::{quote, SqlBuilder};
use sqlx::SqlitePool;

use super::super::dldata_buffer::{
    Cursor, DlDataBuffer, DlDataBufferModel, ListOptions, ListQueryCond, QueryCond, SortKey,
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
    data_id: String,
    unit_id: String,
    unit_code: String,
    application_id: String,
    application_code: String,
    network_id: String,
    network_addr: String,
    device_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    created_at: i64,
    /// i64 as time tick from Epoch in milliseconds.
    expired_at: i64,
}

/// Use "COUNT(*)" instead of "COUNT(fields...)" to simplify the implementation.
#[derive(sqlx::FromRow)]
struct CountSchema {
    #[sqlx(rename = "COUNT(*)")]
    count: i64,
}

const TABLE_NAME: &'static str = "dldata_buffer";
const FIELDS: &'static [&'static str] = &[
    "data_id",
    "unit_id",
    "unit_code",
    "application_id",
    "application_code",
    "network_id",
    "network_addr",
    "device_id",
    "created_at",
    "expired_at",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS dldata_buffer (\
    data_id TEXT NOT NULL UNIQUE,\
    unit_id TEXT NOT NULL,\
    unit_code TEXT NOT NULL,\
    application_id TEXT NOT NULL,\
    application_code TEXT NOT NULL,\
    network_id TEXT NOT NULL,\
    network_addr TEXT NOT NULL,\
    device_id TEXT NOT NULL,\
    created_at INTEGER NOT NULL,\
    expired_at INTEGER NOT NULL,\
    PRIMARY KEY (data_id))";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<SqlitePool>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl DlDataBufferModel for Model {
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
    ) -> Result<(Vec<DlDataBuffer>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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
            list.push(DlDataBuffer {
                data_id: row.data_id,
                unit_id: row.unit_id,
                unit_code: row.unit_code,
                application_id: row.application_id,
                application_code: row.application_code,
                network_id: row.network_id,
                network_addr: row.network_addr,
                device_id: row.device_id,
                created_at: Utc.timestamp_nanos(row.created_at * 1000000),
                expired_at: Utc.timestamp_nanos(row.expired_at * 1000000),
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

    async fn get(&self, data_id: &str) -> Result<Option<DlDataBuffer>, Box<dyn StdError>> {
        let sql = SqlBuilder::select_from(TABLE_NAME)
            .fields(FIELDS)
            .and_where_eq("data_id", quote(data_id))
            .sql()?;

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

        Ok(Some(DlDataBuffer {
            data_id: row.data_id,
            unit_id: row.unit_id,
            unit_code: row.unit_code,
            application_id: row.application_id,
            application_code: row.application_code,
            network_id: row.network_id,
            network_addr: row.network_addr,
            device_id: row.device_id,
            created_at: Utc.timestamp_nanos(row.created_at * 1000000),
            expired_at: Utc.timestamp_nanos(row.expired_at * 1000000),
        }))
    }

    async fn add(&self, dldata: &DlDataBuffer) -> Result<(), Box<dyn StdError>> {
        let values = vec![
            quote(dldata.data_id.as_str()),
            quote(dldata.unit_id.as_str()),
            quote(dldata.unit_code.as_str()),
            quote(dldata.application_id.as_str()),
            quote(dldata.application_code.as_str()),
            quote(dldata.network_id.as_str()),
            quote(dldata.network_addr.as_str()),
            quote(dldata.device_id.as_str()),
            dldata.created_at.timestamp_millis().to_string(),
            dldata.expired_at.timestamp_millis().to_string(),
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
    async fn try_next(&mut self) -> Result<Option<DlDataBuffer>, Box<dyn StdError>> {
        self.offset += 1;
        Ok(None)
    }

    fn offset(&self) -> u64 {
        self.offset
    }
}

/// Transforms query conditions to the SQL builder.
fn build_where<'a>(builder: &'a mut SqlBuilder, cond: &QueryCond<'a>) -> &'a mut SqlBuilder {
    if let Some(value) = cond.data_id {
        builder.and_where_eq("data_id", quote(value));
    }
    if let Some(value) = cond.unit_id {
        builder.and_where_eq("unit_id", quote(value));
    }
    if let Some(value) = cond.application_id {
        builder.and_where_eq("application_id", quote(value));
    }
    if let Some(value) = cond.network_id {
        builder.and_where_eq("network_id", quote(value));
    }
    if let Some(value) = cond.network_addrs {
        let values: Vec<String> = value.iter().map(|&x| quote(x)).collect();
        builder.and_where_in("network_addr", &values);
    }
    if let Some(value) = cond.device_id {
        builder.and_where_eq("device_id", quote(value));
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
    if let Some(value) = cond.network_id {
        builder.and_where_eq("network_id", quote(value));
    }
    if let Some(value) = cond.device_id {
        builder.and_where_eq("device_id", quote(value));
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
                SortKey::ExpiredAt => "expired_at",
                SortKey::ApplicationCode => "application_code",
            };
            builder.order_by(key, !cond.asc);
        }
    }
    builder
}
