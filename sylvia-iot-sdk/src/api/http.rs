//! A wrapped HTTP client that is used for Sylvia-IoT **coremgr** APIs with the following features:
//! - Use `client_credentials` grant type to get access token.
//!     - It is **REQUIRED** to register **private** clients (with secret).
//!     - It is **RECOMMENDED** to use the **service** role for clients, not to use **admin**,
//!       **manager** or **user** roles.
//! - Refresh token automatically to integrate network servers and application servers (or adapters)
//!   conviniently because they do not need to do multiple operations for one API request.
//!
//! Here is an example to create a client to access an API:
//!
//! ```rust
//! use reqwest::Method;
//! use sylvia_iot_sdk::api::http::{Client, ClientOptions};
//!
//! async fn main() {
//!     let opts = ClientOptions {
//!         auth_base: "http://localhost:1080/auth".to_string(),
//!         coremgr_base: "http://localhost:1080/coremgr".to_string(),
//!         client_id: "ADAPTER_CLIENT_ID".to_string(),
//!         client_secret: "ADAPTER_CLIENT_SECRET".to_string(),
//!     };
//!     let mut client = Client::new(opts);
//!     let url = "/api/v1/user";
//!     match client.request(Method::GET, url, None).await {
//!         Err(e) => {
//!             // Handle error.
//!             // Native and OAuth2 errors must be handled in this arm.
//!         },
//!         Ok((status_code, body)) => {
//!             // Handle response.
//!             // All status code except 401 must be handled in this arm.
//!         },
//!     }
//! }
//! ```
use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use actix_web::web::Bytes;
use reqwest::{header, Client as ReqwestClient, Method, StatusCode};
use serde::Deserialize;

/// The HTTP client to request Sylvia-IoT APIs. With this client, you do not need to handle 401
/// refresh token flow.
#[derive(Clone)]
pub struct Client {
    /// The underlying HTTP client instance.
    client: ReqwestClient,
    /// `sylvia-iot-auth` base path.
    auth_base: String,
    /// `sylvia-iot-coremgr` base path.
    coremgr_base: String,
    /// Client ID.
    client_id: String,
    /// Client secret.
    client_secret: String,
    /// The access token.
    access_token: Arc<Mutex<Option<String>>>,
}

/// Options of the HTTP client [`Client`] that contains OAuth2 information.
pub struct ClientOptions {
    /// `sylvia-iot-auth` base path with scheme. For example `http://localhost:1080/auth`
    pub auth_base: String,
    /// `sylvia-iot-coremgr` base path with scheme. For example `http://localhost:1080/coremgr`
    pub coremgr_base: String,
    /// Client ID.
    pub client_id: String,
    /// Client secret.
    pub client_secret: String,
}

#[derive(Debug)]
pub enum Error {
    Std(Box<dyn StdError>),
    Oauth2(Oauth2Error),
    Sylvia(ApiError),
}

/// The OAuth2 error response.
#[derive(Debug, Deserialize)]
pub struct Oauth2Error {
    /// Error code.
    pub error: String,
    /// Detail message.
    pub error_message: Option<String>,
}

/// The Sylvia-IoT API error response.
#[derive(Debug, Deserialize)]
pub struct ApiError {
    /// Error code.
    pub code: String,
    /// Detail message.
    pub message: Option<String>,
}

/// Response from OAuth2 token API.
#[derive(Deserialize)]
struct Oauth2TokenRes {
    access_token: String,
}

impl Client {
    /// Create an instance.
    pub fn new(opts: ClientOptions) -> Self {
        Client {
            client: ReqwestClient::new(),
            auth_base: opts.auth_base,
            coremgr_base: opts.coremgr_base,
            client_id: opts.client_id,
            client_secret: opts.client_secret,
            access_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Execute a Sylvia-IoT API request.
    /// - `api_path` is the relative path (of the coremgr base) the API with query string.
    ///   For example: `/api/v1/user/list?contains=word`, the client will do a request with
    ///   `http://coremgr-host/coremgr/api/v1/user/list?contains=word` URL.
    /// - `body` **MUST** be JSON format.
    pub async fn request(
        &mut self,
        method: Method,
        api_path: &str,
        body: Option<Bytes>,
    ) -> Result<(StatusCode, Bytes), Error> {
        let url = format!("{}{}", self.coremgr_base, api_path);
        let mut retry = 1;
        loop {
            let token;
            {
                let mutex = self.access_token.lock().unwrap();
                token = (*mutex).clone();
            }
            let token = match token {
                None => self.auth_token().await?,
                Some(token) => token,
            };
            let mut builder = self.client.request(method.clone(), url.as_str());
            builder = builder.bearer_auth(token);
            if let Some(body) = body.as_ref() {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                builder = builder.body(body.clone());
            }
            let req = match builder.build() {
                Err(e) => return Err(Error::Std(Box::new(e))),
                Ok(req) => req,
            };
            let resp = match self.client.execute(req).await {
                Err(e) => return Err(Error::Std(Box::new(e))),
                Ok(resp) => resp,
            };
            let status = resp.status();
            let body = match resp.bytes().await {
                Err(e) => return Err(Error::Std(Box::new(e))),
                Ok(body) => body,
            };
            if status != StatusCode::UNAUTHORIZED || retry <= 0 {
                return Ok((status, body));
            }
            retry -= 1;
            {
                let mut mutex = self.access_token.lock().unwrap();
                *mutex = None;
            }
        }
    }

    /// To authorize the client and get access token/refresh token.
    async fn auth_token(&mut self) -> Result<String, Error> {
        let url = format!("{}/oauth2/token", self.auth_base.as_str());
        let body = [("grant_type", "client_credentials")];
        let req = match self
            .client
            .request(Method::POST, url)
            .basic_auth(self.client_id.as_str(), Some(self.client_secret.as_str()))
            .form(&body)
            .build()
        {
            Err(e) => return Err(Error::Std(Box::new(e))),
            Ok(req) => req,
        };
        let resp = match self.client.execute(req).await {
            Err(e) => return Err(Error::Std(Box::new(e))),
            Ok(resp) => resp,
        };
        if resp.status() != StatusCode::OK {
            match resp.json::<Oauth2Error>().await {
                Err(e) => return Err(Error::Std(Box::new(e))),
                Ok(body) => return Err(Error::Oauth2(body)),
            }
        }
        let tokens = match resp.json::<Oauth2TokenRes>().await {
            Err(e) => return Err(Error::Std(Box::new(e))),
            Ok(tokens) => tokens,
        };
        {
            let mut mutex = self.access_token.lock().unwrap();
            *mutex = Some(tokens.access_token.clone());
        }

        Ok(tokens.access_token)
    }
}
