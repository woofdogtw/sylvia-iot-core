use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, Bson, DateTime, Document},
    options::FindOptions,
    Cursor as MongoDbCursor, Database,
};
use serde::{Deserialize, Serialize};

use super::super::application_dldata::{
    ApplicationDlData, ApplicationDlDataModel, Cursor, ListOptions, ListQueryCond, QueryCond,
    SortKey, UpdateQueryCond, Updates, EXPIRES,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resp: Option<DateTime>,
    pub status: i32,
    #[serde(rename = "unitId")]
    pub unit_id: String,
    #[serde(rename = "deviceId", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(rename = "networkCode", skip_serializing_if = "Option::is_none")]
    pub network_code: Option<String>,
    #[serde(rename = "networkAddr", skip_serializing_if = "Option::is_none")]
    pub network_addr: Option<String>,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Document>,
}

const COL_NAME: &'static str = "applicationDlData";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl ApplicationDlDataModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "dataId_1", "key": {"dataId": 1}, "unique": true},
            doc! {"name": "status_1", "key": {"status": 1}},
            doc! {"name": "unitId_1", "key": {"unitId": 1}},
            doc! {"name": "deviceId_1", "key": {"deviceId": 1}},
            doc! {"name": "networkCode_1", "key": {"networkCode": 1}},
            doc! {"name": "networkAddr_1", "key": {"networkAddr": 1}},
            doc! {"name": "proc_1", "key": {"proc": 1}, "expireAfterSeconds": EXPIRES},
            doc! {"name": "resp_1", "key": {"resp": 1}},
        ];
        let command = doc! {
            "createIndexes": COL_NAME,
            "indexes": indexes,
        };
        self.conn.run_command(command, None).await?;
        Ok(())
    }

    async fn count(&self, cond: &ListQueryCond) -> Result<u64, Box<dyn StdError>> {
        let filter = get_list_query_filter(cond);
        let count = self
            .conn
            .collection::<Schema>(COL_NAME)
            .count_documents(filter, None)
            .await?;
        Ok(count)
    }

    async fn list(
        &self,
        opts: &ListOptions,
        cursor: Option<Box<dyn Cursor>>,
    ) -> Result<(Vec<ApplicationDlData>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
        let mut cursor = match cursor {
            None => {
                let filter = get_list_query_filter(opts.cond);
                let options = get_find_options(opts);
                Box::new(DbCursor::new(
                    self.conn
                        .collection::<Schema>(COL_NAME)
                        .find(filter, options)
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

    async fn add(&self, data: &ApplicationDlData) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            data_id: data.data_id.clone(),
            proc: data.proc.into(),
            resp: match data.resp {
                None => None,
                Some(resp) => Some(resp.into()),
            },
            status: data.status,
            unit_id: data.unit_id.clone(),
            device_id: data.device_id.clone(),
            network_code: data.network_code.clone(),
            network_addr: data.network_addr.clone(),
            data: data.data.clone(),
            extension: match data.extension.as_ref() {
                None => None,
                Some(extension) => Some(bson::to_document(extension)?),
            },
        };
        self.conn
            .collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await?;
        Ok(())
    }

    async fn del(&self, cond: &QueryCond) -> Result<(), Box<dyn StdError>> {
        let filter = get_query_filter(cond);
        self.conn
            .collection::<Schema>(COL_NAME)
            .delete_many(filter, None)
            .await?;
        Ok(())
    }

    async fn update(
        &self,
        cond: &UpdateQueryCond,
        updates: &Updates,
    ) -> Result<(), Box<dyn StdError>> {
        let filter = get_update_query_filter(cond, updates.status);
        if let Some(updates) = get_update_doc(updates) {
            self.conn
                .collection::<Schema>(COL_NAME)
                .update_one(filter, updates, None)
                .await?;
        }
        return Ok(());
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
    async fn try_next(&mut self) -> Result<Option<ApplicationDlData>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(ApplicationDlData {
                data_id: item.data_id,
                proc: item.proc.into(),
                resp: match item.resp {
                    None => None,
                    Some(resp) => Some(resp.into()),
                },
                status: item.status,
                unit_id: item.unit_id,
                device_id: item.device_id,
                network_code: item.network_code,
                network_addr: item.network_addr,
                data: item.data,
                extension: match item.extension {
                    None => None,
                    Some(extension) => Some(bson::from_document(extension)?),
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
    if let Some(value) = cond.network_code {
        filter.insert("networkCode", value);
    }
    if let Some(value) = cond.network_addr {
        filter.insert("networkAddr", value);
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
    if let Some(value) = cond.resp_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.resp_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("resp", time_doc);
    }
    filter
}

/// Transforms model options to the options.
fn get_find_options(opts: &ListOptions) -> FindOptions {
    let mut options = FindOptions::builder().build();
    if let Some(offset) = opts.offset {
        options.skip = Some(offset);
    }
    if let Some(limit) = opts.limit {
        if limit > 0 {
            options.limit = Some(limit as i64);
        }
    }
    if let Some(sort_list) = opts.sort.as_ref() {
        if sort_list.len() > 0 {
            let mut sort_opts = Document::new();
            for cond in sort_list.iter() {
                let key = match cond.key {
                    SortKey::Proc => "proc",
                    SortKey::Resp => "resp",
                    SortKey::NetworkCode => "networkCode",
                    SortKey::NetworkAddr => "networkAddr",
                };
                if cond.asc {
                    sort_opts.insert(key.to_string(), 1);
                } else {
                    sort_opts.insert(key.to_string(), -1);
                }
            }
            options.sort = Some(sort_opts);
        }
    }
    options
}

/// Transforms query conditions to the MongoDB document.
fn get_update_query_filter(cond: &UpdateQueryCond, status: i32) -> Document {
    let mut document = doc! {"dataId": cond.data_id};
    if status >= 0 {
        document.insert("status", doc! {"$ne": 0});
    } else {
        document.insert("status", doc! {"$lt": status});
    }
    document
}

/// Transforms the model object to the MongoDB document.
fn get_update_doc(updates: &Updates) -> Option<Document> {
    let document = doc! {
        "resp": DateTime::from_chrono(updates.resp),
        "status": updates.status,
    };
    Some(doc! {"$set": document})
}
