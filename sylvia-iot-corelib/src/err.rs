//! To generate HTTP error response.
//!
//! ```
//! use sylvia_iot_corelib::err::ErrResp;
//! // To generate HTTP request body format error.
//! if format_error(body) {
//!     return Err(ErrResp::ErrParam(Some("input format error".to_string())));
//! }
//! ```

use std::{error::Error, fmt};

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json;

/// The standard error definitions.
#[derive(Debug)]
pub enum ErrResp {
    ErrAuth(Option<String>),
    ErrDb(Option<String>),
    ErrIntMsg(Option<String>),
    ErrNotFound(Option<String>),
    ErrParam(Option<String>),
    ErrPerm(Option<String>),
    ErrRsc(Option<String>),
    ErrUnknown(Option<String>),
    Custom(u16, &'static str, Option<String>),
}

/// Used for generating HTTP body for errors.
#[derive(Serialize)]
struct RespJson<'a> {
    code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
}

/// 401, token not authorized.
pub const E_AUTH: &'static str = "err_auth";
/// 503, database error.
pub const E_DB: &'static str = "err_db";
/// 503, internal service communication error.
pub const E_INT_MSG: &'static str = "err_int_msg";
/// 404, resource (in path) not found.
pub const E_NOT_FOUND: &'static str = "err_not_found";
/// 400, request (body) format error.
pub const E_PARAM: &'static str = "err_param";
/// 403, invalid permission.
pub const E_PERM: &'static str = "err_perm";
/// 503, allocate resource error.
pub const E_RSC: &'static str = "err_rsc";
/// 500, unknown error.
pub const E_UNKNOWN: &'static str = "err_unknown";

/// To generate error JSON string for HTTP body.
pub fn to_json(code: &str, message: Option<&str>) -> String {
    serde_json::to_string(&RespJson { code, message }).unwrap()
}

impl ErrResp {
    fn resp_json(&'_ self) -> RespJson<'_> {
        match *self {
            ErrResp::ErrAuth(ref desc) => RespJson {
                code: E_AUTH,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrDb(ref desc) => RespJson {
                code: E_DB,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrIntMsg(ref desc) => RespJson {
                code: E_INT_MSG,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrNotFound(ref desc) => RespJson {
                code: E_NOT_FOUND,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrParam(ref desc) => RespJson {
                code: E_PARAM,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrPerm(ref desc) => RespJson {
                code: E_PERM,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrRsc(ref desc) => RespJson {
                code: E_RSC,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::ErrUnknown(ref desc) => RespJson {
                code: E_UNKNOWN,
                message: match desc.as_ref() {
                    None => None,
                    Some(desc) => Some(desc.as_str()),
                },
            },
            ErrResp::Custom(_, err_code, ref desc) => RespJson {
                code: err_code,
                message: desc.as_deref(),
            },
        }
    }
}

impl fmt::Display for ErrResp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self.resp_json()).unwrap())
    }
}

impl Error for ErrResp {}

impl IntoResponse for ErrResp {
    fn into_response(self) -> Response {
        match self {
            ErrResp::ErrAuth(_) => (StatusCode::UNAUTHORIZED, Json(self.resp_json())),
            ErrResp::ErrDb(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(self.resp_json())),
            ErrResp::ErrIntMsg(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(self.resp_json())),
            ErrResp::ErrNotFound(_) => (StatusCode::NOT_FOUND, Json(self.resp_json())),
            ErrResp::ErrParam(_) => (StatusCode::BAD_REQUEST, Json(self.resp_json())),
            ErrResp::ErrPerm(_) => (StatusCode::FORBIDDEN, Json(self.resp_json())),
            ErrResp::ErrRsc(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(self.resp_json())),
            ErrResp::ErrUnknown(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(self.resp_json())),
            ErrResp::Custom(code, _, _) => {
                (StatusCode::from_u16(code).unwrap(), Json(self.resp_json()))
            }
        }
        .into_response()
    }
}
