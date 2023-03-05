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
use serde_json::{Deserializer, Map, Value};

use super::{super::State, api_bridge, list_api_bridge, ListResp};

#[derive(Deserialize)]
struct DeviceIdPath {
    device_id: String,
}

#[derive(Deserialize, Serialize)]
struct Device {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
    name: String,
    #[serde(skip_serializing)]
    info: Map<String, Value>,
    #[serde(rename(serialize = "info"))]
    info_str: Option<String>,
}

const CSV_FIELDS: &'static str =
    "deviceId,unitId,unitCode,networkId,networkCode,networkAddr,createdAt,modifiedAt,name,info\n";

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("").route(web::post().to(post_device)))
        .service(web::resource("/bulk").route(web::post().to(post_device_bulk)))
        .service(web::resource("/bulk-delete").route(web::post().to(post_device_bulk_del)))
        .service(web::resource("/range").route(web::post().to(post_device_range)))
        .service(web::resource("/range-delete").route(web::post().to(post_device_range_del)))
        .service(web::resource("/count").route(web::get().to(get_device_count)))
        .service(web::resource("/list").route(web::get().to(get_device_list)))
        .service(
            web::resource("/{device_id}")
                .route(web::get().to(get_device))
                .route(web::patch().to(patch_device))
                .route(web::delete().to(delete_device)),
        )
}

/// `POST /{base}/api/v1/device`
async fn post_device(mut req: HttpRequest, body: Bytes, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_device";
    let api_path = format!("{}/api/v1/device", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `POST /{base}/api/v1/device/bulk`
async fn post_device_bulk(
    mut req: HttpRequest,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_bulk";
    let api_path = format!("{}/api/v1/device/bulk", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `POST /{base}/api/v1/device/bulk-delete`
async fn post_device_bulk_del(
    mut req: HttpRequest,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_bulk_del";
    let api_path = format!("{}/api/v1/device/bulk-delete", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `POST /{base}/api/v1/device/range`
async fn post_device_range(
    mut req: HttpRequest,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_range";
    let api_path = format!("{}/api/v1/device/range", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `POST /{base}/api/v1/device/range-delete`
async fn post_device_range_del(
    mut req: HttpRequest,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "post_device_range_del";
    let api_path = format!("{}/api/v1/device/range-delete", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `GET /{base}/api/v1/device/count`
async fn get_device_count(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_device_count";
    let api_path = format!("{}/api/v1/device/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `GET /{base}/api/v1/device/list`
async fn get_device_list(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_device_list";
    let api_path = format!("{}/api/v1/device/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, mut resp) =
        match list_api_bridge(FN_NAME, &client, &mut req, api_path, false, "device").await {
            ListResp::ActixWeb(resp) => return resp,
            ListResp::ArrayStream(api_resp, resp) => (api_resp, resp),
        };

    let mut resp_stream = api_resp.bytes_stream();
    let stream = async_stream::stream! {
        yield Ok(Bytes::from(vec![0xEF, 0xBB, 0xBF])); // BOM
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

            let mut json_stream = Deserializer::from_slice(&buffer[..]).into_iter::<Device>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(mut v)) = json_stream.next() {
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
                            Deserializer::from_slice(&buffer[index..]).into_iter::<Device>();
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

/// `GET /{base}/api/v1/device/{deviceId}`
async fn get_device(
    mut req: HttpRequest,
    param: web::Path<DeviceIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "get_device";
    let api_path = format!("{}/api/v1/device/{}", state.broker_base, param.device_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `PATCH /{base}/api/v1/device/{deviceId}`
async fn patch_device(
    mut req: HttpRequest,
    param: web::Path<DeviceIdPath>,
    body: Bytes,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "patch_device";
    let api_path = format!("{}/api/v1/device/{}", state.broker_base, param.device_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), Some(body)).await
}

/// `DELETE /{base}/api/v1/device/{deviceId}`
async fn delete_device(
    mut req: HttpRequest,
    param: web::Path<DeviceIdPath>,
    state: web::Data<State>,
) -> impl Responder {
    const FN_NAME: &'static str = "delete_device";
    let api_path = format!("{}/api/v1/device/{}", state.broker_base, param.device_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}
