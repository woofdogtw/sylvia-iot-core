use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use actix_web::{dev::HttpServiceFactory, error, web};
use reqwest;
use sylvia_iot_sdk::util::err::ErrResp;
use sysinfo::{CpuRefreshKind, RefreshKind, System, SystemExt};

use crate::libs::config::Config;

mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `router`, the APIs are
    /// - `http://host:port/router/api/v1/sys/xxx`
    /// - `http://host:port/router/api/v1/net/xxx`
    pub scope_path: &'static str,
    pub config: Config,
    /// The client for internal HTTP requests.
    pub client: reqwest::Client,
    /// System information.
    pub sys_info: Arc<Mutex<System>>,
}

/// To create resources for the service.
pub async fn new_state(
    scope_path: &'static str,
    conf: &Config,
) -> Result<State, Box<dyn StdError>> {
    let mut sys_info = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage())
            .with_memory()
            .with_disks()
            .with_disks_list(),
    );
    sys_info.refresh_cpu();
    let state = State {
        scope_path,
        config: conf.clone(),
        client: reqwest::Client::new(),
        sys_info: Arc::new(Mutex::new(sys_info)),
    };
    Ok(state)
}

/// To register service URIs in the specified root path.
pub fn new_service(state: &State) -> impl HttpServiceFactory {
    web::scope(state.scope_path)
        .app_data(web::JsonConfig::default().error_handler(|err, _| {
            error::ErrorBadRequest(ErrResp::ErrParam(Some(err.to_string())))
        }))
        .app_data(web::QueryConfig::default().error_handler(|err, _| {
            error::ErrorBadRequest(ErrResp::ErrParam(Some(err.to_string())))
        }))
        .app_data(web::Data::new(state.clone()))
        .service(v1::sys::new_service("/api/v1/sys", state))
        .service(v1::net::new_service("/api/v1/net", state))
}
