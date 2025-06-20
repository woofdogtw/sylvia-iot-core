use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use rumqttc::{ClientError as RumqttError, Publish, QoS};
use tokio::{
    task::{self, JoinHandle},
    time,
};

use super::connection::{MqttConnection, PacketHandler};
use crate::{
    Error,
    connection::{GmqConnection, Status as ConnStatus},
    queue::{
        EventHandler, GmqQueue, Message, MessageHandler, QUEUE_NAME_PATTERN, Status, name_validate,
    },
};

/// Manages a MQTT queue.
#[derive(Clone)]
pub struct MqttQueue {
    /// Options of the queue.
    opts: MqttQueueOptions,
    /// The associated [`crate::MqttConnection`].
    conn: Arc<Mutex<MqttConnection>>,
    /// Queue status.
    status: Arc<Mutex<Status>>,
    /// The event handler.
    handler: Arc<Mutex<Option<Arc<dyn EventHandler>>>>,
    /// The message handler.
    msg_handler: Arc<Mutex<Option<Arc<dyn MessageHandler>>>>,
    /// The event loop to manage and monitor the connection.
    ev_loop: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// The queue options.
#[derive(Clone)]
pub struct MqttQueueOptions {
    /// The queue name that is used to map a MQTT topic.
    ///
    /// The pattern is [`QUEUE_NAME_PATTERN`].
    pub name: String,
    /// `true` for the receiver and `false` for the sender.
    pub is_recv: bool,
    /// Reliable by using QoS 1.
    pub reliable: bool,
    /// `true` for broadcast and `false` for unicast.
    ///
    /// **Note**: the unicast queue relies on **shared queue**. See the `shared_prefix` option.
    pub broadcast: bool,
    /// Time in milliseconds from disconnection to reconnection.
    ///
    /// Default or zero value is `1000`.
    pub reconnect_millis: u64,
    /// Used for `broadcast=false`.
    pub shared_prefix: Option<String>,
}

/// The MQTT [`Message`] implementation.
pub struct MqttMessage {
    /// Hold the [`rumqttc::Publish`] instance.
    packet: Publish,
}

/// Default reconnect time in milliseconds.
const DEF_RECONN_TIME_MS: u64 = 1000;

impl MqttQueue {
    /// Create a queue instance.
    pub fn new(opts: MqttQueueOptions, conn: &MqttConnection) -> Result<MqttQueue, String> {
        let name = opts.name.as_str();
        if name.len() == 0 {
            return Err("queue name cannot be empty".to_string());
        } else if !name_validate(name) {
            return Err(format!(
                "queue name {} is not match {}",
                name, QUEUE_NAME_PATTERN
            ));
        }
        let mut opts = opts;
        if opts.reconnect_millis == 0 {
            opts.reconnect_millis = DEF_RECONN_TIME_MS;
        }

        Ok(MqttQueue {
            opts,
            conn: Arc::new(Mutex::new(conn.clone())),
            status: Arc::new(Mutex::new(Status::Closed)),
            handler: Arc::new(Mutex::new(None)),
            msg_handler: Arc::new(Mutex::new(None)),
            ev_loop: Arc::new(Mutex::new(None)),
        })
    }

    /// To get the associated connection status.
    fn conn_status(&self) -> ConnStatus {
        self.conn.lock().unwrap().status()
    }

    /// To get the event handler.
    fn handler(&self) -> Option<Arc<dyn EventHandler>> {
        self.handler.lock().unwrap().clone()
    }

    /// To get the message handler.
    fn msg_handler(&self) -> Option<Arc<dyn MessageHandler>> {
        self.msg_handler.lock().unwrap().clone()
    }

    /// The error handling.
    fn on_error(&self, err: Box<dyn StdError + Send + Sync>) {
        let handler = { (*self.handler.lock().unwrap()).clone() };
        if let Some(handler) = handler {
            let q = Arc::new(self.clone());
            task::spawn(async move {
                handler.on_error(q, err).await;
            });
        }
    }

    /// To get the associated topic.
    fn topic(&self) -> String {
        if self.opts.is_recv && !self.opts.broadcast {
            if let Some(prefix) = self.opts.shared_prefix.as_ref() {
                return format!("{}{}", prefix.as_str(), self.opts.name.as_str());
            }
        }
        self.opts.name.clone()
    }

    /// To get the associated QoS.
    fn qos(&self) -> QoS {
        match self.opts.reliable {
            false => QoS::AtMostOnce,
            true => QoS::AtLeastOnce,
        }
    }
}

#[async_trait]
impl GmqQueue for MqttQueue {
    fn name(&self) -> &str {
        self.opts.name.as_str()
    }

    fn is_recv(&self) -> bool {
        self.opts.is_recv
    }

    fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    fn set_handler(&mut self, handler: Arc<dyn EventHandler>) {
        *self.handler.lock().unwrap() = Some(handler);
    }

