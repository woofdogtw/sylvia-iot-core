use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::TryStreamExt;
use sql_builder::{SqlBuilder, quote};
use sqlx::SqlitePool;

use super::super::network_dldata::{
    Cursor, ListOptions, ListQueryCond, NetworkDlData, NetworkDlDataModel, QueryCond, SortKey,
    UpdateQueryCond, Updates,
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
    pub proc: i64,
    /// i64 as time tick from Epoch in milliseconds.
    #[sqlx(rename = "pub")]
    pub publish: i64,
    /// i64 as time tick from Epoch in milliseconds.
    pub resp: Option<i64>,
    pub status: i32,
    pub unit_id: String,
    /// use empty string as NULL.
    pub device_id: String,
    /// use empty string as NULL.
    pub network_code: String,
    /// use empty string as NULL.
    pub network_addr: String,
    pub profile: String,
    pub data: String,
    /// use empty string as NULL.
    pub extension: String,
}

/// Use "COUNT(*)" instead of "COUNT(fields...)" to simplify the implementation.
#[derive(sqlx::FromRow)]
struct CountSchema {
    #[sqlx(rename = "COUNT(*)")]
    count: i64,
}

const TABLE_NAME: &'static str = "network_dldata";
const FIELDS: &'static [&'static str] = &[
    "data_id",
    "proc",
    "pub",
    "resp",
    "status",
    "unit_id",
    "device_id",
    "network_code",
    "network_addr",
    "profile",
    "data",
    "extension",
];
const TABLE_INIT_SQL: &'static str = "\
    CREATE TABLE IF NOT EXISTS network_dldata (\
    data_id TEXT NOT NULL UNIQUE,\
    proc INTEGER NOT NULL,\
    pub INTEGER NOT NULL,\
    resp INTEGER,\
    status INTEGER NOT NULL,\
    unit_id TEXT NOT NULL,\
    device_id TEXT NOT NULL,\
    network_code TEXT NOT NULL,\
    network_addr TEXT NOT NULL,\
    profile TEXT NOT NULL,\
    data TEXT NOT NULL,\
    extension TEXT NOT NULL,\
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
impl NetworkDlDataModel for Model {
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
    ) -> Result<(Vec<NetworkDlData>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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
            list.push(NetworkDlData {
                data_id: row.data_id,
                proc: Utc.timestamp_nanos(row.proc * 1000000),
                publish: Utc.timestamp_nanos(row.publish * 1000000),
                resp: match row.resp {
                    None => None,
                    Some(resp) => Some(Utc.timestamp_nanos(resp * 1000000)),
                },
                status: row.status,
                unit_id: row.unit_id,
                device_id: row.device_id,
                network_code: row.network_code,
                network_addr: row.network_addr,
                profile: row.profile,
                data: row.data,
                extension: match row.extension.len() {
                    0 => None,
                    _ => serde_json::from_str(row.extension.as_str())?,
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

    async fn add(&self, data: &NetworkDlData) -> Result<(), Box<dyn StdError>> {
        let extension = match data.extension.as_ref() {
            None => quote(""),
            Some(extension) => match serde_json::to_string(extension) {
                Err(_) => quote("{}"),
                Ok(value) => quote(value.as_str()),
            },
        };
        let values = vec![
            quote(data.data_id.as_str()),
            data.proc.timestamp_millis().to_string(),
            data.publish.timestamp_millis().to_string(),
            match data.resp {
                None => "NULL".to_string(),
                Some(resp) => resp.timestamp_millis().to_string(),
            },
            data.status.to_string(),
            quote(data.unit_id.as_str()),
            quote(data.device_id.as_str()),
            quote(data.network_code.as_str()),
            quote(data.network_addr.as_str()),
            quote(data.profile.as_str()),
            quote(data.data.as_str()),
            extension,
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
    async fn try_next(&mut self) -> Result<Option<NetworkDlData>, Box<dyn StdError>> {
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
    if let Some(value) = cond.device_id {
        builder.and_where_eq("device_id", quote(value));
    }
    if let Some(value) = cond.proc_gte {
        builder.and_where_ge("proc", value.timestamp_millis());
    }
    if let Some(value) = cond.proc_lte {
        builder.and_where_le("proc", value.timestamp_millis());
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
    if let Some(value) = cond.device_id {
        builder.and_where_eq("device_id", quote(value));
    }
    if let Some(value) = cond.network_code {
        builder.and_where_eq("network_code", quote(value));
    }
    if let Some(value) = cond.network_addr {
        builder.and_where_eq("network_addr", quote(value));
    }
    if let Some(value) = cond.profile {
        builder.and_where_eq("profile", quote(value));
    }
    if let Some(value) = cond.proc_gte {
        builder.and_where_ge("proc", value.timestamp_millis());
    }
    if let Some(value) = cond.proc_lte {
        builder.and_where_le("proc", value.timestamp_millis());
    }
    if let Some(value) = cond.pub_gte {
        builder.and_where_ge("pub", value.timestamp_millis());
    }
    if let Some(value) = cond.pub_lte {
        builder.and_where_le("pub", value.timestamp_millis());
    }
    if let Some(value) = cond.resp_gte {
        builder.and_where_ge("resp", value.timestamp_millis());
    }
    if let Some(value) = cond.resp_lte {
        builder.and_where_le("resp", value.timestamp_millis());
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
                SortKey::Proc => "proc",
                SortKey::Pub => "pub",
                SortKey::Resp => "resp",
                SortKey::NetworkCode => "network_code",
                SortKey::NetworkAddr => "network_addr",
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
    builder.set("resp", updates.resp.timestamp_millis());
    builder.set("status", updates.status);
    builder.and_where_eq("data_id", quote(cond.data_id));
    if updates.status >= 0 {
        builder.and_where_ne("status", 0);
    } else {
        builder.and_where_lt("status", updates.status);
    }
    Some(builder)
}
