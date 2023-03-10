use std::{
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
};

use actix_web::{dev::HttpServiceFactory, error, web};
use reqwest;
use url::Url;

use async_trait::async_trait;
use general_mq::{
    queue::{Event, EventHandler as QueueEventHandler, Message, Queue as MqQueue, Status},
    Queue,
};
use log::{error, info, warn};
use sylvia_iot_corelib::{
    constants::{CacheEngine, DbEngine},
    err::ErrResp,
};

use crate::{
    libs::{
        config::{self, Config},
        mq::{self, application::ApplicationMgr, network::NetworkMgr, Connection},
    },
    models::{
        self, Cache, CacheConnOptions, ConnOptions, DeviceOptions, DeviceRouteOptions, Model,
        MongoDbOptions, NetworkRouteOptions, SqliteOptions,
    },
};

pub mod middleware;
mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `broker`, the APIs are
    /// - `http://host:port/broker/api/v1/unit/xxx`
    /// - `http://host:port/broker/api/v1/application/xxx`
    pub scope_path: &'static str,
    /// The scopes for accessing APIs.
    pub api_scopes: HashMap<String, Vec<String>>,
    /// The database model.
    pub model: Arc<dyn Model>,
    /// The database cache.
    pub cache: Option<Arc<dyn Cache>>,
    /// The sylvia-iot-auth base API path with host.
    ///
    /// For example, `http://localhost:1080/auth`.
    pub auth_base: String,
    pub amqp_prefetch: u16,
    pub mqtt_shared_prefix: String,
    /// The client for internal HTTP requests.
    pub client: reqwest::Client,
    /// Queue connections. Key is uri.
    pub mq_conns: Arc<Mutex<HashMap<String, Connection>>>,
    /// Application managers. Key is `[unit-code].[application-code]`.
    pub application_mgrs: Arc<Mutex<HashMap<String, ApplicationMgr>>>,
    /// Network managers. Key is `[unit-code].[network-code]`. Unit code `_` means public network.
    pub network_mgrs: Arc<Mutex<HashMap<String, NetworkMgr>>>,
    /// Control channel receivers. Key is function such as `application`, `network`, ....
    pub ctrl_receivers: Arc<Mutex<HashMap<String, Queue>>>,
    /// Control channel senders.
    pub ctrl_senders: CtrlSenders,
    /// Data channel sender.
    pub data_sender: Option<Queue>,
}

/// Control channel senders.
#[derive(Clone)]
pub struct CtrlSenders {
    pub unit: Arc<Mutex<Queue>>,
    pub application: Arc<Mutex<Queue>>,
    pub network: Arc<Mutex<Queue>>,
    pub device: Arc<Mutex<Queue>>,
    pub device_route: Arc<Mutex<Queue>>,
    pub network_route: Arc<Mutex<Queue>>,
}

/// The sylvia-iot module specific error codes in addition to standard [`ErrResp`].
pub struct ErrReq;

struct DataSenderHandler;

