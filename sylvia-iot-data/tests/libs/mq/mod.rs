use std::collections::HashMap;

use general_mq::{connection::GmqConnection, queue::GmqQueue};
use laboratory::{describe, Suite};
use reqwest::{self, Method, StatusCode};
use serde::Deserialize;
use tokio::runtime::Runtime;

use sylvia_iot_corelib::constants::MqEngine;
use sylvia_iot_data::{
    libs::mq::Connection,
    models::{
        application_dldata::QueryCond as AppDlDataCond,
        application_uldata::QueryCond as AppUlDataCond, coremgr_opdata::QueryCond as CmOpDataCond,
        network_dldata::QueryCond as NetDlDataCond, network_uldata::QueryCond as NetUlDataCond,
        Model, SqliteModel, SqliteOptions,
    },
};

use crate::TestState;

mod broker;
mod coremgr;

#[derive(Deserialize)]
struct QueueInfo {
    name: String,
}

const STATE: &'static str = "libs/mq";

pub fn suite(mq_engine: &'static str) -> Suite<TestState> {
    describe(format!("libs.mq - {}", mq_engine), move |context| {
        context.describe("broker.data", |context| {
            context.it("new() with default options", broker::new_default);
            context.it("new() with manual options", broker::new_manual);
            context.it("new() with the same host", broker::new_same_host);
            context.it("new() with wrong opts", broker::new_wrong_opts);
            context.it("application_uldata()", broker::application_uldata);
            context.it(
                "application_uldata() with wrong content",
                broker::application_uldata_wrong,
            );
            context.it("application_dldata()", broker::application_dldata);
            context.it(
                "application_dldata() with wrong content",
                broker::application_dldata_wrong,
            );
            context.it("network_uldata()", broker::network_uldata);
            context.it(
                "network_uldata() with wrong content",
                broker::network_uldata_wrong,
            );
            context.it("network_dldata()", broker::network_dldata);
            context.it(
                "network_dldata() with wrong content",
                broker::network_dldata_wrong,
            );

            context
                .before_all(broker::before_all_fn)
                .after_each(after_each_fn)
                .after_all(broker::after_all_fn);
        });

        context.describe("coremgr.data", |context| {
            context.it("new() with default options", coremgr::new_default);
            context.it("new() with manual options", coremgr::new_manual);
            context.it("new() with the same host", coremgr::new_same_host);
            context.it("new() with wrong opts", coremgr::new_wrong_opts);
            context.it("operation()", coremgr::operation);
            context.it("operation() with wrong content", coremgr::operation_wrong);

            context
                .before_all(coremgr::before_all_fn)
                .after_each(after_each_fn)
                .after_all(coremgr::after_all_fn);
        });

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(mq_engine));
            })
            .after_all(|state| {
                let state = state.get_mut(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();

                if let Some(model) = state.sqlite.take() {
                    runtime.block_on(async {
                        let _ = model.close().await;
                    });
                }
                let mut path = std::env::temp_dir();
                path.push(crate::TEST_SQLITE_PATH);
                remove_sqlite(path.to_str().unwrap());

                if mq_engine.eq(MqEngine::RABBITMQ) {
                    remove_rabbitmq_queues(state);
                }
            });
    })
}

fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let _ = model
                .application_dldata()
                .del(&AppDlDataCond::default())
                .await;
            let _ = model
                .application_uldata()
                .del(&AppUlDataCond::default())
                .await;
            let _ = model.network_dldata().del(&NetDlDataCond::default()).await;
            let _ = model.network_uldata().del(&NetUlDataCond::default()).await;
            let _ = model.coremgr_opdata().del(&CmOpDataCond::default()).await;
        });
    }

    if let Some(mut q) = state.recv_queue.take() {
        runtime.block_on(async move {
            let _ = q.close().await;
        });
    }

    if let Some(conns) = state.recv_conns.as_mut() {
        runtime.block_on(async move {
            for (_, conn) in conns.iter_mut() {
                match conn {
                    Connection::Amqp(conn, _) => {
                        let _ = conn.close().await;
                    }
                    Connection::Mqtt(conn, _) => {
                        let _ = conn.close().await;
                    }
                }
            }
            conns.clear();
        });
    }
}

fn new_state(mq_engine: &'static str) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    let opts = SqliteOptions {
        path: path.to_str().unwrap().to_string(),
    };
    let model = runtime.block_on(async {
        match SqliteModel::new(&opts).await {
            Err(e) => panic!("create DB error: {}", e),
            Ok(model) => model,
        }
    });

    TestState {
        runtime: Some(runtime),
        sqlite: Some(model),
        mq_engine: Some(mq_engine.to_string()),
        recv_conns: Some(HashMap::new()),
        ..Default::default()
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
