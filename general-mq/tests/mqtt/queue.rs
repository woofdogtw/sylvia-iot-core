use std::{
    str,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use laboratory::{expect, SpecContext};
use tokio::{task, time};

use general_mq::{
    connection::Connection,
    queue::{Event, EventHandler, Message, Queue, Status},
    randomstring, MqttConnection, MqttConnectionOptions, MqttQueue, MqttQueueOptions,
    Queue as MqQueue, QueueOptions as MqQueueOptions,
};

use super::{TestState, STATE};

#[derive(Default)]
struct Resources {
    pub conn: Vec<Box<MqttConnection>>,
    pub queues: Vec<Box<MqttQueue>>,
    pub mq_queues: Vec<Box<MqQueue>>,
}

struct TestConnectHandler {
    pub recv_connected: Arc<Mutex<bool>>,
    pub recv_queue_name: Arc<Mutex<String>>,
}

struct TestRemoveHandler {
    pub connected_count: Arc<Mutex<usize>>,
}

struct TestCloseHandler {
    pub recv_closed: Arc<Mutex<bool>>,
    pub recv_queue_name: Arc<Mutex<String>>,
}

struct TestReconnectHandler {
    pub connected_count: Arc<Mutex<usize>>,
    pub recv_connecting: Arc<Mutex<bool>>,
}

#[derive(Clone)]
struct TestRecvMsgHandler {
    pub recv_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    pub ack_errors: Arc<Mutex<Vec<String>>>,
    pub use_nack: Arc<Mutex<bool>>,
    pub nack_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    pub nack_errors: Arc<Mutex<Vec<String>>>,
}

const RETRY_10MS: usize = 100;

#[async_trait]
impl EventHandler for TestConnectHandler {
    async fn on_event(&self, queue: Arc<dyn Queue>, ev: Event) {
        if let Event::Status(status) = ev {
            if status == Status::Connected {
                *self.recv_connected.lock().unwrap() = true;
                *self.recv_queue_name.lock().unwrap() = queue.name().to_string();
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, _msg: Box<dyn Message>) {}
}

#[async_trait]
impl EventHandler for TestRemoveHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, ev: Event) {
        if let Event::Status(status) = ev {
            if status == Status::Connected {
                let mut mutex = self.connected_count.lock().unwrap();
                *mutex += 1;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, _msg: Box<dyn Message>) {}
}

#[async_trait]
impl EventHandler for TestCloseHandler {
    async fn on_event(&self, queue: Arc<dyn Queue>, ev: Event) {
        if let Event::Status(status) = ev {
            if status == Status::Closed {
                *self.recv_closed.lock().unwrap() = true;
                *self.recv_queue_name.lock().unwrap() = queue.name().to_string();
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, _msg: Box<dyn Message>) {}
}

#[async_trait]
impl EventHandler for TestReconnectHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, ev: Event) {
        if let Event::Status(status) = ev {
            if status == Status::Connected {
                let mut mutex = self.connected_count.lock().unwrap();
                *mutex += 1;
            } else if status == Status::Connecting {
                *self.recv_connecting.lock().unwrap() = true;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, _msg: Box<dyn Message>) {}
}

#[async_trait]
impl EventHandler for TestRecvMsgHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, _ev: Event) {}

    async fn on_message(&self, _queue: Arc<dyn Queue>, msg: Box<dyn Message>) {
        let use_nack;
        {
            use_nack = *self.use_nack.lock().unwrap();
        }
        if use_nack {
            if let Err(e) = msg.nack().await {
                self.nack_errors.lock().unwrap().push(e.to_string());
            } else {
                let data = msg.payload().to_vec();
                self.nack_messages.lock().unwrap().push(data);
            }
        } else {
            if let Err(e) = msg.ack().await {
                self.ack_errors.lock().unwrap().push(e.to_string());
            } else {
                let data = msg.payload().to_vec();
                self.recv_messages.lock().unwrap().push(data);
            }
        }
    }
}

/// Test default options.
pub fn new_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        reconnect_millis: 0,
        ..Default::default()
    };
    let queue = MqttQueue::new(opts, &conn);
    expect(queue.is_ok()).to_equal(true)
}

/// Test options with wrong values.
pub fn new_wrong_opts(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqttQueueOptions {
        name: "".to_string(),
        ..Default::default()
    };
    let queue = MqttQueue::new(opts, &conn);
    expect(queue.is_err()).to_equal(true)?;

    let opts = MqttQueueOptions {
        name: "A@".to_string(),
        ..Default::default()
    };
    let queue = MqttQueue::new(opts, &conn);
    expect(queue.is_err()).to_equal(true)
}

/// Test queue properties after `new()`.
pub fn properties(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqttQueueOptions {
        name: "name-send".to_string(),
        ..Default::default()
    };
    let queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() error: {}", e)),
        Ok(q) => q,
    };
    if queue.name() != "name-send" {
        return Err("send name error".to_string());
    } else if queue.is_recv() {
        return Err("send queue not send".to_string());
    } else if queue.status() != Status::Closed {
        return Err("send queue status not Closed".to_string());
    }

    let opts = MqttQueueOptions {
        name: "name-recv".to_string(),
        is_recv: true,
        ..Default::default()
    };
    let queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() error: {}", e)),
        Ok(q) => q,
    };
    if queue.name() != "name-recv" {
        return Err("recv name error".to_string());
    } else if !queue.is_recv() {
        return Err("recv queue not recv".to_string());
    } else if queue.status() != Status::Closed {
        return Err("recv queue status not Closed".to_string());
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
    let opts = MqttQueueOptions {
        name: "name".to_string(),
        is_recv: true,
        ..Default::default()
    };
    let mut queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() error: {}", e)),
        Ok(q) => q,
    };
    state.queues.push(Box::new(queue.clone()));
    let conn: &mut dyn Connection = &mut conn;
    let queue: &mut dyn Queue = &mut queue;

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }
    state.runtime.block_on(wait_connected(queue, RETRY_10MS))
}

