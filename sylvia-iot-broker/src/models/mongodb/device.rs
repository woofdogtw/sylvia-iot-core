use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, Bson, DateTime, Document, Regex},
    error::ErrorKind,
    options::{FindOptions, InsertManyOptions},
    Cursor as MongoDbCursor, Database,
};
use serde::{Deserialize, Serialize};

use super::super::device::{
    Cursor, Device, DeviceModel, ListOptions, ListQueryCond, QueryCond, SortKey, UpdateQueryCond,
    Updates,
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
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    profile: String,
    name: String,
    info: Document,
}

const COL_NAME: &'static str = "device";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl DeviceModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "deviceId_1", "key": {"deviceId": 1}, "unique": true},
            doc! {
                "name": "unitCode_1_networkCode_1_networkAddr_1",
                "key": {"unitCode": 1, "networkCode": 1, "networkAddr": 1},
                "unique": true
            },
            doc! {"name": "unitId_1", "key": {"unitId": 1}},
            doc! {"name": "networkId_1", "key": {"networkId": 1}},
            doc! {"name": "unitCode_1", "key": {"unitCode": 1}},
            doc! {"name": "networkCode_1", "key": {"networkCode": 1}},
            doc! {"name": "networkAddr_1", "key": {"networkAddr": 1}},
            doc! {"name": "createdAt_1", "key": {"createdAt": 1}},
            doc! {"name": "modifiedAt_1", "key": {"modifiedAt": 1}},
            doc! {"name": "profile_1", "key": {"profile": 1}},
            doc! {"name": "name_1", "key": {"name": 1}},
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
    ) -> Result<(Vec<Device>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn get(&self, cond: &QueryCond) -> Result<Option<Device>, Box<dyn StdError>> {
        let filter = get_query_filter(cond);
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter, None)
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(Device {
                device_id: item.device_id,
                unit_id: item.unit_id,
                unit_code: item.unit_code,
                network_id: item.network_id,
                network_code: item.network_code,
                network_addr: item.network_addr,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                profile: item.profile,
                name: item.name,
                info: bson::from_document(item.info)?,
            }));
        }
        Ok(None)
    }

    async fn add(&self, device: &Device) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            device_id: device.device_id.clone(),
            unit_id: device.unit_id.clone(),
            unit_code: device.unit_code.clone(),
            network_id: device.network_id.clone(),
            network_code: device.network_code.clone(),
            network_addr: device.network_addr.clone(),
            created_at: device.created_at.into(),
            modified_at: device.modified_at.into(),
            profile: device.profile.clone(),
            name: device.name.clone(),
            info: bson::to_document(&device.info)?,
        };
        self.conn
            .collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await?;
        Ok(())
    }

    async fn add_bulk(&self, devices: &Vec<Device>) -> Result<(), Box<dyn StdError>> {
        let mut items = vec![];
        for device in devices.iter() {
            items.push(Schema {
                device_id: device.device_id.clone(),
                unit_id: device.unit_id.clone(),
                unit_code: device.unit_code.clone(),
                network_id: device.network_id.clone(),
                network_code: device.network_code.clone(),
                network_addr: device.network_addr.clone(),
                created_at: device.created_at.into(),
                modified_at: device.modified_at.into(),
                profile: device.profile.clone(),
                name: device.name.clone(),
                info: bson::to_document(&device.info)?,
            });
        }
        let opts = InsertManyOptions::builder().ordered(Some(false)).build();
        if let Err(e) = self
            .conn
            .collection::<Schema>(COL_NAME)
            .insert_many(items, Some(opts))
            .await
        {
            match e.kind.as_ref() {
                ErrorKind::BulkWrite(_) => (),
                _ => return Err(Box::new(e)),
            }
        }

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
        let filter = get_update_query_filter(cond);
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
    async fn try_next(&mut self) -> Result<Option<Device>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(Device {
                device_id: item.device_id,
                unit_id: item.unit_id,
                unit_code: item.unit_code,
                network_id: item.network_id,
                network_code: item.network_code,
                network_addr: item.network_addr,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                profile: item.profile,
                name: item.name,
                info: bson::from_document(item.info)?,
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
    if let Some(value) = cond.network_id {
        filter.insert("networkId", value);
    }
    if let Some(value) = cond.network_addrs {
        let mut in_cond = Document::new();
        in_cond.insert("$in", value);
        filter.insert("networkAddr", in_cond);
    }
    if let Some(value) = cond.device.as_ref() {
        if let Some(unit_code) = value.unit_code {
            filter.insert("unitCode", unit_code);
        } else {
            filter.insert("unitCode", Bson::Null);
        }
        filter.insert("networkCode", value.network_code);
        filter.insert("networkAddr", value.network_addr);
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
    if let Some(value) = cond.network_id {
        filter.insert("networkId", value);
    }
    if let Some(value) = cond.network_code {
        filter.insert("networkCode", value);
    }
    if let Some(value) = cond.network_addr {
        filter.insert("networkAddr", value);
    } else if let Some(value) = cond.network_addrs {
        let mut in_cond = Document::new();
        in_cond.insert("$in", value);
        filter.insert("networkAddr", in_cond);
    }
    if let Some(value) = cond.profile {
        filter.insert("profile", value);
    }
    if let Some(value) = cond.name_contains {
        filter.insert(
            "name",
            Regex {
                pattern: value.to_string(),
                options: "i".to_string(),
            },
        );
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
                    SortKey::CreatedAt => "createdAt",
                    SortKey::ModifiedAt => "modifiedAt",
                    SortKey::NetworkCode => "networkCode",
                    SortKey::NetworkAddr => "networkAddr",
                    SortKey::Profile => "profile",
                    SortKey::Name => "name",
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
fn get_update_query_filter(cond: &UpdateQueryCond) -> Document {
    doc! {"deviceId": cond.device_id}
}

/// Transforms the model object to the MongoDB document.
fn get_update_doc(updates: &Updates) -> Option<Document> {
    let mut count = 0;
    let mut document = Document::new();
    if let Some((network_id, network_code)) = updates.network {
        document.insert("networkId", network_id);
        document.insert("networkCode", network_code);
        count += 1;
    }
    if let Some(value) = updates.network_addr {
        document.insert("networkAddr", value);
        count += 1;
    }
    if let Some(value) = updates.modified_at.as_ref() {
        document.insert(
            "modifiedAt",
            DateTime::from_millis(value.timestamp_millis()),
        );
        count += 1;
    }
    if let Some(value) = updates.profile {
        document.insert("profile", value);
        count += 1;
    }
    if let Some(value) = updates.name {
        document.insert("name", value);
        count += 1;
    }
    if let Some(value) = updates.info {
        document.insert(
            "info",
            match bson::to_document(value) {
                Err(_) => return None,
                Ok(doc) => doc,
            },
        );
        count += 1;
    }
    if count == 0 {
        return None;
    }
    Some(doc! {"$set": document})
}
