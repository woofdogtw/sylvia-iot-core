use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    thread::{self, JoinHandle as ThreadHandle},
};

use librumqttd::{
    async_locallink, rumqttlog::Config as RouterConfig, Config, ConnectionSettings,
    ConsoleSettings, ServerCert, ServerSettings,
};
use tokio::task::{self, JoinHandle as TaskHandle};

use sylvia_iot_corelib::server_config::Config as SylviaServerConfig;

use super::super::config::{
    Rumqttd, DEF_RUMQTTD_CONSOLE_PORT, DEF_RUMQTTD_MQTTS_PORT, DEF_RUMQTTD_MQTT_PORT,
};

/// To start a rumqttd broker.
pub fn start_rumqttd(
    server_conf: &SylviaServerConfig,
    rumqttd_conf: &Rumqttd,
) -> (ThreadHandle<()>, TaskHandle<()>) {
    let mut config = Config {
        id: 0,
        router: RouterConfig::default(),
        servers: HashMap::new(),
        cluster: None,
        replicator: None,
        console: ConsoleSettings {
            listen: match rumqttd_conf.console_port {
                None => SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    DEF_RUMQTTD_CONSOLE_PORT,
                ),
                Some(port) => SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port),
            },
        },
    };
    config.servers.insert(
        "mqtt".to_string(),
        ServerSettings {
            listen: match rumqttd_conf.mqtt_port {
                None => {
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), DEF_RUMQTTD_MQTT_PORT)
                }
                Some(port) => SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port),
            },
            cert: None,
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 5000,
                max_client_id_len: 256,
                throttle_delay_ms: 0,
                max_payload_size: 1 * 1024 * 1024,
                max_inflight_count: 200,
                max_inflight_size: 1024,
                login_credentials: None,
            },
        },
    );
    if let Some(cacert_file) = server_conf.cacert_file.as_ref() {
        if let Some(cert_file) = server_conf.cert_file.as_ref() {
            if let Some(key_file) = server_conf.key_file.as_ref() {
                config.servers.insert(
                    "mqtts".to_string(),
                    ServerSettings {
                        listen: match rumqttd_conf.mqtts_port {
                            None => SocketAddr::new(
                                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                                DEF_RUMQTTD_MQTTS_PORT,
                            ),
                            Some(port) => {
                                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
                            }
                        },
                        cert: Some(ServerCert::RustlsCert {
                            ca_path: cacert_file.clone(),
                            cert_path: cert_file.clone(),
                            key_path: key_file.clone(),
                        }),
                        next_connection_delay_ms: 1,
                        connections: ConnectionSettings {
                            connection_timeout_ms: 5000,
                            max_client_id_len: 256,
                            throttle_delay_ms: 0,
                            max_payload_size: 1 * 1024 * 1024,
                            max_inflight_count: 200,
                            max_inflight_size: 1024,
                            login_credentials: None,
                        },
                    },
                );
            }
        }
    }

    let (mut router, console, servers, _builder) = async_locallink::construct_broker(config);
    let router_handle = thread::spawn(move || {
        router.start().unwrap();
    });
    let rumqttd_handle = task::spawn(async move {
        let console_task = task::spawn(console);
        servers.await;
        console_task.await.unwrap();
    });

    (router_handle, rumqttd_handle)
}