/// Test `connect()` with a handler.
pub fn connect_with_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestConnectHandler {
        recv_connected: Arc::new(Mutex::new(false)),
        recv_queue_name: Arc::new(Mutex::new("".to_string())),
    });
    create_conn_rsc(state, &mut resources, Some(handler.clone()), true)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    state.runtime.block_on(async move {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                if *handler.recv_connected.lock().unwrap()
                    && handler.recv_queue_name.lock().unwrap().as_str() == "name"
                {
                    return Ok(());
                }
            }
            retry = retry - 1;
        }
        Err("not connected".to_string())
    })
}

/// Test `connect()` for a conneted queue.
pub fn connect_after_connect(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let opts = MqttQueueOptions {
        name: "name".to_string(),
        is_recv: true,
        ..Default::default()
    };
    let mut queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() error: {}", e)),
        Ok(q) => q,
    };
    state.queues = vec![Box::new(queue.clone())];
    let queue: &mut dyn Queue = &mut queue;

    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }
    expect(queue.connect().is_ok()).to_equal(true)
}

/// Test remove the handler.
pub fn clear_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestRemoveHandler {
        connected_count: Arc::new(Mutex::new(0)),
    });
    create_conn_rsc(state, &mut resources, Some(handler.clone()), false)?;

    let conn = match resources.conn.get_mut(0) {
        None => return Err(format!("should have a connection")),
        Some(conn) => conn,
    };
    let queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };
    queue.clear_handler();

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }

    let count = state.runtime.block_on(async move {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            if queue.status() == Status::Connected {
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err("not connected".to_string());
        }
        time::sleep(Duration::from_millis(10)).await;
        Ok(*handler.connected_count.lock().unwrap())
    })?;
    expect(count).to_equal(0 as usize)
}

/// Test `close()`.
pub fn close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestCloseHandler {
        recv_closed: Arc::new(Mutex::new(false)),
        recv_queue_name: Arc::new(Mutex::new("".to_string())),
    });
    create_conn_rsc(state, &mut resources, Some(handler.clone()), true)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };

    state.runtime.block_on(async move {
        if let Err(e) = queue.close().await {
            return Err(format!("close() error: {}", e));
        }
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                if *handler.recv_closed.lock().unwrap()
                    && handler.recv_queue_name.lock().unwrap().as_str() == "name"
                {
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
    let mut resources = Resources::default();

    create_conn_rsc(state, &mut resources, None, true)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };

    state.runtime.block_on(async move {
        if let Err(e) = queue.close().await {
            return Err(format!("close error: {}", e));
        }
        if queue.status() != Status::Closed {
            return Err("status is not Closed".to_string());
        }
        if let Err(e) = queue.close().await {
            return Err(format!("close again error: {}", e));
        }
        if queue.status() != Status::Closed {
            return Err("status is not Closed".to_string());
        }
        Ok(())
    })
}

/// Test send with an invalid queue.
pub fn send_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();

    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        is_recv: true,
        ..Default::default()
    };
    let queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() recv error: {}", e)),
        Ok(q) => q,
    };

    match state.runtime.block_on(queue.send_msg(vec![])) {
        Err(_) => (),
        Ok(()) => return Err("send to recv queue should error".to_string()),
    }

    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        is_recv: false,
        ..Default::default()
    };
    let queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() send error: {}", e)),
        Ok(conn) => conn,
    };
    match state.runtime.block_on(queue.send_msg(vec![])) {
        Err(_) => (),
        Ok(()) => return Err("send to not-connected queue should error".to_string()),
    }

    Ok(())
}

