use std::{
    collections::HashMap,
    error::Error as StdError,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use regex::Regex;
use rumqttc::{
    AsyncClient as RumqttConnection, ClientError, Event as RumqttEvent,
    MqttOptions as RumqttOption, NetworkOptions, Packet, Publish, TlsConfiguration, Transport,
};
use tokio::{
    task::{self, JoinHandle},
    time,
};

use super::uri::{MQTTScheme, MQTTUri};
use crate::{
    connection::{Connection, Event, EventHandler, Status},
    randomstring, ID_SIZE,
};

/// Manages a MQTT connection.
#[derive(Clone)]
pub struct MqttConnection {
    /// Options of the connection.
    opts: InnerOptions,
    /// Connection status.
    status: Arc<Mutex<Status>>,
    /// Hold the connection instance.
    conn: Arc<Mutex<Option<RumqttConnection>>>,
    /// Event handlers.
    handlers: Arc<Mutex<HashMap<String, Arc<dyn EventHandler>>>>,
    /// Publish packet handlers. The key is **the queue name**.
    ///
    /// Because MQTT is connection-driven, the receiver [`crate::MqttQueue`] queues must register a
    /// handler to receive Publish packets.
    packet_handlers: Arc<Mutex<HashMap<String, Arc<dyn PacketHandler>>>>,
    /// The event loop to manage and monitor the connection instance.
    ev_loop: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// The connection options.
pub struct MqttConnectionOptions {
    /// Connection URI. Use `mqtt|mqtts://username:password@host:port` format.
    ///
    /// Default is `mqtt://localhost`.
    pub uri: String,
    /// Connection timeout in milliseconds.
    ///
    /// Default or zero value is `3000`.
    pub connect_timeout_millis: u64,
    /// Time in milliseconds from disconnection to reconnection.
    ///
    /// Default or zero value is `1000`.
    pub reconnect_millis: u64,
    /// Client identifier. Use `None` to generate a random client identifier.
    pub client_id: Option<String>,
    /// Clean session flag.
    ///
    /// **Note**: this is not stable.
    pub clean_session: bool,
}

/// Packet handler definitions.
pub(super) trait PacketHandler: Send + Sync {
    /// For **Publish** packets.
    fn on_publish(&self, packet: Publish);
}

/// The validated options for management.
#[derive(Clone)]
struct InnerOptions {
    /// The formatted URI resource.
    uri: MQTTUri,
    /// Connection timeout in milliseconds.
    connect_timeout_millis: u64,
    /// Time in milliseconds from disconnection to reconnection.
    reconnect_millis: u64,
    /// Client identifier.
    client_id: String,
    /// Clean session flag.
    clean_session: bool,
}

/// The accepted pattern of the client identifier.
const CLIENT_ID_PATTERN: &'static str = "^[0-9A-Za-z-]{1,23}$";

impl MqttConnection {
    /// Create a connection instance.
    pub fn new(opts: MqttConnectionOptions) -> Result<MqttConnection, String> {
        let uri = MQTTUri::from_str(opts.uri.as_str())?;

        Ok(MqttConnection {
            opts: InnerOptions {
                uri,
                connect_timeout_millis: match opts.connect_timeout_millis {
                    0 => 3000,
                    _ => opts.connect_timeout_millis,
                },
                reconnect_millis: match opts.reconnect_millis {
                    0 => 1000,
                    _ => opts.reconnect_millis,
                },
                client_id: match opts.client_id {
                    None => format!("general-mq-{}", randomstring(12)),
                    Some(client_id) => {
                        let re = Regex::new(CLIENT_ID_PATTERN).unwrap();
                        if !re.is_match(client_id.as_str()) {
                            return Err(format!("client_id is not match {}", CLIENT_ID_PATTERN));
                        }
                        client_id
                    }
                },
                clean_session: opts.clean_session,
            },
            status: Arc::new(Mutex::new(Status::Closed)),
            conn: Arc::new(Mutex::new(None)),
            handlers: Arc::new(Mutex::new(HashMap::<String, Arc<dyn EventHandler>>::new())),
            packet_handlers: Arc::new(Mutex::new(HashMap::<String, Arc<dyn PacketHandler>>::new())),
            ev_loop: Arc::new(Mutex::new(None)),
        })
    }

    /// To add a packet handler for [`crate::MqttQueue`]. The `name` is **the queue name**.
    pub(super) fn add_packet_handler(&mut self, name: &str, handler: Arc<dyn PacketHandler>) {
        self.packet_handlers
            .lock()
            .unwrap()
            .insert(name.to_string(), handler);
    }

    /// To remove a packet handler. The `name` is **the queue name**.
    pub(super) fn remove_packet_handler(&mut self, name: &str) {
        self.packet_handlers.lock().unwrap().remove(name);
    }

    /// To get the raw MQTT connection instance for topic operations such as subscribe or publish.
    pub(super) fn get_raw_connection(&self) -> Option<RumqttConnection> {
        match self.conn.lock().unwrap().as_ref() {
            None => None,
            Some(conn) => Some(conn.clone()),
        }
    }
}

#[async_trait]
impl Connection for MqttConnection {
    fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    fn add_handler(&mut self, handler: Arc<dyn EventHandler>) -> String {
        let id = randomstring(ID_SIZE);
        self.handlers.lock().unwrap().insert(id.clone(), handler);
        id
    }

    fn remove_handler(&mut self, id: &str) {
        self.handlers.lock().unwrap().remove(id);
    }

    fn connect(&mut self) -> Result<(), Box<dyn StdError>> {
        {
            let mut task_handle_mutex = self.ev_loop.lock().unwrap();
            if (*task_handle_mutex).is_some() {
                return Ok(());
            }
            *task_handle_mutex = Some(create_event_loop(self));
            *self.status.lock().unwrap() = Status::Connecting;
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn StdError>> {
        match { self.ev_loop.lock().unwrap().take() } {
            None => return Ok(()),
            Some(handle) => handle.abort(),
        }
        {
            *self.status.lock().unwrap() = Status::Closing;
        }

        let conn = { self.conn.lock().unwrap().take() };
        let mut result: Result<(), ClientError> = Ok(());
        if let Some(conn) = conn {
            result = conn.disconnect().await;
        }

        {
            *self.status.lock().unwrap() = Status::Closed;
        }
        let handlers = { (*self.handlers.lock().unwrap()).clone() };
        for (id, handler) in handlers {
            let conn = Arc::new(self.clone());
            task::spawn(async move {
                handler
                    .on_event(id.clone(), conn, Event::Status(Status::Closed))
                    .await;
            });
        }

        result?;
        Ok(())
    }
}

impl Default for MqttConnectionOptions {
    fn default() -> Self {
        MqttConnectionOptions {
            uri: "mqtt://localhost".to_string(),
            connect_timeout_millis: 3000,
            reconnect_millis: 1000,
            client_id: None,
            clean_session: true,
        }
    }
}

/// To create an event loop runtime task.
fn create_event_loop(conn: &MqttConnection) -> JoinHandle<()> {
    let this = Arc::new(conn.clone());
    task::spawn(async move {
        loop {
            match this.status() {
                Status::Closing | Status::Closed => task::yield_now().await,
                Status::Connecting | Status::Connected => {
                    let mut opts = RumqttOption::new(
                        this.opts.client_id.as_str(),
                        this.opts.uri.host.as_str(),
                        this.opts.uri.port,
                    );
                    opts.set_clean_session(this.opts.clean_session)
                        .set_credentials(
                            this.opts.uri.username.as_str(),
                            this.opts.uri.password.as_str(),
                        );
                    if this.opts.uri.scheme == MQTTScheme::MQTTS {
                        opts.set_transport(Transport::Tls(TlsConfiguration::default()));
                    }

                    let mut to_disconnected = false;
                    let (client, mut event_loop) = RumqttConnection::new(opts, 10);
                    let mut net_opts = NetworkOptions::new();
                    net_opts.set_connection_timeout(this.opts.connect_timeout_millis);
                    event_loop.set_network_options(net_opts);
                    loop {
                        match event_loop.poll().await {
                            Err(_) => {
                                if this.status() == Status::Connected {
                                    to_disconnected = true;
                                }
                                break;
                            }
                            Ok(event) => {
                                let packet = match event {
                                    RumqttEvent::Incoming(packet) => packet,
                                    _ => continue,
                                };
                                match packet {
                                    Packet::Publish(packet) => {
                                        if this.status() != Status::Connected {
                                            continue;
                                        }
                                        let handler = {
                                            let topic = packet.topic.as_str();
                                            match this.packet_handlers.lock().unwrap().get(topic) {
                                                None => continue,
                                                Some(handler) => handler.clone(),
                                            }
                                        };
                                        handler.on_publish(packet);
                                    }
                                    Packet::ConnAck(_) => {
                                        let mut to_connected = false;
                                        {
                                            let mut status_mutex = this.status.lock().unwrap();
                                            let status = *status_mutex;
                                            if status == Status::Closing || status == Status::Closed
                                            {
                                                break;
                                            } else if status != Status::Connected {
                                                *this.conn.lock().unwrap() = Some(client.clone());
                                                *status_mutex = Status::Connected;
                                                to_connected = true;
                                            }
                                        }

                                        if to_connected {
                                            let handlers =
                                                { (*this.handlers.lock().unwrap()).clone() };
                                            for (id, handler) in handlers {
                                                let conn = this.clone();
                                                task::spawn(async move {
                                                    let status = Event::Status(Status::Connected);
                                                    handler
                                                        .on_event(id.clone(), conn, status)
                                                        .await;
                                                });
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    {
                        let mut status_mutex = this.status.lock().unwrap();
                        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
                            continue;
                        }
                        let _ = this.conn.lock().unwrap().take();
                        *status_mutex = Status::Disconnected;
                    }

                    if to_disconnected {
                        let handlers = { (*this.handlers.lock().unwrap()).clone() };
                        for (id, handler) in handlers {
                            let conn = this.clone();
                            task::spawn(async move {
                                handler
                                    .on_event(id.clone(), conn, Event::Status(Status::Disconnected))
                                    .await;
                            });
                        }
                    }
                    time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                    {
                        let mut status_mutex = this.status.lock().unwrap();
                        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
                            continue;
                        }
                        *status_mutex = Status::Connecting;
                    }
                    if to_disconnected {
                        let handlers = { (*this.handlers.lock().unwrap()).clone() };
                        for (id, handler) in handlers {
                            let conn = this.clone();
                            task::spawn(async move {
                                handler
                                    .on_event(id.clone(), conn, Event::Status(Status::Connecting))
                                    .await;
                            });
                        }
                    }
                }
                Status::Disconnected => {
                    *this.status.lock().unwrap() = Status::Connecting;
                }
            }
        }
    })
}
