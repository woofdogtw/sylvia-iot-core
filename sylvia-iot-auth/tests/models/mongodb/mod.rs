use std::collections::HashMap;

use laboratory::{Suite, describe};
use tokio::runtime::Runtime;

use sylvia_iot_auth::models::{MongoDbModel, MongoDbOptions};

use crate::TestState;

mod access_token;
mod authorization_code;
mod client;
mod conn;
mod login_session;
mod refresh_token;
mod user;

pub const STATE: &'static str = "models/mongodb";

pub fn suite() -> Suite<TestState> {
    describe("models.mongodb", |context| {
        context.describe("conn", |context| {
            context.it("connect", conn::conn);
            context.it("models::new()", conn::models_new);
        });

        context.describe_import(describe("collections", |context| {
            context.describe("access_token", |context| {
                context.it("init()", access_token::init);
                context.it("get()", access_token::get);
                context.it("add()", access_token::add);
                context.it("add() with duplicate token", access_token::add_dup);
                context.it("del() by access token", access_token::del_by_access_token);
                context.it("del() by refresh token", access_token::del_by_refresh_token);
                context.it("del() twice", access_token::del_twice);
                context.it("del() by client_id", access_token::del_by_client_id);
                context.it("del() by user_id", access_token::del_by_user_id);
                context.it("del() by user and client", access_token::del_by_user_client);

                context.after_each(access_token::after_each_fn);
            });

            context.describe("authorization_code", |context| {
                context.it("init()", authorization_code::init);
                context.it("get()", authorization_code::get);
                context.it("add()", authorization_code::add);
                context.it("add() with duplicate code", authorization_code::add_dup);
                context.it("del() by code", authorization_code::del_by_code);
                context.it("del() twice", authorization_code::del_twice);
                context.it("del() by client_id", authorization_code::del_by_client_id);
                context.it("del() by user_id", authorization_code::del_by_user_id);
                context.it(
                    "del() by user and client",
                    authorization_code::del_by_user_client,
                );

                context.after_each(authorization_code::after_each_fn);
            });

            context.describe("client", |context| {
                context.it("init()", client::init);
                context.it("get() by client_id", client::get_by_client_id);
                context.it("get() by user and client", client::get_by_user_client);
                context.it("add()", client::add);
                context.it("add() with duplicate ID", client::add_dup);
                context.it("del() by client_id", client::del_by_client_id);
                context.it("del() twice", client::del_twice);
                context.it("del() by user_id", client::del_by_user_id);
                context.it("del() by user and client", client::del_by_user_client);
                context.it("update()", client::update);
                context.it("update() not exist", client::update_not_exist);
                context.it("update() with invalid options", client::update_invalid);
                context.it("count()", client::count);
                context.it("list()", client::list);
                context.it("list() sort", client::list_sort);
                context.it("list() offset limit", client::list_offset_limit);
                context.it("list() cursor", client::list_cursor);

                context.after_each(client::after_each_fn);
            });

            context.describe("login_session", |context| {
                context.it("init()", login_session::init);
                context.it("get()", login_session::get);
                context.it("add()", login_session::add);
                context.it("add() with duplicate session", login_session::add_dup);
                context.it("del() by session_id", login_session::del_by_session);
                context.it("del() twice", login_session::del_twice);
                context.it("del() by user_id", login_session::del_by_user_id);

                context.after_each(login_session::after_each_fn);
            });

            context.describe("refresh_token", |context| {
                context.it("init()", refresh_token::init);
                context.it("get()", refresh_token::get);
                context.it("add()", refresh_token::add);
                context.it("add() with duplicate token", refresh_token::add_dup);
                context.it("del() by refresh token", refresh_token::del_by_token);
                context.it("del() twice", refresh_token::del_twice);
                context.it("del() by client_id", refresh_token::del_by_client_id);
                context.it("del() by user_id", refresh_token::del_by_user_id);
                context.it(
                    "del() by user and client",
                    refresh_token::del_by_user_client,
                );

                context.after_each(refresh_token::after_each_fn);
            });

            context.describe("user", |context| {
                context.it("init()", user::init);
                context.it("get() by user_id", user::get_by_user_id);
                context.it("get() by account", user::get_by_account);
                context.it("add()", user::add);
                context.it("add() with duplicate ID and account", user::add_dup);
                context.it("del()", user::del);
                context.it("del() twice", user::del_twice);
                context.it("update()", user::update);
                context.it("update() not exist", user::update_not_exist);
                context.it("update() with invalid options", user::update_invalid);
                context.it("count()", user::count);
                context.it("list()", user::list);
                context.it("list() sort", user::list_sort);
                context.it("list() offset limit", user::list_offset_limit);
                context.it("list() cursor", user::list_cursor);

                context.after_each(user::after_each_fn);
            });

            context
                .before_all(|state| {
                    state.insert(STATE, new_state(true));
                })
                .after_all(collections_after_all);
        }));

        context.before_all(|state| {
            state.insert(STATE, new_state(false));
        });
    })
}

fn collections_after_all(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let _ = state.runtime.as_ref().unwrap().block_on(async {
        state
            .mongodb
            .as_ref()
            .unwrap()
            .get_connection()
            .drop()
            .await
    });
}

fn new_state(with_model: bool) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    if !with_model {
        return TestState {
            runtime: Some(runtime),
            ..Default::default()
        };
    }
    let model = match runtime.block_on(async {
        MongoDbModel::new(&MongoDbOptions {
            url: crate::TEST_MONGODB_URL.to_string(),
            db: crate::TEST_MONGODB_DB.to_string(),
            pool_size: None,
        })
        .await
    }) {
        Err(e) => panic!("create model error: {}", e),
        Ok(model) => Some(model),
    };
    TestState {
        runtime: Some(runtime),
        mongodb: model,
        ..Default::default()
    }
}
