use std::{
    collections::HashMap,
    error::Error as StdError,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use amqprs::{
    connection::{Connection as AmqprsConnection, OpenConnectionArguments},
    error::Error as AmqprsError,
    security::SecurityCredentials,
    tls::TlsAdaptor,
};
use async_trait::async_trait;
use lapin::uri::{AMQPScheme, AMQPUri};
use tokio::{
    task::{self, JoinHandle},
    time,
};

use crate::{
    ID_SIZE,
    connection::{EventHandler, GmqConnection, Status},
    randomstring,
};

/// Manages an AMQP connection.
#[derive(Clone)]
pub struct AmqpConnection {
    /// Options of the connection.
    opts: InnerOptions,
    /// Connection status.
    status: Arc<Mutex<Status>>,
    /// Hold the connection instance.
    conn: Arc<Mutex<Option<AmqprsConnection>>>,
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
    args: OpenConnectionArguments,
    /// Time in milliseconds from disconnection to reconnection.
    reconnect_millis: u64,
}

/// Default connect timeout in milliseconds.
const DEF_CONN_TIMEOUT_MS: u64 = 3000;
/// Default reconnect time in milliseconds.
const DEF_RECONN_TIME_MS: u64 = 1000;

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
        let mut args = OpenConnectionArguments::default();
        args.host(&uri.authority.host)
            .port(uri.authority.port)
            .credentials(SecurityCredentials::new_plain(
                &uri.authority.userinfo.username,
                &uri.authority.userinfo.password,
            ))
            .virtual_host(&uri.vhost);
        if uri.scheme == AMQPScheme::AMQPS {
            let adaptor = match TlsAdaptor::without_client_auth(None, uri.authority.host.clone()) {
                Err(e) => return Err(e.to_string()),
                Ok(adaptor) => adaptor,
            };
            args.tls_adaptor(adaptor);
        }

        Ok(AmqpConnection {
            opts: InnerOptions {
                args,
                reconnect_millis: match opts.reconnect_millis {
                    0 => DEF_RECONN_TIME_MS,
                    _ => opts.reconnect_millis,
                },
            },
            status: Arc::new(Mutex::new(Status::Closed)),
            conn: Arc::new(Mutex::new(None)),
            handlers: Arc::new(Mutex::new(HashMap::<String, Arc<dyn EventHandler>>::new())),
            ev_loop: Arc::new(Mutex::new(None)),
        })
    }

    /// To get the raw AMQP connection instance for channel declaration.
    pub(super) fn get_raw_connection(&self) -> Option<AmqprsConnection> {
        match self.conn.lock().unwrap().as_ref() {
            None => None,
            Some(conn) => Some(conn.clone()),
        }
    }
}

#[async_trait]
impl GmqConnection for AmqpConnection {
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

        let conn = { self.conn.lock().unwrap().take() };
        let mut result: Result<(), AmqprsError> = Ok(());
        if let Some(conn) = conn {
            result = conn.close().await;
        }

        {
            *self.status.lock().unwrap() = Status::Closed;
        }
        let handlers = { (*self.handlers.lock().unwrap()).clone() };
        for (id, handler) in handlers {
            let conn = Arc::new(self.clone());
            task::spawn(async move {
                handler.on_status(id.clone(), conn, Status::Closed).await;
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
            connect_timeout_millis: DEF_CONN_TIMEOUT_MS,
            reconnect_millis: DEF_RECONN_TIME_MS,
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
                    let conn = match AmqprsConnection::open(&this.opts.args).await {
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
                    {
                        *this.conn.lock().unwrap() = Some(conn);
                    }

                    let handlers = { (*this.handlers.lock().unwrap()).clone() };
                    for (id, handler) in handlers {
                        let conn = this.clone();
                        task::spawn(async move {
                            handler.on_status(id.clone(), conn, Status::Connected).await;
                        });
                    }
                }
                Status::Connected => {
                    time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                    let mut to_disconnected = true;
                    {
                        if let Some(conn) = (*this.conn.lock().unwrap()).as_ref() {
                            if conn.is_open() {
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
                    let conn = { this.conn.lock().unwrap().take() };
                    if let Some(conn) = conn {
                        let _ = conn.close().await;
                    }

                    let handlers = { (*this.handlers.lock().unwrap()).clone() };
                    for (id, handler) in handlers {
                        let conn = this.clone();
                        task::spawn(async move {
                            handler
                                .on_status(id.clone(), conn, Status::Disconnected)
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
                                .on_status(id.clone(), conn, Status::Connecting)
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
