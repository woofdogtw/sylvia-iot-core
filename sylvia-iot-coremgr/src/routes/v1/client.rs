use std::error::Error as StdError;

use actix_web::{
    dev::HttpServiceFactory,
    web::{self, Bytes, BytesMut},
    HttpRequest, Responder,
};
use csv::WriterBuilder;
use futures_util::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use super::{super::State, api_bridge, list_api_bridge, ListResp};

#[derive(Deserialize)]
struct ClientIdPath {
    client_id: String,
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

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("").route(web::post().to(post_client)))
        .service(web::resource("/count").route(web::get().to(get_client_count)))
        .service(web::resource("/list").route(web::get().to(get_client_list)))
        .service(
            web::resource("/{client_id}")
                .route(web::get().to(get_client))
                .route(web::patch().to(patch_client))
                .route(web::delete().to(delete_client)),
        )
}

/// `POST /{base}/api/v1/client`
async fn post_client(mut req: HttpRequest, body: Bytes, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_client";
    let api_path = format!("{}/api/v1/client", state.auth_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `GET /{base}/api/v1/client/count`
async fn get_client_count(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_client_count";
    let api_path = format!("{}/api/v1/client/count", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `GET /{base}/api/v1/client/list`
async fn get_client_list(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_client_list";
    let api_path = format!("{}/api/v1/client/list", state.auth_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, mut resp) =
        match list_api_bridge(FN_NAME, &client, &mut req, api_path, false, "client").await {
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
    };
    resp.streaming(stream)
}

/// `GET /{base}/api/v1/client/{clientId}`
async fn get_client(
    mut req: HttpRequest,
    param: web::Path<ClientIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_client";
    let api_path = format!("{}/api/v1/client/{}", state.auth_base, param.client_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `PATCH /{base}/api/v1/client/{clientId}`
async fn patch_client(
    mut req: HttpRequest,
    param: web::Path<ClientIdPath>,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "patch_client";
    let api_path = format!("{}/api/v1/client/{}", state.auth_base, param.client_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `DELETE /{base}/api/v1/client/{clientId}`
async fn delete_client(
    mut req: HttpRequest,
    param: web::Path<ClientIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_client";
    let api_path = format!("{}/api/v1/client/{}", state.auth_base, param.client_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}
