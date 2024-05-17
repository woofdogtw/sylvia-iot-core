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
use serde_json::{Deserializer, Map, Value};

use sylvia_iot_corelib::err::ErrResp;

use super::{super::State as AppState, api_bridge, list_api_bridge, ListResp};

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
    profile: String,
    name: String,
    #[serde(skip_serializing)]
    info: Map<String, Value>,
    #[serde(rename(serialize = "info"))]
    info_str: Option<String>,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFdeviceId,unitId,unitCode,networkId,networkCode,networkAddr,createdAt,modifiedAt,profile,name,info\n";

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/", routing::post(post_device))
            .route("/bulk", routing::post(post_device_bulk))
            .route("/bulk-delete", routing::post(post_device_bulk_del))
            .route("/range", routing::post(post_device_range))
            .route("/range-delete", routing::post(post_device_range_del))
            .route("/count", routing::get(get_device_count))
            .route("/list", routing::get(get_device_list))
            .route(
                "/:device_id",
                routing::get(get_device)
                    .patch(patch_device)
                    .delete(delete_device),
            )
            .with_state(state.clone()),
    )
}

/// `POST /{base}/api/v1/device`
async fn post_device(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device";
    let api_path = format!("{}/api/v1/device", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device/bulk`
async fn post_device_bulk(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_bulk";
    let api_path = format!("{}/api/v1/device/bulk", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device/bulk-delete`
async fn post_device_bulk_del(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_bulk_del";
    let api_path = format!("{}/api/v1/device/bulk-delete", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device/range`
async fn post_device_range(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_range";
    let api_path = format!("{}/api/v1/device/range", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device/range-delete`
async fn post_device_range_del(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_range_del";
    let api_path = format!("{}/api/v1/device/range-delete", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/device/count`
async fn get_device_count(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device_count";
    let api_path = format!("{}/api/v1/device/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/device/list`
async fn get_device_list(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device_list";
    let api_path = format!("{}/api/v1/device/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, resp_builder) =
        match list_api_bridge(FN_NAME, &client, req, api_path, false, "device").await {
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
    });
    match resp_builder.body(body) {
        Err(e) => ErrResp::ErrRsc(Some(e.to_string())).into_response(),
        Ok(resp) => resp,
    }
}

/// `GET /{base}/api/v1/device/{deviceId}`
async fn get_device(
    state: State<AppState>,
    Path(param): Path<DeviceIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device";
    let api_path = format!("{}/api/v1/device/{}", state.broker_base, param.device_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `PATCH /{base}/api/v1/device/{deviceId}`
async fn patch_device(
    state: State<AppState>,
    Path(param): Path<DeviceIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "patch_device";
    let api_path = format!("{}/api/v1/device/{}", state.broker_base, param.device_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `DELETE /{base}/api/v1/device/{deviceId}`
async fn delete_device(
    state: State<AppState>,
    Path(param): Path<DeviceIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_device";
    let api_path = format!("{}/api/v1/device/{}", state.broker_base, param.device_id);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}
