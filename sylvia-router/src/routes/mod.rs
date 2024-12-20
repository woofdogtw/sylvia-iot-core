use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use axum::{response::IntoResponse, Router};
use reqwest;
use serde::{Deserialize, Serialize};
use sylvia_iot_sdk::util::http::{Json, Query};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::libs::config::Config;

mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `/router`, the APIs are
    /// - `http://host:port/router/api/v1/sys/xxx`
    /// - `http://host:port/router/api/v1/net/xxx`
    pub scope_path: &'static str,
    pub config: Config,
    /// The client for internal HTTP requests.
    pub client: reqwest::Client,
    /// System information.
    pub sys_info: Arc<Mutex<System>>,
    /// Disk information.
    pub disk_info: Arc<Mutex<Disks>>,
}

/// Query parameters for `GET /version`
#[derive(Deserialize)]
pub struct GetVersionQuery {
    q: Option<String>,
}

#[derive(Serialize)]
struct GetVersionRes<'a> {
    data: GetVersionResData<'a>,
}

#[derive(Serialize)]
struct GetVersionResData<'a> {
    name: &'a str,
    version: &'a str,
}

const SERV_NAME: &'static str = env!("CARGO_PKG_NAME");
const SERV_VER: &'static str = env!("CARGO_PKG_VERSION");

/// To create resources for the service.
pub async fn new_state(
    scope_path: &'static str,
    conf: &Config,
) -> Result<State, Box<dyn StdError>> {
    let mut sys_info = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram().with_swap()),
    );
    let disk_info = Disks::new_with_refreshed_list();
    sys_info.refresh_cpu_all();
    let state = State {
        scope_path,
        config: conf.clone(),
        client: reqwest::Client::new(),
        sys_info: Arc::new(Mutex::new(sys_info)),
        disk_info: Arc::new(Mutex::new(disk_info)),
    };
    Ok(state)
}

/// To register service URIs in the specified root path.
pub fn new_service(state: &State) -> Router {
    Router::new().nest(
        &state.scope_path,
        Router::new()
            .merge(v1::sys::new_service("/api/v1/sys", state))
            .merge(v1::net::new_service("/api/v1/net", state)),
    )
}

pub async fn get_version(Query(query): Query<GetVersionQuery>) -> impl IntoResponse {
    if let Some(q) = query.q.as_ref() {
        match q.as_str() {
            "name" => return SERV_NAME.into_response(),
            "version" => return SERV_VER.into_response(),
            _ => (),
        }
    }

    Json(GetVersionRes {
        data: GetVersionResData {
            name: SERV_NAME,
            version: SERV_VER,
        },
    })
    .into_response()
}
