use std::collections::HashMap;

use actix_web::web::Bytes;
use chrono::{DateTime, Utc};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::http::{ApiError, Client, Error};
use crate::util::err;

#[derive(Deserialize)]
pub struct GetResData {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub account: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "modifiedAt")]
    pub modified_at: DateTime<Utc>,
    #[serde(rename = "verifiedAt")]
    pub verified_at: Option<DateTime<Utc>>,
    pub roles: HashMap<String, bool>,
    pub name: String,
    pub info: Map<String, Value>,
}

#[derive(Default, Serialize)]
pub struct PatchReqData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct GetRes {
    data: GetResData,
}

#[derive(Serialize)]
struct PatchReq {
    data: PatchReqData,
}

/// `GET /coremgr/api/v1/user`
pub async fn get(client: &mut Client) -> Result<GetResData, Error> {
    let (status, body) = client.request(Method::GET, "/api/v1/user", None).await?;
    if status == StatusCode::OK {
        match serde_json::from_slice::<GetRes>(body.to_vec().as_slice()) {
            Err(e) => {
                return Err(Error::Sylvia(ApiError {
                    code: err::E_UNKNOWN.to_string(),
                    message: Some(e.to_string()),
                }))
            }
            Ok(resp) => return Ok(resp.data),
        }
    }
    match serde_json::from_slice::<ApiError>(body.to_vec().as_slice()) {
        Err(e) => Err(Error::Sylvia(ApiError {
            code: err::E_UNKNOWN.to_string(),
            message: Some(e.to_string()),
        })),
        Ok(err) => Err(Error::Sylvia(err)),
    }
}

/// `PATCH /coremgr/api/v1/user`
pub async fn update(client: &mut Client, data: PatchReqData) -> Result<(), Error> {
    let body = match serde_json::to_vec(&PatchReq { data }) {
        Err(e) => {
            return Err(Error::Sylvia(ApiError {
                code: err::E_UNKNOWN.to_string(),
                message: Some(e.to_string()),
            }))
        }
        Ok(body) => Some(Bytes::from(body)),
    };

    let (status, body) = client.request(Method::PATCH, "/api/v1/user", body).await?;
    if status == StatusCode::NO_CONTENT {
        return Ok(());
    }
    match serde_json::from_slice::<ApiError>(body.to_vec().as_slice()) {
        Err(e) => Err(Error::Sylvia(ApiError {
            code: err::E_UNKNOWN.to_string(),
            message: Some(e.to_string()),
        })),
        Ok(err) => Err(Error::Sylvia(err)),
    }
}
