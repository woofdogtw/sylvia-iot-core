use std::error::Error as StdError;

use axum::{
    Router,
    body::Body,
    extract::{Path, Request, State},
    response::IntoResponse,
    routing,
};
use bytes::{Bytes, BytesMut};
use csv::WriterBuilder;
use futures_util::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use sylvia_iot_corelib::err::ErrResp;

use super::{super::State as AppState, ListResp, api_bridge, list_api_bridge};

#[derive(Deserialize)]
struct DataIdPath {
    data_id: String,
}

#[derive(Deserialize, Serialize)]
struct DlDataBuffer {
    #[serde(rename = "dataId")]
    data_id: String,
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
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "expiredAt")]
    expired_at: String,
}

const CSV_FIELDS: &'static [u8] =
    b"\xEF\xBB\xBFdataId,unitId,applicationId,applicationCode,deviceId,networkId,createdAt,expiredAt\n";

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/count", routing::get(get_dldata_buffer_count))
            .route("/list", routing::get(get_dldata_buffer_list))
            .route("/{data_id}", routing::delete(delete_dldata_buffer))
            .with_state(state.clone()),
    )
}

/// `GET /{base}/api/v1/dldata-buffer/count`
async fn get_dldata_buffer_count(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_dldata_buffer_count";
    let api_path = format!("{}/api/v1/dldata-buffer/count", state.broker_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `GET /{base}/api/v1/dldata-buffer/list`
async fn get_dldata_buffer_list(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_dldata_buffer_list";
    let api_path = format!("{}/api/v1/dldata-buffer/list", state.broker_base.as_str());
    let api_path = api_path.as_str();
    let client = state.client.clone();

    let (api_resp, resp_builder) =
        match list_api_bridge(FN_NAME, &client, req, api_path, false, "dldata-buffer").await {
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
                Deserializer::from_slice(&buffer[..]).into_iter::<DlDataBuffer>();
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
                            Deserializer::from_slice(&buffer[index..]).into_iter::<DlDataBuffer>();
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

/// `DELETE /{base}/api/v1/dldata-buffer/{dataId}`
async fn delete_dldata_buffer(
    state: State<AppState>,
    Path(param): Path<DataIdPath>,
    req: Request,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "delete_dldata_buffer";
    let api_path = format!(
        "{}/api/v1/dldata-buffer/{}",
        state.broker_base, param.data_id
    );
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}
