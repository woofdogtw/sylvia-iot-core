use std::fmt;

use actix_web::{http::StatusCode, ResponseError};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Deserialize, Serialize)]
pub struct OAuth2Error {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_description: Option<String>,
}

const INVALID_REQUEST: &'static str = "invalid_request";

impl OAuth2Error {
    pub fn new(error: &str, description: Option<String>) -> Self {
        OAuth2Error {
            error: error.to_string(),
            error_description: description,
        }
    }

    pub fn new_request(description: Option<String>) -> Self {
        OAuth2Error {
            error: INVALID_REQUEST.to_string(),
            error_description: description,
        }
    }
}

impl fmt::Display for OAuth2Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl ResponseError for OAuth2Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}
