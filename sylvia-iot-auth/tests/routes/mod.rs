use std::{collections::HashMap, env, fs};

use actix_web::{dev::ServiceResponse, http::header};
use laboratory::{describe, expect, SpecContext, Suite};
use url::Url;

use sylvia_iot_auth::{
    libs::config::{self, Config},
    models::{self, ConnOptions, SqliteOptions},
    routes,
};
use sylvia_iot_corelib::constants::DbEngine;

use crate::TestState;

mod libs;
pub mod oauth2;
pub mod v1;

use libs::new_state;

pub const STATE: &'static str = "routes";

pub fn suite() -> Suite<TestState> {
    describe("routes", |context| {
        context.it("new_state", fn_new_state);
        context.it("new_service", fn_new_service);
        context.it("new_service with API scopes", fn_api_scopes);

        context.before_all(|state| {
            state.insert(STATE, new_state(None));
        });
        context.after_all(|_state| {
            remove_sqlite(config::DEF_SQLITE_PATH);
            let mut path = std::env::temp_dir();
            path.push(config::DEF_SQLITE_PATH);
            remove_sqlite(path.to_str().unwrap());
            let mut path = std::env::temp_dir();
            path.push(crate::TEST_SQLITE_PATH);
            remove_sqlite(path.to_str().unwrap());
        });
    })
}

fn read_location(resp: &ServiceResponse) -> Result<Url, String> {
    let location = match resp.headers().get(header::LOCATION) {
        None => return Err("no location header".to_string()),
        Some(location) => match location.to_str() {
            Err(e) => return Err(format!("location to_str() error: {}", e)),
            Ok(location) => location,
        },
    };
    match Url::parse(location) {
        Err(e) => match e {
            url::ParseError::RelativeUrlWithoutBase => {
                let url_with_base = format!("http://localhost{}", location);
                match Url::parse(url_with_base.as_str()) {
                    Err(e) => return Err(format!("parse url with base error: {}", e)),
                    Ok(url) => return Ok(url),
                }
            }
            _ => return Err(format!("parse url error: {}", e)),
        },
        Ok(url) => return Ok(url),
    }
}

fn remove_sqlite(path: &str) {
    if let Err(e) = std::fs::remove_file(path) {
        println!("remove file {} error: {}", path, e);
    }
    let file = format!("{}-shm", path);
    if let Err(e) = std::fs::remove_file(file.as_str()) {
        println!("remove file {} error: {}", file.as_str(), e);
    }
    let file = format!("{}-wal", path);
    if let Err(e) = std::fs::remove_file(file.as_str()) {
        println!("remove file {} error: {}", file.as_str(), e);
    }
}

fn fn_new_state(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let conf = Config {
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("default config error: {}", e)),
        Ok(state) => match runtime.block_on(async { state.model.close().await }) {
            Err(e) => return Err(format!("disconnect default model error: {}", e)),
            Ok(_) => state,
        },
    };
    expect(state.scope_path).to_equal("scope")?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some(DbEngine::MONGODB.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("mongodb config error: {}", e)),
        Ok(state) => match runtime.block_on(async { state.model.close().await }) {
            Err(e) => return Err(format!("disconnect mongodb model error: {}", e)),
            Ok(_) => state,
        },
    };
    expect(state.scope_path).to_equal("scope")?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some(DbEngine::SQLITE.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("sqlite config error: {}", e)),
        Ok(state) => match runtime.block_on(async { state.model.close().await }) {
            Err(e) => return Err(format!("disconnect sqlite model error: {}", e)),
            Ok(_) => state,
        },
    };
    expect(state.scope_path).to_equal("scope")?;

    let conf = Config {
        db: Some(config::Db {
            engine: Some("test".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("test config error: {}", e)),
        Ok(state) => match runtime.block_on(async { state.model.close().await }) {
            Err(e) => return Err(format!("disconnect test model error: {}", e)),
            Ok(_) => state,
        },
    };
    expect(state.scope_path).to_equal("scope")
}

fn fn_new_service(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => return Err(format!("new model error: {}", e)),
        Ok(model) => model,
    };

    let _ = routes::new_service(&routes::State {
        scope_path: "test",
        api_scopes: HashMap::new(),
        templates: HashMap::new(),
        model: model.clone(),
    });
    if let Err(e) = runtime.block_on(async { model.close().await }) {
        return Err(format!("close model error: {}", e));
    }
    Ok(())
}

fn fn_api_scopes(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => return Err(format!("new model error: {}", e)),
        Ok(model) => model,
    };

    let mut api_scopes: HashMap<String, Vec<String>> = HashMap::new();
    api_scopes.insert("auth.tokeninfo.get".to_string(), vec![]);
    api_scopes.insert("auth.logout.post".to_string(), vec![]);
    api_scopes.insert("user.get".to_string(), vec![]);
    api_scopes.insert("user.patch".to_string(), vec![]);
    api_scopes.insert("user.post.admin".to_string(), vec![]);
    api_scopes.insert("user.get.admin".to_string(), vec![]);
    api_scopes.insert("user.patch.admin".to_string(), vec![]);
    api_scopes.insert("user.delete.admin".to_string(), vec![]);
    api_scopes.insert("client.post".to_string(), vec![]);
    api_scopes.insert("client.get".to_string(), vec![]);
    api_scopes.insert("client.patch".to_string(), vec![]);
    api_scopes.insert("client.delete".to_string(), vec![]);
    api_scopes.insert("client.delete.user".to_string(), vec![]);

    let template_path = env::temp_dir().join("path");
    if let Err(_) = fs::write(template_path.as_path(), "<html></html>") {
        return Err("cannot write template".to_string());
    }
    let mut templates: HashMap<String, String> = HashMap::new();
    templates.insert(
        "login".to_string(),
        template_path.to_str().unwrap().to_string(),
    );
    templates.insert(
        "grant".to_string(),
        template_path.to_str().unwrap().to_string(),
    );

    let _ = routes::new_service(&routes::State {
        scope_path: "test",
        api_scopes,
        templates,
        model: model.clone(),
    });
    if let Err(e) = runtime.block_on(async { model.close().await }) {
        return Err(format!("close model error: {}", e));
    }
    Ok(())
}
