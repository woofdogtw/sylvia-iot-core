use std::{
    error::Error as StdError,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use amq_protocol_types::FieldTable;
use async_trait::async_trait;
use lapin::{
    message::{Delivery, DeliveryResult},
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions,
        BasicQosOptions, ConfirmSelectOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    protocol::basic::AMQPProperties,
    Channel, ConsumerDelegate, Error as LapinError, ExchangeKind,
};
use tokio::{
    task::{self, JoinHandle},
    time,
};

use super::connection::AmqpConnection;
use crate::{
    connection::{Connection, Status as ConnStatus},
    queue::{name_validate, Event, EventHandler, Message, Queue, Status, QUEUE_NAME_PATTERN},
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
    /// Hold the [`lapin::message::Delivery`] instance.
    delivery: Delivery,
}

/// The [`lapin::ConsumerDelegate`] implementation.
struct Delegate {
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
impl Queue for AmqpQueue {
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

        let mut result: Result<(), LapinError> = Ok(());
        if let Some(channel) = channel {
            result = channel.close(0, "").await;
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

        let opts = match self.opts.reliable {
            false => BasicPublishOptions::default(),
            true => BasicPublishOptions {
                mandatory: true,
                ..Default::default()
            },
        };
        let prop = AMQPProperties::default();
        let ex;
        let rk;
        if self.opts.broadcast {
            ex = self.opts.name.as_str();
            rk = "";
        } else {
            ex = "";
            rk = self.opts.name.as_str();
        }

        channel
            .basic_publish(ex, rk, opts, payload.as_slice(), prop)
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
        self.delivery.data.as_slice()
    }

    async fn ack(&self) -> Result<(), Box<dyn StdError>> {
        self.delivery.acker.ack(BasicAckOptions::default()).await?;
        Ok(())
    }

    async fn nack(&self) -> Result<(), Box<dyn StdError>> {
        let opts = BasicNackOptions {
            requeue: true,
            ..Default::default()
        };
        self.delivery.acker.nack(opts).await?;
        Ok(())
    }
}

impl ConsumerDelegate for Delegate {
    fn on_new_delivery(
        &self,
        delivery: DeliveryResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let queue = self.queue.clone();
        Box::pin(async move {
            if let Ok(Some(delivery)) = delivery {
                let handler = {
                    match queue.handler().as_ref() {
                        None => return (),
                        Some(handler) => handler.clone(),
                    }
                };
                handler
                    .on_message(queue, Box::new(AmqpMessage { delivery }))
                    .await;
            }
        })
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

                    let conn = { this.conn.lock().unwrap().clone() };
                    let channel = match conn.create_channel().await {
                        Err(e) => {
                            this.on_error(e);
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }
                        Ok(channel) => channel,
                    };
                    if this.opts.reliable {
                        let opts = ConfirmSelectOptions::default();
                        if let Err(e) = channel.confirm_select(opts).await {
                            this.on_error(Box::new(e));
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }
                    }

                    let name = this.opts.name.as_str();
                    let args = FieldTable::default();
                    if this.opts.broadcast {
                        let opts = ExchangeDeclareOptions::default();
                        if let Err(e) = channel
                            .exchange_declare(name, ExchangeKind::Fanout, opts, args.clone())
                            .await
                        {
                            this.on_error(Box::new(e));
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }

                        if this.opts.is_recv {
                            let opts = QueueDeclareOptions {
                                exclusive: true,
                                ..Default::default()
                            };
                            let queue = match channel.queue_declare("", opts, args.clone()).await {
                                Err(e) => {
                                    this.on_error(Box::new(e));
                                    time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                        .await;
                                    continue;
                                }
                                Ok(queue) => queue,
                            };
                            let queue_name = queue.name().as_str();

                            let opts = QueueBindOptions::default();
                            if let Err(e) = channel
                                .queue_bind(queue_name, name, "", opts, args.clone())
                                .await
                            {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }

                            let opts = BasicQosOptions::default();
                            if let Err(e) = channel.basic_qos(this.opts.prefetch, opts).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }

                            let opts = BasicConsumeOptions::default();
                            let consumer = match channel
                                .basic_consume(queue_name, "", opts, args)
                                .await
                            {
                                Err(e) => {
                                    this.on_error(Box::new(e));
                                    time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                        .await;
                                    continue;
                                }
                                Ok(consumer) => consumer,
                            };
                            consumer.set_delegate(Delegate {
                                queue: this.clone(),
                            });
                        }
                    } else {
                        let args = FieldTable::default();
                        let opts = QueueDeclareOptions {
                            durable: true,
                            ..Default::default()
                        };
                        if let Err(e) = channel.queue_declare(name, opts, args.clone()).await {
                            this.on_error(Box::new(e));
                            time::sleep(Duration::from_millis(this.opts.reconnect_millis)).await;
                            continue;
                        }

                        if this.opts.is_recv {
                            let opts = BasicQosOptions::default();
                            if let Err(e) = channel.basic_qos(this.opts.prefetch, opts).await {
                                this.on_error(Box::new(e));
                                time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                    .await;
                                continue;
                            }

                            let opts = BasicConsumeOptions::default();
                            let consumer = match channel.basic_consume(name, "", opts, args).await {
                                Err(e) => {
                                    this.on_error(Box::new(e));
                                    time::sleep(Duration::from_millis(this.opts.reconnect_millis))
                                        .await;
                                    continue;
                                }
                                Ok(consumer) => consumer,
                            };
                            consumer.set_delegate(Delegate {
                                queue: this.clone(),
                            });
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
                            if channel.status().connected() {
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