    fn clear_handler(&mut self) {
        let _ = (*self.handler.lock().unwrap()).take();
    }

    fn set_msg_handler(&mut self, handler: Arc<dyn MessageHandler>) {
        *self.msg_handler.lock().unwrap() = Some(handler);
    }

    fn connect(&mut self) -> Result<(), Box<dyn StdError>> {
        if self.opts.is_recv && self.msg_handler().is_none() {
            return Err(Box::new(Error::NoMsgHandler));
        }

        {
            let mut task_handle_mutex = self.ev_loop.lock().unwrap();
            if (*task_handle_mutex).is_some() {
                return Ok(());
            }
            *self.status.lock().unwrap() = Status::Connecting;
            *task_handle_mutex = Some(create_event_loop(self));
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        match { self.ev_loop.lock().unwrap().take() } {
            None => return Ok(()),
            Some(handle) => handle.abort(),
        }
        {
            *self.status.lock().unwrap() = Status::Closing;
        }

        let raw_conn;
        {
            let mut conn = self.conn.lock().unwrap();
            conn.remove_packet_handler(self.opts.name.as_str());
            raw_conn = conn.get_raw_connection();
        }

        let mut result: Result<(), RumqttError> = Ok(());
        if let Some(raw_conn) = raw_conn {
            result = raw_conn.unsubscribe(self.topic()).await;
        }

        {
            *self.status.lock().unwrap() = Status::Closed;
        }
        if let Some(handler) = { (*self.handler.lock().unwrap()).clone() } {
            let queue = Arc::new(self.clone());
            task::spawn(async move {
                handler.on_status(queue, Status::Closed).await;
            });
        }

        result?;
        Ok(())
    }

    async fn send_msg(&self, payload: Vec<u8>) -> Result<(), Box<dyn StdError + Send + Sync>> {
        if self.opts.is_recv {
            return Err(Box::new(Error::QueueIsReceiver));
        } else if self.status() != Status::Connected {
            return Err(Box::new(Error::NotConnected));
        }

        let raw_conn = {
            match self.conn.lock().unwrap().get_raw_connection() {
                None => return Err(Box::new(Error::NotConnected)),
                Some(raw_conn) => raw_conn,
            }
        };

        raw_conn
            .publish(self.topic(), self.qos(), false, payload)
            .await?;
        Ok(())
    }
}

impl PacketHandler for MqttQueue {
    fn on_publish(&self, packet: Publish) {
        if let Some(handler) = self.msg_handler() {
            let this = Arc::new(self.clone());
            task::spawn(async move {
                handler
                    .on_message(this, Box::new(MqttMessage::new(packet)))
                    .await;
            });
        }
    }
}

impl Default for MqttQueueOptions {
    fn default() -> Self {
        MqttQueueOptions {
            name: "".to_string(),
            is_recv: false,
            reliable: false,
            broadcast: false,
            reconnect_millis: DEF_RECONN_TIME_MS,
            shared_prefix: None,
        }
    }
}

impl MqttMessage {
    /// Create a message instance.
    pub fn new(packet: Publish) -> Self {
        MqttMessage { packet }
    }
}

#[async_trait]
impl Message for MqttMessage {
    fn payload(&self) -> &[u8] {
        self.packet.payload.as_ref()
    }

    async fn ack(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        Ok(())
    }

    async fn nack(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        Ok(())
    }
}

/// To create an event loop runtime task.
fn create_event_loop(queue: &MqttQueue) -> JoinHandle<()> {
    let this = Arc::new(queue.clone());
    task::spawn(async move {
        loop {
            match this.status() {
                Status::Closing | Status::Closed => task::yield_now().await,
                Status::Connecting => {
                    if this.conn_status() != ConnStatus::Connected {
                        time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                        continue;
                    }

                    if this.opts.is_recv {
                        let raw_conn;
                        {
                            let mut conn = this.conn.lock().unwrap();
                            conn.add_packet_handler(this.opts.name.as_str(), this.clone());
                            raw_conn = conn.get_raw_connection();
                        }
                        if let Some(raw_conn) = raw_conn {
                            if let Err(e) = raw_conn.subscribe(this.topic(), this.qos()).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }
                        } else {
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }
                    }

                    {
                        *this.status.lock().unwrap() = Status::Connected;
                    }
                    if let Some(handler) = this.handler() {
                        let queue = this.clone();
                        task::spawn(async move {
                            handler.on_status(queue, Status::Connected).await;
                        });
                    }
                }
                Status::Connected => {
                    time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                    if this.conn_status() != ConnStatus::Connected {
                        if let Some(handler) = this.handler() {
                            let queue = this.clone();
                            task::spawn(async move {
                                handler.on_status(queue, Status::Connecting).await;
                            });
                        }
                        *this.status.lock().unwrap() = Status::Connecting;
                    }
                }
                Status::Disconnected => {
                    *this.status.lock().unwrap() = Status::Connecting;
                }
            }
        }
    })
}
