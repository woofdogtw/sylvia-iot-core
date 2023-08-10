use std::{
    error::Error as StdError,
    fs::{self, File},
    io::{BufReader, Error as IoError, ErrorKind},
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
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
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile;
use serde::Deserialize;
use tokio;

use sylvia_iot_auth::{libs, routes};
use sylvia_iot_corelib::{
    logger::{self, ACTIX_LOGGER_FORMAT},
    server_config,
};

#[derive(Deserialize)]
struct AppConfig {
    log: logger::Config,
    server: server_config::Config,
    auth: libs::config::Config,
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

    let auth_state = match routes::new_state("/auth", &conf.auth).await {
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
            .service(routes::new_service(&auth_state))
            .route("/version", web::get().to(routes::get_version))
            .service(actix_files::Files::new("/", static_path).index_file("index.html"))
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(60)));
    let ipv4_addr = Ipv4Addr::from([0u8; 4]);
    let ipv6_addr = Ipv6Addr::from([0u8; 16]);
    let addrs = match conf.server.http_port {
        None => [
            SocketAddr::V4(SocketAddrV4::new(ipv4_addr, HTTP_PORT)),
            SocketAddr::V6(SocketAddrV6::new(ipv6_addr, HTTP_PORT, 0, 0)),
        ],
        Some(port) => [
            SocketAddr::V4(SocketAddrV4::new(ipv4_addr, port)),
            SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0)),
        ],
    };
    serv = serv.bind(addrs.as_slice())?;
    if let Some(cert_file) = conf.server.cert_file.as_ref() {
        if let Some(key_file) = conf.server.key_file.as_ref() {
            let cert = rustls_pemfile::certs(&mut BufReader::new(File::open(cert_file)?))?
                .into_iter()
                .map(Certificate)
                .collect();
            let mut keys = match rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(
                File::open(key_file.as_str())?,
            )) {
                Err(_) => rustls_pemfile::rsa_private_keys(&mut BufReader::new(File::open(
                    key_file.as_str(),
                )?))?,
                Ok(keys) => keys,
            };
            if keys.len() != 1 {
                keys = rustls_pemfile::rsa_private_keys(&mut BufReader::new(File::open(
                    key_file.as_str(),
                )?))?;
                if keys.len() != 1 {
                    return Err(IoError::new(ErrorKind::InvalidData, "invalid private key"));
                }
            }
            let key = PrivateKey(keys[0].clone());
            let secure_config = match ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(cert, key)
            {
                Err(e) => return Err(IoError::new(ErrorKind::InvalidData, e)),
                Ok(conf) => conf,
            };
            let addrs = match conf.server.https_port {
                None => [
                    SocketAddr::V4(SocketAddrV4::new(ipv4_addr, HTTPS_PORT)),
                    SocketAddr::V6(SocketAddrV6::new(ipv6_addr, HTTPS_PORT, 0, 0)),
                ],
                Some(port) => [
                    SocketAddr::V4(SocketAddrV4::new(ipv4_addr, port)),
                    SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0)),
                ],
            };
            serv = serv.bind_rustls(addrs.as_slice(), secure_config)?;
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
    args = libs::config::reg_args(args);
    let args = args.get_matches();

    if let Some(v) = args.get_one::<String>("file") {
        let conf_str = fs::read_to_string(v)?;
        let conf: AppConfig = json5::from_str(conf_str.as_str())?;
        return Ok(AppConfig {
            log: logger::apply_default(&conf.log),
            server: server_config::apply_default(&conf.server),
            auth: libs::config::apply_default(&conf.auth),
        });
    }

    Ok(AppConfig {
        log: logger::read_args(&args),
        server: server_config::read_args(&args),
        auth: libs::config::read_args(&args),
    })
}
