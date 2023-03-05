use std::{env, ffi::OsStr};

use clap::Command;
use laboratory::{expect, SpecContext};

use sylvia_iot_corelib::constants::MqEngine;
use sylvia_iot_coremgr::libs::config::{self, Config};

use crate::TestState;

/// Test [`config::reg_args`].
pub fn reg_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    config::reg_args(Command::new("test"));
    Ok(())
}

/// Test [`config::read_args`].
pub fn read_args(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let args = Command::new("test").get_matches();
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal(config::DEF_AUTH)?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal(config::DEF_BROKER)?;
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_AMQP)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_MQTT)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.is_some()).to_equal(true)?;
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_USERNAME)?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_PASSWORD)?;
    expect(rabbitmq.ttl.is_none()).to_equal(true)?;
    expect(rabbitmq.length.is_none()).to_equal(true)?;
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_KEY)?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_SECRET)?;
    expect(emqx.hosts.is_none()).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(config::DEF_RUMQTTD_MQTT_PORT))?;
    expect(rumqttd.mqtts_port).to_equal(Some(config::DEF_RUMQTTD_MQTTS_PORT))?;
    expect(rumqttd.console_port).to_equal(Some(config::DEF_RUMQTTD_CONSOLE_PORT))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    env::set_var(&OsStr::new("COREMGR_AUTH"), "sylvia2");
    env::set_var(&OsStr::new("COREMGR_BROKER"), "sylvia3");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_AMQP"), "rabbitmq2");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_MQTT"), MqEngine::RUMQTTD);
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_USERNAME"), "rabbituser");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_PASSWORD"), "rabbitpass");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_TTL"), "100");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_LENGTH"), "1000");
    env::set_var(
        &OsStr::new("COREMGR_MQ_RABBITMQ_HOSTS"),
        "local1,localhost1,false",
    );
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_API_KEY"), "emqxkey");
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_API_SECRET"), "emqxsecret");
    env::set_var(
        &OsStr::new("COREMGR_MQ_EMQX_HOSTS"),
        "local2,localhost2,false",
    );
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTT_PORT"), "1884");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTTS_PORT"), "8884");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_CONSOLE_PORT"), "18084");
    env::set_var(&OsStr::new("COREMGR_MQCHANNELS_DATA_URL"), "url9");
    let args = Command::new("test").get_matches();
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia3")?;
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(MqEngine::RABBITMQ)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(MqEngine::RUMQTTD)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.is_some()).to_equal(true)?;
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal("rabbituser")?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal("rabbitpass")?;
    expect(rabbitmq.ttl.is_some()).to_equal(true)?;
    expect(*rabbitmq.ttl.as_ref().unwrap()).to_equal(100)?;
    expect(rabbitmq.length.is_some()).to_equal(true)?;
    expect(*rabbitmq.length.as_ref().unwrap()).to_equal(1000)?;
    expect(rabbitmq.hosts.is_some()).to_equal(true)?;
    let hosts = rabbitmq.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("local1")?;
    expect(hosts[0].host.as_str()).to_equal("localhost1")?;
    expect(hosts[0].active).to_equal(false)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal("emqxkey")?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal("emqxsecret")?;
    expect(emqx.hosts.is_some()).to_equal(true)?;
    let hosts = emqx.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("local2")?;
    expect(hosts[0].host.as_str()).to_equal("localhost2")?;
    expect(hosts[0].active).to_equal(false)?;
    expect(rumqttd.mqtt_port).to_equal(Some(1884))?;
    expect(rumqttd.mqtts_port).to_equal(Some(8884))?;
    expect(rumqttd.console_port).to_equal(Some(18084))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url9")
}

