use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::TimeDelta;
use futures::TryStreamExt;
use mongodb::{
    Database,
    bson::{DateTime, Document, doc},
};
use serde::{Deserialize, Serialize};

use sylvia_iot_corelib::err::E_UNKNOWN;

use super::super::access_token::{AccessToken, AccessTokenModel, EXPIRES, QueryCond};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<Database>,
}

/// MongoDB schema.
#[derive(Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken", skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    scope: Option<String>,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "accessToken";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl AccessTokenModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "accessToken_1", "key": {"accessToken": 1}, "unique": true},
            doc! {"name": "refreshToken_1", "key": {"refreshToken": 1}},
            doc! {"name": "clientId_1", "key": {"clientId": 1}},
            doc! {"name": "userId_1", "key": {"userId": 1}},
            doc! {"name": "ttl_1", "key": {"createdAt": 1}, "expireAfterSeconds": EXPIRES + 60},
        ];
        let command = doc! {
            "createIndexes": COL_NAME,
            "indexes": indexes,
        };
        self.conn.run_command(command).await?;
        Ok(())
    }

    async fn get(&self, access_token: &str) -> Result<Option<AccessToken>, Box<dyn StdError>> {
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(doc! {"accessToken": access_token})
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(AccessToken {
                access_token: item.access_token,
                refresh_token: item.refresh_token,
                expires_at: item.expires_at.into(),
                scope: item.scope,
                client_id: item.client_id,
                redirect_uri: item.redirect_uri,
                user_id: item.user_id,
            }));
        }
        Ok(None)
    }

    async fn add(&self, token: &AccessToken) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            expires_at: token.expires_at.into(),
            scope: token.scope.clone(),
            client_id: token.client_id.clone(),
            redirect_uri: token.redirect_uri.clone(),
            user_id: token.user_id.clone(),
            created_at: match TimeDelta::try_seconds(EXPIRES) {
                None => panic!("{}", E_UNKNOWN),
                Some(t) => (token.expires_at - t).into(),
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

/// Transforms query conditions to the MongoDB document.
fn get_query_filter(cond: &QueryCond) -> Document {
    let mut filter = Document::new();
    if let Some(value) = cond.access_token {
        filter.insert("accessToken", value);
    }
    if let Some(value) = cond.refresh_token {
        filter.insert("refreshToken", value);
    }
    if let Some(value) = cond.client_id {
        filter.insert("clientId", value);
    }
    if let Some(value) = cond.user_id {
        filter.insert("userId", value);
    }
    filter
}
