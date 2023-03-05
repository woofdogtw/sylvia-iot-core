use std::collections::HashMap;

use laboratory::{describe, LabResult};
use tokio::{runtime::Runtime, task};

use general_mq::{connection::Connection, queue::Queue};

mod amqp;
mod mqtt;

pub struct TestState {
    pub runtime: Runtime,
    pub conn: Vec<Box<dyn Connection>>,
    pub queues: Vec<Box<dyn Queue>>,
}

pub const STATE: &'static str = "general_mq";

#[tokio::test]
async fn integration_test() -> LabResult {
    let handle = task::spawn_blocking(|| {
        describe("function test", |context| {
            context.describe_import(amqp::suite());
            context.describe_import(mqtt::suite());
        })
        .run()
    });

    match handle.await {
        Err(e) => Err(format!("join error: {}", e)),
        Ok(result) => result,
    }
}

pub(crate) fn new_state() -> TestState {
    match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => TestState {
            runtime,
            conn: vec![],
            queues: vec![],
        },
    }
}

pub(crate) fn clear_state(state: &mut HashMap<&str, TestState>) {
    let state = state.get_mut(STATE).unwrap();

    state.runtime.block_on(async {
        while let Some(mut q) = state.queues.pop() {
            let _ = q.close().await;
        }
        while let Some(mut conn) = state.conn.pop() {
            let _ = conn.close().await;
        }
    });
}
