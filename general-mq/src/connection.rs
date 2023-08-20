//! Traits and enumerations for connections.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;

/// Connection status.
#[derive(Debug, PartialEq)]
pub enum Status {
    /// The connection is closing.
    Closing,
    /// The connection is closed by the program.
    Closed,
    /// Connecting to the message broker.
    Connecting,
    /// Connected to the message broker.
    Connected,
    /// The connection is not connected. It will retry connecting to the broker automatically.
    Disconnected,
}

/// The operations for connections.
#[async_trait]
pub trait GmqConnection: Send + Sync {
    /// To get the connection status.
    fn status(&self) -> Status;

    /// To add a connection event handler. This will return an identifier for applications to manage
    /// handlers.
    fn add_handler(&mut self, handler: Arc<dyn EventHandler>) -> String;

    /// To remove a handler with an idenfier from [`GmqConnection::add_handler`].
    fn remove_handler(&mut self, id: &str);

    /// To connect to the message broker. The [`GmqConnection`] will connect to the broker using
    /// another runtime task and report status with [`Status`]s.
    fn connect(&mut self) -> Result<(), Box<dyn StdError>>;

    /// To close the connection.
    async fn close(&mut self) -> Result<(), Box<dyn StdError>>;
}

/// The event handler for connections.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Triggered when there are errors.
    async fn on_error(
        &self,
        handler_id: String,
        conn: Arc<dyn GmqConnection>,
        err: Box<dyn StdError + Send + Sync>,
    );

    /// Triggered by [`Status`].
    async fn on_status(&self, handler_id: String, conn: Arc<dyn GmqConnection>, status: Status);
}

impl Copy for Status {}

impl Clone for Status {
    fn clone(&self) -> Status {
        *self
    }
}