/// Test default options.
pub fn mq_new_default(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name".to_string(),
            reconnect_millis: 0,
            ..Default::default()
        },
        &conn,
    );
    let queue = MqQueue::new(opts);
    expect(queue.is_ok()).to_equal(true)
}

/// Test options with wrong values.
pub fn mq_new_wrong_opts(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "".to_string(),
            ..Default::default()
        },
        &conn,
    );
    let queue = MqQueue::new(opts);
    expect(queue.is_err()).to_equal(true)?;

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "A@".to_string(),
            ..Default::default()
        },
        &conn,
    );
    let queue = MqQueue::new(opts);
    expect(queue.is_err()).to_equal(true)
}

/// Test queue properties after `new()`.
pub fn mq_properties(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name-send".to_string(),
            ..Default::default()
        },
        &conn,
    );
    let queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() error: {}", e)),
        Ok(q) => q,
    };
    if queue.name() != "name-send" {
        return Err("send name error".to_string());
    } else if queue.is_recv() {
        return Err("send queue not send".to_string());
    } else if queue.status() != Status::Closed {
        return Err("send queue status not Closed".to_string());
    }

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name-recv".to_string(),
            is_recv: true,
            ..Default::default()
        },
        &conn,
    );
    let queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() error: {}", e)),
        Ok(q) => q,
    };
    if queue.name() != "name-recv" {
        return Err("recv name error".to_string());
    } else if !queue.is_recv() {
        return Err("recv queue not recv".to_string());
    } else if queue.status() != Status::Closed {
        return Err("recv queue status not Closed".to_string());
    }

    Ok(())
}

/// Test `connect()` without handlers.
pub fn mq_connect_no_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name".to_string(),
            is_recv: true,
            ..Default::default()
        },
        &conn,
    );
    let mut queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() error: {}", e)),
        Ok(q) => q,
    };
    state.queues.push(Box::new(queue.clone()));
    let conn: &mut dyn Connection = &mut conn;
    let queue: &mut dyn Queue = &mut queue;

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }
    state.runtime.block_on(wait_connected(queue, RETRY_10MS))
}

/// Test `connect()` with a handler.
pub fn mq_connect_with_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestConnectHandler {
        recv_connected: Arc::new(Mutex::new(false)),
        recv_queue_name: Arc::new(Mutex::new("".to_string())),
    });
    mq_create_conn_rsc(state, &mut resources, Some(handler.clone()), true)?;

    for queue in resources.mq_queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    state.runtime.block_on(async move {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                if *handler.recv_connected.lock().unwrap()
                    && handler.recv_queue_name.lock().unwrap().as_str() == "name"
                {
                    return Ok(());
                }
            }
            retry = retry - 1;
        }
        Err("not connected".to_string())
    })
}

/// Test `connect()` for a conneted queue.
pub fn mq_connect_after_connect(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();

    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name".to_string(),
            is_recv: true,
            ..Default::default()
        },
        &conn,
    );
    let mut queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() error: {}", e)),
        Ok(q) => q,
    };
    state.queues = vec![Box::new(queue.clone())];
    let queue: &mut dyn Queue = &mut queue;

    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }
    expect(queue.connect().is_ok()).to_equal(true)
}

