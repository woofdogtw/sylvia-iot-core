//! General purposed interfaces for message queues. Now we provide the following implementations:
//!
//! - AMQP 0-9-1
//! - MQTT
//!
//! By using these classes, you can configure queues with the following properties:
//!
//! - Unicast or broadcast.
//! - Reliable or best-effort.
//!
//! **Notes**
//!
//! - MQTT uses **shared queues** to implement unicast.
//! - AMQP uses **confirm channels** to implement reliable publish, and MQTT uses **QoS 1** to
//!   implement reliable publish/subscribe.
//!
//! # Relationships of Connections and Queues
//!
//! The term **connection** describes a TCP/TLS connection to the message broker.
//! The term **queue** describes a message queue or a topic within a connection.
//! You can use one connection to manage multiple queues, or one connection to manage one queue.
//!
//! A queue can only be a receiver or a sender at a time.
//!
//! ### Connections for sender/receiver queues with the same name
//!
//! The sender and the receiver are usually different programs, there are two connections to hold
//! two queues.
//!
//! For the special case that a program acts both the sender and the receiver using the same queue:
//!
//! - The AMQP implementation uses one **Channel** for one queue, so the program can manages all
//!   queues with one connection.
//! - The MQTT implementation **MUST** uses one connection for one queue, or both sender and
//!   receiver will receive packets.
//!
//! # Test
//!
//! Please prepare a [RabbitMQ](https://www.rabbitmq.com/) broker and a [EMQX](https://emqx.io/)
//! broker at **localhost** for testing.
//!
//! - To install using Docker:
//!
//!       $ docker run --rm --name rabbitmq -d -p 5672:5672 rabbitmq:management-alpine
//!       $ docker run --rm --name emqx -d -p 1883:1883 emqx/emqx
//!
//! Then run the test:
//!
//!     $ cargo test --test integration_test -- --nocapture
//!
//! # Example
//!
//! Run RabbitMQ and then run AMQP example:
//!
//!     $ cargo run --example simple
//!
//! Run EMQX and then run MQTT example:
//!
//!     $ RUN_MQTT= cargo run --example simple

use std::{error::Error as StdError, fmt, sync::Arc};

use async_trait::async_trait;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub mod connection;
pub mod queue;

mod amqp;
mod mqtt;

pub use amqp::{AmqpConnection, AmqpConnectionOptions, AmqpQueue, AmqpQueueOptions};
pub use mqtt::{MqttConnection, MqttConnectionOptions, MqttQueue, MqttQueueOptions};
use queue::{EventHandler, GmqQueue, MessageHandler, Status};

/// general-mq error.
#[derive(Clone, Debug)]
pub enum Error {
    /// The queue does not have [`MessageHandler`].
    NoMsgHandler,
    /// The connection is not connected or the queue (topic) is not
    /// connected (declared/subscribed).
    NotConnected,
    /// The queue is a receiver that cannot send messages.
    QueueIsReceiver,
}

#[derive(Clone)]
pub enum Queue {
    Amqp(AmqpQueue),
    Mqtt(MqttQueue),
}

#[derive(Clone)]
pub enum QueueOptions<'a> {
    Amqp(AmqpQueueOptions, &'a AmqpConnection),
    Mqtt(MqttQueueOptions, &'a MqttConnection),
}

/// Identifier length of inner handlers.
pub(crate) const ID_SIZE: usize = 24;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NoMsgHandler => write!(f, "no message handler"),
            Error::NotConnected => write!(f, "not connected"),
            Error::QueueIsReceiver => write!(f, "this queue is a receiver"),
        }
    }
}

impl StdError for Error {}

impl Queue {
    pub fn new(opts: QueueOptions) -> Result<Self, String> {
        match opts {
            QueueOptions::Amqp(opts, conn) => Ok(Queue::Amqp(AmqpQueue::new(opts, conn)?)),
            QueueOptions::Mqtt(opts, conn) => Ok(Queue::Mqtt(MqttQueue::new(opts, conn)?)),
        }
    }
}

#[async_trait]
impl GmqQueue for Queue {
    fn name(&self) -> &str {
        match self {
            Queue::Amqp(q) => q.name(),
            Queue::Mqtt(q) => q.name(),
        }
    }

    fn is_recv(&self) -> bool {
        match self {
            Queue::Amqp(q) => q.is_recv(),
            Queue::Mqtt(q) => q.is_recv(),
        }
    }

    fn status(&self) -> Status {
        match self {
            Queue::Amqp(q) => q.status(),
            Queue::Mqtt(q) => q.status(),
        }
    }

    fn set_handler(&mut self, handler: Arc<dyn EventHandler>) {
        match self {
            Queue::Amqp(q) => q.set_handler(handler),
            Queue::Mqtt(q) => q.set_handler(handler),
        }
    }

    fn clear_handler(&mut self) {
        match self {
            Queue::Amqp(q) => q.clear_handler(),
            Queue::Mqtt(q) => q.clear_handler(),
        }
    }

    fn set_msg_handler(&mut self, handler: Arc<dyn MessageHandler>) {
        match self {
            Queue::Amqp(q) => q.set_msg_handler(handler),
            Queue::Mqtt(q) => q.set_msg_handler(handler),
        }
    }

    fn connect(&mut self) -> Result<(), Box<dyn StdError>> {
        match self {
            Queue::Amqp(q) => q.connect(),
            Queue::Mqtt(q) => q.connect(),
        }
    }

    async fn close(&mut self) -> Result<(), Box<dyn StdError>> {
        match self {
            Queue::Amqp(q) => q.close().await,
            Queue::Mqtt(q) => q.close().await,
        }
    }

    async fn send_msg(&self, payload: Vec<u8>) -> Result<(), Box<dyn StdError>> {
        match self {
            Queue::Amqp(q) => q.send_msg(payload).await,
            Queue::Mqtt(q) => q.send_msg(payload).await,
        }
    }
}

/// Generate random alphanumeric with the specified length.
pub fn randomstring(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}
