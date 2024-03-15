use laboratory::{describe, Suite};
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisResult};
use tokio::runtime::Runtime;

use sylvia_iot_auth::models::redis::conn::{self as models_conn, Options};

use crate::TestState;

mod access_token;
mod authorization_code;
mod conn;
mod refresh_token;

pub const STATE: &'static str = "models/redis";

pub fn get_test_db_path() -> String {
    "redis://localhost:6379".to_string()
}

pub fn suite() -> Suite<TestState> {
    describe("models.redis", |context| {
        context.describe("conn", |context| {
            context.it("connect", conn::conn);
        });

        context.describe_import(describe("collections", |context| {
            context.describe("access_token", |context| {
                context.it("get()", access_token::get);
                context.it("add()", access_token::add);
                context.it("del()", access_token::del);
            });

            context.describe("authorization_code", |context| {
                context.it("get() with none", authorization_code::get_none);
                context.it("get() with some", authorization_code::get_some);
                context.it("add() with none", authorization_code::add_none);
                context.it("add() with some", authorization_code::add_some);
                context.it("del()", authorization_code::del);
            });

            context.describe("refresh_token", |context| {
                context.it("get()", refresh_token::get);
                context.it("add()", refresh_token::add);
                context.it("del()", refresh_token::del);
            });

            context
                .before_all(|state| {
                    state.insert(STATE, new_state(true));
                })
                .after_all(|state| {
                    let state = state.get_mut(STATE).unwrap();
                    let runtime = &mut state.runtime;
                    let pool = state.redis.as_mut().unwrap();
                    let _ = runtime
                        .as_ref()
                        .unwrap()
                        .block_on(async { remove_db(pool).await });
                });
        }));

        context.before_all(|state| {
            state.insert(STATE, new_state(false));
        });
    })
}

fn new_state(with_pool: bool) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    if !with_pool {
        return TestState {
            runtime: Some(runtime),
            ..Default::default()
        };
    }
    let pool = match runtime.block_on(async {
        models_conn::connect(&Options {
            url: get_test_db_path().to_string(),
        })
        .await
    }) {
        Err(e) => panic!("create pool error: {}", e),
        Ok(pool) => Some(pool),
    };
    TestState {
        runtime: Some(runtime),
        redis: pool,
        ..Default::default()
    }
}

async fn remove_db(conn: &mut MultiplexedConnection) {
    let result: RedisResult<Vec<String>> = conn.keys("*").await;
    let result = match result {
        Err(_) => {
            return;
        }
        Ok(str) => str,
    };
    let _: RedisResult<()> = conn.del(result).await;
}
