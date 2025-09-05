use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Cursor as MongoDbCursor, Database,
    action::Find,
    bson::{self, Bson, DateTime, Document, doc},
};
use serde::{Deserialize, Serialize};

use super::super::coremgr_opdata::{
    CoremgrOpData, CoremgrOpDataModel, Cursor, EXPIRES, ListOptions, ListQueryCond, QueryCond,
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
    #[serde(rename = "reqTime")]
    pub req_time: DateTime,
    #[serde(rename = "resTime")]
    pub res_time: DateTime,
    #[serde(rename = "latencyMs")]
    pub latency_ms: i64,
    pub status: i32,
    #[serde(rename = "sourceIp")]
    pub source_ip: String,
    pub method: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Document>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "errCode", skip_serializing_if = "Option::is_none")]
    pub err_code: Option<String>,
    #[serde(rename = "errMessage", skip_serializing_if = "Option::is_none")]
    pub err_message: Option<String>,
}

const COL_NAME: &'static str = "coremgrOpData";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl CoremgrOpDataModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "dataId_1", "key": {"dataId": 1}, "unique": true},
            doc! {"name": "userId_1", "key": {"userId": 1}},
            doc! {"name": "clientId_1", "key": {"clientId": 1}},
            doc! {"name": "reqTime_1", "key": {"reqTime": 1}, "expireAfterSeconds": EXPIRES},
            doc! {"name": "resTime_1", "key": {"resTime": 1}},
            doc! {"name": "latencyMs_1", "key": {"latencyMs": 1}},
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
    ) -> Result<(Vec<CoremgrOpData>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn add(&self, data: &CoremgrOpData) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            data_id: data.data_id.clone(),
            req_time: data.req_time.into(),
            res_time: data.res_time.into(),
            latency_ms: data.latency_ms,
            status: data.status,
            source_ip: data.source_ip.clone(),
            method: data.method.clone(),
            path: data.path.clone(),
            body: match data.body.as_ref() {
                None => None,
                Some(body) => Some(bson::serialize_to_document(body)?),
            },
            user_id: data.user_id.clone(),
            client_id: data.client_id.clone(),
            err_code: data.err_code.clone(),
            err_message: data.err_message.clone(),
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
    async fn try_next(&mut self) -> Result<Option<CoremgrOpData>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(CoremgrOpData {
                data_id: item.data_id,
                req_time: item.req_time.into(),
                res_time: item.res_time.into(),
                latency_ms: item.latency_ms,
                status: item.status,
                source_ip: item.source_ip,
                method: item.method,
                path: item.path,
                body: match item.body {
                    None => None,
                    Some(body) => bson::deserialize_from_document(body)?,
                },
                user_id: item.user_id,
                client_id: item.client_id,
                err_code: item.err_code,
                err_message: item.err_message,
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
    if let Some(value) = cond.user_id {
        filter.insert("userId", value);
    }
    if let Some(value) = cond.client_id {
        filter.insert("clientId", value);
    }
    let mut time_doc = Document::new();
    if let Some(value) = cond.req_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.req_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("reqTime", time_doc);
    }
    filter
}

/// Transforms query conditions to the MongoDB document.
fn get_list_query_filter(cond: &ListQueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.user_id {
        filter.insert("userId", value);
    }
    if let Some(value) = cond.client_id {
        filter.insert("clientId", value);
    }
    let mut time_doc = Document::new();
    if let Some(value) = cond.req_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.req_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("reqTime", time_doc);
    }
    time_doc = Document::new();
    if let Some(value) = cond.res_gte {
        time_doc.insert("$gte", Bson::DateTime(value.into()));
    }
    if let Some(value) = cond.res_lte {
        time_doc.insert("$lte", Bson::DateTime(value.into()));
    }
    if time_doc.len() > 0 {
        filter.insert("resTime", time_doc);
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
                    SortKey::ReqTime => "reqTime",
                    SortKey::ResTime => "resTime",
                    SortKey::Latency => "latencyMs",
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
