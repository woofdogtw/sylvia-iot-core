use std::{
    error::Error as StdError,
    fs::{self},
    io::{Error as IoError, ErrorKind},
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    time::Duration,
};

use axum::{routing, Router};
use axum_prometheus::PrometheusMetricLayer;
use axum_server::{self, tls_rustls::RustlsConfig};
use clap::{Arg as ClapArg, Command};
use json5;
use log::{self, error, info};
use serde::Deserialize;
use tokio::{self, net::TcpListener};
use tower_http::{
    cors::CorsLayer, normalize_path::NormalizePathLayer, services::ServeDir, timeout::TimeoutLayer,
};

use sylvia_iot_auth::{libs as auth_libs, routes as auth_routes};
use sylvia_iot_broker::{libs as broker_libs, routes as broker_routes};
use sylvia_iot_corelib::{
    constants::MqEngine,
    logger::{self, LoggerLayer},
    server_config,
};
use sylvia_iot_coremgr::{libs as coremgr_libs, routes as coremgr_routes};
use sylvia_iot_data::{libs as data_libs, routes as data_routes};
use sylvia_router::{libs, routes};

#[derive(Deserialize)]
struct AppConfig {
    log: logger::Config,
    server: server_config::Config,
    auth: auth_libs::config::Config,
    broker: broker_libs::config::Config,
    coremgr: coremgr_libs::config::Config,
    data: data_libs::config::Config,
    router: libs::config::Config,
}

const PROJ_NAME: &'static str = env!("CARGO_BIN_NAME");
const PROJ_VER: &'static str = env!("CARGO_PKG_VERSION");
const HTTP_PORT: u16 = 1080;
const HTTPS_PORT: u16 = 1443;
const STATIC_PATH: &'static str = "./static";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    const FN_NAME: &'static str = "main";

    let conf = match init_config() {
        Err(e) => {
            let conf = &logger::Config {
                ..Default::default()
            };
            logger::init(PROJ_NAME, &conf);
            error!("[{}] read config error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(conf) => conf,
    };

    logger::init(PROJ_NAME, &conf.log);
    let _rumqttd_handle = {
        let engine = conf.coremgr.mq.as_ref().unwrap().engine.as_ref().unwrap();
        let engine = engine.mqtt.as_ref().unwrap();
        let rumqttd_conf = conf.coremgr.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
        match engine.as_str() {
            MqEngine::RUMQTTD => Some(coremgr_libs::mq::rumqttd::start_rumqttd(
                &conf.server,
                &rumqttd_conf,
            )),
            _ => None,
        }
    };

    let auth_state = match auth_routes::new_state("/auth", &conf.auth).await {
        Err(e) => {
            error!("[{}] new routes state error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(state) => state,
    };
    let broker_state = match broker_routes::new_state("/broker", &conf.broker).await {
        Err(e) => {
            error!("[{}] new routes state error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(state) => state,
    };
    let coremgr_state = match coremgr_routes::new_state("/coremgr", &conf.coremgr).await {
        Err(e) => {
            error!("[{}] new routes state error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(state) => state,
    };
    let data_state = match data_routes::new_state("/data", &conf.data).await {
        Err(e) => {
            error!("[{}] new routes state error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(state) => state,
    };
    let router_state = match routes::new_state("/router", &conf.router).await {
        Err(e) => {
            error!("[{}] new routes state error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(state) => state,
    };
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let static_path = match conf.server.static_path.as_ref() {
        None => STATIC_PATH,
        Some(path) => path.as_str(),
    };

    let app = Router::new()
        .merge(auth_routes::new_service(&auth_state))
        .merge(broker_routes::new_service(&broker_state))
        .merge(coremgr_routes::new_service(&coremgr_state))
        .merge(data_routes::new_service(&data_state))
        .merge(routes::new_service(&router_state))
        .route("/version", routing::get(routes::get_version))
        .route(
            "/metrics",
            routing::get(|| async move { metric_handle.render() }),
        )
        .nest_service("/", ServeDir::new(static_path))
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(CorsLayer::permissive())
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(prometheus_layer)
        .layer(LoggerLayer::new());

    // Serve HTTP.
    let ipv6_addr = Ipv6Addr::from([0u8; 16]);
    let http_addr = match conf.server.http_port {
        None => SocketAddr::V6(SocketAddrV6::new(ipv6_addr, HTTP_PORT, 0, 0)),
        Some(port) => SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0)),
    };

    // Serve HTTPS.
    if let Some(cert_file) = conf.server.cert_file.as_ref() {
        if let Some(key_file) = conf.server.key_file.as_ref() {
            if let Err(_e) = rustls::crypto::aws_lc_rs::default_provider().install_default() {
                error!("[{}] init crypto erorr", FN_NAME);
                return Ok(());
            }
            let config = match RustlsConfig::from_pem_file(cert_file, key_file).await {
                Err(e) => {
                    error!("[{}] read cert/key error: {}", FN_NAME, e);
                    return Ok(());
                }
                Ok(config) => config,
            };
            let addr = match conf.server.https_port {
                None => SocketAddr::V6(SocketAddrV6::new(ipv6_addr, HTTPS_PORT, 0, 0)),
                Some(port) => SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0)),
            };
            let http_app = app.clone();
            let http_serv = tokio::spawn(async move {
                axum_server::bind(http_addr)
                    .serve(http_app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                    .unwrap()
            });
            let https_serv = tokio::spawn(async move {
                axum_server::bind_rustls(addr, config)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                    .unwrap()
            });
            info!(
                "[{}] running {} service (v{})",
                FN_NAME, PROJ_NAME, PROJ_VER
            );
            let _ = tokio::join!(http_serv, https_serv);
            return Ok(());
        }
    }

    let listener = match TcpListener::bind(http_addr).await {
        Err(e) => {
            error!("[{}] bind addr {} error: {}", FN_NAME, http_addr, e);
            return Ok(());
        }
        Ok(listener) => listener,
    };
    info!(
        "[{}] running {} service (v{})",
        FN_NAME, PROJ_NAME, PROJ_VER
    );
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        error!("[{}] launch server error: {}", FN_NAME, e);
        return Ok(());
    }
    Ok(())
}

fn init_config() -> Result<AppConfig, Box<dyn StdError>> {
    let mut args = Command::new(PROJ_NAME).version(PROJ_VER).arg(
        ClapArg::new("file")
            .short('f')
            .long("file")
            .help("config file")
            .num_args(1)
            .required(true),
    );
    args = logger::reg_args(args);
    args = server_config::reg_args(args);
    args = auth_libs::config::reg_args(args);
    args = broker_libs::config::reg_args(args);
    args = coremgr_libs::config::reg_args(args);
    args = data_libs::config::reg_args(args);
    let args = args.get_matches();

    if let Some(v) = args.get_one::<String>("file") {
        let conf_str = fs::read_to_string(v)?;
        let conf: AppConfig = json5::from_str(conf_str.as_str())?;
        return Ok(AppConfig {
            log: logger::apply_default(&conf.log),
            server: server_config::apply_default(&conf.server),
            auth: auth_libs::config::apply_default(&conf.auth),
            broker: broker_libs::config::apply_default(&conf.broker),
            coremgr: coremgr_libs::config::apply_default(&conf.coremgr),
            data: data_libs::config::apply_default(&conf.data),
            router: conf.router,
        });
    }

    // Never run here.
    Err(Box::new(IoError::new(ErrorKind::InvalidInput, "use -h")))
}
