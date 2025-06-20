use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use laboratory::{Suite, describe};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use tokio::{runtime::Runtime, time};
use url::Url;

use general_mq::{
    AmqpConnection, AmqpConnectionOptions, MqttConnection, MqttConnectionOptions,
    connection::{GmqConnection, Status},
    queue::GmqQueue,
};
use sylvia_iot_sdk::mq::Connection;

pub mod application;
pub mod network;

use crate::{TestState, WAIT_COUNT, WAIT_TICK};

pub struct MqEngine;

#[derive(Deserialize)]
struct QueueInfo {
    name: String,
}

impl MqEngine {
    pub const EMQX: &'static str = "emqx";
    pub const RABBITMQ: &'static str = "rabbitmq";
}

const STATE: &'static str = "libs/mq";

pub fn suite(mq_engine: &'static str) -> Suite<TestState> {
    describe(format!("libs.mq - {}", mq_engine), move |context| {
        context.describe("ApplicationMgr", |context| {
            context.it("new() with default options", application::new_default);
            context.it("new() with manual options", application::new_manual);
            context.it("new() with wrong opts", application::new_wrong_opts);
            context.it("close()", application::close);

            context.it("uldata", application::uldata);
            context.it("uldata with wrong content", application::uldata_wrong);
            context.it("dldata", application::dldata);
            context.it("dldata with wrong content", application::dldata_wrong);
            context.it("dldata-resp", application::dldata_resp);
            context.it(
                "dldata-resp with wrong content",
                application::dldata_resp_wrong,
            );
            context.it("dldata-result", application::dldata_result);
            context.it(
                "dldata-result with wrong content",
                application::dldata_result_wrong,
            );

            context.after_each(after_each_fn);
        });

        context.describe("NetworkMgr", |context| {
            context.it("new() with default options", network::new_default);
            context.it("new() with manual options", network::new_manual);
            context.it("new() with wrong opts", network::new_wrong_opts);
            context.it("close()", network::close);

            context.it("uldata", network::uldata);
            context.it("uldata with wrong content", network::uldata_wrong);
            context.it("dldata", network::dldata);
            context.it("dldata with wrong content", network::dldata_wrong);
            context.it("dldata-result", network::dldata_result);
            context.it(
                "dldata-result with wrong content",
                network::dldata_result_wrong,
            );
            context.it("ctrl", network::ctrl);
            context.it("ctrl with wrong content", network::ctrl_wrong);

            context.after_each(after_each_fn);
        });

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(mq_engine));
            })
            .after_all(|state| {
                let state = state.get(STATE).unwrap();
                remove_rabbitmq_queues(state);
            });
    })
}

fn new_connection(runtime: &Runtime, mq_engine: &str) -> Result<Connection, String> {
    match mq_engine {
        MqEngine::RABBITMQ => {
            let opts = AmqpConnectionOptions {
                ..Default::default()
            };
            let mut conn = AmqpConnection::new(opts)?;
            if let Err(e) = conn.connect() {
                return Err(format!("connect() error: {}", e));
            }
            for _ in 0..WAIT_COUNT {
                if conn.status() == Status::Connected {
                    break;
                }
                runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
            }
            Ok(Connection::Amqp(conn, Arc::new(Mutex::new(0))))
        }
        MqEngine::EMQX => {
            let opts = MqttConnectionOptions {
                ..Default::default()
            };
            let mut conn = MqttConnection::new(opts)?;
            if let Err(e) = conn.connect() {
                return Err(format!("connect() error: {}", e));
            }
            for _ in 0..WAIT_COUNT {
                if conn.status() == Status::Connected {
                    break;
                }
                runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
            }
            Ok(Connection::Mqtt(conn, Arc::new(Mutex::new(0))))
        }
        _ => Err(format!("unsupport mq_engine {}", mq_engine)),
    }
}

