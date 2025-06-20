use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use url::Url;

use general_mq::{
    AmqpQueueOptions, MqttQueueOptions, Queue, QueueOptions,
    queue::{EventHandler, GmqQueue, MessageHandler},
};

use super::{Connection, get_connection};

const QUEUE_PREFIX: &'static str = "broker.ctrl";

/// The default prefetch value for AMQP.
const DEF_PREFETCH: u16 = 100;

/// To create a broadcast queue for a function to send or receive control messages.
pub fn new(
    conn_pool: Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: &Url,
    prefetch: Option<u16>,
    func_name: &str,
    is_recv: bool,
    handler: Arc<dyn EventHandler>,
    msg_handler: Arc<dyn MessageHandler>,
) -> Result<Queue, String> {
    if func_name.len() == 0 {
        return Err("`func_name` cannot be empty for control queue".to_string());
    }

    let conn = get_connection(&conn_pool, host_uri)?;
    let mut queue = match conn {
        Connection::Amqp(conn, counter) => {
            let prefetch = match prefetch {
                None => DEF_PREFETCH,
                Some(prefetch) => match prefetch {
                    0 => DEF_PREFETCH,
                    _ => prefetch,
                },
            };
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: format!("{}.{}", QUEUE_PREFIX, func_name),
                    is_recv,
                    reliable: true,
                    broadcast: true,
                    prefetch,
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
                    name: format!("{}.{}", QUEUE_PREFIX, func_name),
                    is_recv,
                    reliable: true,
                    broadcast: true,
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
    if is_recv {
        queue.set_msg_handler(msg_handler);
    }
    if let Err(e) = queue.connect() {
        return Err(e.to_string());
    }
    Ok(queue)
}
