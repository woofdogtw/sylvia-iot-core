use std::{collections::HashMap, error::Error as StdError, sync::Arc};

use actix_web::{dev::HttpServiceFactory, error, web, HttpResponse, Responder};
use general_mq::Queue;
use reqwest;
use serde::{Deserialize, Serialize};

use sylvia_iot_corelib::{constants::DbEngine, err::ErrResp};

use crate::{
    libs::{
        config::{self, Config},
        mq::{self, Connection},
    },
    models::{self, ConnOptions, Model, MongoDbOptions, SqliteOptions},
};

pub mod middleware;
mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `/data`, the APIs are
    /// - `http://host:port/data/api/v1/application-uldata/xxx`
    /// - `http://host:port/data/api/v1/network-uldata/xxx`
    pub scope_path: &'static str,
    /// The database model.
    pub model: Arc<dyn Model>,
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
    /// Queue connections. Key is uri.
    pub mq_conns: HashMap<String, Connection>,
    /// Data channel receivers. Key is data channel name such as `broker.data`, `coremgr.data`, ...
    pub data_receivers: HashMap<String, Queue>,
}

/// The sylvia-iot module specific error codes in addition to standard [`ErrResp`].
pub struct ErrReq;

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
    pub const UNIT_NOT_EXIST: (u16, &'static str) = (400, "err_data_unit_not_exist");
    pub const USER_NOT_EXIST: (u16, &'static str) = (400, "err_data_user_not_exist");
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
    let model = models::new(&db_opts).await?;
    let auth_base = conf.auth.as_ref().unwrap().clone();
    let broker_base = conf.broker.as_ref().unwrap().clone();
    let mut mq_conns = HashMap::new();
    let ch_conf = conf.mq_channels.as_ref().unwrap();
    let data_receivers = new_data_receivers(&model, &mut mq_conns, ch_conf)?;
    let state = State {
        scope_path,
        model,
        auth_base,
        broker_base,
        client: reqwest::Client::new(),
        mq_conns,
        data_receivers,
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
        .service(v1::application_uldata::new_service(
            "/api/v1/application-uldata",
            state,
        ))
        .service(v1::application_dldata::new_service(
            "/api/v1/application-dldata",
            state,
        ))
        .service(v1::network_uldata::new_service(
            "/api/v1/network-uldata",
            state,
        ))
        .service(v1::network_dldata::new_service(
            "/api/v1/network-dldata",
            state,
        ))
        .service(v1::coremgr_opdata::new_service(
            "/api/v1/coremgr-opdata",
            state,
        ))
}

pub fn new_data_receivers(
    model: &Arc<dyn Model>,
    mq_conns: &mut HashMap<String, Connection>,
    ch_conf: &config::MqChannels,
) -> Result<HashMap<String, Queue>, Box<dyn StdError>> {
    let mut data_receivers = HashMap::<String, Queue>::new();

    let conf = ch_conf.broker.as_ref().unwrap();
    let q = mq::broker::new(model.clone(), mq_conns, &conf)?;
    data_receivers.insert("broker.data".to_string(), q);

    let conf = ch_conf.coremgr.as_ref().unwrap();
    let q = mq::coremgr::new(model.clone(), mq_conns, &conf)?;
    data_receivers.insert("coremgr.data".to_string(), q);

    Ok(data_receivers)
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