fn new_state(mq_engine: &'static str) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    TestState {
        runtime: Some(runtime),
        mq_engine: Some(mq_engine.to_string()),
        mgr_conns: Some(Arc::new(Mutex::new(HashMap::new()))),
        mqtt_shared_prefix: Some("$share/sylvia-iot-sdk/".to_string()),
        ..Default::default()
    }
}

fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(mgrs) = state.app_mgrs.take() {
        runtime.block_on(async {
            for i in mgrs.iter() {
                let _ = i.close().await;
            }
        });
    }
    if let Some(mgrs) = state.net_mgrs.take() {
        runtime.block_on(async {
            for i in mgrs.iter() {
                let _ = i.close().await;
            }
        });
    }
    if let Some(mq_conns) = state.mgr_conns.as_ref() {
        runtime.block_on(async move {
            let conns = { mq_conns.lock().unwrap().clone() };
            for (_, conn) in conns {
                match conn {
                    Connection::Amqp(mut conn, _) => {
                        let _ = conn.close().await;
                    }
                    Connection::Mqtt(mut conn, _) => {
                        let _ = conn.close().await;
                    }
                }
            }
            (*mq_conns.lock().unwrap()).clear();
        });
    }
    if let Some(mut queues) = state.app_net_queues.take() {
        runtime.block_on(async {
            for i in queues.iter_mut() {
                let _ = i.close().await;
            }
        });
    }
    if let Some(conn) = state.app_net_conn.take() {
        runtime.block_on(async {
            match conn {
                Connection::Amqp(mut conn, _) => {
                    let _ = conn.close().await;
                }
                Connection::Mqtt(mut conn, _) => {
                    let _ = conn.close().await;
                }
            }
        });
    }
}

fn conn_host_uri(mq_engine: &str) -> Result<Url, String> {
    match mq_engine {
        MqEngine::RABBITMQ => match Url::parse(crate::TEST_AMQP_HOST_URI) {
            Err(e) => Err(format!("AMQP URI error: {}", e)),
            Ok(uri) => Ok(uri),
        },
        MqEngine::EMQX => match Url::parse(crate::TEST_MQTT_HOST_URI) {
            Err(e) => Err(format!("MQTT URI error: {}", e)),
            Ok(uri) => Ok(uri),
        },
        _ => Err(format!("unsupport mq_engine {}", mq_engine)),
    }
}

fn remove_rabbitmq_queues(state: &TestState) {
    let runtime = state.runtime.as_ref().unwrap();
    let client = reqwest::Client::new();

    let req = match client
        .request(Method::GET, "http://localhost:15672/api/queues/%2f")
        .basic_auth("guest", Some("guest"))
        .build()
    {
        Err(e) => {
            println!("generate get request error: {}", e);
            return;
        }
        Ok(req) => req,
    };
    if let Err(e) = runtime.block_on(async {
        let resp = match client.execute(req).await {
            Err(e) => return Err(format!("execute get request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::OK => resp,
                _ => {
                    return Err(format!(
                        "execute get request with status: {}",
                        resp.status()
                    ));
                }
            },
        };
        let queues = match resp.json::<Vec<QueueInfo>>().await {
            Err(e) => return Err(format!("get response error: {}", e)),
            Ok(resp) => resp,
        };

        for queue in queues {
            if queue.name.starts_with("amq.") {
                continue;
            }
            let uri = format!("http://localhost:15672/api/queues/%2f/{}", queue.name);
            let req = match client
                .request(Method::DELETE, uri)
                .basic_auth("guest", Some("guest"))
                .build()
            {
                Err(e) => {
                    return Err(format!("generate delete request error: {}", e));
                }
                Ok(req) => req,
            };
            match client.execute(req).await {
                Err(e) => return Err(format!("execute delete request error: {}", e)),
                Ok(resp) => match resp.status() {
                    StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                    _ => println!("delete queue {} error: {}", queue.name, resp.status()),
                },
            };
        }
        Ok(())
    }) {
        println!("{}", e);
    }
}
