use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    web, App, HttpRequest, HttpResponse, Responder,
};
use async_trait::async_trait;
use general_mq::{
    connection::GmqConnection,
    queue::{Event, EventHandler, GmqQueue, Message},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
};
use laboratory::{describe, expect, SpecContext, Suite};
use serde::Deserialize;
use serde_json::{Map, Value};

use sylvia_iot_coremgr::{libs::mq::Connection, routes::middleware::LogService};
use tokio::time;

use super::{
    clear_state,
    libs::{create_users_tokens, new_state, TOKEN_MANAGER},
    stop_auth_broker_svc,
};
use crate::{libs::mq::emqx, TestState, WAIT_COUNT, WAIT_TICK};

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum RecvDataMsg {
    #[serde(rename = "operation")]
    Operation { data: OperationData },
}

#[derive(Deserialize)]
struct OperationData {
    #[serde(rename = "dataId")]
    _data_id: String,
    #[serde(rename = "reqTime")]
    _req_time: String,
    #[serde(rename = "resTime")]
    _res_time: String,
    #[serde(rename = "latencyMs")]
    latency_ms: i64,
    #[serde(rename = "status")]
    _status: isize,
    #[serde(rename = "sourceIp")]
    _source_ip: String,
    #[serde(rename = "method")]
    _method: String,
    #[serde(rename = "path")]
    _path: String,
    #[serde(rename = "body")]
    body: Option<Map<String, Value>>,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "clientId")]
    _client_id: String,
    #[serde(rename = "errCode")]
    _err_code: Option<String>,
    #[serde(rename = "errMessage")]
    _err_message: Option<String>,
}

#[derive(Clone)]
struct TestHandler {
    recv_data: Arc<Mutex<Vec<RecvDataMsg>>>,
}

pub const STATE: &'static str = "routes/middleware";

pub fn suite(mqtt_engine: Option<&'static str>, data_host: &'static str) -> Suite<TestState> {
    let suite_name = format!("routes.middleware - {}", data_host);
    describe(suite_name, move |context| {
        context.it("GET", test_get);
        context.it("POST", test_post);
        context.it("PATCH with password", test_patch_password);
        context.it("DELETE for more coverage", test_delete_cover);

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(mqtt_engine, Some(data_host)));

                let state = state.get_mut(STATE).unwrap();
                create_users_tokens(state);
                if let Err(e) = create_data_recv_queue(state, data_host) {
                    panic!("{}", e);
                }
            })
            .after_all(after_all_fn)
            .after_each(after_each_fn);
    })
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let _ = state.rumqttd_handles.take();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(state) = state.routes_state.as_mut() {
        runtime.block_on(async {
            clear_state(state).await;
        });
    }

    if let Some(state) = state.routes_state.as_mut() {
        if let Some(mut q) = state.data_sender.take() {
            runtime.block_on(async {
                if let Err(e) = q.close().await {
                    println!("close data channel {} error: {}", q.name(), e);
                }
            });
        }
    }

    if let Some(mut q) = state.data_queue.take() {
        runtime.block_on(async {
            if let Err(e) = q.close().await {
                println!("close data recv queue {} error: {}", q.name(), e);
            }
        });
    }

    if let Some(conn) = state.mq_conn.take() {
        runtime.block_on(async {
            match conn {
                Connection::Amqp(mut conn, _) => {
                    let _ = conn.close().await;
                }
                Connection::Mqtt(mut conn, _) => {
                    let _ = conn.close().await;
                }
            }
        })
    }

    if let Err(e) = runtime.block_on(async { emqx::after_del_api_key().await }) {
        println!("delete EMQX API key error: {}", e);
    }
    if let Err(e) = runtime.block_on(async { emqx::after_del_superuser().await }) {
        println!("delete EMQX superuser error: {}", e);
    }

    stop_auth_broker_svc(state);
}

fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();

    if let Some(q) = state.data_queue.as_mut() {
        q.clear_handler();
    }
}

impl TestHandler {
    fn new() -> Self {
        TestHandler {
            recv_data: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EventHandler for TestHandler {
    async fn on_event(&self, _queue: Arc<dyn GmqQueue>, _ev: Event) {}

    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let _ = msg.ack().await;

        let data = match serde_json::from_slice::<RecvDataMsg>(msg.payload()) {
            Err(e) => {
                println!("unmarshal error: {}", e);
                return;
            }
            Ok(data) => data,
        };
        {
            self.recv_data.lock().unwrap().push(data);
        }
    }
}

fn test_get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();
    let data_sender = state.routes_state.as_ref().unwrap().data_sender.clone();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(LogService::new(auth_uri.clone(), data_sender))
                .route("/", web::get().to(dummy_handler)),
        )
        .await
    });

    let data_recv_queue = state.data_queue.as_mut().unwrap();
    let handler = TestHandler::new();
    data_recv_queue.set_handler(Arc::new(handler.clone()));

    let req = TestRequest::get()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }

    runtime.block_on(async {
        let mut is_data_recv = false;
        for _ in 0..WAIT_COUNT {
            if { handler.recv_data.lock().unwrap().pop() }.is_some() {
                is_data_recv = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        expect(is_data_recv).to_equal(false)
    })
}

fn test_post(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();
    let data_sender = state.routes_state.as_ref().unwrap().data_sender.clone();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(LogService::new(auth_uri.clone(), data_sender))
                .route("/", web::post().to(dummy_handler)),
        )
        .await
    });

    let data_recv_queue = state.data_queue.as_mut().unwrap();
    let handler = TestHandler::new();
    data_recv_queue.set_handler(Arc::new(handler.clone()));

    let mut req_body = Map::<String, Value>::new();
    let mut req_data = Map::<String, Value>::new();
    req_data.insert("key".to_string(), Value::String("value".to_string()));
    req_body.insert("data".to_string(), Value::Object(req_data));
    let req = TestRequest::post()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&req_body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }

    runtime.block_on(async {
        let mut data_recv = None;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { handler.recv_data.lock().unwrap().pop() } {
                data_recv = Some(d);
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        expect(data_recv.is_some()).to_equal(true)?;
        let d = data_recv.unwrap();
        match d {
            RecvDataMsg::Operation { data } => {
                expect(data.latency_ms >= 0).to_equal(true)?;
                expect(data.user_id.as_str()).to_equal("manager")?;
                expect(data.body).to_equal(Some(req_body))?;
            }
        }
        Ok(())
    })
}

