use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use general_mq::queue::{Event, EventHandler, Message, Queue, Status};
use laboratory::{expect, SpecContext};
use tokio::time;

use sylvia_iot_broker::libs::mq::{control, Connection};

use super::STATE;
use crate::{libs::libs::conn_host_uri, TestState, WAIT_COUNT, WAIT_TICK};

struct TestHandler {
    // Use Mutex to implement interior mutability.
    status_changed: Arc<Mutex<bool>>,
}

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            status_changed: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_event(&self, _queue: Arc<dyn Queue>, ev: Event) {
        if let Event::Status(status) = ev {
            if status == Status::Connected {
                *self.status_changed.lock().unwrap() = true;
            }
        }
    }

    async fn on_message(&self, _queue: Arc<dyn Queue>, _msg: Box<dyn Message>) {}
}

/// Test new control queue.
pub fn new(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn_pool: Arc<Mutex<HashMap<String, Connection>>> = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler1 = Arc::new(TestHandler::new());
    let handler2 = Arc::new(TestHandler::new());
    let handler3 = Arc::new(TestHandler::new());

    let queue1 = control::new(
        conn_pool.clone(),
        &host_uri,
        None,
        "func1",
        false,
        handler1.clone(),
    )?;
    let queue2 = control::new(
        conn_pool.clone(),
        &host_uri,
        Some(1),
        "func2",
        false,
        handler2.clone(),
    )?;
    let queue3 = control::new(
        conn_pool,
        &host_uri,
        Some(0),
        "func3",
        false,
        handler3.clone(),
    )?;
    state.ctrl_queues = Some(vec![queue1.clone(), queue2.clone(), queue3.clone()]);

    expect(queue1.status() == Status::Connecting).equals(true)?;
    expect(queue2.status() == Status::Connecting).equals(true)?;
    expect(queue3.status() == Status::Connecting).equals(true)?;

    for _ in 0..WAIT_COUNT {
        if *handler1.status_changed.lock().unwrap() {
            break;
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    for _ in 0..WAIT_COUNT {
        if *handler2.status_changed.lock().unwrap() {
            break;
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    for _ in 0..WAIT_COUNT {
        if *handler3.status_changed.lock().unwrap() {
            break;
        }
        runtime.block_on(async { time::sleep(Duration::from_millis(WAIT_TICK)).await });
    }
    expect(queue1.status() == Status::Connected).equals(true)?;
    expect(queue2.status() == Status::Connected).equals(true)?;
    expect(queue3.status() == Status::Connected).equals(true)
}

/// Test new control queue with wrong options.
pub fn new_wrong_opts(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let mq_engine = state.mq_engine.as_ref().unwrap().as_str();

    let conn_pool: Arc<Mutex<HashMap<String, Connection>>> = Arc::new(Mutex::new(HashMap::new()));
    let host_uri = conn_host_uri(mq_engine)?;
    let handler = Arc::new(TestHandler::new());

    let queue = control::new(conn_pool, &host_uri, None, "", false, handler);
    expect(queue.is_err()).equals(true)
}
