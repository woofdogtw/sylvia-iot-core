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
    // Default.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);
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
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_none()).to_equal(true)?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;

    // Modified default by command-line arguments.
    let args = vec![
        "test",
        "--coremgr.auth",
        "sylvia11",
        "--coremgr.broker",
        "sylvia12",
        "--coremgr.mq.engine.amqp",
        "rabbitmq",
        "--coremgr.mq.engine.mqtt",
        "emqx",
        "--coremgr.mq.rabbitmq.username",
        "rabbituser1",
        "--coremgr.mq.rabbitmq.password",
        "rabbitpass1",
        "--coremgr.mq.rabbitmq.ttl",
        "11",
        "--coremgr.mq.rabbitmq.length",
        "12",
        "--coremgr.mq.rabbitmq.hosts",
        "[{\"name\":\"name11\",\"host\":\"host11\",\"external\":\"external11\",\"active\":true}]",
        "--coremgr.mq.emqx.apikey",
        "emqxkey1",
        "--coremgr.mq.emqx.apisecret",
        "emqxsecret1",
        "--coremgr.mq.emqx.hosts",
        "[{\"name\":\"name12\",\"host\":\"host12\",\"external\":\"external12\",\"active\":false}]",
        "--coremgr.mq.rumqttd.mqtt-port",
        "111",
        "--coremgr.mq.rumqttd.mqtts-port",
        "112",
        "--coremgr.mq.rumqttd.console-port",
        "113",
        "--coremgr.mq-channels.data.url",
        "url1",
        "--coremgr.mq-channels.data.persistent",
        "false",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia11")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia12")?;
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(MqEngine::RABBITMQ)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(MqEngine::EMQX)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.is_some()).to_equal(true)?;
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal("rabbituser1")?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal("rabbitpass1")?;
    expect(rabbitmq.ttl).to_equal(Some(11))?;
    expect(rabbitmq.length).to_equal(Some(12))?;
    expect(rabbitmq.hosts.is_some()).to_equal(true)?;
    let hosts = rabbitmq.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("name11")?;
    expect(hosts[0].host.as_str()).to_equal("host11")?;
    expect(hosts[0].external.as_str()).to_equal("external11")?;
    expect(hosts[0].active).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal("emqxkey1")?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal("emqxsecret1")?;
    expect(emqx.hosts.is_some()).to_equal(true)?;
    let hosts = emqx.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("name12")?;
    expect(hosts[0].host.as_str()).to_equal("host12")?;
    expect(hosts[0].external.as_str()).to_equal("external12")?;
    expect(hosts[0].active).to_equal(false)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(111))?;
    expect(rumqttd.mqtts_port).to_equal(Some(112))?;
    expect(rumqttd.console_port).to_equal(Some(113))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url1")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)?;

    let args = vec![
        "test",
        "--coremgr.mq.rabbitmq.hosts",
        "",
        "--coremgr.mq.emqx.hosts",
        "",
        "--coremgr.mq-channels.data.persistent",
        "true",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().emqx.is_some()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.hosts.is_none()).to_equal(true)?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_none()).to_equal(true)?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(true)?;

    // Test wrong command-line arguments.
    let args = vec![
        "test",
        "--coremgr.mq.rabbitmq.hosts",
        "{",
        "--coremgr.mq.emqx.hosts",
        "}",
    ];
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().emqx.is_some()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.hosts.is_none()).to_equal(true)?;

    // Clear command-line arguments.
    let args = config::reg_args(Command::new("test")).get_matches_from(vec!["test"]);

    // Modified default by environment variables.
    env::set_var(&OsStr::new("COREMGR_AUTH"), "sylvia21");
    env::set_var(&OsStr::new("COREMGR_BROKER"), "sylvia22");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_AMQP"), "rabbitmq");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_MQTT"), "rumqttd");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_USERNAME"), "rabbituser2");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_PASSWORD"), "rabbitpass2");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_TTL"), "21");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_LENGTH"), "22");
    env::set_var(
        &OsStr::new("COREMGR_MQ_RABBITMQ_HOSTS"),
        "[{\"name\":\"name21\",\"host\":\"host21\",\"external\":\"external21\",\"active\":false}]",
    );
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_APIKEY"), "emqxkey2");
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_APISECRET"), "emqxsecret2");
    env::set_var(
        &OsStr::new("COREMGR_MQ_EMQX_HOSTS"),
        "[{\"name\":\"name22\",\"host\":\"host22\",\"external\":\"external22\",\"active\":true}]",
    );
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTT_PORT"), "121");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTTS_PORT"), "122");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_CONSOLE_PORT"), "123");
    env::set_var(&OsStr::new("COREMGR_MQCHANNELS_DATA_URL"), "url2");
    env::set_var(&OsStr::new("COREMGR_MQCHANNELS_DATA_PERSISTENT"), "false");
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia21")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia22")?;
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
    expect(rabbitmq.ttl.unwrap()).to_equal(21)?;
    expect(rabbitmq.length.is_some()).to_equal(true)?;
    expect(rabbitmq.length.unwrap()).to_equal(22)?;
    expect(rabbitmq.hosts.is_some()).to_equal(true)?;
    let hosts = rabbitmq.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("name21")?;
    expect(hosts[0].host.as_str()).to_equal("host21")?;
    expect(hosts[0].external.as_str()).to_equal("external21")?;
    expect(hosts[0].active).to_equal(false)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal("emqxkey2")?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal("emqxsecret2")?;
    expect(emqx.hosts.is_some()).to_equal(true)?;
    let hosts = emqx.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("name22")?;
    expect(hosts[0].host.as_str()).to_equal("host22")?;
    expect(hosts[0].external.as_str()).to_equal("external22")?;
    expect(hosts[0].active).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(121))?;
    expect(rumqttd.mqtts_port).to_equal(Some(122))?;
    expect(rumqttd.console_port).to_equal(Some(123))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url2")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)?;

    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_HOSTS"), "");
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_HOSTS"), "");
    let conf = config::read_args(&args);
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().emqx.is_some()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.hosts.is_none()).to_equal(true)?;

    // Test wrong environment variables.
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_AMQP"), "rabbitmq1");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_MQTT"), "emqx1");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_TTL"), "12_000");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_LENGTH"), "12_000");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_HOSTS"), "}");
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_HOSTS"), "{");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTT_PORT"), "12_000");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTTS_PORT"), "12_000");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_CONSOLE_PORT"), "12_000");
    env::set_var(&OsStr::new("COREMGR_MQCHANNELS_DATA_PERSISTENT"), "0");
    let conf = config::read_args(&args);
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_AMQP)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(config::DEF_ENGINE_MQTT)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.ttl.is_none()).to_equal(true)?;
    expect(rabbitmq.length.is_none()).to_equal(true)?;
    expect(rabbitmq.hosts.is_none()).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.hosts.is_none()).to_equal(true)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(config::DEF_RUMQTTD_MQTT_PORT))?;
    expect(rumqttd.mqtts_port).to_equal(Some(config::DEF_RUMQTTD_MQTTS_PORT))?;
    expect(rumqttd.console_port).to_equal(Some(config::DEF_RUMQTTD_CONSOLE_PORT))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;

    // Test command-line arguments overwrite environment variables.
    let args = vec![
        "test",
        "--coremgr.auth",
        "sylvia31",
        "--coremgr.broker",
        "sylvia32",
        "--coremgr.mq.engine.amqp",
        "rabbitmq",
        "--coremgr.mq.engine.mqtt",
        "emqx",
        "--coremgr.mq.rabbitmq.username",
        "rabbituser3",
        "--coremgr.mq.rabbitmq.password",
        "rabbitpass3",
        "--coremgr.mq.rabbitmq.ttl",
        "31",
        "--coremgr.mq.rabbitmq.length",
        "32",
        "--coremgr.mq.rabbitmq.hosts",
        "[{\"name\":\"name31\",\"host\":\"host31\",\"external\":\"external31\",\"active\":true}]",
        "--coremgr.mq.emqx.apikey",
        "emqxkey3",
        "--coremgr.mq.emqx.apisecret",
        "emqxsecret3",
        "--coremgr.mq.emqx.hosts",
        "[{\"name\":\"name32\",\"host\":\"host32\",\"external\":\"external32\",\"active\":false}]",
        "--coremgr.mq.rumqttd.mqtt-port",
        "131",
        "--coremgr.mq.rumqttd.mqtts-port",
        "132",
        "--coremgr.mq.rumqttd.console-port",
        "133",
        "--coremgr.mq-channels.data.url",
        "url3",
        "--coremgr.mq-channels.data.persistent",
        "false",
    ];
    env::set_var(&OsStr::new("COREMGR_AUTH"), "sylvia41");
    env::set_var(&OsStr::new("COREMGR_BROKER"), "sylvia42");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_AMQP"), "rabbitmq");
    env::set_var(&OsStr::new("COREMGR_MQ_ENGINE_MQTT"), "rumqttd");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_USERNAME"), "rabbituser4");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_PASSWORD"), "rabbitpass4");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_TTL"), "41");
    env::set_var(&OsStr::new("COREMGR_MQ_RABBITMQ_LENGTH"), "42");
    env::set_var(
        &OsStr::new("COREMGR_MQ_RABBITMQ_HOSTS"),
        "[{\"name\":\"name41\",\"host\":\"host41\",\"external\":\"external41\",\"active\":false}]",
    );
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_APIKEY"), "emqxkey4");
    env::set_var(&OsStr::new("COREMGR_MQ_EMQX_APISECRET"), "emqxsecret4");
    env::set_var(
        &OsStr::new("COREMGR_MQ_EMQX_HOSTS"),
        "[{\"name\":\"name42\",\"host\":\"host42\",\"external\":\"external42\",\"active\":true}]",
    );
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTT_PORT"), "141");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_MQTTS_PORT"), "142");
    env::set_var(&OsStr::new("COREMGR_MQ_RUMQTTD_CONSOLE_PORT"), "143");
    env::set_var(&OsStr::new("COREMGR_MQCHANNELS_DATA_URL"), "url4");
    env::set_var(&OsStr::new("COREMGR_MQCHANNELS_DATA_PERSISTENT"), "true");
    let args = config::reg_args(Command::new("test")).get_matches_from(args);
    let conf = config::read_args(&args);
    expect(conf.auth.is_some()).to_equal(true)?;
    expect(conf.auth.as_ref().unwrap().as_str()).to_equal("sylvia31")?;
    expect(conf.broker.is_some()).to_equal(true)?;
    expect(conf.broker.as_ref().unwrap().as_str()).to_equal("sylvia32")?;
    expect(conf.mq.is_some()).to_equal(true)?;
    expect(conf.mq.as_ref().unwrap().engine.is_some()).to_equal(true)?;
    let engine = conf.mq.as_ref().unwrap().engine.as_ref().unwrap();
    expect(engine.amqp.is_some()).to_equal(true)?;
    expect(engine.amqp.as_ref().unwrap().as_str()).to_equal(MqEngine::RABBITMQ)?;
    expect(engine.mqtt.is_some()).to_equal(true)?;
    expect(engine.mqtt.as_ref().unwrap().as_str()).to_equal(MqEngine::EMQX)?;
    expect(conf.mq.as_ref().unwrap().rabbitmq.is_some()).to_equal(true)?;
    let rabbitmq = conf.mq.as_ref().unwrap().rabbitmq.as_ref().unwrap();
    expect(rabbitmq.username.is_some()).to_equal(true)?;
    expect(rabbitmq.username.as_ref().unwrap().as_str()).to_equal("rabbituser3")?;
    expect(rabbitmq.password.is_some()).to_equal(true)?;
    expect(rabbitmq.password.as_ref().unwrap().as_str()).to_equal("rabbitpass3")?;
    expect(rabbitmq.ttl).to_equal(Some(31))?;
    expect(rabbitmq.length).to_equal(Some(32))?;
    expect(rabbitmq.hosts.is_some()).to_equal(true)?;
    let hosts = rabbitmq.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("name31")?;
    expect(hosts[0].host.as_str()).to_equal("host31")?;
    expect(hosts[0].external.as_str()).to_equal("external31")?;
    expect(hosts[0].active).to_equal(true)?;
    let emqx = conf.mq.as_ref().unwrap().emqx.as_ref().unwrap();
    expect(emqx.api_key.is_some()).to_equal(true)?;
    expect(emqx.api_key.as_ref().unwrap().as_str()).to_equal("emqxkey3")?;
    expect(emqx.api_secret.is_some()).to_equal(true)?;
    expect(emqx.api_secret.as_ref().unwrap().as_str()).to_equal("emqxsecret3")?;
    expect(emqx.hosts.is_some()).to_equal(true)?;
    let hosts = emqx.hosts.as_ref().unwrap();
    expect(hosts.len()).to_equal(1)?;
    expect(hosts[0].name.as_str()).to_equal("name32")?;
    expect(hosts[0].host.as_str()).to_equal("host32")?;
    expect(hosts[0].external.as_str()).to_equal("external32")?;
    expect(hosts[0].active).to_equal(false)?;
    let rumqttd = conf.mq.as_ref().unwrap().rumqttd.as_ref().unwrap();
    expect(rumqttd.mqtt_port).to_equal(Some(131))?;
    expect(rumqttd.mqtts_port).to_equal(Some(132))?;
    expect(rumqttd.console_port).to_equal(Some(133))?;
    let mq_channels_conf = conf.mq_channels.as_ref().unwrap();
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_some()).to_equal(true)?;
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url3")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)
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
    expect(mq_channels_conf.data.is_some()).to_equal(true)?;
    let data_conf = mq_channels_conf.data.as_ref().unwrap();
    expect(data_conf.url.is_none()).to_equal(true)?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(config::DEF_MQ_PERSISTENT)?;

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
                persistent: Some(false),
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
    expect(rabbitmq.ttl.unwrap()).to_equal(100)?;
    expect(rabbitmq.length.is_some()).to_equal(true)?;
    expect(rabbitmq.length.unwrap()).to_equal(1000)?;
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
    expect(data_conf.url.as_ref().unwrap().as_str()).to_equal("url9")?;
    expect(data_conf.persistent.is_some()).to_equal(true)?;
    expect(data_conf.persistent.unwrap()).to_equal(false)
}
