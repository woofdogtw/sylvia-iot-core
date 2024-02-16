use std::{
    error::Error as StdError,
    fs::{self, File},
    io::{BufReader, Error as IoError, ErrorKind},
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    time::Duration,
};

use actix_cors::Cors;
use actix_files;
use actix_http::KeepAlive;
use actix_web::{
    middleware::{Logger, NormalizePath},
    web, App, HttpServer,
};
use actix_web_prom::PrometheusMetricsBuilder;
use clap::{Arg as ClapArg, Command};
use json5;
use log::{self, error};
use rustls::ServerConfig;
use rustls_pemfile;
use serde::Deserialize;
use tokio;

use sylvia_iot_auth::{libs as auth_libs, routes as auth_routes};
use sylvia_iot_broker::{libs as broker_libs, routes as broker_routes};
use sylvia_iot_corelib::{
    constants::MqEngine,
    logger::{self, ACTIX_LOGGER_FORMAT},
    server_config,
};
use sylvia_iot_coremgr::{libs as coremgr_libs, routes as coremgr_routes};

#[derive(Deserialize)]
struct AppConfig {
    log: logger::Config,
    server: server_config::Config,
    auth: auth_libs::config::Config,
    broker: broker_libs::config::Config,
    coremgr: coremgr_libs::config::Config,
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
    let prometheus = match PrometheusMetricsBuilder::new(PROJ_NAME.replace("-", "_").as_str())
        .endpoint("/metrics")
        .build()
    {
        Err(e) => {
            error!("[{}] new Prometheus error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(p) => p,
    };

    let mut serv = HttpServer::new(move || {
        let static_path = match conf.server.static_path.as_ref() {
            None => STATIC_PATH,
            Some(path) => path.as_str(),
        };
        App::new()
            .wrap(Logger::new(ACTIX_LOGGER_FORMAT))
            .wrap(prometheus.clone())
            .wrap(NormalizePath::trim())
            .wrap(Cors::permissive())
            .service(auth_routes::new_service(&auth_state))
            .service(broker_routes::new_service(&broker_state))
            .service(coremgr_routes::new_service(&coremgr_state))
            .route("/version", web::get().to(coremgr_routes::get_version))
            .service(actix_files::Files::new("/", static_path).index_file("index.html"))
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(60)));
    let ipv6_addr = Ipv6Addr::from([0u8; 16]);
    let addrs = match conf.server.http_port {
        None => [SocketAddr::V6(SocketAddrV6::new(
            ipv6_addr, HTTP_PORT, 0, 0,
        ))],
        Some(port) => [SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0))],
    };
    serv = serv.bind(addrs.as_slice())?;
    if let Some(cert_file) = conf.server.cert_file.as_ref() {
        if let Some(key_file) = conf.server.key_file.as_ref() {
            let cert = rustls_pemfile::certs(&mut BufReader::new(File::open(cert_file)?))
                .filter_map(|result| result.ok())
                .collect();
            let key = match rustls_pemfile::private_key(&mut BufReader::new(File::open(
                key_file.as_str(),
            )?)) {
                Err(_) => return Err(IoError::new(ErrorKind::InvalidData, "invalid private key")),
                Ok(key) => match key {
                    None => {
                        return Err(IoError::new(ErrorKind::InvalidData, "invalid private key"))
                    }
                    Some(key) => key,
                },
            };
            let secure_config = match ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert, key)
            {
                Err(e) => return Err(IoError::new(ErrorKind::InvalidData, e)),
                Ok(conf) => conf,
            };
            let addrs = match conf.server.https_port {
                None => [SocketAddr::V6(SocketAddrV6::new(
                    ipv6_addr, HTTPS_PORT, 0, 0,
                ))],
                Some(port) => [SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0))],
            };
            serv = serv.bind_rustls_0_22(addrs.as_slice(), secure_config)?;
        }
    }
    serv.run().await
}

fn init_config() -> Result<AppConfig, Box<dyn StdError>> {
    let mut args = Command::new(PROJ_NAME).version(PROJ_VER).arg(
        ClapArg::new("file")
            .short('f')
            .long("file")
            .help("config file")
            .num_args(1),
    );
    args = logger::reg_args(args);
    args = server_config::reg_args(args);
    args = auth_libs::config::reg_args(args);
    args = broker_libs::config::reg_args(args);
    args = coremgr_libs::config::reg_args(args);
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
        });
    }

    Ok(AppConfig {
        log: logger::read_args(&args),
        server: server_config::read_args(&args),
        auth: auth_libs::config::read_args(&args),
        broker: broker_libs::config::read_args(&args),
        coremgr: coremgr_libs::config::read_args(&args),
    })
}
