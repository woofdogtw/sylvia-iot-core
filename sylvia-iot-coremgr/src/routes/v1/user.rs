use std::{collections::HashMap, error::Error as StdError};

use actix_web::{
    dev::HttpServiceFactory,
    web::{self, Bytes, BytesMut},
    HttpRequest, Responder,
};
use csv::WriterBuilder;
use futures_util::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Map, Value};

use super::{super::State, api_bridge, list_api_bridge, ListResp};

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

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(
            web::resource("")
                .route(web::get().to(get_user))
                .route(web::patch().to(patch_user))
                .route(web::post().to(post_admin_user)),
        )
        .service(web::resource("/count").route(web::get().to(get_admin_user_count)))
        .service(web::resource("/list").route(web::get().to(get_admin_user_list)))
        .service(
            web::resource("/{user_id}")
                .route(web::get().to(get_admin_user))
                .route(web::patch().to(patch_admin_user))
                .route(web::delete().to(delete_admin_user)),
        )
}

/// `GET /{base}/api/v1/user`
async fn get_user(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_user";
    let api_path = format!("{}/api/v1/user", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `PATCH /{base}/api/v1/user`
async fn patch_user(mut req: HttpRequest, body: Bytes, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "patch_user";
    let api_path = format!("{}/api/v1/user", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `POST /{base}/api/v1/user`
async fn post_admin_user(
    mut req: HttpRequest,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_admin_user";
    let api_path = format!("{}/api/v1/user", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `GET /{base}/api/v1/user/count`
async fn get_admin_user_count(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_admin_user_count";
    let api_path = format!("{}/api/v1/user/count", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `GET /{base}/api/v1/user/list`
async fn get_admin_user_list(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_admin_user_list";
    let api_path = format!("{}/api/v1/user/list", state.auth_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, mut resp) =
        match list_api_bridge(FN_NAME, &client, &mut req, api_path, false, "user").await {
            ListResp::ActixWeb(resp) => return resp,
            ListResp::ArrayStream(api_resp, resp) => (api_resp, resp),
        };

    let mut resp_stream = api_resp.bytes_stream();
    let stream = async_stream::stream! {
        yield Ok(Bytes::from(CSV_FIELDS));

        let mut buffer = BytesMut::new();
        while let Some(body) = resp_stream.next().await {
            match body {
                Err(e) => {
                    error!("[{}] get body error: {}", FN_NAME, e);
                    let err: Box<dyn StdError> = Box::new(e);
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
                        let err: Box<dyn StdError> = Box::new(e);
                        yield Err(err);
                        finish = true;
                        break;
                    }
                    match writer.into_inner() {
                        Err(e) => {
                            let err: Box<dyn StdError> = Box::new(e);
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
    };
    resp.streaming(stream)
}

/// `GET /{base}/api/v1/user/{userId}`
async fn get_admin_user(
    mut req: HttpRequest,
    param: web::Path<UserIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_admin_user";
    let api_path = format!("{}/api/v1/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `PATCH /{base}/api/v1/user/{userId}`
async fn patch_admin_user(
    mut req: HttpRequest,
    param: web::Path<UserIdPath>,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "patch_admin_user";
    let api_path = format!("{}/api/v1/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `DELETE /{base}/api/v1/user/{userId}`
async fn delete_admin_user(
    mut req: HttpRequest,
    param: web::Path<UserIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_admin_user";
    let api_path = format!("{}/api/v1/user/{}", state.auth_base, param.user_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}
