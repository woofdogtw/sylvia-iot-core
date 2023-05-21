use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use general_mq::{
    connection::GmqConnection, AmqpConnection, AmqpConnectionOptions, MqttConnection,
    MqttConnectionOptions,
};
use url::Url;

pub mod data;
pub mod emqx;
pub mod rabbitmq;
pub mod rumqttd;

/// The general connection type with reference counter for upper layer maintenance.
#[derive(Clone)]
pub enum Connection {
    Amqp(AmqpConnection, Arc<Mutex<isize>>),
    Mqtt(MqttConnection, Arc<Mutex<isize>>),
}

/// Broker message queue type.
pub enum QueueType {
    Application,
    Network,
}

impl fmt::Display for QueueType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            QueueType::Application => fmt.write_str("application"),
            QueueType::Network => fmt.write_str("network"),
        }
    }
}

impl Copy for QueueType {}

impl Clone for QueueType {
    fn clone(&self) -> QueueType {
        *self
    }
}

/// Transfer queue type, unit code, application/network code to AMQP virtual host name and queue
/// name.
pub fn to_username(q_type: QueueType, unit: &str, code: &str) -> String {
    format!("{}.{}.{}", q_type, unit_code(unit), code)
}

/// Unit code part for queue name.
fn unit_code(code: &str) -> &str {
    match code {
        "" => "_",
        _ => code,
    }
}

/// Utility function to get the message queue connection instance. A new connection will be created
/// if the host does not exist.
fn get_connection(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: &Url,
) -> Result<Connection, String> {
    let uri = host_uri.to_string();
    {
        let mutex = conn_pool.lock().unwrap();
        if let Some(conn) = mutex.get(&uri) {
            return Ok(conn.clone());
        }
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
            {
                conn_pool.lock().unwrap().insert(uri, conn.clone());
            }
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
            {
                conn_pool.lock().unwrap().insert(uri, conn.clone());
            }
            Ok(conn)
        }
        s => Err(format!("unsupport scheme {}", s)),
    }
}