/// Test [`config::apply_default`].
pub fn apply_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conf = Config {
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal(config::DEF_AUTH)?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal(config::DEF_BROKER)?;
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_AMQP)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_MQTT)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_USERNAME)?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_PASSWORD)?;
    expect(rabbitmq.ttl.is_none()).to_equal(true)?;
    expect(rabbitmq.length.is_none()).to_equal(true)?;
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_KEY)?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_SECRET)?;
    expect(emqx.hosts.is_none()).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(config::DEF_RUMQTTD_MQTT_PORT))?;
    expect(rumqttd.mqtts_port).to_equal(Some(config::DEF_RUMQTTD_MQTTS_PORT))?;
    expect(rumqttd.console_port).to_equal(Some(config::DEF_RUMQTTD_CONSOLE_PORT))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    let conf = Config {
        mq: Some(config::Mq {
            ..Default::default()
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_AMQP)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_MQTT)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_USERNAME)?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_PASSWORD)?;
    expect(rabbitmq.ttl.is_none()).to_equal(true)?;
    expect(rabbitmq.length.is_none()).to_equal(true)?;
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_KEY)?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_SECRET)?;
    expect(emqx.hosts.is_none()).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(config::DEF_RUMQTTD_MQTT_PORT))?;
    expect(rumqttd.mqtts_port).to_equal(Some(config::DEF_RUMQTTD_MQTTS_PORT))?;
    expect(rumqttd.console_port).to_equal(Some(config::DEF_RUMQTTD_CONSOLE_PORT))?;

    let conf = Config {
        mq: Some(config::Mq {
            engine: Some(config::Engine {
                ..Default::default()
            }),
            rabbitmq: Some(config::RabbitMq {
                ..Default::default()
            }),
            emqx: Some(config::Emqx {
                ..Default::default()
            }),
            rumqttd: Some(config::Rumqttd {
                ..Default::default()
            }),
        }),
        mq_channels: Some(config::MqChannels {
            ..Default::default()
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_AMQP)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_MQTT)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_USERNAME)?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal(config::DEF_RABBITMQ_PASSWORD)?;
    expect(rabbitmq.ttl.is_none()).to_equal(true)?;
    expect(rabbitmq.length.is_none()).to_equal(true)?;
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_KEY)?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal(config::DEF_EMQX_API_SECRET)?;
    expect(emqx.hosts.is_none()).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(config::DEF_RUMQTTD_MQTT_PORT))?;
    expect(rumqttd.mqtts_port).to_equal(Some(config::DEF_RUMQTTD_MQTTS_PORT))?;
    expect(rumqttd.console_port).to_equal(Some(config::DEF_RUMQTTD_CONSOLE_PORT))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    let conf = Config {
        mq: Some(config::Mq {
            engine: Some(config::Engine {
                amqp: Some("rabbitmq2".to_string()),
                mqtt: Some("mqtt2".to_string()),
            }),
            rabbitmq: Some(config::RabbitMq {
                ..Default::default()
            }),
            emqx: Some(config::Emqx {
                ..Default::default()
            }),
            rumqttd: Some(config::Rumqttd {
                ..Default::default()
            }),
        }),
        mq_channels: Some(config::MqChannels {
            data: Some(config::CoremgrData {
                ..Default::default()
            }),
        }),
        ..Default::default()
    };
    let conf = config::apply_default(&conf);
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_AMQP)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_MQTT)?;
    expect(rabbitmq.ttl.is_none()).to_equal(true)?;
    expect(rabbitmq.length.is_none()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_none()).to_equal(true)?;

    let conf = Config {
        auth: Some("sylvia2".to_string()),
        broker: Some("sylvia3".to_string()),
        mq: Some(config::Mq {
            engine: Some(config::Engine {
                amqp: Some("rabbitmq2".to_string()),
                mqtt: Some(MqEngine::RUMQTTD.to_string()),
            }),
            rabbitmq: Some(config::RabbitMq {
                username: Some("rabbituser2".to_string()),
                password: Some("rabbitpass2".to_string()),
                ttl: Some(100),
                length: Some(1000),
                hosts: Some(vec![
                    config::MqHost {
                        name: "rabbitmq1-name".to_string(),
                        host: "rabbitmq1".to_string(),
                        external: "rabbitmq1-ext".to_string(),
                        active: true,
                    },
                    config::MqHost {
                        name: "rabbitmq2-name".to_string(),
                        host: "rabbitmq2".to_string(),
                        external: "rabbitmq2-ext".to_string(),
                        active: false,
                    },
                ]),
            }),
            emqx: Some(config::Emqx {
                api_key: Some("emqxkey2".to_string()),
                api_secret: Some("emqxsecret2".to_string()),
                hosts: Some(vec![
                    config::MqHost {
                        name: "emqx2-name".to_string(),
                        host: "emqx2".to_string(),
                        external: "emqx2-ext".to_string(),
                        active: false,
                    },
                    config::MqHost {
                        name: "emqx1-name".to_string(),
                        host: "emqx1".to_string(),
                        external: "emqx1-ext".to_string(),
                        active: true,
                    },
                ]),
            }),
            rumqttd: Some(config::Rumqttd {
                mqtt_port: Some(1884),
                mqtts_port: Some(8884),
                console_port: Some(18084),
            }),
        }),
        mq_channels: Some(config::MqChannels {
            data: Some(config::CoremgrData {
                url: Some("url9".to_string()),
            }),
        }),
    };
    let conf = config::apply_default(&conf);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia2")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia3")?;
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(MqEngine::RABBITMQ)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(MqEngine::RUMQTTD)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.is_some()).to_equal(true)?;
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal("rabbituser2")?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal("rabbitpass2")?;
    expect(rabbitmq.ttl.is_some()).to_equal(true)?;
    expect(*rabbitmq.ttl.as_ref().unwrap()).to_equal(100)?;
    expect(rabbitmq.length.is_some()).to_equal(true)?;
    expect(*rabbitmq.length.as_ref().unwrap()).to_equal(1000)?;
    expect(rabbitmq.hosts.is_some()).to_equal(true)?;
    let hosts = rabbitmq.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(2)?;
    expect(hosts[0].name.as_str()).to_equal("rabbitmq1-name")?;
    expect(hosts[0].host.as_str()).to_equal("rabbitmq1")?;
    expect(hosts[0].external.as_str()).to_equal("rabbitmq1-ext")?;
    expect(hosts[0].active).to_equal(true)?;
    expect(hosts[1].name.as_str()).to_equal("rabbitmq2-name")?;
    expect(hosts[1].host.as_str()).to_equal("rabbitmq2")?;
    expect(hosts[1].external.as_str()).to_equal("rabbitmq2-ext")?;
    expect(hosts[1].active).to_equal(false)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal("emqxkey2")?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal("emqxsecret2")?;
    expect(emqx.hosts.is_some()).to_equal(true)?;
    let hosts = emqx.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(2)?;
    expect(hosts[0].name.as_str()).to_equal("emqx2-name")?;
    expect(hosts[0].host.as_str()).to_equal("emqx2")?;
    expect(hosts[0].external.as_str()).to_equal("emqx2-ext")?;
    expect(hosts[0].active).to_equal(false)?;
    expect(hosts[1].name.as_str()).to_equal("emqx1-name")?;
    expect(hosts[1].host.as_str()).to_equal("emqx1")?;
    expect(hosts[1].external.as_str()).to_equal("emqx1-ext")?;
    expect(hosts[1].active).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(1884))?;
    expect(rumqttd.mqtts_port).to_equal(Some(8884))?;
    expect(rumqttd.console_port).to_equal(Some(18084))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url9")
}