/// Test remove the handler.
pub fn mq_clear_handler(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestRemoveHandler {
        connected_count: Arc::new(Mutex::new(0)),
    });
    mq_create_conn_rsc(state, &mut resources, Some(handler.clone()), false)?;

    let conn = match resources.conn.get_mut(0) {
        None => return Err(format!("should have a connection")),
        Some(conn) => conn,
    };
    let queue = match resources.mq_queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };
    queue.clear_handler();

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }

    let count = state.runtime.block_on(async move {
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            if queue.status() == Status::Connected {
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err("not connected".to_string());
        }
        time::sleep(Duration::from_millis(10)).await;
        Ok(*handler.connected_count.lock().unwrap())
    })?;
    expect(count).to_equal(0 as usize)
}

/// Test `close()`.
pub fn mq_close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestCloseHandler {
        recv_closed: Arc::new(Mutex::new(false)),
        recv_queue_name: Arc::new(Mutex::new("".to_string())),
    });
    mq_create_conn_rsc(state, &mut resources, Some(handler.clone()), true)?;

    for queue in resources.mq_queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let queue = match resources.mq_queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };

    state.runtime.block_on(async move {
        if let Err(e) = queue.close().await {
            return Err(format!("close() error: {}", e));
        }
        let mut retry = RETRY_10MS;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                if *handler.recv_closed.lock().unwrap()
                    && handler.recv_queue_name.lock().unwrap().as_str() == "name"
                {
                    return Ok(());
                }
            }
            retry = retry - 1;
        }
        Err("not closed".to_string())
    })
}

/// Test `close()` for a closed connection.
pub fn mq_close_after_close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    mq_create_conn_rsc(state, &mut resources, None, true)?;

    for queue in resources.mq_queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let queue = match resources.mq_queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };

    state.runtime.block_on(async move {
        if let Err(e) = queue.close().await {
            return Err(format!("close error: {}", e));
        }
        if queue.status() != Status::Closed {
            return Err("status is not Closed".to_string());
        }
        if let Err(e) = queue.close().await {
            return Err(format!("close again error: {}", e));
        }
        if queue.status() != Status::Closed {
            return Err("status is not Closed".to_string());
        }
        Ok(())
    })
}

/// Test send with an invalid queue.
pub fn mq_send_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();

    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name".to_string(),
            is_recv: true,
            ..Default::default()
        },
        &conn,
    );
    let queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() recv error: {}", e)),
        Ok(q) => q,
    };

    match state.runtime.block_on(queue.send_msg(vec![])) {
        Err(_) => (),
        Ok(()) => return Err("send to recv queue should error".to_string()),
    }

    let conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };

    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name".to_string(),
            is_recv: false,
            ..Default::default()
        },
        &conn,
    );
    let queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() send error: {}", e)),
        Ok(conn) => conn,
    };
    match state.runtime.block_on(queue.send_msg(vec![])) {
        Err(_) => (),
        Ok(()) => return Err("send to not-connected queue should error".to_string()),
    }

    Ok(())
}

/// Test reconnect by closing/connecting the associated connection.
pub fn reconnect(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let handler = Arc::new(TestReconnectHandler {
        connected_count: Arc::new(Mutex::new(0)),
        recv_connecting: Arc::new(Mutex::new(false)),
    });
    create_conn_rsc(state, &mut resources, Some(handler.clone()), true)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let conn = match resources.conn.get_mut(0) {
        None => return Err(format!("should have a connection")),
        Some(conn) => conn,
    };
    let queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have a queue")),
        Some(q) => q,
    };

    state.runtime.block_on(async move {
        if let Err(e) = conn.close().await {
            return Err(format!("close connection error: {}", e));
        }

        let mut retry = 200;
        let mut recv_connecting = false;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            if *handler.recv_connecting.lock().unwrap() {
                recv_connecting = true;
                break;
            }
            retry = retry - 1;
        }
        if !recv_connecting {
            return Err("no connecting event".to_string());
        }

        if let Err(e) = conn.connect() {
            return Err(format!("Connect::connect() again error: {}", e));
        }
        if let Err(e) = wait_connected(queue.as_ref(), 1000).await {
            return Err(format!("wait reconnect connected error: {}", e));
        }
        Ok(())
    })
}

