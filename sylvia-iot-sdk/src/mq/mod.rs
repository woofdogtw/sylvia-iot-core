//! To management queues for applications and networks.
//!
//! For applications, the [`application::ApplicationMgr`] manages the following kind of queues:
//! - uldata: uplink data from the broker to the application.
//! - dldata: downlink data from the application to the broker.
//! - dldata-resp: the response of downlink data.
//! - dldata-result: the data process result from the network.
//!
//! For networks, the [`network::NetworkMgr`] manages the following kind of queues:
//! - uldata: device uplink data from the network to the broker.
//! - dldata: downlink data from the broker to the network.
//! - dldata-result: the data process result from the network.
//! - ctrl: the control messages from the broker to the network

use std::{
    collections::HashMap,
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use url::Url;

use general_mq::{
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions, connection::GmqConnection, queue::Status,
};

pub mod application;
pub mod network;

/// The general connection type with reference counter for upper layer maintenance.
#[derive(Clone)]
pub enum Connection {
    Amqp(AmqpConnection, Arc<Mutex<isize>>),
    Mqtt(MqttConnection, Arc<Mutex<isize>>),
}

/// Manager status.
#[derive(PartialEq)]
pub enum MgrStatus {
    /// One or more queues are not connected.
    NotReady,
    /// All queues are connected.
    Ready,
}

/// Detail queue connection status.
pub struct DataMqStatus {
    /// For `uldata`.
    pub uldata: Status,
    /// For `dldata`.
    pub dldata: Status,
    /// For `dldata-resp`.
    pub dldata_resp: Status,
    /// For `dldata-result`.
    pub dldata_result: Status,
    /// For `ctrl`.
    pub ctrl: Status,
}

/// The options of the application/network manager.
#[derive(Default, Deserialize, Serialize)]
pub struct Options {
    /// The associated unit ID of the application/network. Empty for public network.
    #[serde(rename = "unitId")]
    pub unit_id: String,
    /// The associated unit code of the application/network. Empty for public network.
    #[serde(rename = "unitCode")]
    pub unit_code: String,
    /// The associated application/network ID.
    pub id: String,
    /// The associated application/network code.
    pub name: String,
    /// AMQP prefetch option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefetch: Option<u16>,
    /// AMQP persistent option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
    /// MQTT shared queue prefix option.
    #[serde(rename = "sharedPrefix", skip_serializing_if = "Option::is_none")]
    pub shared_prefix: Option<String>,
}

/// Support application/network host schemes.
pub const SUPPORT_SCHEMES: &'static [&'static str] = &["amqp", "amqps", "mqtt", "mqtts"];

/// The default prefetch value for AMQP.
const DEF_PREFETCH: u16 = 100;
/// The default persistent value for AMQP.
const DEF_PERSISTENT: bool = false;

impl Copy for MgrStatus {}

impl Clone for MgrStatus {
    fn clone(&self) -> MgrStatus {
        *self
    }
}

/// Utility function to get the message queue connection instance. A new connection will be created
/// if the host does not exist.
fn get_connection(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: &Url,
) -> Result<Connection, String> {
    let uri = host_uri.to_string();
    let mut mutex = conn_pool.lock().unwrap();
    if let Some(conn) = mutex.get(&uri) {
        return Ok(conn.clone());
    }

    match host_uri.scheme() {
        "amqp" | "amqps" => {
            let opts = AmqpConnectionOptions {
                uri: host_uri.to_string(),
                ..Default::default()
            };
            let mut conn = AmqpConnection::new(opts)?;
            let _ = conn.connect();
            let conn = Connection::Amqp(conn, Arc::new(Mutex::new(0)));
            mutex.insert(uri, conn.clone());
            Ok(conn)
        }
        "mqtt" | "mqtts" => {
            let opts = MqttConnectionOptions {
                uri: host_uri.to_string(),
                ..Default::default()
            };
            let mut conn = MqttConnection::new(opts)?;
            let _ = conn.connect();
            let conn = Connection::Mqtt(conn, Arc::new(Mutex::new(0)));
            mutex.insert(uri, conn.clone());
            Ok(conn)
        }
        s => Err(format!("unsupport scheme {}", s)),
    }
}

/// Utility function to remove connection from the pool if the reference count meet zero.
async fn remove_connection(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: &String,
    count: isize,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let conn = {
        let mut mutex = conn_pool.lock().unwrap();
        match mutex.get(host_uri) {
            None => return Ok(()),
            Some(conn) => match conn {
                Connection::Amqp(_, counter) => {
                    let mut mutex = counter.lock().unwrap();
                    *mutex -= count;
                    if *mutex > 0 {
                        return Ok(());
                    }
                }
                Connection::Mqtt(_, counter) => {
                    let mut mutex = counter.lock().unwrap();
                    *mutex -= count;
                    if *mutex > 0 {
                        return Ok(());
                    }
                }
            },
        }
        mutex.remove(host_uri)
    };
    if let Some(conn) = conn {
        match conn {
            Connection::Amqp(mut conn, _) => {
                conn.close().await?;
            }
            Connection::Mqtt(mut conn, _) => {
                conn.close().await?;
            }
        }
    }
    Ok(())
}

