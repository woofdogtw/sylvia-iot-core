use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use laboratory::{SpecContext, expect};
use tokio::{net::TcpListener, time};

use general_mq::{
    AmqpConnection, AmqpConnectionOptions,
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

/// If the first connection attempt fails, the next attempt starts after `reconnect_millis` (default 1
/// second), so wait longer than that.
const RETRY_10MS: usize = 300;

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
    let conn = AmqpConnection::new(AmqpConnectionOptions::default());
    expect(conn.is_ok()).to_equal(true)
}

/// Test zero value options.
pub fn new_zero(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let opts = AmqpConnectionOptions {
        uri: "amqp://localhost/".to_string(),
        connect_timeout_millis: 0,
        reconnect_millis: 0,
    };
    let conn = AmqpConnection::new(opts);
    expect(conn.is_ok()).to_equal(true)
}

/// Test options with wrong values.
pub fn new_wrong_opts(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let opts = AmqpConnectionOptions {
        uri: "amq://localhost".to_string(),
        ..Default::default()
    };
    let conn = AmqpConnection::new(opts);
    expect(conn.is_err()).to_equal(true)?;

    // The error message must not contain the password.
    let opts = AmqpConnectionOptions {
        uri: "amqp:user:secret@localhost".to_string(),
        ..Default::default()
    };
    match AmqpConnection::new(opts) {
        Err(e) => {
            if e.contains("secret") {
                return Err(format!("error message contains the password: {}", e));
            }
        }
        Ok(_) => return Err("URI without // should error".to_string()),
    }

    Ok(())
}

/// Test connection properties after `new()`.
pub fn properties(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
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

    let mut conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    state.runtime.block_on(wait_connected(conn))
}

/// Test `connect()` with a handler.
pub fn connect_with_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
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

    let mut conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }
    expect(conn.connect().is_ok()).to_equal(true)
}

/// Test `connect()` with a server that accepts TCP connections but never replies, to check that
/// `connect_timeout_millis` takes effect.
pub fn connect_timeout(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let listener = match state.runtime.block_on(TcpListener::bind("127.0.0.1:0")) {
        Err(e) => return Err(format!("bind listener error: {}", e)),
        Ok(listener) => listener,
    };
    let port = match listener.local_addr() {
        Err(e) => return Err(format!("get listener address error: {}", e)),
        Ok(addr) => addr.port(),
    };
    let accept_count = Arc::new(Mutex::new(0));
    let count = accept_count.clone();
    let server = state.runtime.spawn(async move {
        let mut sockets = vec![];
        while let Ok((socket, _)) = listener.accept().await {
            *count.lock().unwrap() += 1;
            sockets.push(socket);
        }
    });

    let opts = AmqpConnectionOptions {
        uri: format!("amqp://127.0.0.1:{}", port),
        connect_timeout_millis: 1000,
        reconnect_millis: 1000,
        ..Default::default()
    };
    let mut conn = match AmqpConnection::new(opts) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    // Connections should be accepted at about 0s and 2s (1s timeout + 1s reconnect).
    state
        .runtime
        .block_on(time::sleep(Duration::from_millis(3500)));
    server.abort();
    let count = *accept_count.lock().unwrap();
    if count < 2 {
        return Err(format!(
            "accepted {} connection(s), expect at least 2",
            count
        ));
    }
    Ok(())
}

/// Test reconnection with a server that closes TCP connections immediately, to check that
/// `reconnect_millis` takes effect.
pub fn connect_reconnect(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let listener = match state.runtime.block_on(TcpListener::bind("127.0.0.1:0")) {
        Err(e) => return Err(format!("bind listener error: {}", e)),
        Ok(listener) => listener,
    };
    let port = match listener.local_addr() {
        Err(e) => return Err(format!("get listener address error: {}", e)),
        Ok(addr) => addr.port(),
    };
    let accept_times = Arc::new(Mutex::new(vec![]));
    let times = accept_times.clone();
    let server = state.runtime.spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            times.lock().unwrap().push(Instant::now());
            drop(socket);
        }
    });

    let opts = AmqpConnectionOptions {
        uri: format!("amqp://127.0.0.1:{}", port),
        connect_timeout_millis: 1000,
        reconnect_millis: 2000,
        ..Default::default()
    };
    let mut conn = match AmqpConnection::new(opts) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    // Connections should be accepted at about 0s, 2s and 4s.
    state
        .runtime
        .block_on(time::sleep(Duration::from_millis(4500)));
    server.abort();
    let times = accept_times.lock().unwrap().clone();
    if times.len() < 2 {
        return Err(format!(
            "accepted {} connection(s), expect at least 2",
            times.len()
        ));
    }
    for pair in times.windows(2) {
        let interval = pair[1].duration_since(pair[0]).as_millis();
        if interval < 1950 || interval >= 3000 {
            return Err(format!(
                "reconnect interval {} ms, expect about 2000 ms",
                interval
            ));
        }
    }
    Ok(())
}

/// Test `connect()` to an IPv6 address.
pub fn connect_ipv6(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let listener = match state.runtime.block_on(TcpListener::bind("[::1]:0")) {
        Err(e) => return Err(format!("bind listener error: {}", e)),
        Ok(listener) => listener,
    };
    let port = match listener.local_addr() {
        Err(e) => return Err(format!("get listener address error: {}", e)),
        Ok(addr) => addr.port(),
    };
    let accept_count = Arc::new(Mutex::new(0));
    let count = accept_count.clone();
    let server = state.runtime.spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            *count.lock().unwrap() += 1;
            drop(socket);
        }
    });

    let opts = AmqpConnectionOptions {
        uri: format!("amqp://[::1]:{}", port),
        ..Default::default()
    };
    let mut conn = match AmqpConnection::new(opts) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let conn: &mut dyn GmqConnection = &mut conn;

    if let Err(e) = conn.connect() {
        return Err(format!("GmqConnection::connect() error: {}", e));
    }

    let result = state.runtime.block_on(async {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            if *accept_count.lock().unwrap() > 0 {
                return Ok(());
            }
            retry = retry - 1;
        }
        Err("not connected to the IPv6 address".to_string())
    });
    server.abort();
    result
}

/// Test remove handlers.
pub fn remove_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
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

    let mut conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
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

    let mut conn = match AmqpConnection::new(AmqpConnectionOptions::default()) {
        Err(e) => return Err(format!("AmqpConnection::new() error: {}", e)),
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