/// Send unitcast data to one receiver.
pub fn data_unicast_1to1(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        shared_prefix: Some("$share/general-mq/".to_string()),
        ..Default::default()
    };
    let handlers = create_msg_rsc(state, &mut resources, &opts, 1)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let send_queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have send queue")),
        Some(q) => q,
    };
    let handler = match handlers.get(0) {
        None => return Err(format!("should have a handler")),
        Some(handler) => handler,
    };

    let dataset = ["1", "2"];
    for data in dataset {
        let queue_clone = send_queue.clone();
        task::spawn(async move {
            let _ = queue_clone.send_msg(data.as_bytes().to_vec()).await;
        });
    }

    state.runtime.block_on(async move {
        let mut len = 0;
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                len = handler.recv_messages.lock().unwrap().len();
            }
            if len == 2 {
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("received {}/2 messages", len));
        }
        let msg1;
        let msg2;
        {
            let messages = handler.recv_messages.lock().unwrap();
            let slice = messages.as_slice();
            msg1 = get_message(slice, 0)?;
            msg2 = get_message(slice, 1)?;
        }
        if msg1 == msg2 {
            return Err("duplicate message".to_string());
        }
        Ok(())
    })
}

/// Send unitcast data to 3 receivers.
pub fn data_unicast_1to3(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        shared_prefix: Some("$share/general-mq/".to_string()),
        ..Default::default()
    };
    let handlers = create_msg_rsc(state, &mut resources, &opts, 3)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let send_queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have send queue")),
        Some(q) => q,
    };
    let handler1 = match handlers.get(0) {
        None => return Err(format!("should have a handler 1")),
        Some(handler) => handler,
    };
    let handler2 = match handlers.get(1) {
        None => return Err(format!("should have a handler 2")),
        Some(handler) => handler,
    };
    let handler3 = match handlers.get(2) {
        None => return Err(format!("should have a handler 3")),
        Some(handler) => handler,
    };

    let dataset = ["1", "2", "3", "4", "5", "6"];
    for data in dataset {
        let queue_clone = send_queue.clone();
        task::spawn(async move {
            let _ = queue_clone.send_msg(data.as_bytes().to_vec()).await;
        });
    }

    state.runtime.block_on(async move {
        let mut len = 0;
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                len = handler1.recv_messages.lock().unwrap().len()
                    + handler2.recv_messages.lock().unwrap().len()
                    + handler3.recv_messages.lock().unwrap().len();
            }
            if len == 6 {
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("received {}/6 messages", len));
        }
        let mut all_msg = vec![];
        {
            let messages = handler1.recv_messages.lock().unwrap();
            let messages = messages.as_slice();
            let len = messages.len();
            for i in 0..len {
                let str = get_message(messages, i)?;
                if all_msg.contains(&str) {
                    return Err("duplicate message".to_string());
                }
                all_msg.push(str);
            }
            let messages = handler2.recv_messages.lock().unwrap();
            let messages = messages.as_slice();
            let len = messages.len();
            for i in 0..len {
                let str = get_message(messages, i)?;
                if all_msg.contains(&str) {
                    return Err("duplicate message".to_string());
                }
                all_msg.push(str);
            }
            let messages = handler3.recv_messages.lock().unwrap();
            let messages = messages.as_slice();
            let len = messages.len();
            for i in 0..len {
                let str = get_message(messages, i)?;
                if all_msg.contains(&str) {
                    return Err("duplicate message".to_string());
                }
                all_msg.push(str);
            }
        }
        Ok(())
    })
}

/// Send broadcast data to one receiver.
pub fn data_broadcast_1to1(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        broadcast: true,
        ..Default::default()
    };
    let handlers = create_msg_rsc(state, &mut resources, &opts, 1)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let send_queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have send queue")),
        Some(q) => q,
    };
    let handler = match handlers.get(0) {
        None => return Err(format!("should have a handler")),
        Some(handler) => handler,
    };

    let dataset = ["1", "2"];
    for data in dataset {
        let queue_clone = send_queue.clone();
        task::spawn(async move {
            let _ = queue_clone.send_msg(data.as_bytes().to_vec()).await;
        });
    }

    state.runtime.block_on(async move {
        let mut len = 0;
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                len = handler.recv_messages.lock().unwrap().len();
            }
            if len == 2 {
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("received {}/2 messages", len));
        }
        let msg1;
        let msg2;
        {
            let messages = handler.recv_messages.lock().unwrap();
            let slice = messages.as_slice();
            msg1 = get_message(slice, 0)?;
            msg2 = get_message(slice, 1)?;
        }
        if msg1 == msg2 {
            return Err("duplicate message".to_string());
        }
        Ok(())
    })
}

