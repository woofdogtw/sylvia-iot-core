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
struct RouteIdPath {
    route_id: String,
}

#[derive(Deserialize, Serialize)]
struct DeviceRoute {
    #[serde(rename = "routeId")]
    route_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    profile: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFrouteId,unitId,applicationId,applicationCode,deviceId,networkId,networkCode,networkAddr,profile,createdAt,modifiedAt\n";

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/", routing::post(post_device_route))
            .route("/bulk", routing::post(post_device_route_bulk))
            .route("/bulk-delete", routing::post(post_device_route_bulk_del))
            .route("/range", routing::post(post_device_route_range))
            .route("/range-delete", routing::post(post_device_route_range_del))
            .route("/count", routing::get(get_device_route_count))
            .route("/list", routing::get(get_device_route_list))
            .route("/{route_id}", routing::delete(delete_device_route))
            .with_state(state.clone()),
    )
}

/// `POST /{base}/api/v1/device-route`
async fn post_device_route(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_route";
    let api_path = format!("{}/api/v1/device-route", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device-route/bulk`
async fn post_device_route_bulk(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_route_bulk";
    let api_path = format!("{}/api/v1/device-route/bulk", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device-route/bulk-delete`
async fn post_device_route_bulk_del(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_route_bulk_del";
    let api_path = format!("{}/api/v1/device-route/bulk-delete", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device-route/range`
async fn post_device_route_range(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_route_range";
    let api_path = format!("{}/api/v1/device-route/range", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/device-route/range-delete`
async fn post_device_route_range_del(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_device_route_range_del";
    let api_path = format!("{}/api/v1/device-route/range-delete", state.broker_base);
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/device-route/count`
async fn get_device_route_count(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device_route_count";
    let api_path = format!("{}/api/v1/device-route/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/device-route/list`
async fn get_device_route_list(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_device_route_list";
    let api_path = format!("{}/api/v1/device-route/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, resp_builder) =
        match list_api_bridge(FN_NAME, &client, req, api_path, false, "device-route").await {
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

            let mut json_stream =
                Deserializer::from_slice(&buffer[..]).into_iter::<DeviceRoute>();
            let mut index = 0;
            let mut finish = false;
            loop {
                if let Some(Ok(v)) = json_stream.next() {
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
                            Deserializer::from_slice(&buffer[index..]).into_iter::<DeviceRoute>();
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

/// `DELETE /{base}/api/v1/device-route/{routeId}`
async fn delete_device_route(
    state: State<AppState>,
    Path(param): Path<RouteIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_device_route";
    let api_path = format!(
        "{}/api/v1/device-route/{}",
        state.broker_base, param.route_id
    );
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}
