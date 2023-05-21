use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
    time::Duration,
};

use amqprs::{
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicNackArguments, BasicPublishArguments,
        BasicQosArguments, Channel, ConfirmSelectArguments, ExchangeDeclareArguments, ExchangeType,
        QueueBindArguments, QueueDeclareArguments,
    },
    consumer::AsyncConsumer,
    error::Error as AmqprsError,
    BasicProperties, Deliver,
};
use async_trait::async_trait;
use tokio::{
    task::{self, JoinHandle},
    time,
};

use super::connection::AmqpConnection;
use crate::{
    connection::{GmqConnection, Status as ConnStatus},
    queue::{name_validate, Event, EventHandler, GmqQueue, Message, Status, QUEUE_NAME_PATTERN},
    Error,
};

/// Manages an AMQP queue.
#[derive(Clone)]
pub struct AmqpQueue {
    /// Options of the queue.
    opts: AmqpQueueOptions,
    /// The associated [`crate::AmqpConnection`].
    conn: Arc<Mutex<AmqpConnection>>,
    /// Hold the channel instance.
    channel: Arc<Mutex<Option<Channel>>>,
    /// Queue status.
    status: Arc<Mutex<Status>>,
    /// The event handler.
    handler: Arc<Mutex<Option<Arc<dyn EventHandler>>>>,
    /// The event loop to manage and monitor the channel instance.
    ev_loop: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// The queue options.
#[derive(Clone)]
pub struct AmqpQueueOptions {
    /// The queue name that is used to map a AMQP queue (unicast) or an exchange (broadcast).
    ///
    /// The pattern is [`QUEUE_NAME_PATTERN`].
    pub name: String,
    /// `true` for the receiver and `false` for the sender.
    pub is_recv: bool,
    /// Reliable by selecting the confirm channel (for publish).
    pub reliable: bool,
    /// `true` for broadcast and `false` for unicast.
    pub broadcast: bool,
    /// Time in milliseconds from disconnection to reconnection.
    ///
    /// Default or zero value is `1000`.
    pub reconnect_millis: u64,
    /// The QoS of the receiver queue.
    ///
    /// **Note**: this value **MUST** be a positive value.
    pub prefetch: u16,
}

/// The AMQP [`Message`] implementation.
struct AmqpMessage {
    /// Hold the consumer callback channel to operate ack/nack.
    channel: Channel,
    /// Hold the consumer callback deliver to operate ack/nack.
    delivery_tag: u64,
    /// Hold the consumer callback content.
    content: Vec<u8>,
}

/// The [`amqprs::consumer::AsyncConsumer`] implementation.
struct Consumer {
    /// The associated [`AmqpQueue`].
    queue: Arc<AmqpQueue>,
}

/// Default reconnect time in milliseconds.
const DEF_RECONN_TIMEOUT_MS: u64 = 1000;

impl AmqpQueue {
    /// Create a queue instance.
    pub fn new(opts: AmqpQueueOptions, conn: &AmqpConnection) -> Result<AmqpQueue, String> {
        let name = opts.name.as_str();
        if name.len() == 0 {
            return Err("queue name cannot be empty".to_string());
        } else if !name_validate(name) {
            return Err(format!(
                "queue name {} is not match {}",
                name, QUEUE_NAME_PATTERN
            ));
        } else if opts.is_recv && opts.prefetch == 0 {
            return Err("prefetch cannot be zero for a receiver".to_string());
        }
        let mut opts = opts;
        if opts.reconnect_millis == 0 {
            opts.reconnect_millis = DEF_RECONN_TIMEOUT_MS;
        }

        Ok(AmqpQueue {
            opts,
            conn: Arc::new(Mutex::new(conn.clone())),
            channel: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(Status::Closed)),
            handler: Arc::new(Mutex::new(None)),
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

    /// The error handling.
    fn on_error(&self, err: Box<dyn StdError + Send + Sync>) {
        let handler = { (*self.handler.lock().unwrap()).clone() };
        if let Some(handler) = handler {
            let q = Arc::new(self.clone());
            task::spawn(async move {
                handler.on_event(q, Event::Error(err)).await;
            });
        }
    }
}

#[async_trait]
impl GmqQueue for AmqpQueue {
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

    async fn close(&mut self) -> Result<(), Box<dyn StdError>> {
        match { self.ev_loop.lock().unwrap().take() } {
            None => return Ok(()),
            Some(handle) => handle.abort(),
        }
        {
            *self.status.lock().unwrap() = Status::Closing;
        }

        let channel = { self.channel.lock().unwrap().take() };

        let mut result: Result<(), AmqprsError> = Ok(());
        if let Some(channel) = channel {
            result = channel.close().await;
        }

        {
            *self.status.lock().unwrap() = Status::Closed;
        }
        if let Some(handler) = { (*self.handler.lock().unwrap()).clone() } {
            let queue = Arc::new(self.clone());
            task::spawn(async move {
                handler.on_event(queue, Event::Status(Status::Closed)).await;
            });
        }

        result?;
        Ok(())
    }

    async fn send_msg(&self, payload: Vec<u8>) -> Result<(), Box<dyn StdError>> {
        if self.opts.is_recv {
            return Err(Box::new(Error::QueueIsReceiver));
        }

        let channel = {
            match self.channel.lock().unwrap().as_ref() {
                None => return Err(Box::new(Error::NotConnected)),
                Some(channel) => channel.clone(),
            }
        };

        let mut args = match self.opts.reliable {
            false => BasicPublishArguments::default(),
            true => BasicPublishArguments {
                mandatory: true,
                ..Default::default()
            },
        };
        if self.opts.broadcast {
            args.exchange(self.opts.name.clone());
        } else {
            args.routing_key(self.opts.name.clone());
        }

        channel
            .basic_publish(BasicProperties::default(), payload, args)
            .await?;
        Ok(())
    }
}

impl Default for AmqpQueueOptions {
    fn default() -> Self {
        AmqpQueueOptions {
            name: "".to_string(),
            is_recv: false,
            reliable: false,
            broadcast: false,
            reconnect_millis: DEF_RECONN_TIMEOUT_MS,
            prefetch: 1,
        }
    }
}

#[async_trait]
impl Message for AmqpMessage {
    fn payload(&self) -> &[u8] {
        &self.content
    }

    async fn ack(&self) -> Result<(), Box<dyn StdError>> {
        let args = BasicAckArguments {
            delivery_tag: self.delivery_tag,
            ..Default::default()
        };
        self.channel.basic_ack(args).await?;
        Ok(())
    }

    async fn nack(&self) -> Result<(), Box<dyn StdError>> {
        let args = BasicNackArguments {
            delivery_tag: self.delivery_tag,
            requeue: true,
            ..Default::default()
        };
        self.channel.basic_nack(args).await?;
        Ok(())
    }
}

#[async_trait]
impl AsyncConsumer for Consumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let queue = self.queue.clone();
        let handler = {
            match self.queue.handler().as_ref() {
                None => return (),
                Some(handler) => handler.clone(),
            }
        };
        let message = Box::new(AmqpMessage {
            channel: channel.clone(),
            delivery_tag: deliver.delivery_tag(),
            content,
        });

        task::spawn(async move {
            handler.on_message(queue, message).await;
        });
    }
}

/// To create an event loop runtime task.
fn create_event_loop(queue: &AmqpQueue) -> JoinHandle<()> {
    let this = Arc::new(queue.clone());
    task::spawn(async move {
        loop {
            match this.status() {
                Status::Closing | Status::Closed => break,
                Status::Connecting => {
                    if this.conn_status() != ConnStatus::Connected {
                        time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                        continue;
                    }

                    let raw_conn = { this.conn.lock().unwrap().get_raw_connection() };
                    let channel = if let Some(raw_conn) = raw_conn {
                        match raw_conn.open_channel(None).await {
                            Err(e) => {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }
                            Ok(channel) => channel,
                        }
                    } else {
                        time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                        continue;
                    };
                    if this.opts.reliable {
                        let args = ConfirmSelectArguments::default();
                        if let Err(e) = channel.confirm_select(args).await {
                            this.on_error(Box::new(e));
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }
                    }

                    let name = this.opts.name.as_str();
                    if this.opts.broadcast {
                        let args = ExchangeDeclareArguments::of_type(name, ExchangeType::Fanout);
                        if let Err(e) = channel.exchange_declare(args).await {
                            this.on_error(Box::new(e));
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }

                        if this.opts.is_recv {
                            let mut args = QueueDeclareArguments::default();
                            args.exclusive(true);
                            let queue_name = match channel.queue_declare(args).await {
                                Err(e) => {
                                    this.on_error(Box::new(e));
                                    time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                        .await;
                                    continue;
                                }
                                Ok(Some((queue_name, _, _))) => queue_name,
                                _ => {
                                    this.on_error(Box::new(AmqprsError::ChannelUseError(
                                        "unknown queue_declare error".to_string(),
                                    )));
                                    time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                        .await;
                                    continue;
                                }
                            };

                            let args = QueueBindArguments {
                                queue: queue_name.clone(),
                                exchange: name.to_string(),
                                routing_key: "".to_string(),
                                ..Default::default()
                            };
                            if let Err(e) = channel.queue_bind(args).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }

                            let args = BasicQosArguments {
                                prefetch_count: this.opts.prefetch,
                                ..Default::default()
                            };
                            if let Err(e) = channel.basic_qos(args).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }

                            let args = BasicConsumeArguments::new(&queue_name, "");
                            let consumer = Consumer {
                                queue: this.clone(),
                            };
                            if let Err(e) = channel.basic_consume(consumer, args).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }
                        }
                    } else {
                        let mut args = QueueDeclareArguments::new(name);
                        args.durable(true);
                        if let Err(e) = channel.queue_declare(args).await {
                            this.on_error(Box::new(e));
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }

                        if this.opts.is_recv {
                            let args = BasicQosArguments {
                                prefetch_count: this.opts.prefetch,
                                ..Default::default()
                            };
                            if let Err(e) = channel.basic_qos(args).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }

                            let args = BasicConsumeArguments::new(name, "");
                            let consumer = Consumer {
                                queue: this.clone(),
                            };
                            if let Err(e) = channel.basic_consume(consumer, args).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }
                        }
                    }

                    {
                        *this.channel.lock().unwrap() = Some(channel);
                        *this.status.lock().unwrap() = Status::Connected;
                    }
                    let handler = { (*this.handler.lock().unwrap()).clone() };
                    if let Some(handler) = handler {
                        let queue = this.clone();
                        task::spawn(async move {
                            handler
                                .on_event(queue, Event::Status(Status::Connected))
                                .await;
                        });
                    }
                }
                Status::Connected => {
                    time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                    let mut to_disconnected = true;
                    {
                        if let Some(channel) = (*this.channel.lock().unwrap()).as_ref() {
                            if channel.is_open() {
                                to_disconnected = false;
                            }
                        }
                    }
                    if to_disconnected {
                        to_disconnected_fn(this.clone()).await;
                    }
                }
                Status::Disconnected => {
                    *this.status.lock().unwrap() = Status::Connecting;
                }
            }
        }
    })
}

/// The utilization function for handling disconnected.
async fn to_disconnected_fn(queue: Arc<AmqpQueue>) {
    {
        let mut status_mutex = queue.status.lock().unwrap();
        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
            return;
        }
        queue.channel.lock().unwrap().take();
        *status_mutex = Status::Disconnected;
    }

    let handler = { (*queue.handler.lock().unwrap()).clone() };
    if let Some(handler) = handler {
        let q = queue.clone();
        task::spawn(async move {
            handler
                .on_event(q, Event::Status(Status::Disconnected))
                .await;
        });
    }
    time::sleep(Duration::from_millis(queue.opts.reconnect_millis)).await;
    {
        let mut status_mutex = queue.status.lock().unwrap();
        if *status_mutex == Status::Closing || *status_mutex == Status::Closed {
            return;
        }
        *status_mutex = Status::Connecting;
    }

    let handler = { (*queue.handler.lock().unwrap()).clone() };
    if let Some(handler) = handler {
        let q = queue.clone();
        task::spawn(async move {
            handler.on_event(q, Event::Status(Status::Connecting)).await;
        });
    }
}
