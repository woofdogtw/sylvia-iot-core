use std::{
    collections::HashMap,
    error::Error as StdError,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_lock::Mutex as AsyncMutex;
use async_trait::async_trait;
use lapin::{
    uri::AMQPUri, Channel, Connection as LapinConnection, ConnectionProperties, Error as LapinError,
};
use tokio::{
    task::{self, JoinHandle},
    time,
};
use tokio_executor_trait;

use crate::{
    connection::{Connection, Event, EventHandler, Status},
    randomstring, Error, ID_SIZE,
};

/// Manages an AMQP connection.
#[derive(Clone)]
pub struct AmqpConnection {
    /// Options of the connection.
    opts: InnerOptions,
    /// Connection status.
    status: Arc<Mutex<Status>>,
    /// Hold the connection instance.
    conn: Arc<AsyncMutex<Option<LapinConnection>>>,
    /// Event handlers.
    handlers: Arc<Mutex<HashMap<String, Arc<dyn EventHandler>>>>,
    /// The event loop to manage and monitor the connection instance.
    ev_loop: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// The connection options.
pub struct AmqpConnectionOptions {
    /// Connection URI. Use `amqp|amqps://username:password@host:port/vhost` format.
    ///
    /// Default is `amqp://localhost/%2f`.
    pub uri: String,
    /// Connection timeout in milliseconds.
    ///
    /// Default or zero value is `3000`.
    pub connect_timeout_millis: u64,
    /// Time in milliseconds from disconnection to reconnection.
    ///
    /// Default or zero value is `1000`.
    pub reconnect_millis: u64,
}

/// The validated options for management.
#[derive(Clone)]
struct InnerOptions {
    /// The formatted URI resource.
    uri: AMQPUri,
    /// Time in milliseconds from disconnection to reconnection.
    reconnect_millis: u64,
}

/// Default connect timeout in milliseconds.
const DEF_CONN_TIMEOUT_MS: u64 = 3000;
/// Default reconnect time in milliseconds.
const DEF_RECONN_TIMEOUT_MS: u64 = 1000;

impl AmqpConnection {
    /// Create a connection instance.
    pub fn new(opts: AmqpConnectionOptions) -> Result<AmqpConnection, String> {
        let mut uri = AMQPUri::from_str(opts.uri.as_str())?;
        uri.query.connection_timeout = match opts.connect_timeout_millis {
            0 => Some(DEF_CONN_TIMEOUT_MS),
            _ => Some(opts.connect_timeout_millis),
        };
        if uri.vhost.len() == 0 {
            uri.vhost = "/".to_string();
        }

        Ok(AmqpConnection {
            opts: InnerOptions {
                uri,
                reconnect_millis: match opts.reconnect_millis {
                    0 => DEF_RECONN_TIMEOUT_MS,
                    _ => opts.reconnect_millis,
                },
            },
            status: Arc::new(Mutex::new(Status::Closed)),
            conn: Arc::new(AsyncMutex::new(None)),
            handlers: Arc::new(Mutex::new(HashMap::<String, Arc<dyn EventHandler>>::new())),
            ev_loop: Arc::new(Mutex::new(None)),
        })
    }

    /// For AmqpQueue to declare a channel.
    ///
    /// This is a helper to utilize the connection instance because [`lapin::Connection`] does not
    /// implement [`Clone`].
    pub(super) async fn create_channel(&self) -> Result<Channel, Box<dyn StdError + Send + Sync>> {
        match (*self.conn.lock().await).as_ref() {
            None => return Err(Box::new(Error::NotConnected)),
            // TODO: this may cause lock too long.
            Some(conn) => match conn.create_channel().await {
                Err(e) => Err(Box::new(e)),
                Ok(channel) => Ok(channel),
            },
        }
    }
}

#[async_trait]
impl Connection for AmqpConnection {
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

        let conn = { self.conn.lock().await.take() };
        let mut result: Result<(), LapinError> = Ok(());
        if let Some(conn) = conn {
            result = conn.close(0, "").await;
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

impl Default for AmqpConnectionOptions {
    fn default() -> Self {
        AmqpConnectionOptions {
            uri: "amqp://localhost".to_string(),
            connect_timeout_millis: 3000,
            reconnect_millis: 1000,
        }
    }
}

/// To create an event loop runtime task.
fn create_event_loop(conn: &AmqpConnection) -> JoinHandle<()> {
    let this = Arc::new(conn.clone());
    task::spawn(async move {
        loop {
            match this.status() {
                Status::Closing | Status::Closed => break,
                Status::Connecting => {
                    let opts = ConnectionProperties::default()
                        .with_executor(tokio_executor_trait::Tokio::current());
                    let conn = match LapinConnection::connect_uri(this.opts.uri.clone(), opts).await
                    {
                        Err(_) => {
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }
                        Ok(conn) => conn,
                    };
                    {
                        let mut status_mutex = this.status.lock().unwrap();
                        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
                            continue;
                        }
                        *status_mutex = Status::Connected;
                    }
                    // FIXME: to lock before Connected.
                    {
                        *this.conn.lock().await = Some(conn);
                    }

                    let handlers = { (*this.handlers.lock().unwrap()).clone() };
                    for (id, handler) in handlers {
                        let conn = this.clone();
                        task::spawn(async move {
                            handler
                                .on_event(id.clone(), conn, Event::Status(Status::Connected))
                                .await;
                        });
                    }
                }
                Status::Connected => {
                    time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                    let mut to_disconnected = true;
                    {
                        if let Some(conn) = (*this.conn.lock().await).as_ref() {
                            if conn.status().connected() {
                                to_disconnected = false;
                            }
                        }
                    }
                    if !to_disconnected {
                        continue;
                    }

                    {
                        let mut status_mutex = this.status.lock().unwrap();
                        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
                            continue;
                        }
                        *status_mutex = Status::Disconnected;
                    }
                    // FIXME: to lock before Disconnected.
                    {
                        let _ = this.conn.lock().await.take();
                    }

                    let handlers = { (*this.handlers.lock().unwrap()).clone() };
                    for (id, handler) in handlers {
                        let conn = this.clone();
                        task::spawn(async move {
                            handler
                                .on_event(id.clone(), conn, Event::Status(Status::Disconnected))
                                .await;
                        });
                    }
                    time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                    {
                        let mut status_mutex = this.status.lock().unwrap();
                        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
                            continue;
                        }
                        *status_mutex = Status::Connecting;
                    }
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
                Status::Disconnected => {
                    *this.status.lock().unwrap() = Status::Connecting;
                }
            }
        }
    })
}
