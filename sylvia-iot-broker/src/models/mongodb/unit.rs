use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Cursor as MongoDbCursor, Database,
    action::Find,
    bson::{self, DateTime, Document, Regex, doc},
};
use serde::{Deserialize, Serialize};

use super::super::unit::{
    Cursor, ListOptions, ListQueryCond, QueryCond, SortKey, Unit, UnitModel, UpdateQueryCond,
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
    #[serde(rename = "unitId")]
    unit_id: String,
    code: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    #[serde(rename = "ownerId")]
    owner_id: String,
    #[serde(rename = "memberIds")]
    member_ids: Vec<String>,
    name: String,
    info: Document,
}

const COL_NAME: &'static str = "unit";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl UnitModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "unitId_1", "key": {"unitId": 1}, "unique": true},
            doc! {"name": "code_1", "key": {"code": 1}, "unique": true},
            doc! {"name": "createdAt_1", "key": {"createdAt": 1}},
            doc! {"name": "modifiedAt_1", "key": {"modifiedAt": 1}},
            doc! {"name": "ownerId_1", "key": {"ownerId": 1}},
            doc! {"name": "memberIds_1", "key": {"memberIds": 1}},
            doc! {"name": "name_1", "key": {"name": 1}},
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
    ) -> Result<(Vec<Unit>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn get(&self, cond: &QueryCond) -> Result<Option<Unit>, Box<dyn StdError>> {
        let filter = get_query_filter(cond);
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter)
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(Unit {
                unit_id: item.unit_id,
                code: item.code,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                owner_id: item.owner_id,
                member_ids: item.member_ids,
                name: item.name,
                info: bson::from_document(item.info)?,
            }));
        }
        Ok(None)
    }

    async fn add(&self, unit: &Unit) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            unit_id: unit.unit_id.clone(),
            code: unit.code.clone(),
            created_at: unit.created_at.into(),
            modified_at: unit.modified_at.into(),
            owner_id: unit.owner_id.clone(),
            member_ids: unit.member_ids.clone(),
            name: unit.name.clone(),
            info: bson::to_document(&unit.info)?,
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

    async fn update(
        &self,
        cond: &UpdateQueryCond,
        updates: &Updates,
    ) -> Result<(), Box<dyn StdError>> {
        let filter = get_update_query_filter(cond);
        if let Some(updates) = get_update_doc(updates) {
            self.conn
                .collection::<Schema>(COL_NAME)
                .update_one(filter, updates)
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
    async fn try_next(&mut self) -> Result<Option<Unit>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(Unit {
                unit_id: item.unit_id,
                code: item.code,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                owner_id: item.owner_id,
                member_ids: item.member_ids,
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
    if let Some(value) = cond.code {
        filter.insert("code", value);
    }
    if let Some(value) = cond.owner_id {
        filter.insert("ownerId", value);
    }
    if let Some(value) = cond.member_id {
        filter.insert("memberIds", value);
    }
    filter
}

/// Transforms query conditions to the MongoDB document.
fn get_list_query_filter(cond: &ListQueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.owner_id {
        filter.insert("ownerId", value);
    }
    if let Some(value) = cond.member_id {
        filter.insert("memberIds", value);
    }
    if let Some(value) = cond.unit_id {
        filter.insert("unitId", value);
    }
    if let Some(value) = cond.code_contains {
        filter.insert(
            "code",
            Regex {
                pattern: value.to_string(),
                options: "i".to_string(),
            },
        );
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
                    SortKey::ModifiedAt => "modifiedAt",
                    SortKey::Code => "code",
                    SortKey::Name => "name",
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

/// Transforms query conditions to the MongoDB document.
fn get_update_query_filter(cond: &UpdateQueryCond) -> Document {
    doc! {"unitId": cond.unit_id}
}

/// Transforms the model object to the MongoDB document.
fn get_update_doc(updates: &Updates) -> Option<Document> {
    let mut count = 0;
    let mut document = Document::new();
    if let Some(value) = updates.modified_at.as_ref() {
        document.insert(
            "modifiedAt",
            DateTime::from_millis(value.timestamp_millis()),
        );
        count += 1;
    }
    if let Some(value) = updates.owner_id {
        document.insert("ownerId", value);
        count += 1;
    }
    if let Some(value) = updates.member_ids {
        document.insert("memberIds", value);
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
