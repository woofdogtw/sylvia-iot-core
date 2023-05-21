use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use general_mq::{
    queue::{EventHandler, GmqQueue},
    AmqpQueueOptions, MqttQueueOptions, Queue, QueueOptions,
};
use url::Url;

use super::{get_connection, Connection};

const QUEUE_NAME: &'static str = "coremgr.data";

/// To create a reliable unicast queue to send data messages.
pub fn new(
    conn_pool: &Arc<Mutex<HashMap<String, Connection>>>,
    host_uri: &Url,
    handler: Arc<dyn EventHandler>,
) -> Result<Queue, String> {
    let conn = get_connection(&conn_pool, host_uri)?;
    let mut queue = match conn {
        Connection::Amqp(conn, counter) => {
            let opts = QueueOptions::Amqp(
                AmqpQueueOptions {
                    name: QUEUE_NAME.to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
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
                    name: QUEUE_NAME.to_string(),
                    is_recv: false,
                    reliable: true,
                    broadcast: false,
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
    if let Err(e) = queue.connect() {
        return Err(e.to_string());
    }
    Ok(queue)
}
