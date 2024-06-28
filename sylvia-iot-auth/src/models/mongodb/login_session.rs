use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use chrono::TimeDelta;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime, Document},
    Database,
};
use serde::{Deserialize, Serialize};

use sylvia_iot_corelib::err::E_UNKNOWN;

use super::super::login_session::{LoginSession, LoginSessionModel, QueryCond, EXPIRES};

/// Model instance.
pub struct Model {
    /// The associated database connection.
    conn: Arc<Database>,
}

/// MongoDB schema.
#[derive(Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "loginSession";

impl Model {
    /// To create the model instance with a database connection.
    pub async fn new(conn: Arc<Database>) -> Result<Self, Box<dyn StdError>> {
        let model = Model { conn };
        model.init().await?;
        Ok(model)
    }
}

#[async_trait]
impl LoginSessionModel for Model {
    async fn init(&self) -> Result<(), Box<dyn StdError>> {
        let indexes = vec![
            doc! {"name": "sessionId_1", "key": {"sessionId": 1}, "unique": true},
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

    async fn get(&self, session_id: &str) -> Result<Option<LoginSession>, Box<dyn StdError>> {
        let mut cursor = self
            .conn
            .collection::<Schema>(COL_NAME)
            .find(doc! {"sessionId": session_id})
            .await?;
        if let Some(item) = cursor.try_next().await? {
            return Ok(Some(LoginSession {
                session_id: item.session_id,
                expires_at: item.expires_at.into(),
                user_id: item.user_id,
            }));
        }
        Ok(None)
    }

    async fn add(&self, session: &LoginSession) -> Result<(), Box<dyn StdError>> {
        let item = Schema {
            session_id: session.session_id.clone(),
            expires_at: session.expires_at.into(),
            user_id: session.user_id.clone(),
            created_at: match TimeDelta::try_seconds(EXPIRES) {
                None => panic!("{}", E_UNKNOWN),
                Some(t) => (session.expires_at - t).into(),
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
    if let Some(value) = cond.session_id {
        filter.insert("sessionId", value);
    }
    if let Some(value) = cond.user_id {
        filter.insert("userId", value);
    }
    filter
}