fn test_patch_password(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();
    let data_sender = state.routes_state.as_ref().unwrap().data_sender.clone();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(LogService::new(auth_uri.clone(), data_sender))
                .route("/", web::patch().to(dummy_handler)),
        )
        .await
    });

    let data_recv_queue = state.data_queue.as_mut().unwrap();
    let handler = TestHandler::new();
    data_recv_queue.set_handler(Arc::new(handler.clone()));

    let mut req_body = Map::<String, Value>::new();
    let mut req_data = Map::<String, Value>::new();
    req_data.insert("password".to_string(), Value::String("value".to_string()));
    req_body.insert("data".to_string(), Value::Object(req_data));
    let req = TestRequest::patch()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", TOKEN_MANAGER)))
        .set_json(&req_body)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::NO_CONTENT {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }
    let mut req_data = Map::<String, Value>::new();
    req_data.insert("password".to_string(), Value::String("".to_string()));
    req_body.insert("data".to_string(), Value::Object(req_data));

    runtime.block_on(async {
        let mut data_recv = None;
        for _ in 0..WAIT_COUNT {
            if let Some(d) = { handler.recv_data.lock().unwrap().pop() } {
                data_recv = Some(d);
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        expect(data_recv.is_some()).to_equal(true)?;
        let d = data_recv.unwrap();
        match d {
            RecvDataMsg::Operation { data } => {
                expect(data.latency_ms >= 0).to_equal(true)?;
                expect(data.user_id.as_str()).to_equal("manager")?;
                expect(data.body).to_equal(Some(req_body))?;
            }
        }
        Ok(())
    })
}

fn test_delete_cover(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_uri = state.auth_uri.as_ref().unwrap();
    let data_sender = state.routes_state.as_ref().unwrap().data_sender.clone();

    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .wrap(LogService::new(auth_uri.clone(), data_sender))
                .route("/", web::delete().to(dummy_err_handler)),
        )
        .await
    });

    let data_recv_queue = state.data_queue.as_mut().unwrap();
    let handler = TestHandler::new();
    data_recv_queue.set_handler(Arc::new(handler.clone()));

    let req = TestRequest::delete().uri("/").to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::BAD_REQUEST {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }

    let req = TestRequest::delete()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer ")))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::BAD_REQUEST {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }

    let req = TestRequest::delete()
        .uri("/")
        .insert_header((header::AUTHORIZATION, format!("Bearer test")))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::BAD_REQUEST {
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!("status {}, body: {:?}", status, body));
    }

    runtime.block_on(async {
        let mut is_data_recv = false;
        for _ in 0..WAIT_COUNT {
            if { handler.recv_data.lock().unwrap().pop() }.is_some() {
                is_data_recv = true;
                break;
            }
            time::sleep(Duration::from_millis(WAIT_TICK)).await;
        }
        expect(is_data_recv).to_equal(false)
    })
}

async fn dummy_handler(_req: HttpRequest) -> impl Responder {
    HttpResponse::NoContent().finish()
}

async fn dummy_err_handler(_req: HttpRequest) -> impl Responder {
    HttpResponse::BadRequest().finish()
}

fn create_data_recv_queue(state: &mut TestState, data_host: &'static str) -> Result<(), String> {
    if data_host.starts_with("mqtt") {
        let runtime = state.runtime.as_ref().unwrap();
        if let Err(e) = runtime.block_on(async { emqx::before_add_superuser().await }) {
            println!("create EMQX superuser error: {}", e);
        }
        let opts = MqttConnectionOptions {
            uri: data_host.to_string(),
            ..Default::default()
        };
        let mut conn = MqttConnection::new(opts)?;
        if let Err(e) = conn.connect() {
            return Err(format!("create MQTT data recv connection error: {}", e));
        }
        state.mq_conn = Some(Connection::Mqtt(conn.clone(), Arc::new(Mutex::new(0))));
        let opts = MqttQueueOptions {
            name: "coremgr.data".to_string(),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut q = Queue::new(QueueOptions::Mqtt(opts, &conn))?;
        q.set_handler(Arc::new(TestHandler::new()));
        if let Err(e) = q.connect() {
            return Err(format!("create MQTT data recv queue error: {}", e));
        }
        state.data_queue = Some(q);
    } else {
        let opts = AmqpConnectionOptions {
            uri: data_host.to_string(),
            ..Default::default()
        };
        let mut conn = AmqpConnection::new(opts)?;
        if let Err(e) = conn.connect() {
            return Err(format!("create AMQP data recv connection error: {}", e));
        }
        state.mq_conn = Some(Connection::Amqp(conn.clone(), Arc::new(Mutex::new(0))));
        let opts = AmqpQueueOptions {
            name: "coremgr.data".to_string(),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut q = Queue::new(QueueOptions::Amqp(opts, &conn))?;
        q.set_handler(Arc::new(TestHandler::new()));
        if let Err(e) = q.connect() {
            return Err(format!("create AMQP data recv queue error: {}", e));
        }
        state.data_queue = Some(q);
    }
    Ok(())
}