/// Send broadcast data to 3 receivers.
pub fn data_broadcast_1to3(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        broadcast: true,
        ..Default::default()
    };
    let handlers = create_msg_rsc(state, &mut resources, &opts, 3)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let send_queue = match resources.queues.get_mut(0) {
        None => return Err(format!("should have send queue")),
        Some(q) => q,
    };
    let handler1 = match handlers.get(0) {
        None => return Err(format!("should have a handler 1")),
        Some(handler) => handler,
    };
    let handler2 = match handlers.get(1) {
        None => return Err(format!("should have a handler 2")),
        Some(handler) => handler,
    };
    let handler3 = match handlers.get(2) {
        None => return Err(format!("should have a handler 3")),
        Some(handler) => handler,
    };

    let dataset = ["1", "2"];
    for data in dataset {
        let queue_clone = send_queue.clone();
        task::spawn(async move {
            let _ = queue_clone.send_msg(data.as_bytes().to_vec()).await;
        });
    }

    state.runtime.block_on(async move {
        let mut len1 = 0;
        let mut len2 = 0;
        let mut len3 = 0;
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            {
                len1 = handler1.recv_messages.lock().unwrap().len();
                len2 = handler2.recv_messages.lock().unwrap().len();
                len3 = handler3.recv_messages.lock().unwrap().len();
            }
            if len1 + len2 + len3 == 6 {
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("received {}/6 messages", len1 + len2 + len3));
        } else if len1 != len2 || len2 != len3 {
            return Err(format!("receive count not all 2"));
        }
        let mut msg1;
        let mut msg2;
        {
            let messages = handler1.recv_messages.lock().unwrap();
            let slice = messages.as_slice();
            msg1 = get_message(slice, 0)?;
            msg2 = get_message(slice, 1)?;
        }
        if msg1 == msg2 {
            return Err("duplicate message handler 1".to_string());
        }
        {
            let messages = handler2.recv_messages.lock().unwrap();
            let slice = messages.as_slice();
            msg1 = get_message(slice, 0)?;
            msg2 = get_message(slice, 1)?;
        }
        if msg1 == msg2 {
            return Err("duplicate message handler 2".to_string());
        }
        {
            let messages = handler3.recv_messages.lock().unwrap();
            let slice = messages.as_slice();
            msg1 = get_message(slice, 0)?;
            msg2 = get_message(slice, 1)?;
        }
        if msg1 == msg2 {
            return Err("duplicate message handler 3".to_string());
        }
        Ok(())
    })
}

/// Send reliable data by sending data to a closed queue then it will receive after connecting.
pub fn data_reliable(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        reliable: true,
        ..Default::default()
    };
    let handlers = create_msg_rsc(state, &mut resources, &opts, 1)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let handler = match handlers.get(0) {
        None => return Err(format!("should have a handler")),
        Some(handler) => handler,
    };

    state.runtime.block_on(async move {
        let queue = match resources.queues.get_mut(0) {
            None => return Err(format!("should have send queue")),
            Some(q) => q,
        };
        if let Err(e) = queue.send_msg(b"1".to_vec()).await {
            return Err(format!("send 1 error: {}", e));
        }
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            let len;
            {
                len = handler.recv_messages.lock().unwrap().len();
            }
            if len == 1 {
                let msg = match get_message(handler.recv_messages.lock().unwrap().as_slice(), 0) {
                    Err(e) => return Err(format!("cannot get message[0]: {}", e)),
                    Ok(s) => s,
                };
                if !msg.eq("1") {
                    return Err(format!("should receive 1, not {}", msg.as_str()));
                }
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("cannot receive 1"));
        }

        let queue = match resources.queues.get_mut(1) {
            None => return Err(format!("should have recv queue")),
            Some(q) => q,
        };
        if let Err(e) = queue.close().await {
            return Err(format!("close recv error: {}", e));
        }
        let queue = match resources.queues.get_mut(0) {
            None => return Err(format!("should have send queue - 2")),
            Some(q) => q,
        };
        if let Err(e) = queue.send_msg(b"2".to_vec()).await {
            return Err(format!("send 2 error: {}", e));
        }
        let queue = match resources.queues.get_mut(1) {
            None => return Err(format!("should have recv queue - 2")),
            Some(q) => q,
        };
        if let Err(e) = queue.connect() {
            return Err(format!("connect recv error: {}", e));
        }
        let mut retry = 300;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            let len;
            {
                len = handler.recv_messages.lock().unwrap().len();
            }
            if len == 2 {
                let msg = match get_message(handler.recv_messages.lock().unwrap().as_slice(), 1) {
                    Err(e) => return Err(format!("cannot get message[1]: {}", e)),
                    Ok(s) => s,
                };
                if !msg.eq("2") {
                    return Err(format!("should receive 2, not {}", msg.as_str()));
                }
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("cannot receive 2"));
        }
        Ok(())
    })
}

