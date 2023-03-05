use actix_web::{
    http::header::{self, HeaderValue},
    HttpResponse, HttpResponseBuilder, ResponseError,
};
use log::error;
use reqwest::{self, Client, Method, StatusCode};
use serde::Deserialize;

use sylvia_iot_corelib::err::ErrResp;

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
) -> Result<Option<User>, HttpResponse> {
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
                Err(ErrResp::ErrIntMsg(Some(e)).error_response())
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
) -> Result<Option<Unit>, HttpResponse> {
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
                Err(ErrResp::ErrIntMsg(Some(e)).error_response())
            }
            Ok(unit) => Ok(Some(unit.data)),
        },
    }
}

async fn get_stream_resp(
    fn_name: &str,
    token: &HeaderValue,
    client: &Client,
    uri: &str,
) -> Result<reqwest::Response, HttpResponse> {
    match client
        .request(Method::GET, uri)
        .header(reqwest::header::AUTHORIZATION, token)
        .build()
    {
        Err(e) => {
            let e = format!("generate request error: {}", e);
            error!("[{}] {}", fn_name, e);
            Err(ErrResp::ErrRsc(Some(e)).error_response())
        }
        Ok(req) => match client.execute(req).await {
            Err(e) => {
                let e = format!("execute request error: {}", e);
                error!("[{}] {}", fn_name, e);
                Err(ErrResp::ErrIntMsg(Some(e)).error_response())
            }
            Ok(resp) => match resp.status() {
                StatusCode::OK => Ok(resp),
                _ => {
                    let mut new_resp = HttpResponseBuilder::new(resp.status());
                    if let Some(content_type) = resp.headers().get(header::CONTENT_TYPE) {
                        new_resp.insert_header((header::CONTENT_TYPE, content_type.clone()));
                    }
                    if let Some(auth) = resp.headers().get(header::WWW_AUTHENTICATE) {
                        new_resp.insert_header((header::WWW_AUTHENTICATE, auth.clone()));
                    }
                    Err(new_resp.streaming(resp.bytes_stream()))
                }
            },
        },
    }
}
