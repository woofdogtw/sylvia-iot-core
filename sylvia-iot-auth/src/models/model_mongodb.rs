//! Pure MongoDB model.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use mongodb::Database;

use super::{
    access_token, authorization_code, client,
    mongodb::{
        access_token::Model as AccessTokenModel,
        authorization_code::Model as AuthorizationCodeModel,
        client::Model as ClientModel,
        conn::{self, Options},
        refresh_token::Model as RefreshTokenModel,
        user::Model as UserModel,
    },
    refresh_token, user,
};

/// Pure MongoDB model.
#[derive(Clone)]
pub struct Model {
    conn: Arc<Database>,
    user: Arc<UserModel>,
    client: Arc<ClientModel>,
    authorization_code: Arc<AuthorizationCodeModel>,
    access_token: Arc<AccessTokenModel>,
    refresh_token: Arc<RefreshTokenModel>,
}

impl Model {
    /// Create an instance.
    pub async fn new(opts: &Options) -> Result<Self, Box<dyn StdError>> {
        let conn = Arc::new(conn::connect(opts).await?);
        Ok(Model {
            conn: conn.clone(),
            user: Arc::new(UserModel::new(conn.clone()).await?),
            client: Arc::new(ClientModel::new(conn.clone()).await?),
            authorization_code: Arc::new(AuthorizationCodeModel::new(conn.clone()).await?),
            access_token: Arc::new(AccessTokenModel::new(conn.clone()).await?),
            refresh_token: Arc::new(RefreshTokenModel::new(conn.clone()).await?),
        })
    }

    /// Get the raw database connection ([`Database`]).
    pub fn get_connection(&self) -> &Database {
        &self.conn
    }
}

#[async_trait]
impl super::Model for Model {
    async fn close(&self) -> Result<(), Box<dyn StdError>> {
        Ok(())
    }

    fn user(&self) -> &dyn user::UserModel {
        self.user.as_ref()
    }

    fn client(&self) -> &dyn client::ClientModel {
        self.client.as_ref()
    }

    fn authorization_code(&self) -> &dyn authorization_code::AuthorizationCodeModel {
        self.authorization_code.as_ref()
    }

    fn access_token(&self) -> &dyn access_token::AccessTokenModel {
        self.access_token.as_ref()
    }

    fn refresh_token(&self) -> &dyn refresh_token::RefreshTokenModel {
        self.refresh_token.as_ref()
    }
}