/// Send reliable data by sending data to a closed queue then it may receive after connecting.
pub fn data_best_effort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mut resources = Resources::default();

    let opts = MqttQueueOptions {
        name: "name".to_string(),
        reliable: false,
        ..Default::default()
    };
    let handlers = create_msg_rsc(state, &mut resources, &opts, 1)?;

    for queue in resources.queues.iter() {
        state
            .runtime
            .block_on(wait_connected(queue.as_ref(), RETRY_10MS))?;
    }

    let handler = match handlers.get(0) {
        None => return Err(format!("should have a handler")),
        Some(handler) => handler,
    };

    state.runtime.block_on(async move {
        let queue = match resources.queues.get_mut(0) {
            None => return Err(format!("should have send queue")),
            Some(q) => q,
        };
        if let Err(e) = queue.send_msg(b"1".to_vec()).await {
            return Err(format!("send 1 error: {}", e));
        }
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            let len;
            {
                len = handler.recv_messages.lock().unwrap().len();
            }
            if len == 1 {
                let msg = match get_message(handler.recv_messages.lock().unwrap().as_slice(), 0) {
                    Err(e) => return Err(format!("cannot get message[0]: {}", e)),
                    Ok(s) => s,
                };
                if !msg.eq("1") {
                    return Err(format!("should receive 1, not {}", msg.as_str()));
                }
                break;
            }
            retry = retry - 1;
        }
        if retry == 0 {
            return Err(format!("cannot receive 1"));
        }

        let queue = match resources.queues.get_mut(1) {
            None => return Err(format!("should have recv queue")),
            Some(q) => q,
        };
        if let Err(e) = queue.close().await {
            return Err(format!("close recv error: {}", e));
        }
        let queue = match resources.queues.get_mut(0) {
            None => return Err(format!("should have send queue - 2")),
            Some(q) => q,
        };
        if let Err(e) = queue.send_msg(b"2".to_vec()).await {
            return Err(format!("send 2 error: {}", e));
        }
        let queue = match resources.queues.get_mut(1) {
            None => return Err(format!("should have recv queue - 2")),
            Some(q) => q,
        };
        if let Err(e) = queue.connect() {
            return Err(format!("connect recv error: {}", e));
        }
        let mut retry = 150;
        while retry > 0 {
            time::sleep(Duration::from_millis(10)).await;
            let len;
            {
                len = handler.recv_messages.lock().unwrap().len();
            }
            if len == 2 {
                let msg = match get_message(handler.recv_messages.lock().unwrap().as_slice(), 1) {
                    Err(e) => return Err(format!("cannot get message[1]: {}", e)),
                    Ok(s) => s,
                };
                if !msg.eq("2") {
                    return Err(format!("should receive 2, not {}", msg.as_str()));
                }
                break;
            }
            retry = retry - 1;
        }
        Ok(())
    })
}

/// Create connected (optional) connections/queues for testing connections.
fn create_conn_rsc(
    state: &mut TestState,
    resources: &mut Resources,
    handler: Option<Arc<dyn EventHandler>>,
    connect: bool,
) -> Result<(), String> {
    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    resources.conn = vec![Box::new(conn.clone())];
    let opts = MqttQueueOptions {
        name: "name".to_string(),
        is_recv: true,
        ..Default::default()
    };
    let mut queue = match MqttQueue::new(opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() error: {}", e)),
        Ok(q) => q,
    };
    state.queues = vec![Box::new(queue.clone())];
    resources.queues = vec![Box::new(queue.clone())];

    if let Some(handler) = handler {
        queue.set_handler(handler);
    }

    if !connect {
        return Ok(());
    }

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }
    Ok(())
}

