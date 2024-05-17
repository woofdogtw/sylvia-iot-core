use std::{collections::HashMap, error::Error as StdError};

use axum::{
    body::Body,
    extract::{Path, Request, State},
    response::IntoResponse,
    routing, Router,
};
use bytes::{Bytes, BytesMut};
use csv::WriterBuilder;
use futures_util::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Map, Value};

use sylvia_iot_corelib::err::ErrResp;

use super::{super::State as AppState, api_bridge, list_api_bridge, ListResp};

#[derive(Deserialize)]
struct UserIdPath {
    user_id: String,
}

#[derive(Deserialize, Serialize)]
struct User {
    account: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "verifiedAt")]
    verified_at: Option<String>,
    #[serde(skip_serializing)]
    roles: HashMap<String, bool>,
    #[serde(rename(serialize = "role"))]
    roles_str: Option<String>,
    name: String,
    #[serde(skip_serializing)]
    info: Map<String, Value>,
    #[serde(rename(serialize = "info"))]
    info_str: Option<String>,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFaccount,createdAt,modifiedAt,verifiedAt,roles,name,info\n";

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/",
                routing::get(get_user)
                    .patch(patch_user)
                    .post(post_admin_user),
            )
            .route("/count", routing::get(get_admin_user_count))
            .route("/list", routing::get(get_admin_user_list))
            .route(
                "/:user_id",
                routing::get(get_admin_user)
                    .patch(patch_admin_user)
                    .delete(delete_admin_user),
            )
            .with_state(state.clone()),
    )
}

/// `GET /{base}/api/v1/user`
async fn get_user(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_user";
    let api_path = format!("{}/api/v1/user", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `PATCH /{base}/api/v1/user`
async fn patch_user(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_user";
    let api_path = format!("{}/api/v1/user", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/user`
async fn post_admin_user(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_admin_user";
    let api_path = format!("{}/api/v1/user", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/user/count`
async fn get_admin_user_count(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_admin_user_count";
    let api_path = format!("{}/api/v1/user/count", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/user/list`
async fn get_admin_user_list(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_admin_user_list";
    let api_path = format!("{}/api/v1/user/list", state.auth_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, resp_builder) =
        match list_api_bridge(FN_NAME, &client, req, api_path, false, "user").await {
            ListResp::Axum(resp) => return resp,
            ListResp::ArrayStream(api_resp, resp_builder) => (api_resp, resp_builder),
        };

    let mut resp_stream = api_resp.bytes_stream();
    let body = Body::from_stream(async_stream::stream! {
        yield Ok(Bytes::from(CSV_FIELDS));

        let mut buffer = BytesMut::new();
        while let Some(body) = resp_stream.next().await {
            match body {
                Err(e) => {
                    error!("[{}] get body error: {}", FN_NAME, e);
                    let err: Box<dyn StdError + Send + Sync> = Box::new(e);
                    yield Err(err);
                    break;
                }
                Ok(body) => buffer.extend_from_slice(&body[..]),
            }

            let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<User>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(mut v)) = json_stream.next() {
                    if let Ok(roles_str) = serde_json::to_string(&v.roles) {
                        v.roles_str = Some(roles_str);
                    }
                    if let Ok(info_str) = serde_json::to_string(&v.info) {
                        v.info_str = Some(info_str);
                    }
                    let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
                    if let Err(e) = writer.serialize(v) {
                        let err: Box<dyn StdError + Send + Sync> = Box::new(e);
                        yield Err(err);
                        finish = true;
                        break;
                    }
                    match writer.into_inner() {
                        Err(e) => {
                            let err: Box<dyn StdError + Send + Sync> = Box::new(e);
                            yield Err(err);
                            finish = true;
                            break;
                        }
                        Ok(row) => yield Ok(Bytes::copy_from_slice(row.as_slice())),
                    }
                    continue;
                }
                let offset = json_stream.byte_offset();
                if buffer.len() <= index + offset {
                    index = buffer.len();
                    break;
                }
                match buffer[index+offset] {
                    b'[' | b',' => {
                        index += offset + 1;
                        if buffer.len() <= index {
                            break;
                        }
                        json_stream =
                            Deserializer::from_slice(&buffer[index..]).into_iter::<User>();
                    }
                    b']' => {
                        finish = true;
                        break;
                    }
                    _ => break,
                }
            }
            if finish {
                break;
            }
            buffer = buffer.split_off(index);
        }
    });
    match resp_builder.body(body) {
        Err(e) => ErrResp::ErrRsc(Some(e.to_string())).into_response(),
        Ok(resp) => resp,
    }
}

/// `GET /{base}/api/v1/user/{userId}`
async fn get_admin_user(
    state: State<AppState>,
    Path(param): Path<UserIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_admin_user";
    let api_path = format!("{}/api/v1/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `PATCH /{base}/api/v1/user/{userId}`
async fn patch_admin_user(
    state: State<AppState>,
    Path(param): Path<UserIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_admin_user";
    let api_path = format!("{}/api/v1/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `DELETE /{base}/api/v1/user/{userId}`
async fn delete_admin_user(
    state: State<AppState>,
    Path(param): Path<UserIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_admin_user";
    let api_path = format!("{}/api/v1/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}
