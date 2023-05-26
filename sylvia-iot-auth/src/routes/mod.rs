use std::{collections::HashMap, error::Error as StdError, sync::Arc};

use actix_web::{dev::HttpServiceFactory, error, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use sylvia_iot_corelib::{constants::DbEngine, err::ErrResp};

use crate::{
    libs::config::{self, Config},
    models::{self, ConnOptions, Model, MongoDbOptions, SqliteOptions},
};

pub mod oauth2;
mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `/auth`, the APIs are
    /// - `http://host:port/auth/oauth2/xxx`
    /// - `http://host:port/auth/api/v1/user/xxx`
    /// - `http://host:port/auth/api/v1/client/xxx`
    pub scope_path: &'static str,
    /// The scopes for accessing APIs.
    pub api_scopes: HashMap<String, Vec<String>>,
    /// Jinja2 templates.
    pub templates: HashMap<String, String>,
    /// The database model.
    pub model: Arc<dyn Model>,
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
    pub const USER_EXIST: (u16, &'static str) = (400, "err_auth_user_exist");
    pub const USER_NOT_EXIST: (u16, &'static str) = (400, "err_auth_user_not_exist");
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
    Ok(State {
        scope_path,
        api_scopes: conf.api_scopes.as_ref().unwrap().clone(),
        templates: conf.templates.as_ref().unwrap().clone(),
        model,
    })
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
        .service(web::scope("/oauth2").configure(oauth2::gen_configure(state)))
        .service(v1::auth::new_service("/api/v1/auth", state))
        .service(v1::user::new_service("/api/v1/user", state))
        .service(v1::client::new_service("/api/v1/client", state))
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