/// Create connected (optional) connections/queues for testing connections.
fn mq_create_conn_rsc(
    state: &mut TestState,
    resources: &mut Resources,
    handler: Option<Arc<dyn EventHandler>>,
    connect: bool,
) -> Result<(), String> {
    let mut conn = match MqttConnection::new(MqttConnectionOptions::default()) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    resources.conn = vec![Box::new(conn.clone())];
    let opts = MqQueueOptions::Mqtt(
        MqttQueueOptions {
            name: "name".to_string(),
            is_recv: true,
            ..Default::default()
        },
        &conn,
    );
    let mut queue = match MqQueue::new(opts) {
        Err(e) => return Err(format!("Queue::new() error: {}", e)),
        Ok(q) => q,
    };
    state.queues = vec![Box::new(queue.clone())];
    resources.mq_queues = vec![Box::new(queue.clone())];

    if let Some(handler) = handler {
        queue.set_handler(handler);
    }

    if !connect {
        return Ok(());
    }

    if let Err(e) = conn.connect() {
        return Err(format!("Connect::connect() error: {}", e));
    }
    if let Err(e) = queue.connect() {
        return Err(format!("Queue::connect() error: {}", e));
    }
    Ok(())
}

/// Create connected (optional) connections/queues for testing messages.
fn create_msg_rsc(
    state: &mut TestState,
    resources: &mut Resources,
    opts: &MqttQueueOptions,
    receiver_count: usize,
) -> Result<Vec<TestRecvMsgHandler>, String> {
    let conn_opts = MqttConnectionOptions {
        client_id: Some(format!("sender-{}", randomstring(8))),
        ..Default::default()
    };
    let conn = match MqttConnection::new(conn_opts) {
        Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
        Ok(conn) => conn,
    };
    state.conn = vec![Box::new(conn.clone())];
    resources.conn = vec![Box::new(conn.clone())];
    let mut send_opts = opts.clone();
    send_opts.is_recv = false;
    let queue = match MqttQueue::new(send_opts, &conn) {
        Err(e) => return Err(format!("MqttQueue::new() send error: {}", e)),
        Ok(q) => q,
    };
    state.queues.push(Box::new(queue.clone()));
    resources.queues.push(Box::new(queue.clone()));

    let mut ret_handlers = vec![];
    for i in 0..receiver_count {
        let conn_opts = MqttConnectionOptions {
            client_id: match opts.reliable {
                false => Some(format!("receiver-{}-{}", i, randomstring(8))),
                true => Some(format!("receiver-{}", i)),
            },
            clean_session: !opts.reliable,
            ..Default::default()
        };
        let conn = match MqttConnection::new(conn_opts) {
            Err(e) => return Err(format!("MqttConnection::new() error: {}", e)),
            Ok(conn) => conn,
        };
        state.conn.push(Box::new(conn.clone()));
        resources.conn.push(Box::new(conn.clone()));

        let mut recv_opts = opts.clone();
        recv_opts.is_recv = true;
        let mut queue = match MqttQueue::new(recv_opts, &conn) {
            Err(e) => return Err(format!("MqttQueue::new() recv error: {}", e)),
            Ok(q) => q,
        };
        state.queues.push(Box::new(queue.clone()));
        resources.queues.push(Box::new(queue.clone()));

        let handler = TestRecvMsgHandler {
            recv_messages: Arc::new(Mutex::new(vec![])),
            ack_errors: Arc::new(Mutex::new(vec![])),
            use_nack: Arc::new(Mutex::new(false)),
            nack_messages: Arc::new(Mutex::new(vec![])),
            nack_errors: Arc::new(Mutex::new(vec![])),
        };
        queue.set_handler(Arc::new(handler.clone()));
        ret_handlers.push(handler);
    }

    for conn in resources.conn.iter_mut() {
        if let Err(e) = conn.connect() {
            return Err(format!("Connect::connect() error: {}", e));
        }
    }
    for queue in resources.queues.iter_mut() {
        if let Err(e) = queue.connect() {
            return Err(format!("Queue::connect() error: {}", e));
        }
    }
    Ok(ret_handlers)
}

async fn wait_connected(queue: &dyn Queue, mut retry: usize) -> Result<(), String> {
    while retry > 0 {
        time::sleep(Duration::from_millis(10)).await;
        if queue.status() == Status::Connected {
            return Ok(());
        }
        retry = retry - 1;
    }
    Err("not connected".to_string())
}

fn get_message(messages: &[Vec<u8>], index: usize) -> Result<String, String> {
    match messages.get(index) {
        None => Err(format!("messages[{}] get none", index)),
        Some(msg) => match str::from_utf8(msg) {
            Err(e) => Err(format!("messages[{}] from UTF8 error: {}", index, e)),
            Ok(msg) => Ok(msg.to_string()),
        },
    }
}
