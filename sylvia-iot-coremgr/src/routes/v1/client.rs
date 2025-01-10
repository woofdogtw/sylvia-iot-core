use std::error::Error as StdError;

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
use serde_json::Deserializer;

use sylvia_iot_corelib::err::ErrResp;

use super::{super::State as AppState, api_bridge, list_api_bridge, ListResp};

#[derive(Deserialize)]
struct ClientIdPath {
    client_id: String,
}

#[derive(Deserialize)]
struct UserIdPath {
    user_id: String,
}

#[derive(Deserialize, Serialize)]
struct Client {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    #[serde(rename = "clientSecret")]
    client_secret: Option<String>,
    #[serde(rename = "redirectUris", skip_serializing)]
    redirect_uris: Vec<String>,
    #[serde(rename(serialize = "redirectUris"))]
    redirect_uris_str: Option<String>,
    #[serde(skip_serializing)]
    scopes: Vec<String>,
    #[serde(rename(serialize = "scopes"))]
    scopes_str: Option<String>,
    #[serde(rename = "userId")]
    user_id: Option<String>,
    name: String,
    image: Option<String>,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFclientId,createdAt,modifiedAt,clientSecret,redirectUris,scopes,userId,name,image\n";

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/", routing::post(post_client))
            .route("/count", routing::get(get_client_count))
            .route("/list", routing::get(get_client_list))
            .route(
                "/{client_id}",
                routing::get(get_client)
                    .patch(patch_client)
                    .delete(delete_client),
            )
            .route("/user/{user_id}", routing::delete(delete_client_user))
            .with_state(state.clone()),
    )
}

/// `POST /{base}/api/v1/client`
async fn post_client(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_client";
    let api_path = format!("{}/api/v1/client", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/client/count`
async fn get_client_count(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_client_count";
    let api_path = format!("{}/api/v1/client/count", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/client/list`
async fn get_client_list(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_client_list";
    let api_path = format!("{}/api/v1/client/list", state.auth_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, resp_builder) =
        match list_api_bridge(FN_NAME, &client, req, api_path, false, "client").await {
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

            let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<Client>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(mut v)) = json_stream.next() {
                    if let Ok(redirect_uris_str) = serde_json::to_string(&v.redirect_uris) {
                        v.redirect_uris_str = Some(redirect_uris_str);
                    }
                    if let Ok(scopes_str) = serde_json::to_string(&v.scopes) {
                        v.scopes_str = Some(scopes_str);
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
                            Deserializer::from_slice(&buffer[index..]).into_iter::<Client>();
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

/// `GET /{base}/api/v1/client/{clientId}`
async fn get_client(
    state: State<AppState>,
    Path(param): Path<ClientIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_client";
    let api_path = format!("{}/api/v1/client/{}", state.auth_base, param.client_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `PATCH /{base}/api/v1/client/{clientId}`
async fn patch_client(
    state: State<AppState>,
    Path(param): Path<ClientIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_client";
    let api_path = format!("{}/api/v1/client/{}", state.auth_base, param.client_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `DELETE /{base}/api/v1/client/{clientId}`
async fn delete_client(
    state: State<AppState>,
    Path(param): Path<ClientIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_client";
    let api_path = format!("{}/api/v1/client/{}", state.auth_base, param.client_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `DELETE /{base}/api/v1/client/user/{userId}`
async fn delete_client_user(
    state: State<AppState>,
    Path(param): Path<UserIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_client_user";
    let api_path = format!("{}/api/v1/client/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}
