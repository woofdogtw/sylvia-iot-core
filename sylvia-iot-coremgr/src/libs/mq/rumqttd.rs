use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    thread::{self, JoinHandle as ThreadHandle},
};

use rumqttd::{
    Broker, Config, ConnectionSettings, ConsoleSettings, RouterConfig, ServerSettings, TlsConfig,
};

use sylvia_iot_corelib::server_config::Config as SylviaServerConfig;

use super::super::config::{
    Rumqttd, DEF_RUMQTTD_CONSOLE_PORT, DEF_RUMQTTD_MQTTS_PORT, DEF_RUMQTTD_MQTT_PORT,
};

/// To start a rumqttd broker.
pub fn start_rumqttd(
    server_conf: &SylviaServerConfig,
    rumqttd_conf: &Rumqttd,
) -> (ThreadHandle<()>, ThreadHandle<()>) {
    let mut console_setting = ConsoleSettings::default();
    console_setting.listen = match rumqttd_conf.console_port {
        None => format!("0.0.0.0:{}", DEF_RUMQTTD_CONSOLE_PORT),
        Some(port) => format!("0.0.0.0:{}", port),
    };
    let mut config = Config {
        router: RouterConfig {
            max_connections: 10000,
            max_outgoing_packet_count: 200,
            max_segment_size: 104857600,
            max_segment_count: 10,
            ..Default::default()
        },
        v4: HashMap::new(),
        console: console_setting,
        ..Default::default()
    };
    config.v4.insert(
        "mqtt".to_string(),
        ServerSettings {
            name: "mqtt".to_string(),
            listen: match rumqttd_conf.mqtt_port {
                None => {
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), DEF_RUMQTTD_MQTT_PORT)
                }
                Some(port) => SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port),
            },
            tls: None,
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 5000,
                max_payload_size: 1 * 1024 * 1024,
                max_inflight_count: 200,
                auth: None,
                dynamic_filters: true,
            },
        },
    );
    if let Some(cacert_file) = server_conf.cacert_file.as_ref() {
        if let Some(cert_file) = server_conf.cert_file.as_ref() {
            if let Some(key_file) = server_conf.key_file.as_ref() {
                config.v4.insert(
                    "mqtts".to_string(),
                    ServerSettings {
                        name: "mqtts".to_string(),
                        listen: match rumqttd_conf.mqtt_port {
                            None => SocketAddr::new(
                                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                                DEF_RUMQTTD_MQTTS_PORT,
                            ),
                            Some(port) => {
                                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
                            }
                        },
                        tls: Some(TlsConfig::Rustls {
                            capath: cacert_file.clone(),
                            certpath: cert_file.clone(),
                            keypath: key_file.clone(),
                        }),
                        next_connection_delay_ms: 1,
                        connections: ConnectionSettings {
                            connection_timeout_ms: 5000,
                            max_payload_size: 1 * 1024 * 1024,
                            max_inflight_count: 200,
                            auth: None,
                            dynamic_filters: true,
                        },
                    },
                );
            }
        }
    }

    let mut broker = Broker::new(config);
    let (mut link_tx, mut link_rx) = broker.link("sylvia-iot-core").unwrap();
    let router_handle = thread::spawn(move || {
        let _ = broker.start();
    });
    let _ = link_tx.subscribe("#");
    let rx_handle = thread::spawn(move || loop {
        let _ = link_rx.id(); // XXX: add this line to prevent not ACK notifications.
        let _ = link_rx.recv().unwrap();
    });

    (router_handle, rx_handle)
}
