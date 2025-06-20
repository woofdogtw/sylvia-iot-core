use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Cursor as MongoDbCursor, Database,
    action::Find,
    bson::{DateTime, Document, doc},
};
use serde::{Deserialize, Serialize};

use super::super::dldata_buffer::{
    Cursor, DlDataBuffer, DlDataBufferModel, ListOptions, ListQueryCond, QueryCond, SortKey,
};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<Database>,
}

/// Cursor instance.
struct DbCursor {
    /// The associated collection cursor.
    cursor: MongoDbCursor<Schema>,
    /// (Useless) only for Cursor trait implementation.
    offset: u64,
}

/// MongoDB schema.
#[derive(Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "expiredAt")]
    expired_at: DateTime,
}

const COL_NAME: &'static str = "dldataBuffer";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl DlDataBufferModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "dataId_1", "key": {"dataId": 1}, "unique": true},
            doc! {"name": "unitId_1", "key": {"unitId": 1}},
            doc! {"name": "applicationId_1", "key": {"applicationId": 1}},
            doc! {"name": "applicationCode", "key": {"applicationCode": 1}},
            doc! {"name": "networkId_1", "key": {"networkId": 1}},
            doc! {"name": "networkAddr_1", "key": {"networkAddr": 1}},
            doc! {"name": "deviceId_1", "key": {"deviceId": 1}},
            doc! {"name": "createdAt_1", "key": {"createdAt": 1}},
            doc! {"name": "expiredAt_1", "key": {"expiredAt": 1}},
        ];
        let command = doc! {
            "createIndexes": COL_NAME,
            "indexes": indexes,
        };
        self.conn.run_command(command).await?;
        Ok(())
    }

    async fn count(&self, cond: &ListQueryCond) -> Result<u64, Box<dyn StdError>> {
        let filter = get_list_query_filter(cond);
        let count = self
            .conn
            .collection::<Schema>(COL_NAME)
            .count_documents(filter)
            .await?;
        Ok(count)
    }

    async fn list(
        &self,
        opts: &ListOptions,
        cursor: Option<Box<dyn Cursor>>,
    ) -> Result<(Vec<DlDataBuffer>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
        let mut cursor = match cursor {
            None => {
                let filter = get_list_query_filter(opts.cond);
                Box::new(DbCursor::new(
                    build_find_options(opts, self.conn.collection::<Schema>(COL_NAME).find(filter))
                        .await?,
                ))
            }
            Some(cursor) => cursor,
        };

        let mut count: u64 = 0;
        let mut list = Vec::new();
        while let Some(item) = cursor.try_next().await? {
            list.push(item);
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
        let filter = doc! {"dataId": data_id};
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter)
            .await?;
        if let Some(data) = cursor.try_next().await? {
            return Ok(Some(DlDataBuffer {
                data_id: data.data_id,
                unit_id: data.unit_id,
                unit_code: data.unit_code,
                application_id: data.application_id,
                application_code: data.application_code,
                network_id: data.network_id,
                network_addr: data.network_addr,
                device_id: data.device_id,
                created_at: data.created_at.into(),
                expired_at: data.expired_at.into(),
            }));
        }
        Ok(None)
    }

    async fn add(&self, dldata: &DlDataBuffer) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            data_id: dldata.data_id.clone(),
            unit_id: dldata.unit_id.clone(),
            unit_code: dldata.unit_code.clone(),
            application_id: dldata.application_id.clone(),
            application_code: dldata.application_code.clone(),
            network_id: dldata.network_id.clone(),
            network_addr: dldata.network_addr.clone(),
            device_id: dldata.device_id.clone(),
            created_at: dldata.created_at.into(),
            expired_at: dldata.expired_at.into(),
        };
        self.conn
            .collection::<Schema>(COL_NAME)
            .insert_one(item)
            .await?;
        Ok(())
    }

    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>> {
        let filter = get_query_filter(cond);
        self.conn
            .collection::<Schema>(COL_NAME)
            .delete_many(filter)
            .await?;
        Ok(())
    }
}

impl DbCursor {
    /// To create the cursor instance with a collection cursor.
    pub fn new(cursor: MongoDbCursor<Schema>) -> Self {
        DbCursor { cursor, offset: 0 }
    }
}

#[async_trait]
impl Cursor for DbCursor {
    async fn try_next(&mut self) -> Result<Option<DlDataBuffer>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(DlDataBuffer {
                data_id: item.data_id,
                unit_id: item.unit_id,
                unit_code: item.unit_code,
                application_id: item.application_id,
                application_code: item.application_code,
                network_id: item.network_id,
                network_addr: item.network_addr,
                device_id: item.device_id,
                created_at: item.created_at.into(),
                expired_at: item.expired_at.into(),
            }));
        }
        Ok(None)
    }

    fn offset(&self) -> u64 {
        self.offset
    }
}

/// Transforms query conditions to the MongoDB document.
fn get_query_filter(cond: &QueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.data_id {
        filter.insert("dataId", value);
    }
    if let Some(value) = cond.unit_id {
        filter.insert("unitId", value);
    }
    if let Some(value) = cond.application_id {
        filter.insert("applicationId", value);
    }
    if let Some(value) = cond.network_id {
        filter.insert("networkId", value);
    }
    if let Some(value) = cond.network_addrs {
        let mut in_cond = Document::new();
        in_cond.insert("$in", value);
        filter.insert("networkAddr", in_cond);
    }
    if let Some(value) = cond.device_id {
        filter.insert("deviceId", value);
    }
    filter
}

/// Transforms query conditions to the MongoDB document.
fn get_list_query_filter(cond: &ListQueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.unit_id {
        filter.insert("unitId", value);
    }
    if let Some(value) = cond.application_id {
        filter.insert("applicationId", value);
    }
    if let Some(value) = cond.network_id {
        filter.insert("networkId", value);
    }
    if let Some(value) = cond.device_id {
        filter.insert("deviceId", value);
    }
    filter
}

/// Transforms model options to the options.
fn build_find_options<'a, T>(opts: &ListOptions, mut find: Find<'a, T>) -> Find<'a, T>
where
    T: Send + Sync,
{
    if let Some(offset) = opts.offset {
        find = find.skip(offset);
    }
    if let Some(limit) = opts.limit {
        if limit > 0 {
            find = find.limit(limit as i64);
        }
    }
    if let Some(sort_list) = opts.sort.as_ref() {
        if sort_list.len() > 0 {
            let mut sort_opts = Document::new();
            for cond in sort_list.iter() {
                let key = match cond.key {
                    SortKey::CreatedAt => "createdAt",
                    SortKey::ExpiredAt => "expiredAt",
                    SortKey::ApplicationCode => "applicationCode",
                };
                if cond.asc {
                    sort_opts.insert(key.to_string(), 1);
                } else {
                    sort_opts.insert(key.to_string(), -1);
                }
            }
            find = find.sort(sort_opts);
        }
    }
    find
}
