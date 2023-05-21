//! Traits and enumerations for queues.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use regex::Regex;

/// Events of queues.
pub enum Event {
    /// Queue status changed.
    Status(Status),
    /// Queue error.
    Error(Box<dyn StdError + Send + Sync>),
}

/// Queue status.
#[derive(Debug, PartialEq)]
pub enum Status {
    /// The queue is closing.
    Closing,
    /// The queue is closed by the program.
    Closed,
    /// Connecting to the message queue.
    Connecting,
    /// Connected to the message queue.
    Connected,
    /// The queue is not connected. It will retry connecting to the queue automatically.
    Disconnected,
}

/// The accepted pattern of the queue name.
pub(crate) const QUEUE_NAME_PATTERN: &'static str = r"^[a-z0-9_-]+([\.]{1}[a-z0-9_-]+)*$";

/// The operations for queues.
#[async_trait]
pub trait GmqQueue: Send + Sync {
    /// To get the queue name.
    fn name(&self) -> &str;

    /// Is the queue a receiver.
    fn is_recv(&self) -> bool;

    /// To get the connection status.
    fn status(&self) -> Status;

    /// To set the queue event handler.
    fn set_handler(&mut self, handler: Arc<dyn EventHandler>);

    /// To remove the queue event handler.
    fn clear_handler(&mut self);

    /// To connect to the message queue. The [`GmqQueue`] will connect to the queue using another
    /// runtime task and report status with [`Event`]s.
    fn connect(&mut self) -> Result<(), Box<dyn StdError>>;

    /// To close the queue.
    async fn close(&mut self) -> Result<(), Box<dyn StdError>>;

    /// To send a message (for **senders** only).
    async fn send_msg(&self, payload: Vec<u8>) -> Result<(), Box<dyn StdError>>;
}

/// The operations for incoming messages.
#[async_trait]
pub trait Message: Send + Sync {
    /// To get the payload.
    fn payload(&self) -> &[u8];

    /// Use this if the message is processed successfully.
    async fn ack(&self) -> Result<(), Box<dyn StdError>>;

    /// To requeue the message and the broker will send the message in the future.
    ///
    /// **Note**: only AMQP or protocols that support requeuing are effective.
    async fn nack(&self) -> Result<(), Box<dyn StdError>>;
}

/// The event handler for connections.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Triggered by [`Event`]s.
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: Event);

    /// Triggered for new incoming [`Message`]s.
    async fn on_message(&self, queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>);
}

impl Copy for Status {}

impl Clone for Status {
    fn clone(&self) -> Status {
        *self
    }
}

/// To validate the queue name.
pub(crate) fn name_validate(name: &str) -> bool {
    let re = Regex::new(QUEUE_NAME_PATTERN).unwrap();
    re.is_match(name)
}
