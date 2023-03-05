use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime, Document},
    error::ErrorKind,
    options::{FindOptions, InsertManyOptions},
    Cursor as MongoDbCursor, Database,
};
use serde::{Deserialize, Serialize};

use super::super::device_route::{
    Cursor, DeviceRoute, DeviceRouteModel, ListOptions, ListQueryCond, QueryCond, SortKey,
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
    #[serde(rename = "routeId")]
    route_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "deviceRoute";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl DeviceRouteModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "routeId_1", "key": {"routeId": 1}, "unique": true},
            doc! {
                "name": "applicationId_1_deviceId_1",
                "key": {"applicationId": 1, "deviceId": 1},
                "unique": true
            },
            doc! {"name": "unitId_1", "key": {"unitId": 1}},
            doc! {"name": "applicationId_1", "key": {"applicationId": 1}},
            doc! {"name": "deviceId_1", "key": {"deviceId": 1}},
            doc! {"name": "networkId_1", "key": {"networkId": 1}},
            doc! {"name": "createdAt_1", "key": {"createdAt": 1}},
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
    ) -> Result<(Vec<DeviceRoute>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn get(&self, route_id: &str) -> Result<Option<DeviceRoute>, Box<dyn StdError>> {
        let filter = doc! {"routeId": route_id};
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter, None)
            .await?;
        if let Some(route) = cursor.try_next().await? {
            return Ok(Some(DeviceRoute {
                route_id: route.route_id,
                unit_id: route.unit_id,
                unit_code: route.unit_code,
                application_id: route.application_id,
                application_code: route.application_code,
                device_id: route.device_id,
                network_id: route.network_id,
                network_code: route.network_code,
                network_addr: route.network_addr,
                created_at: route.created_at.into(),
            }));
        }
        Ok(None)
    }

    async fn add(&self, route: &DeviceRoute) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            route_id: route.route_id.clone(),
            unit_id: route.unit_id.clone(),
            unit_code: route.unit_code.clone(),
            application_id: route.application_id.clone(),
            application_code: route.application_code.clone(),
            device_id: route.device_id.clone(),
            network_id: route.network_id.clone(),
            network_code: route.network_code.clone(),
            network_addr: route.network_addr.clone(),
            created_at: route.created_at.into(),
        };
        self.conn
            .collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await?;
        Ok(())
    }

    async fn add_bulk(&self, routes: &Vec<DeviceRoute>) -> Result<(), Box<dyn StdError>> {
        let mut items = vec![];
        for route in routes.iter() {
            items.push(Schema {
                route_id: route.route_id.clone(),
                unit_id: route.unit_id.clone(),
                unit_code: route.unit_code.clone(),
                application_id: route.application_id.clone(),
                application_code: route.application_code.clone(),
                device_id: route.device_id.clone(),
                network_id: route.network_id.clone(),
                network_code: route.network_code.clone(),
                network_addr: route.network_addr.clone(),
                created_at: route.created_at.into(),
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
}

impl DbCursor {
    /// To create the cursor instance with a collection cursor.
    pub fn new(cursor: MongoDbCursor<Schema>) -> Self {
        DbCursor { cursor, offset: 0 }
    }
}

#[async_trait]
impl Cursor for DbCursor {
    async fn try_next(&mut self) -> Result<Option<DeviceRoute>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(DeviceRoute {
                route_id: item.route_id,
                unit_id: item.unit_id,
                unit_code: item.unit_code,
                application_id: item.application_id,
                application_code: item.application_code,
                device_id: item.device_id,
                network_id: item.network_id,
                network_code: item.network_code,
                network_addr: item.network_addr,
                created_at: item.created_at.into(),
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
    if let Some(value) = cond.route_id {
        filter.insert("routeId", value);
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
    if let Some(value) = cond.device_id {
        filter.insert("deviceId", value);
    }
    if let Some(value) = cond.network_addrs {
        let mut in_cond = Document::new();
        in_cond.insert("$in", value);
        filter.insert("networkAddr", in_cond);
    }
    filter
}

/// Transforms query conditions to the MongoDB document.
fn get_list_query_filter(cond: &ListQueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.route_id {
        filter.insert("routeId", value);
    }
    if let Some(value) = cond.unit_id {
        filter.insert("unitId", value);
    }
    if let Some(value) = cond.unit_code {
        filter.insert("unitCode", value);
    }
    if let Some(value) = cond.application_id {
        filter.insert("applicationId", value);
    }
    if let Some(value) = cond.application_code {
        filter.insert("applicationCode", value);
    }
    if let Some(value) = cond.network_id {
        filter.insert("networkId", value);
    }
    if let Some(value) = cond.network_code {
        filter.insert("networkCode", value);
    }
    if let Some(value) = cond.network_addr {
        filter.insert("networkAddr", value);
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
                    SortKey::ApplicationCode => "applicationCode",
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
