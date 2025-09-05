use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Cursor as MongoDbCursor, Database,
    action::Find,
    bson::{self, Bson, DateTime, Document, doc},
};
use serde::{Deserialize, Serialize};

use super::super::network_uldata::{
    Cursor, EXPIRES, ListOptions, ListQueryCond, NetworkUlData, NetworkUlDataModel, QueryCond,
    SortKey,
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
    pub data_id: String,
    pub proc: DateTime,
    #[serde(rename = "unitCode")]
    pub unit_code: Option<String>,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    #[serde(rename = "unitId", skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub time: DateTime,
    pub profile: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Document>,
}

const COL_NAME: &'static str = "networkUlData";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl NetworkUlDataModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "dataId_1", "key": {"dataId": 1}, "unique": true},
            doc! {"name": "unitId_1", "key": {"unitId": 1}},
            doc! {"name": "deviceId_1", "key": {"deviceId": 1}},
            doc! {"name": "networkCode_1", "key": {"networkCode": 1}},
            doc! {"name": "networkAddr_1", "key": {"networkAddr": 1}},
            doc! {"name": "proc_1", "key": {"proc": 1}, "expireAfterSeconds": EXPIRES},
            doc! {"name": "time_1", "key": {"time": 1}},
            doc! {"name": "profile_1", "key": {"profile": 1}},
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
    ) -> Result<(Vec<NetworkUlData>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn add(&self, data: &NetworkUlData) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            data_id: data.data_id.clone(),
            proc: data.proc.into(),
            unit_code: data.unit_code.clone(),
            network_code: data.network_code.clone(),
            network_addr: data.network_addr.clone(),
            unit_id: data.unit_id.clone(),
            device_id: data.device_id.clone(),
            time: data.time.into(),
            profile: data.profile.clone(),
            data: data.data.clone(),
            extension: match data.extension.as_ref() {
                None => None,
                Some(extension) => Some(bson::serialize_to_document(extension)?),
            },
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
    async fn try_next(&mut self) -> Result<Option<NetworkUlData>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(NetworkUlData {
                data_id: item.data_id,
                proc: item.proc.into(),
                unit_code: item.unit_code,
                network_code: item.network_code,
                network_addr: item.network_addr,
                unit_id: item.unit_id,
                device_id: item.device_id,
                time: item.time.into(),
                profile: item.profile,
                data: item.data,
                extension: match item.extension {
                    None => None,
                    Some(extension) => Some(bson::deserialize_from_document(extension)?),
                },
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
    if let Some(value) = cond.unit_id {
        filter.insert("unitId", value);
    }
    if let Some(value) = cond.device_id {
        filter.insert("deviceId", value);
    }
    let mut time_doc = Document::new();
    if let Some(value) = cond.proc_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.proc_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("proc", time_doc);
    }
    filter
}

/// Transforms query conditions to the MongoDB document.
fn get_list_query_filter(cond: &ListQueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.unit_id {
        filter.insert("unitId", value);
    }
    if let Some(value) = cond.device_id {
        filter.insert("deviceId", value);
    }
    if let Some(value) = cond.network_code {
        filter.insert("networkCode", value);
    }
    if let Some(value) = cond.network_addr {
        filter.insert("networkAddr", value);
    }
    if let Some(value) = cond.profile {
        filter.insert("profile", value);
    }
    let mut time_doc = Document::new();
    if let Some(value) = cond.proc_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.proc_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("proc", time_doc);
    }
    time_doc = Document::new();
    if let Some(value) = cond.time_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.time_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("time", time_doc);
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
                    SortKey::Proc => "proc",
                    SortKey::Time => "time",
                    SortKey::NetworkCode => "networkCode",
                    SortKey::NetworkAddr => "networkAddr",
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
