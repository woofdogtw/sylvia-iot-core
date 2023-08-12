use std::{
    collections::HashMap,
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
};

use actix_web::{dev::HttpServiceFactory, error, web, HttpResponse, Responder};
use async_trait::async_trait;
use log::{error, info, warn};
use reqwest;
use serde::{Deserialize, Serialize};
use url::Url;

use general_mq::{
    queue::{Event, EventHandler as QueueEventHandler, GmqQueue, Message, Status},
    Queue,
};
use sylvia_iot_corelib::{constants::MqEngine, err::ErrResp};

use crate::libs::{
    config::{self, Config},
    mq::{
        self, emqx::ManagementOpts as EmqxOpts, rabbitmq::ManagementOpts as RabbitMqOpts,
        Connection,
    },
};

pub mod middleware;
mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `/coremgr`, the APIs are
    /// - `http://host:port/coremgr/api/v1/user/xxx`
    /// - `http://host:port/coremgr/api/v1/unit/xxx`
    pub scope_path: &'static str,
    /// The sylvia-iot-auth base API path with host.
    ///
    /// For example, `http://localhost:1080/auth`.
    pub auth_base: String,
    /// The sylvia-iot-broker base API path with host.
    ///
    /// For example, `http://localhost:2080/broker`.
    pub broker_base: String,
    /// The client for internal HTTP requests.
    pub client: reqwest::Client,
    /// AMQP broker management information.
    pub amqp: AmqpState,
    /// MQTT broker management information.
    pub mqtt: MqttState,
    /// Queue connections. Key is uri.
    pub mq_conns: Arc<Mutex<HashMap<String, Connection>>>,
    /// Data channel sender.
    pub data_sender: Option<Queue>,
}

/// AMQP broker management information.
#[derive(Clone)]
pub enum AmqpState {
    /// For RabbitMQ.
    RabbitMq(RabbitMqOpts),
}

/// MQTT broker management information.
#[derive(Clone)]
pub enum MqttState {
    /// For EMQX.
    Emqx(EmqxOpts),
    /// For rumqttd.
    Rumqttd,
}

/// The sylvia-iot module specific error codes in addition to standard [`ErrResp`].
pub struct ErrReq;

struct DataSenderHandler;

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

impl ErrReq {
    pub const UNIT_NOT_EXIST: (u16, &'static str) = (400, "err_broker_unit_not_exist");
    pub const DEVICE_NOT_EXIST: (u16, &'static str) = (400, "err_broker_device_not_exist");
}

#[async_trait]
impl QueueEventHandler for DataSenderHandler {
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: Event) {
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

    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, _msg: Box<dyn Message>) {}
}

/// To create resources for the service.
pub async fn new_state(
    scope_path: &'static str,
    conf: &Config,
) -> Result<State, Box<dyn StdError>> {
    let conf = config::apply_default(conf);
    let auth_base = conf.auth.as_ref().unwrap().clone();
    let broker_base = conf.broker.as_ref().unwrap().clone();
    let mq_engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    let amqp = {
        let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
        AmqpState::RabbitMq(RabbitMqOpts {
            username: rabbitmq.username.as_ref().unwrap().clone(),
            password: rabbitmq.password.as_ref().unwrap().clone(),
            ttl: rabbitmq.ttl,
            length: rabbitmq.length,
        })
    };
    let mqtt = match mq_engine.mqtt.as_ref().unwrap().as_str() {
        MqEngine::RUMQTTD => MqttState::Rumqttd,
        _ => {
            let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
            MqttState::Emqx(EmqxOpts {
                api_key: emqx.api_key.as_ref().unwrap().clone(),
                api_secret: emqx.api_secret.as_ref().unwrap().clone(),
            })
        }
    };
    let mq_conns = Arc::new(Mutex::new(HashMap::new()));
    let ch_conf = conf.mq_channels.as_ref().unwrap();
    let data_sender = match ch_conf.data.as_ref() {
        None => None,
        Some(conf) => match conf.url.as_ref() {
            None => None,
            Some(_) => Some(new_data_sender(&mq_conns, conf)?),
        },
    };
    let state = State {
        scope_path,
        auth_base,
        broker_base,
        client: reqwest::Client::new(),
        amqp,
        mqtt,
        mq_conns,
        data_sender,
    };
    Ok(state)
}

/// To register service URIs in the specified root path.
pub fn new_service(state: &State) -> impl HttpServiceFactory {
    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    web::scope(state.scope_path)
        .wrap(middleware::LogService::new(
            auth_uri,
            state.data_sender.clone(),
        ))
        .app_data(web::JsonConfig::default().error_handler(|err, _| {
            error::ErrorBadRequest(ErrResp::ErrParam(Some(err.to_string())))
        }))
        .app_data(web::Data::new(state.clone()))
        .service(v1::auth::new_service("/api/v1/auth"))
        .service(v1::user::new_service("/api/v1/user"))
        .service(v1::client::new_service("/api/v1/client"))
        .service(v1::unit::new_service("/api/v1/unit"))
        .service(v1::application::new_service("/api/v1/application"))
        .service(v1::network::new_service("/api/v1/network"))
        .service(v1::device::new_service("/api/v1/device"))
        .service(v1::device_route::new_service("/api/v1/device-route"))
        .service(v1::network_route::new_service("/api/v1/network-route"))
        .service(v1::dldata_buffer::new_service("/api/v1/dldata-buffer"))
}

/// Create data channel sender queue.
fn new_data_sender(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    config: &config::CoremgrData,
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
    let persistent = match config.persistent {
        None => false,
        Some(persistent) => persistent,
    };

    match mq::data::new(conn_pool, &url, persistent, Arc::new(DataSenderHandler {})) {
        Err(e) => Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(q) => Ok(q),
    }
}

pub async fn get_version(query: web::Query<GetVersionQuery>) -> impl Responder {
    if let Some(q) = query.q.as_ref() {
        match q.as_str() {
            "name" => return HttpResponse::Ok().body(SERV_NAME),
            "version" => return HttpResponse::Ok().body(SERV_VER),
            _ => (),
        }
    }

    HttpResponse::Ok().json(GetVersionRes {
        data: GetVersionResData {
            name: SERV_NAME,
            version: SERV_VER,
        },
    })
}
