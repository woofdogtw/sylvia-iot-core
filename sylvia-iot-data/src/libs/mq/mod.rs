use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use url::Url;

use general_mq::{
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
    connection::GmqConnection,
    queue::{EventHandler, GmqQueue, MessageHandler},
};

pub mod broker;
pub mod coremgr;

use super::config::DataData as DataMqConfig;

/// The general connection type with reference counter for upper layer maintenance.
#[derive(Clone)]
pub enum Connection {
    Amqp(AmqpConnection, Arc<Mutex<isize>>),
    Mqtt(MqttConnection, Arc<Mutex<isize>>),
}

/// The default prefetch value for AMQP.
const DEF_PREFETCH: u16 = 100;

/// To create a reliable unicast queue to receive data messages.
fn new_data_queue(
    conn_pool: &mut HashMap<String, Connection>,
    config: &DataMqConfig,
    queue_name: &str,
    handler: Arc<dyn EventHandler>,
    msg_handler: Arc<dyn MessageHandler>,
) -> Result<Queue, String> {
    let host_uri = match config.url.as_ref() {
        None => return Err("host_uri empty".to_string()),
        Some(host_uri) => match Url::parse(host_uri) {
            Err(e) => return Err(format!("host_uri error: {}", e)),
            Ok(uri) => uri,
        },
    };
    let conn = get_connection(conn_pool, &host_uri)?;
    let mut queue = match conn {
        Connection::Amqp(conn, counter) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: queue_name.to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    prefetch: match config.prefetch {
                        None => DEF_PREFETCH,
                        Some(prefetch) => prefetch,
                    },
                    ..Default::default()
                },
                &conn,
            );
            {
                *counter.lock().unwrap() += 1;
            }
            Queue::new(opts)?
        }
        Connection::Mqtt(conn, counter) => {
            let opts = QueueOptions::Mqtt(
                MqttQueueOptions {
                    name: queue_name.to_string(),
                    is_recv: true,
                    reliable: true,
                    broadcast: false,
                    shared_prefix: config.shared_prefix.clone(),
                    ..Default::default()
                },
                &conn,
            );
            {
                *counter.lock().unwrap() += 1;
            }
            Queue::new(opts)?
        }
    };
    queue.set_handler(handler);
    queue.set_msg_handler(msg_handler);
    if let Err(e) = queue.connect() {
        return Err(e.to_string());
    }
    Ok(queue)
}

/// Utility function to get the message queue connection instance. A new connection will be created
/// if the host does not exist.
fn get_connection(
    conn_pool: &mut HashMap<String, Connection>,
    host_uri: &Url,
) -> Result<Connection, String> {
    let uri = host_uri.to_string();
    if let Some(conn) = conn_pool.get(&uri) {
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
            conn_pool.insert(uri, conn.clone());
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
            conn_pool.insert(uri, conn.clone());
            Ok(conn)
        }
        s => Err(format!("unsupport scheme {}", s)),
    }
}
