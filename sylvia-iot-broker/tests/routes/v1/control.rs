use std::{collections::HashMap, error::Error as StdError, sync::Arc, time::Duration};

use async_trait::async_trait;
use laboratory::SpecContext;
use serde::Serialize;
use serde_json;
use tokio::time;
use url::Url;

use general_mq::queue::{EventHandler, GmqQueue, Message, MessageHandler, Status};
use sylvia_iot_broker::libs::{
    config::DEF_MQ_CHANNEL_URL,
    mq::{control, Options as MgrOptions},
};

use super::STATE;
use crate::{TestState, WAIT_COUNT, WAIT_TICK};

#[derive(Serialize)]
struct AddManagerMsg<'a> {
    operation: &'a str,
    new: CtrlAddManager,
}

#[derive(Serialize)]
struct CtrlAddManager {
    #[serde(rename = "hostUri")]
    host_uri: String,
    #[serde(rename = "mgrOptions")]
    mgr_options: MgrOptions,
}

struct TestHandler;

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_error(&self, _queue: Arc<dyn GmqQueue>, _err: Box<dyn StdError + Send + Sync>) {}

    async fn on_status(&self, _queue: Arc<dyn GmqQueue>, _status: Status) {}
}

#[async_trait]
impl MessageHandler for TestHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, _msg: Box<dyn Message>) {}
}

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(mut queues) = state.ctrl_queues.take() {
        runtime.block_on(async {
            for i in queues.iter_mut() {
                let _ = i.close().await;
            }
        });
    }
}

pub fn test_wrong_data(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let url = Url::parse(DEF_MQ_CHANNEL_URL).unwrap();
    let app_ctrl_sender = control::new(
        routes_state.mq_conns.clone(),
        &url,
        None,
        "application",
        false,
        Arc::new(TestHandler {}),
        Arc::new(TestHandler {}),
    )
    .unwrap();
    let net_ctrl_sender = control::new(
        routes_state.mq_conns.clone(),
        &url,
        None,
        "network",
        false,
        Arc::new(TestHandler {}),
        Arc::new(TestHandler {}),
    )
    .unwrap();
    state.ctrl_queues = Some(vec![app_ctrl_sender.clone(), net_ctrl_sender.clone()]);

    runtime.block_on(async {
        for _ in 0..WAIT_COUNT {
            if app_ctrl_sender.status() == Status::Connected
                && net_ctrl_sender.status() == Status::Connected
            {
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
    });
    if app_ctrl_sender.status() != Status::Connected
        || net_ctrl_sender.status() != Status::Connected
    {
        return Err("control senders are not connected".to_string());
    }

    runtime.block_on(async {
        let payload = "{\"}";
        let _ = app_ctrl_sender.send_msg(payload.into()).await;
        let _ = net_ctrl_sender.send_msg(payload.into()).await;

        let payload = "{\"operation\":\"add-manager\",\"new\":\"id\"}";
        let _ = app_ctrl_sender.send_msg(payload.into()).await;
        let _ = net_ctrl_sender.send_msg(payload.into()).await;

        let mut msg = AddManagerMsg {
            operation: "del-manager",
            new: CtrlAddManager {
                host_uri: "://".to_string(),
                mgr_options: MgrOptions {
                    unit_id: "".to_string(),
                    unit_code: "".to_string(),
                    id: "".to_string(),
                    name: "".to_string(),
                    ..Default::default()
                },
            },
        };
        let payload = serde_json::to_vec(&msg).unwrap();
        let _ = app_ctrl_sender.send_msg(payload.clone()).await;
        let _ = net_ctrl_sender.send_msg(payload).await;

        msg.operation = "add-manager";
        let payload = serde_json::to_vec(&msg).unwrap();
        let _ = app_ctrl_sender.send_msg(payload.clone()).await;
        let _ = net_ctrl_sender.send_msg(payload).await;

        msg.new.host_uri = "amqp://localhost".to_string();
        let payload = serde_json::to_vec(&msg).unwrap();
        let _ = app_ctrl_sender.send_msg(payload.clone()).await;
        let _ = net_ctrl_sender.send_msg(payload).await;

        time::sleep(Duration::from_secs(2)).await;
    });

    Ok(())
}
