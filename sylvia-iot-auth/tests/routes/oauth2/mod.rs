use std::collections::HashMap;

use chrono::{TimeDelta, Utc};
use laboratory::{describe, Suite};
use tokio::runtime::Runtime;

use sylvia_iot_auth::models::{
    mongodb_conn::{self, Options as MongoDbOptions},
    Model,
};

use super::{
    libs::{create_client, create_user, new_state},
    remove_sqlite,
};
use crate::TestState;

mod api;
mod request;
mod response;

pub const STATE: &'static str = "routes/oauth2";

pub fn suite(db_engine: &'static str) -> Suite<TestState> {
    describe(format!("routes.oauth2 - {}", db_engine), move |context| {
        context.it("GET /oauth2/auth", api::get_auth);
        context.it("GET /oauth2/login", api::get_login);
        context.it("POST /oauth2/login", api::post_login);
        context.it("GET /oauth2/authorize", api::get_authorize);
        context.it("POST /oauth2/authorize", api::post_authorize);
        context.it(
            "POST /oauth2/token with authorization code",
            api::post_token,
        );
        context.it(
            "POST /oauth2/token with client credentials",
            api::post_token_client,
        );
        context.it("POST /oauth2/refresh", api::post_refresh);
        context.it("middleware with API scopes", api::middleware_api_scope);

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(db_engine)));
                let state = state.get(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();
                let model = state.routes_state.as_ref().unwrap().model.as_ref();
                before_all_dataset(runtime, model);
            })
            .after_all(after_all_fn);
    })
}

fn before_all_dataset(runtime: &Runtime, model: &dyn Model) {
    runtime.block_on(async {
        let now = Utc::now();
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("admin".to_string(), true);
        if let Err(e) = model.user().add(&create_user("admin", now, roles)).await {
            println!("add user admin error: {}", e);
        }

        let now = now + TimeDelta::try_seconds(1).unwrap();
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("dev".to_string(), true);
        if let Err(e) = model.user().add(&create_user("dev", now, roles)).await {
            println!("add user dev error: {}", e);
        }

        let now = now + TimeDelta::try_seconds(1).unwrap();
        let mut roles = HashMap::<String, bool>::new();
        roles.insert("manager".to_string(), true);
        if let Err(e) = model.user().add(&create_user("manager", now, roles)).await {
            println!("add user manager error: {}", e);
        }

        let now = now + TimeDelta::try_seconds(1).unwrap();
        let roles = HashMap::<String, bool>::new();
        if let Err(e) = model.user().add(&create_user("user", now, roles)).await {
            println!("add user user error: {}", e);
        }

        let client = create_client("public", "dev", None);
        if let Err(e) = model.client().add(&client).await {
            println!("add client public error: {}", e);
        }

        let mut client = create_client("private", "dev", Some("private".to_string()));
        client.scopes = vec!["scope1".to_string()];
        if let Err(e) = model.client().add(&client).await {
            println!("add client private error: {}", e);
        }

        let mut client = create_client("no-redirect", "dev", Some("no-redirect".to_string()));
        client.redirect_uris = vec![];
        if let Err(e) = model.client().add(&client).await {
            println!("add client no-redirect error: {}", e);
        }

        let mut client = create_client("bad-redirect", "dev", Some("bad-redirect".to_string()));
        client.redirect_uris = vec![":://".to_string()];
        if let Err(e) = model.client().add(&client).await {
            println!("add client bad-redirect error: {}", e);
        }
    });
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    runtime.block_on(async {
        let opts = MongoDbOptions {
            url: crate::TEST_MONGODB_URL.to_string(),
            db: crate::TEST_MONGODB_DB.to_string(),
            pool_size: None,
        };
        let conn = match mongodb_conn::connect(&opts).await {
            Err(_) => return (),
            Ok(conn) => conn,
        };
        if let Err(e) = conn.drop().await {
            println!("remove database error: {}", e);
        }
    });
    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}
