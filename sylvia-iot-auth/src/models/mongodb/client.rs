use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Cursor as MongoDbCursor, Database,
    action::Find,
    bson::{Bson, DateTime, Document, Regex, doc, raw::CString},
};
use serde::{Deserialize, Serialize};

use super::super::client::{
    Client, ClientModel, Cursor, ListOptions, ListQueryCond, QueryCond, SortKey, UpdateQueryCond,
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
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    #[serde(rename = "clientSecret")]
    client_secret: Option<String>,
    #[serde(rename = "redirectUris")]
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
    #[serde(rename = "userId")]
    user_id: String,
    name: String,
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

const COL_NAME: &'static str = "client";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl ClientModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "clientId_1", "key": {"clientId": 1}, "unique": true},
            doc! {"name": "createdAt_1", "key": {"createdAt": 1}},
            doc! {"name": "modifiedAt_1", "key": {"modifiedAt": 1}},
            doc! {"name": "userId_1", "key": {"userId": 1}},
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
    ) -> Result<(Vec<Client>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn get(&self, cond: &QueryCond) -> Result<Option<Client>, Box<dyn StdError>> {
        let filter = get_query_filter(cond);
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter)
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(Client {
                client_id: item.client_id,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                client_secret: item.client_secret,
                redirect_uris: item.redirect_uris,
                scopes: item.scopes,
                user_id: item.user_id,
                name: item.name,
                image_url: item.image_url,
            }));
        }
        Ok(None)
    }

    async fn add(&self, client: &Client) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            client_id: client.client_id.clone(),
            created_at: client.created_at.into(),
            modified_at: client.modified_at.into(),
            client_secret: client.client_secret.clone(),
            redirect_uris: client.redirect_uris.clone(),
            scopes: client.scopes.clone(),
            user_id: client.user_id.clone(),
            name: client.name.clone(),
            image_url: client.image_url.clone(),
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
    async fn try_next(&mut self) -> Result<Option<Client>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(Client {
                client_id: item.client_id,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                client_secret: item.client_secret,
                redirect_uris: item.redirect_uris,
                scopes: item.scopes,
                user_id: item.user_id,
                name: item.name,
                image_url: item.image_url,
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
    if let Some(value) = cond.name_contains {
        if let Ok(pattern) = CString::try_from(value) {
            if let Ok(options) = CString::try_from("i") {
                filter.insert("name", Regex { pattern, options });
            }
        }
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
    doc! {
        "userId": cond.user_id,
        "clientId": cond.client_id,
    }
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
    if let Some(value) = updates.client_secret.as_ref() {
        match value {
            None => {
                document.insert("clientSecret", Bson::Null);
            }
            Some(value) => {
                document.insert("clientSecret", value);
            }
        }
        count += 1;
    }
    if let Some(value) = updates.redirect_uris {
        document.insert("redirectUris", value);
        count += 1;
    }
    if let Some(value) = updates.scopes {
        document.insert("scopes", value);
        count += 1;
    }
    if let Some(value) = updates.name {
        document.insert("name", value);
        count += 1;
    }
    if let Some(value) = updates.image_url.as_ref() {
        match value {
            None => {
                document.insert("imageUrl", Bson::Null);
            }
            Some(value) => {
                document.insert("imageUrl", value);
            }
        }
        count += 1;
    }
    if count == 0 {
        return None;
    }
    Some(doc! {"$set": document})
}
