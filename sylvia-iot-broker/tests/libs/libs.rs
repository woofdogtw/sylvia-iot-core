use reqwest::{self, Method, StatusCode};
use serde::Deserialize;
use url::Url;

use sylvia_iot_corelib::constants::MqEngine;

use crate::TestState;

#[derive(Deserialize)]
struct QueueInfo {
    name: String,
}

pub fn conn_host_uri(mq_engine: &str) -> Result<Url, String> {
    match mq_engine {
        MqEngine::RABBITMQ => match Url::parse(crate::TEST_AMQP_HOST_URI) {
            Err(e) => Err(format!("AMQP URI error: {}", e)),
            Ok(uri) => Ok(uri),
        },
        MqEngine::EMQX => match Url::parse(crate::TEST_MQTT_HOST_URI) {
            Err(e) => Err(format!("MQTT URI error: {}", e)),
            Ok(uri) => Ok(uri),
        },
        _ => Err(format!("unsupport mq_engine {}", mq_engine)),
    }
}

pub fn remove_rabbitmq_queues(state: &TestState) {
    let runtime = state.runtime.as_ref().unwrap();
    let client = reqwest::Client::new();

    let req = match client
        .request(Method::GET, "http://localhost:15672/api/queues/%2f")
        .basic_auth("guest", Some("guest"))
        .build()
    {
        Err(e) => {
            println!("generate get request error: {}", e);
            return;
        }
        Ok(req) => req,
    };
    if let Err(e) = runtime.block_on(async {
        let resp = match client.execute(req).await {
            Err(e) => return Err(format!("execute get request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::OK => resp,
                _ => {
                    return Err(format!(
                        "execute get request with status: {}",
                        resp.status()
                    ));
                }
            },
        };
        let queues = match resp.json::<Vec<QueueInfo>>().await {
            Err(e) => return Err(format!("get response error: {}", e)),
            Ok(resp) => resp,
        };

        for queue in queues {
            if queue.name.starts_with("amq.") {
                continue;
            }
            let uri = format!("http://localhost:15672/api/queues/%2f/{}", queue.name);
            let req = match client
                .request(Method::DELETE, uri)
                .basic_auth("guest", Some("guest"))
                .build()
            {
                Err(e) => {
                    return Err(format!("generate delete request error: {}", e));
                }
                Ok(req) => req,
            };
            match client.execute(req).await {
                Err(e) => return Err(format!("execute delete request error: {}", e)),
                Ok(resp) => match resp.status() {
                    StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                    _ => println!("delete queue {} error: {}", queue.name, resp.status()),
                },
            };
        }
        Ok(())
    }) {
        println!("{}", e);
    }
}
