use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use laboratory::{SpecContext, Suite, describe, expect};
use reqwest;

use general_mq::queue::GmqQueue;
use sylvia_iot_auth::libs::config as sylvia_iot_auth_config;
use sylvia_iot_broker::libs::config as sylvia_iot_broker_config;
use sylvia_iot_corelib::constants::MqEngine;
use sylvia_iot_coremgr::{
    libs::{
        config::{self, Config, MqHost},
        mq::{emqx::ManagementOpts as EmqxOpts, rabbitmq::ManagementOpts as RabbitMqOpts},
    },
    routes::{self, AmqpState, MqHosts, MqttState},
};

use crate::{TestState, libs::mq::emqx};

mod libs;
pub mod middleware;
pub mod v1;

use libs::new_state;

pub const STATE: &'static str = "routes";

pub fn suite() -> Suite<TestState> {
    describe("routes", |context| {
        context.it("new_state", fn_new_state);
        context.it("new_service", fn_new_service);
        context
            .before_all(|state| {
                state.insert(STATE, new_state(None, None));
            })
            .after_all(|state| {
                let state = state.get_mut(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();
                runtime.block_on(async {
                    if let Err(e) = emqx::after_del_api_key().await {
                        println!("delete EMQX API key error: {}", e);
                    }
                });
            });
    })
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
        Ok(state) => state,
    };
    expect(state.scope_path).to_equal("scope")?;
    expect(state.mq_hosts.amqp.len()).to_equal(0)?;
    expect(state.mq_hosts.mqtt.len()).to_equal(0)?;

    // Hosts are normalized.
    let mq_host = |name: &str, host: &str, external: &str| MqHost {
        name: name.to_string(),
        host: host.to_string(),
        external: external.to_string(),
        active: true,
    };
    let new_conf = |amqp: Vec<MqHost>, mqtt: Vec<MqHost>, mqtt_engine: &str| Config {
        mq: Some(config::Mq {
            engine: Some(config::Engine {
                amqp: None,
                mqtt: Some(mqtt_engine.to_string()),
            }),
            rabbitmq: Some(config::RabbitMq {
                hosts: Some(amqp),
                ..Default::default()
            }),
            emqx: Some(config::Emqx {
                hosts: Some(mqtt),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let conf = new_conf(
        vec![
            mq_host("name1", " RabbitMQ.Internal ", "MQ.example.com"),
            mq_host("name2", "10.0.0.1", "::1"),
        ],
        vec![mq_host("name3", "emqx.internal", "[2001:DB8::1]")],
        MqEngine::EMQX,
    );
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("hosts config error: {}", e)),
        Ok(state) => state,
    };
    let amqp = &state.mq_hosts.amqp;
    expect(amqp.len()).to_equal(2)?;
    expect(amqp[0].name.as_str()).to_equal("name1")?;
    expect(amqp[0].host.as_str()).to_equal("rabbitmq.internal")?;
    expect(amqp[0].external.as_str()).to_equal("mq.example.com")?;
    expect(amqp[0].active).to_equal(true)?;
    expect(amqp[1].host.as_str()).to_equal("10.0.0.1")?;
    expect(amqp[1].external.as_str()).to_equal("[::1]")?;
    let mqtt = &state.mq_hosts.mqtt;
    expect(mqtt.len()).to_equal(1)?;
    expect(mqtt[0].external.as_str()).to_equal("[2001:db8::1]")?;

    // rumqttd does not use hosts.
    let conf = new_conf(
        vec![],
        vec![mq_host("name3", "emqx.internal", "emqx.example.com")],
        MqEngine::RUMQTTD,
    );
    let state = match runtime.block_on(async { routes::new_state("scope", &conf).await }) {
        Err(e) => return Err(format!("rumqttd hosts config error: {}", e)),
        Ok(state) => state,
    };
    expect(state.mq_hosts.mqtt.len()).to_equal(0)?;

    // Invalid hosts.
    let invalid_hosts = vec![
        vec![mq_host("", "host", "external")],
        vec![mq_host("name", "", "external")],
        vec![mq_host("name", "host", " ")],
        vec![mq_host("name", "host:5672", "external")],
        vec![mq_host("name", "amqp://host", "external")],
        vec![mq_host("name", "host", "external/path")],
        vec![mq_host("name", "user@host", "external")],
        vec![
            mq_host("name1", "host", "external1"),
            mq_host("name2", "HOST", "external2"),
        ],
        vec![
            mq_host("name1", "host1", "external"),
            mq_host("name2", "host2", "External"),
        ],
    ];
    for hosts in invalid_hosts {
        for (amqp, mqtt) in [(hosts.clone(), vec![]), (vec![], hosts.clone())] {
            let conf = new_conf(amqp, mqtt, MqEngine::EMQX);
            if runtime
                .block_on(async { routes::new_state("scope", &conf).await })
                .is_ok()
            {
                let names: Vec<String> = hosts
                    .iter()
                    .map(|h| format!("{}/{}/{}", h.name, h.host, h.external))
                    .collect();
                return Err(format!("invalid hosts {:?} should fail", names));
            }
        }
    }
    Ok(())
}

fn fn_new_service(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let _ = routes::new_service(&routes::State {
        auth_base: config::DEF_AUTH.to_string(),
        broker_base: config::DEF_BROKER.to_string(),
        scope_path: "/test",
        client: reqwest::Client::new(),
        amqp: AmqpState::RabbitMq(RabbitMqOpts {
            username: "guest".to_string(),
            password: "guest".to_string(),
            ttl: Some(86400),
            length: Some(1000),
        }),
        mqtt: MqttState::Emqx(EmqxOpts {
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
        }),
        mq_hosts: MqHosts::default(),
        mq_conns: Arc::new(Mutex::new(HashMap::new())),
        data_sender: None,
    });
    Ok(())
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

fn stop_auth_broker_svc(state: &mut TestState) {
    if let Some(svc) = state.auth_broker_svc.take() {
        svc.abort();
    }
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_auth_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
    let mut path = std::env::temp_dir();
    path.push(sylvia_iot_broker_config::DEF_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}

async fn clear_state(state: &mut routes::State) {
    if let Some(mut q) = state.data_sender.take() {
        if let Err(e) = q.close().await {
            println!("close data channel {} error: {}", q.name(), e);
        }
    }
}