/// The utility function for creating application/network queue. The return tuple contains:
/// - `[prefix].[unit].[code].uldata`
/// - `[prefix].[unit].[code].dldata`
/// - `[prefix].[unit].[code].dldata-resp`: `Some` for applications and `None` for networks.
/// - `[prefix].[unit].[code].dldata-result`
/// - `[prefix].[unit].[code].ctrl`
fn new_data_queues(
    conn: &Connection,
    opts: &Options,
    prefix: &str,
    is_network: bool,
) -> Result<
    (
        Arc<Mutex<Queue>>,
        Arc<Mutex<Queue>>,
        Option<Arc<Mutex<Queue>>>,
        Arc<Mutex<Queue>>,
        Option<Arc<Mutex<Queue>>>,
    ),
    String,
> {
    let uldata: Arc<Mutex<Queue>>;
    let dldata: Arc<Mutex<Queue>>;
    let dldata_resp: Option<Arc<Mutex<Queue>>>;
    let dldata_result: Arc<Mutex<Queue>>;
    let ctrl: Option<Arc<Mutex<Queue>>>;

    if opts.unit_id.len() == 0 {
        if opts.unit_code.len() != 0 {
            return Err("unit_id and unit_code must both empty or non-empty".to_string());
        }
    } else {
        if opts.unit_code.len() == 0 {
            return Err("unit_id and unit_code must both empty or non-empty".to_string());
        }
    }
    if opts.id.len() == 0 {
        return Err("`id` cannot be empty".to_string());
    }
    if opts.name.len() == 0 {
        return Err("`name` cannot be empty".to_string());
    }

    let unit = match opts.unit_code.len() {
        0 => "_",
        _ => opts.unit_code.as_str(),
    };

    match conn {
        Connection::Amqp(conn, _) => {
            let prefetch = match opts.prefetch {
                None => DEF_PREFETCH,
                Some(prefetch) => match prefetch {
                    0 => DEF_PREFETCH,
                    _ => prefetch,
                },
            };
            let persistent = match opts.persistent {
                None => DEF_PERSISTENT,
                Some(persistent) => persistent,
            };

            let uldata_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.uldata", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    persistent,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let dldata_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.dldata", prefix, unit, opts.name.as_str()),
                    is_recv: is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let dldata_resp_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.dldata-resp", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let dldata_result_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.dldata-result", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            let ctrl_opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}.{}.ctrl", prefix, unit, opts.name.as_str()),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    prefetch,
                    ..Default::default()
                },
                conn,
            );
            uldata = Arc::new(Mutex::new(Queue::new(uldata_opts)?));
            dldata = Arc::new(Mutex::new(Queue::new(dldata_opts)?));
            dldata_resp = match is_network {
                false => Some(Arc::new(Mutex::new(Queue::new(dldata_resp_opts)?))),
                true => None,
            };
            dldata_result = Arc::new(Mutex::new(Queue::new(dldata_result_opts)?));
            ctrl = match is_network {
                false => None,
                true => Some(Arc::new(Mutex::new(Queue::new(ctrl_opts)?))),
            };
        }
        Connection::Mqtt(conn, _) => {
            let uldata_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.uldata", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let dldata_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.dldata", prefix, unit, opts.name.as_str()),
                    is_recv: is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let dldata_resp_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.dldata-resp", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let dldata_result_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.dldata-result", prefix, unit, opts.name.as_str()),
                    is_recv: !is_network,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            let ctrl_opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: format!("{}.{}.{}.ctrl", prefix, unit, opts.name.as_str()),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: opts.shared_prefix.clone(),
                    ..Default::default()
                },
                conn,
            );
            uldata = Arc::new(Mutex::new(Queue::new(uldata_opts)?));
            dldata = Arc::new(Mutex::new(Queue::new(dldata_opts)?));
            dldata_resp = match is_network {
                false => Some(Arc::new(Mutex::new(Queue::new(dldata_resp_opts)?))),
                true => None,
            };
            dldata_result = Arc::new(Mutex::new(Queue::new(dldata_result_opts)?));
            ctrl = match is_network {
                false => None,
                true => Some(Arc::new(Mutex::new(Queue::new(ctrl_opts)?))),
            };
        }
    }

    Ok((uldata, dldata, dldata_resp, dldata_result, ctrl))
}
