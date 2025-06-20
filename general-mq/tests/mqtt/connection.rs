use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use laboratory::{SpecContext, expect};
use tokio::time;

use general_mq::{
    MqttConnection, MqttConnectionOptions,
    connection::{EventHandler, GmqConnection, Status},
};

use super::{STATE, TestState};

struct TestConnectHandler {
    pub recv_connected: Arc<Mutex<bool>>,
}

struct TestRemoveHandler {
    pub connected_count: Arc<Mutex<usize>>,
}

struct TestCloseHandler {
    pub recv_closed: Arc<Mutex<bool>>,
}

const RETRY_10MS: usize = 100;

#[async_trait]
impl EventHandler for TestConnectHandler {
    async fn on_error(
        &self,
        _handler_id: String,
        _conn: Arc<dyn GmqConnection>,
        _err: Box<dyn StdError + Send + Sync>,
    ) {
    }

    async fn on_status(&self, _handler_id: String, _conn: Arc<dyn GmqConnection>, status: Status) {
        if status == Status::Connected {
            *self.recv_connected.lock().unwrap() = true;
        }
    }
}

#[async_trait]
impl EventHandler for TestRemoveHandler {
    async fn on_error(
        &self,
        _handler_id: String,
        _conn: Arc<dyn GmqConnection>,
        _err: Box<dyn StdError + Send + Sync>,
    ) {
    }

    async fn on_status(&self, _handler_id: String, _conn: Arc<dyn GmqConnection>, status: Status) {
        if status == Status::Connected {
            let mut mutex = self.connected_count.lock().unwrap();
            *mutex += 1;
        }
    }
}

#[async_trait]
impl EventHandler for TestCloseHandler {
    async fn on_error(
        &self,
        _handler_id: String,
        _conn: Arc<dyn GmqConnection>,
        _err: Box<dyn StdError + Send + Sync>,
    ) {
    }

    async fn on_status(&self, _handler_id: String, _conn: Arc<dyn GmqConnection>, status: Status) {
        if status == Status::Closed {
            *self.recv_closed.lock().unwrap() = true;
        }
    }
}

/// Test default options.
pub fn new_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = MqttConnection::new(MqttConnectionOptions::default());
    expect(conn.is_ok()).to_equal(true)
}

/// Test zero value options.
pub fn new_zero(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let opts = MqttConnectionOptions {
        uri: "mqtts://user:@localhost".to_string(),
        connect_timeout_millis: 0,
        reconnect_millis: 0,
        client_id: Some("test".to_string()),
        ..Default::default()
    };
    let conn = MqttConnection::new(opts);
    expect(conn.is_ok()).to_equal(true)
}

/// Test options with wrong values.
pub fn new_wrong_opts(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let opts = MqttConnectionOptions {
        uri: "mqt://localhost".to_string(),
        ..Default::default()
    };
    let conn = MqttConnection::new(opts);
    expect(conn.is_err()).to_equal(true)?;

    let opts = MqttConnectionOptions {
        uri: "mqtt:localhost".to_string(),
        ..Default::default()
    };
    let conn = MqttConnection::new(opts);
    expect(conn.is_err()).to_equal(true)?;

    let opts = MqttConnectionOptions {
        client_id: Some("A@".to_string()),
        ..Default::default()
    };
    let conn = MqttConnection::new(opts);
    expect(conn.is_err()).to_equal(true)
}

/// Test connection properties after `new()`.
pub fn properties(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    if conn.status() != Status::Closed {
        return Err("connection status not Closed".to_string());
    }

    Ok(())
}

/// Test `connect()` without handlers.
pub fn connect_no_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }
    state.runtime.block_on(wait_connected(conn))
}

/// Test `connect()` with a handler.
pub fn connect_with_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    let handler = Arc::new(TestConnectHandler {
        recv_connected: Arc::new(Mutex::new(false)),
    });
    let _ = conn.add_handler(handler.clone());

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    state.runtime.block_on(async move {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                if *handler.recv_connected.lock().unwrap() {
                    return Ok(());
                }
            }
            retry = retry - 1;
        }
        Err("not connected".to_string())
    })
}

/// Test `connect()` for a conneted connection.
pub fn connect_after_connect(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }
    expect(conn.connect().is_ok()).to_equal(true)
}

/// Test remove handlers.
pub fn remove_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    let handler = Arc::new(TestRemoveHandler {
        connected_count: Arc::new(Mutex::new(0)),
    });
    let _ = conn.add_handler(handler.clone());
    let id = conn.add_handler(handler.clone());
    conn.remove_handler(id.as_str());

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    let result = state.runtime.block_on(async move {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                let count = *handler.connected_count.lock().unwrap();
                if count > 0 {
                    return Ok(count);
                }
            }
            retry = retry - 1;
        }
        Err("not connected".to_string())
    });
    expect(result).to_equal(Ok(1 as usize))
}

/// Test `close()`.
pub fn close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    let closed_handler = Arc::new(TestCloseHandler {
        recv_closed: Arc::new(Mutex::new(false)),
    });
    let _ = conn.add_handler(closed_handler.clone());

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    if let Err(e) = state.runtime.block_on(wait_connected(conn)) {
        return Err(e.to_string());
    }

    state.runtime.block_on(async move {
        if let Err(e) = conn.close().await {
            return Err(format!("close() error: {}", e));
        }
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                if *closed_handler.recv_closed.lock().unwrap() {
                    return Ok(());
                }
            }
            retry = retry - 1;
        }
        Err("not closed".to_string())
    })
}

/// Test `close()` for a closed connection.
pub fn close_after_close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    if let Err(e) = state.runtime.block_on(wait_connected(conn)) {
        return Err(e.to_string());
    }

    state.runtime.block_on(async move {
        if let Err(e) = conn.close().await {
            return Err(format!("close error: {}", e));
        }
        if conn.status() != Status::Closed {
            return Err("status is not Closed".to_string());
        }
        if let Err(e) = conn.close().await {
            return Err(format!("close again error: {}", e));
        }
        if conn.status() != Status::Closed {
            return Err("status is not Closed".to_string());
        }
        Ok(())
    })
}

async fn wait_connected(conn: &dyn GmqConnection) -> Result<(), String> {
    let mut retry = RETRY_10MS;
    while retry > 0 {
        time::sleep(Duration::from_millis(10)).await;
        if conn.status() == Status::Connected {
            return Ok(());
        }
        retry = retry - 1;
    }
    Err("not connected".to_string())
}
