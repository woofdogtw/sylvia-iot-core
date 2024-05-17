use axum::{
    body::Body,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use log::error;
use reqwest::{self, Client, Method, StatusCode};
use serde::Deserialize;

use sylvia_iot_corelib::{err::ErrResp, role::Role};

use super::{middleware::GetTokenInfoData, ErrReq, State};

pub mod application_dldata;
pub mod application_uldata;
pub mod coremgr_opdata;
pub mod network_dldata;
pub mod network_uldata;

#[derive(Deserialize)]
struct GetUser {
    data: User,
}

/// The user response that is from auth GET API.
#[derive(Deserialize)]
struct User {
    #[serde(rename = "userId")]
    _user_id: String,
    #[serde(rename = "account")]
    _account: String,
}

#[derive(Deserialize)]
struct GetUnit {
    data: Unit,
}

/// The unit response that is from broker GET API.
#[derive(Deserialize)]
struct Unit {
    #[serde(rename = "unitId")]
    _unit_id: String,
    #[serde(rename = "code")]
    _code: String,
}

async fn get_user_inner(
    fn_name: &str,
    client: &Client,
    auth_base: &str,
    user_id: &str,
    token: &HeaderValue,
) -> Result<Option<User>, Response> {
    let uri = format!("{}/api/v1/user/{}", auth_base, user_id);
    match get_stream_resp(fn_name, token, &client, uri.as_str()).await {
        Err(resp) => match resp.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(resp),
        },
        Ok(resp) => match resp.json::<GetUser>().await {
            Err(e) => {
                let e = format!("wrong response of user: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).into_response())
            }
            Ok(user) => Ok(Some(user.data)),
        },
    }
}

async fn get_unit_inner(
    fn_name: &str,
    client: &Client,
    broker_base: &str,
    unit_id: &str,
    token: &HeaderValue,
) -> Result<Option<Unit>, Response> {
    let uri = format!("{}/api/v1/unit/{}", broker_base, unit_id);
    match get_stream_resp(fn_name, token, &client, uri.as_str()).await {
        Err(resp) => match resp.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(resp),
        },
        Ok(resp) => match resp.json::<GetUnit>().await {
            Err(e) => {
                let e = format!("wrong response of unit: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).into_response())
            }
            Ok(unit) => Ok(Some(unit.data)),
        },
    }
}

async fn get_unit_cond(
    fn_name: &str,
    token_info: &GetTokenInfoData,
    query_unit: Option<&String>,
    state: &State,
) -> Result<Option<String>, Response> {
    let broker_base = state.broker_base.as_str();
    let client = state.client.clone();

    match query_unit {
        None => {
            if !Role::is_role(&token_info.roles, Role::ADMIN)
                && !Role::is_role(&token_info.roles, Role::MANAGER)
            {
                return Err(ErrResp::ErrParam(Some("missing `unit`".to_string())).into_response());
            }
            Ok(None)
        }
        Some(unit_id) => match unit_id.len() {
            0 => Ok(None),
            _ => {
                let token =
                    match HeaderValue::from_str(format!("Bearer {}", token_info.token).as_str()) {
                        Err(e) => {
                            error!("[{}] get token error: {}", fn_name, e);
                            return Err(ErrResp::ErrRsc(Some(format!("get token error: {}", e)))
                                .into_response());
                        }
                        Ok(value) => value,
                    };
                match get_unit_inner(fn_name, &client, broker_base, unit_id, &token).await {
                    Err(e) => {
                        error!("[{}] get unit error", fn_name);
                        return Err(e);
                    }
                    Ok(unit) => match unit {
                        None => {
                            return Err(ErrResp::Custom(
                                ErrReq::UNIT_NOT_EXIST.0,
                                ErrReq::UNIT_NOT_EXIST.1,
                                None,
                            )
                            .into_response())
                        }
                        Some(_) => Ok(Some(unit_id.clone())),
                    },
                }
            }
        },
    }
}

async fn get_stream_resp(
    fn_name: &str,
    token: &HeaderValue,
    client: &Client,
    uri: &str,
) -> Result<reqwest::Response, Response> {
    match client
        .request(Method::GET, uri)
        .header(reqwest::header::AUTHORIZATION, token)
        .build()
    {
        Err(e) => {
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", fn_name, e);
            Err(ErrResp::ErrRsc(Some(e)).into_response())
        }
        Ok(req) => match client.execute(req).await {
            Err(e) => {
                let e = format!("execute request error: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).into_response())
            }
            Ok(resp) => match resp.status() {
                StatusCode::OK => Ok(resp),
                _ => {
                    let mut resp_builder = Response::builder().status(resp.status());
                    for (k, v) in resp.headers() {
                        resp_builder = resp_builder.header(k, v);
                    }
                    match resp_builder.body(Body::from_stream(resp.bytes_stream())) {
                        Err(e) => {
                            let e = format!("wrap response body error: {}", e);
                            error!("[{}] {}", fn_name, e);
                            Err(ErrResp::ErrIntMsg(Some(e)).into_response())
                        }
                        Ok(resp) => Err(resp),
                    }
                }
            },
        },
    }
}
