use std::{collections::HashMap, error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, Bson, DateTime, Document, Regex},
    options::FindOptions,
    Cursor as MongoDbCursor, Database,
};
use serde::{Deserialize, Serialize};

use super::super::user::{
    Cursor, ListOptions, ListQueryCond, QueryCond, SortKey, Updates, User, UserModel,
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
    #[serde(rename = "userId")]
    user_id: String,
    account: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    #[serde(rename = "verifiedAt")]
    verified_at: Option<DateTime>,
    #[serde(rename = "expiredAt")]
    expired_at: Option<DateTime>,
    #[serde(rename = "disabledAt")]
    disabled_at: Option<DateTime>,
    roles: HashMap<String, bool>,
    password: String,
    salt: String,
    name: String,
    info: Document,
}

const COL_NAME: &'static str = "user";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl UserModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "userId_1", "key": {"userId": 1}, "unique": true},
            doc! {"name": "account_1", "key": {"account": 1}, "unique": true},
            doc! {"name": "createdAt_1", "key": {"createdAt": 1}},
            doc! {"name": "modifiedAt_1", "key": {"modifiedAt": 1}},
            doc! {"name": "verifiedAt_1", "key": {"verifiedAt": 1}},
            doc! {"name": "expiredAt_1", "key": {"expiredAt": 1}},
            doc! {"name": "disabledAt_1", "key": {"disabledAt": 1}},
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
    ) -> Result<(Vec<User>, Option<Box<dyn Cursor>>), Box<dyn StdError>> {
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

    async fn get(&self, cond: &QueryCond) -> Result<Option<User>, Box<dyn StdError>> {
        let filter = get_query_filter(cond);
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(filter, None)
            .await?;
        if let Some(user) = cursor.try_next().await? {
            return Ok(Some(User {
                user_id: user.user_id,
                account: user.account,
                created_at: user.created_at.into(),
                modified_at: user.modified_at.into(),
                verified_at: match user.verified_at {
                    None => None,
                    Some(value) => Some(value.into()),
                },
                expired_at: match user.expired_at {
                    None => None,
                    Some(value) => Some(value.into()),
                },
                disabled_at: match user.disabled_at {
                    None => None,
                    Some(value) => Some(value.into()),
                },
                roles: user.roles,
                password: user.password,
                salt: user.salt,
                name: user.name,
                info: bson::from_document(user.info)?,
            }));
        }
        Ok(None)
    }

    async fn add(&self, user: &User) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            user_id: user.user_id.clone(),
            account: user.account.to_lowercase(),
            created_at: user.created_at.into(),
            modified_at: user.modified_at.into(),
            verified_at: match user.verified_at {
                None => None,
                Some(value) => Some(value.into()),
            },
            expired_at: match user.expired_at {
                None => None,
                Some(value) => Some(value.into()),
            },
            disabled_at: match user.disabled_at {
                None => None,
                Some(value) => Some(value.into()),
            },
            roles: user.roles.clone(),
            password: user.password.clone(),
            salt: user.salt.clone(),
            name: user.name.clone(),
            info: bson::to_document(&user.info)?,
        };
        self.conn
            .collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await?;
        Ok(())
    }

    async fn del(&self, user_id: &str) -> Result<(), Box<dyn StdError>> {
        let filter = doc! {"userId": user_id};
        self.conn
            .collection::<Schema>(COL_NAME)
            .delete_one(filter, None)
            .await?;
        Ok(())
    }

    async fn update(&self, user_id: &str, updates: &Updates) -> Result<(), Box<dyn StdError>> {
        let filter = doc! {"userId": user_id};
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
    async fn try_next(&mut self) -> Result<Option<User>, Box<dyn StdError>> {
        if let Some(item) = self.cursor.try_next().await? {
            self.offset += 1;
            return Ok(Some(User {
                user_id: item.user_id,
                account: item.account,
                created_at: item.created_at.into(),
                modified_at: item.modified_at.into(),
                verified_at: match item.verified_at {
                    None => None,
                    Some(value) => Some(value.into()),
                },
                expired_at: match item.expired_at {
                    None => None,
                    Some(value) => Some(value.into()),
                },
                disabled_at: match item.disabled_at {
                    None => None,
                    Some(value) => Some(value.into()),
                },
                roles: item.roles,
                password: item.password,
                salt: item.salt,
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
    if let Some(value) = cond.user_id {
        filter.insert("userId", value);
    }
    if let Some(value) = cond.account {
        filter.insert("account", value.to_lowercase().as_str());
    }
    filter
}

/// Transforms query conditions to the MongoDB document.
fn get_list_query_filter(cond: &ListQueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.user_id {
        filter.insert("userId", value);
    }
    if let Some(value) = cond.account {
        filter.insert("account", value.to_lowercase().as_str());
    }
    if let Some(value) = cond.account_contains {
        filter.insert(
            "account",
            Regex {
                pattern: value.to_lowercase(),
                options: "i".to_string(),
            },
        );
    }
    if let Some(value) = cond.verified_at {
        if value {
            filter.insert("verifiedAt", doc! {"$ne": Bson::Null});
        } else {
            filter.insert("verifiedAt", Bson::Null);
        }
    }
    if let Some(value) = cond.disabled_at {
        if value {
            filter.insert("disabledAt", doc! {"$ne": Bson::Null});
        } else {
            filter.insert("disabledAt", Bson::Null);
        }
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
                    SortKey::Account => "account",
                    SortKey::CreatedAt => "createdAt",
                    SortKey::ModifiedAt => "modifiedAt",
                    SortKey::VerifiedAt => "verifiedAt",
                    SortKey::ExpiredAt => "expiredAt",
                    SortKey::DisabledAt => "disabledAt",
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
    if let Some(value) = updates.verified_at.as_ref() {
        document.insert(
            "verifiedAt",
            DateTime::from_millis(value.timestamp_millis()),
        );
        count += 1;
    }
    if let Some(value) = updates.expired_at.as_ref() {
        match value {
            None => {
                document.insert("expiredAt", Bson::Null);
            }
            Some(value) => {
                document.insert("expiredAt", DateTime::from_millis(value.timestamp_millis()));
            }
        }
        count += 1;
    }
    if let Some(value) = updates.disabled_at.as_ref() {
        match value {
            None => {
                document.insert("disabledAt", Bson::Null);
            }
            Some(value) => {
                document.insert(
                    "disabledAt",
                    DateTime::from_millis(value.timestamp_millis()),
                );
            }
        }
        count += 1;
    }
    if let Some(value) = updates.roles {
        let mut doc = Document::new();
        for (k, v) in value {
            doc.insert(k, v);
        }
        document.insert("roles", doc);
        count += 1;
    }
    if let Some(value) = updates.password.as_ref() {
        document.insert("password", value);
        count += 1;
    }
    if let Some(value) = updates.salt.as_ref() {
        document.insert("salt", value);
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
