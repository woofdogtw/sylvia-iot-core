use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use general_mq::{
    connection::{Connection as MqConnection, Status},
    queue::Queue as MqQueue,
    AmqpConnection, AmqpConnectionOptions, MqttConnection, MqttConnectionOptions,
};
use laboratory::{describe, Suite};
use tokio::{runtime::Runtime, time};

use sylvia_iot_broker::libs::mq::Connection;
use sylvia_iot_corelib::constants::MqEngine;

pub mod application;
pub mod control;
pub mod network;

use super::libs::remove_rabbitmq_queues;
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

const STATE: &'static str = "libs/mq";

pub fn suite(mq_engine: &'static str) -> Suite<TestState> {
    describe(format!("libs.mq - {}", mq_engine), move |context| {
        context.describe("ApplicationMgr", |context| {
            context.it("new() with default options", application::new_default);
            context.it("new() with manual options", application::new_manual);
            context.it("new() with wrong opts", application::new_wrong_opts);
            context.it("close()", application::close);

            context.it("uldata", application::uldata);
            context.it("dldata", application::dldata);
            context.it("dldata with wrong parameters", application::dldata_wrong);
            context.it("dldata-result", application::dldata_result);

            context.after_each(after_each_fn);
        });

        context.describe("NetworkMgr", |context| {
            context.it("new() with default options", network::new_default);
            context.it("new() with manual options", network::new_manual);
            context.it("new() with wrong opts", network::new_wrong_opts);
            context.it("close()", network::close);

            context.it("uldata", network::uldata);
            context.it("uldata with wrong parameters", network::uldata_wrong);
            context.it("dldata", network::dldata);
            context.it("dldata-result", network::dldata_result);
            context.it(
                "dldata-result with wrong parameters",
                network::dldata_result_wrong,
            );

            context.after_each(after_each_fn);
        });

        context.describe("control channel", |context| {
            context.it("new()", control::new);
            context.it("new() with wrong opts", control::new_wrong_opts);

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
            if conn.status() != Status::Connected {
                return Err("new_connection() not connected".to_string());
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
            if conn.status() != Status::Connected {
                return Err("new_connection() not connected".to_string());
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
        amqp_prefetch: Some(100),
        mqtt_shared_prefix: Some("$share/sylvia-iot-broker/".to_string()),
        ..Default::default()
    }
}

fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(conn) = state.mq_conn.take() {
        runtime.block_on(async move {
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
    if let Some(mut queues) = state.ctrl_queues.take() {
        runtime.block_on(async {
            for i in queues.iter_mut() {
                let _ = i.close().await;
            }
        });
    }
}