impl ErrReq {
    pub const APPLICATION_EXIST: (u16, &'static str) = (400, "err_broker_application_exist");
    pub const APPLICATION_NOT_EXIST: (u16, &'static str) =
        (400, "err_broker_application_not_exist");
    pub const DEVICE_NOT_EXIST: (u16, &'static str) = (400, "err_broker_device_not_exist");
    pub const MEMBER_NOT_EXIST: (u16, &'static str) = (400, "err_broker_member_not_exist");
    pub const NETWORK_ADDR_EXIST: (u16, &'static str) = (400, "err_broker_network_addr_exist");
    pub const NETWORK_EXIST: (u16, &'static str) = (400, "err_broker_network_exist");
    pub const NETWORK_NOT_EXIST: (u16, &'static str) = (400, "err_broker_network_not_exist");
    pub const OWNER_NOT_EXIST: (u16, &'static str) = (400, "err_broker_owner_not_exist");
    pub const ROUTE_EXIST: (u16, &'static str) = (400, "err_broker_route_exist");
    pub const UNIT_EXIST: (u16, &'static str) = (400, "err_broker_unit_exist");
    pub const UNIT_NOT_EXIST: (u16, &'static str) = (400, "err_broker_unit_not_exist");
    pub const UNIT_NOT_MATCH: (u16, &'static str) = (400, "err_broker_unit_not_match");
}

#[async_trait]
impl QueueEventHandler for DataSenderHandler {
    async fn on_event(&self, queue: Arc<dyn MqQueue>, ev: Event) {
        const FN_NAME: &'static str = "DataSenderHandler::on_event";
        let queue_name = queue.name();

        match ev {
            Event::Error(e) => error!("[{}] {} error: {}", FN_NAME, queue_name, e),
            Event::Status(status) => match status {
                Status::Connected => info!("[{}] {} connected", queue_name, FN_NAME),
                _ => warn!("[{}] {} status to {:?}", FN_NAME, queue_name, status),
            },
        }
    }

    async fn on_message(&self, _queue: Arc<dyn MqQueue>, _msg: Box<dyn Message>) {}
}

/// To create resources for the service.
pub async fn new_state(
    scope_path: &'static str,
    conf: &Config,
) -> Result<State, Box<dyn StdError>> {
    let conf = config::apply_default(conf);
    let db_opts = match conf.db.as_ref().unwrap().engine.as_ref().unwrap().as_str() {
        DbEngine::MONGODB => {
            let conf = conf.db.as_ref().unwrap().mongodb.as_ref().unwrap();
            ConnOptions::MongoDB(MongoDbOptions {
                url: conf.url.as_ref().unwrap().to_string(),
                db: conf.database.as_ref().unwrap().to_string(),
                pool_size: conf.pool_size,
            })
        }
        _ => {
            let conf = conf.db.as_ref().unwrap().sqlite.as_ref().unwrap();
            ConnOptions::Sqlite(SqliteOptions {
                path: conf.path.as_ref().unwrap().to_string(),
            })
        }
    };
    let cache_opts = match conf.cache.as_ref().unwrap().engine.as_ref() {
        None => None,
        Some(engine) => match engine.as_str() {
            CacheEngine::MEMORY => {
                let conf = conf.cache.as_ref().unwrap().memory.as_ref().unwrap();
                Some(CacheConnOptions::Memory {
                    device: DeviceOptions {
                        uldata_size: conf.device.unwrap(),
                    },
                    device_route: DeviceRouteOptions {
                        uldata_size: conf.device_route.unwrap(),
                        dldata_size: conf.device_route.unwrap(),
                        dldata_pub_size: conf.device_route.unwrap(),
                    },
                    network_route: NetworkRouteOptions {
                        uldata_size: conf.device_route.unwrap(),
                    },
                })
            }
            _ => None,
        },
    };
    let mq_conf = conf.mq.as_ref().unwrap();
    let model = models::new(&db_opts).await?;
    let cache = match cache_opts {
        None => None,
        Some(opts) => Some(models::new_cache(&opts, &model).await?),
    };
    let auth_base = conf.auth.as_ref().unwrap().clone();
    let mq_conns = Arc::new(Mutex::new(HashMap::new()));
    let ch_conf = conf.mq_channels.as_ref().unwrap();
    let ctrl_senders = new_ctrl_senders(&mq_conns, &ch_conf, cache.clone())?;
    let data_sender = match ch_conf.data.as_ref() {
        None => None,
        Some(conf) => match conf.url.as_ref() {
            None => None,
            Some(_) => Some(new_data_sender(&mq_conns, conf)?),
        },
    };
    let state = State {
        scope_path,
        api_scopes: conf.api_scopes.as_ref().unwrap().clone(),
        model,
        cache,
        auth_base,
        amqp_prefetch: *mq_conf.prefetch.as_ref().unwrap(),
        mqtt_shared_prefix: mq_conf.shared_prefix.as_ref().unwrap().to_string(),
        client: reqwest::Client::new(),
        mq_conns,
        application_mgrs: Arc::new(Mutex::new(HashMap::new())),
        network_mgrs: Arc::new(Mutex::new(HashMap::new())),
        ctrl_receivers: Arc::new(Mutex::new(HashMap::new())),
        ctrl_senders,
        data_sender,
    };
    let (r1, r2, r3, r4, r5, r6) = tokio::join!(
        v1::unit::init(&state, &ch_conf.unit.as_ref().unwrap()),
        v1::application::init(&state, &ch_conf.application.as_ref().unwrap()),
        v1::network::init(&state, &ch_conf.network.as_ref().unwrap()),
        v1::device::init(&state, &ch_conf.device.as_ref().unwrap()),
        v1::device_route::init(&state, &ch_conf.device_route.as_ref().unwrap()),
        v1::network_route::init(&state, &ch_conf.network_route.as_ref().unwrap())
    );
    r1?;
    r2?;
    r3?;
    r4?;
    r5?;
    r6?;
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
        .service(v1::unit::new_service("/api/v1/unit", state))
        .service(v1::application::new_service("/api/v1/application", state))
        .service(v1::network::new_service("/api/v1/network", state))
        .service(v1::device::new_service("/api/v1/device", state))
        .service(v1::device_route::new_service("/api/v1/device-route", state))
        .service(v1::network_route::new_service(
            "/api/v1/network-route",
            state,
        ))
        .service(v1::dldata_buffer::new_service(
            "/api/v1/dldata-buffer",
            state,
        ))
}

pub fn new_ctrl_senders(
    mq_conns: &Arc<Mutex<HashMap<String, Connection>>>,
    ch_conf: &config::MqChannels,
    cache: Option<Arc<dyn Cache>>,
) -> Result<CtrlSenders, Box<dyn StdError>> {
    let unit_ctrl_cfg = ch_conf.unit.as_ref().unwrap();
    let app_ctrl_cfg = ch_conf.application.as_ref().unwrap();
    let net_ctrl_cfg = ch_conf.network.as_ref().unwrap();
    let dev_ctrl_cfg = ch_conf.device.as_ref().unwrap();
    let devr_ctrl_cfg = ch_conf.device_route.as_ref().unwrap();
    let netr_ctrl_cfg = ch_conf.network_route.as_ref().unwrap();

    Ok(CtrlSenders {
        unit: v1::unit::new_ctrl_sender(mq_conns, unit_ctrl_cfg)?,
        application: v1::application::new_ctrl_sender(mq_conns, app_ctrl_cfg)?,
        network: v1::network::new_ctrl_sender(mq_conns, net_ctrl_cfg, cache.clone())?,
        device: v1::device::new_ctrl_sender(mq_conns, dev_ctrl_cfg, cache.clone())?,
        device_route: v1::device_route::new_ctrl_sender(mq_conns, devr_ctrl_cfg, cache.clone())?,
        network_route: v1::network_route::new_ctrl_sender(mq_conns, netr_ctrl_cfg)?,
    })
}

/// Create data channel sender queue.
pub fn new_data_sender(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    config: &config::BrokerData,
) -> Result<Queue, Box<dyn StdError>> {
    let url = match config.url.as_ref() {
        None => {
            return Err(Box::new(IoError::new(
                ErrorKind::InvalidInput,
                "empty control url",
            )))
        }
        Some(url) => match Url::parse(url.as_str()) {
            Err(e) => return Err(Box::new(e)),
            Ok(url) => url,
        },
    };

    match mq::data::new(conn_pool, &url, Arc::new(DataSenderHandler {})) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(q) => Ok(q),
    }
}
