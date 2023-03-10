use std::{collections::HashMap, time::Duration};

use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use general_mq::{
    connection::{Connection, Status as ConnStatus},
    queue::{Queue, Status as QueueStatus},
    AmqpConnection, AmqpConnectionOptions, AmqpQueue, AmqpQueueOptions,
};
use laboratory::SpecContext;
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json;
use tokio::time;

use sylvia_iot_corelib::err::ErrResp;
use sylvia_iot_coremgr::libs::mq::{rabbitmq, QueueType};

use super::STATE;
use crate::TestState;

#[derive(Deserialize)]
struct GetUsersResp {
    password_hash: String,
}

#[derive(Deserialize)]
struct GetPolicies {
    definition: GetPoliciesDefinition,
}

#[derive(Deserialize)]
struct GetPoliciesDefinition {
    #[serde(rename = "message-ttl")]
    message_ttl: Option<usize>,
    #[serde(rename = "max-length")]
    max_length: Option<usize>,
}

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let amqp_conns = state.amqp_conn.take();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let user_wrong = crate::TEST_MQ_USER_WRONG;

        let uri = format!("http://{}:15672/api/users/{}", host, user);
        match client
            .request(Method::DELETE, uri)
            .basic_auth(crate::TEST_RABBITMQ_USER, Some(crate::TEST_RABBITMQ_PASS))
            .build()
        {
            Err(e) => println!("create request for delete user {} error: {}", user, e),
            Ok(req) => match client.execute(req).await {
                Err(e) => println!("execute user request error: {}", e),
                Ok(resp) => match resp.status() {
                    StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                    _ => println!("execute user request with status: {}", resp.status()),
                },
            },
        }

        let uri = format!("http://{}:15672/api/users/{}", host, user_wrong);
        match client
            .request(Method::DELETE, uri)
            .basic_auth(crate::TEST_RABBITMQ_USER, Some(crate::TEST_RABBITMQ_PASS))
            .build()
        {
            Err(e) => println!("create request for delete user {} error: {}", user_wrong, e),
            Ok(req) => match client.execute(req).await {
                Err(e) => println!("execute user request error: {}", e),
                Ok(resp) => match resp.status() {
                    StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                    _ => println!("execute user request with status: {}", resp.status()),
                },
            },
        }

        let uri = format!("http://{}:15672/api/vhosts/{}", host, user);
        match client
            .request(Method::DELETE, uri)
            .basic_auth(crate::TEST_RABBITMQ_USER, Some(crate::TEST_RABBITMQ_PASS))
            .build()
        {
            Err(e) => println!("create request for delete vhosts {} error: {}", user, e),
            Ok(req) => match client.execute(req).await {
                Err(e) => println!("execute vhosts request error: {}", e),
                Ok(resp) => match resp.status() {
                    StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                    _ => println!("execute vhosts request with status: {}", resp.status()),
                },
            },
        }

        if let Some(mut conn) = amqp_conns {
            for c in conn.iter_mut() {
                if let Err(e) = c.close().await {
                    println!("close AMQP connection error: {}", e);
                }
            }
        }
    })
}

pub fn put_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = rabbitmq::put_user(client, opts, host, user, pass).await {
            return Err(format!("put_user error: {}", e));
        }
        let first_password = match get_object(client, "users", user).await? {
            (StatusCode::OK, body) => match serde_json::from_str::<GetUsersResp>(body.as_str()) {
                Err(e) => {
                    return Err(format!(
                        "put_user deserialize user {} error: {}, body: {}",
                        user, e, body
                    ))
                }
                Ok(user) => user.password_hash,
            },
            (status, body) => {
                return Err(format!(
                    "put_user get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        };
        if let Err(e) = rabbitmq::put_user(client, opts, host, user, "changed password").await {
            return Err(format!("put_user error: {}", e));
        }
        let second_password = match get_object(client, "users", user).await? {
            (StatusCode::OK, body) => match serde_json::from_str::<GetUsersResp>(body.as_str()) {
                Err(e) => {
                    return Err(format!(
                        "put_user deserialize user {} error: {}, body: {}",
                        user, e, body
                    ))
                }
                Ok(user) => user.password_hash,
            },
            (status, body) => {
                return Err(format!(
                    "put_user get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        };
        if first_password.as_str().eq(second_password.as_str()) {
            return Err("password not changed".to_string());
        }
        Ok(())
    })
}

pub fn put_user_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = "";
        let pass = crate::TEST_MQ_PASS;
        match rabbitmq::put_user(client, opts, host, user, pass).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::put_user(client, opts, host, user, pass).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn delete_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = rabbitmq::put_user(client, opts, host, user, pass).await {
            return Err(format!("put_user error: {}", e));
        }
        if let Err(e) = rabbitmq::delete_user(client, opts, host, user).await {
            return Err(format!("delete_user error: {}", e));
        }
        match get_object(client, "users", user).await? {
            (StatusCode::NOT_FOUND, _) => Ok(()),
            (status, body) => Err(format!(
                "delete_user {} status: {}, body: {}",
                user, status, body
            )),
        }
    })
}

pub fn delete_user_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        match rabbitmq::delete_user(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::delete_user(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn put_vhost(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        match get_object(client, "vhosts", user).await? {
            (StatusCode::OK, _) => Ok(()),
            (status, body) => Err(format!(
                "put_vhost {} status: {}, body: {}",
                user, status, body
            )),
        }
    })
}

pub fn put_vhost_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        match rabbitmq::put_vhost(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::put_vhost(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn delete_vhost(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        if let Err(e) = rabbitmq::delete_vhost(client, opts, host, user).await {
            return Err(format!("delete_vhost error: {}", e));
        }
        match get_object(client, "vhosts", user).await? {
            (StatusCode::NOT_FOUND, _) => Ok(()),
            (status, body) => Err(format!(
                "delete_vhost {} status: {}, body: {}",
                user, status, body
            )),
        }
    })
}

pub fn delete_vhost_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        match rabbitmq::delete_vhost(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::delete_vhost(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn put_permissions(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = rabbitmq::put_user(client, opts, host, user, pass).await {
            return Err(format!("put_user error: {}", e));
        }
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        let mut q_type = QueueType::Application;
        if let Err(e) = rabbitmq::put_permissions(client, opts, host, q_type, user).await {
            return Err(format!("put_permissions application error: {}", e));
        }
        q_type = QueueType::Network;
        if let Err(e) = rabbitmq::put_permissions(client, opts, host, q_type, user).await {
            return Err(format!("put_permissions network error: {}", e));
        }
        Ok(())
    })
}

pub fn put_permissions_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        match rabbitmq::put_permissions(client, opts, host, QueueType::Application, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::put_permissions(client, opts, host, QueueType::Application, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn put_policies(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let policies = rabbitmq::BrokerPolicies {
            ttl: Some(10),
            length: Some(20),
        };
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        if let Err(e) = rabbitmq::put_policies(client, opts, host, user, &policies).await {
            return Err(format!("put_policies error: {}", e));
        }
        let policy = format!("{}/sylvia-iot-broker", user);
        match get_object(client, "policies", policy.as_str()).await {
            Err(e) => return Err(format!("get policies error: {}", e)),
            Ok(resp) => match resp {
                (StatusCode::OK, body) => {
                    match serde_json::from_str::<GetPolicies>(body.as_str()) {
                        Err(e) => return Err(format!("parse policies body error: {}", e)),
                        Ok(policy) => {
                            match policy.definition.message_ttl {
                                Some(10) => (),
                                _ => return Err("set TTL error".to_string()),
                            }
                            match policy.definition.max_length {
                                Some(20) => (),
                                _ => return Err("set length error".to_string()),
                            }
                        }
                    }
                }
                (status, body) => {
                    return Err(format!(
                        "get unexpected policies status: {}, body: {}",
                        status, body
                    ))
                }
            },
        }
        let policies = rabbitmq::BrokerPolicies {
            ttl: Some(0),
            length: Some(0),
        };
        if let Err(e) = rabbitmq::put_policies(client, opts, host, user, &policies).await {
            return Err(format!("put_policies to None error: {}", e));
        }
        match get_object(client, "policies", policy.as_str()).await {
            Err(e) => Err(format!("get policies None error: {}", e)),
            Ok(resp) => match resp {
                (StatusCode::NOT_FOUND, _) => Ok(()),
                (StatusCode::OK, body) => {
                    match serde_json::from_str::<GetPolicies>(body.as_str()) {
                        Err(e) => return Err(format!("parse policies body error: {}", e)),
                        Ok(policy) => {
                            match policy.definition.message_ttl {
                                None => (),
                                _ => return Err("set TTL None error".to_string()),
                            }
                            match policy.definition.max_length {
                                None => (),
                                _ => return Err("set length None error".to_string()),
                            }
                            Ok(())
                        }
                    }
                }
                (status, body) => Err(format!(
                    "get unexpected TTL 0 status: {}, body: {}",
                    status, body
                )),
            },
        }
    })
}

pub fn put_policies_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let mut policies = rabbitmq::BrokerPolicies {
            ttl: Some(0),
            length: Some(0),
        };
        match rabbitmq::put_policies(client, opts, host, "", &policies).await {
            Err(e) => return Err(format!("unexpected response: {}", e)),
            Ok(_) => (),
        }
        policies.ttl = Some(1);
        match rabbitmq::put_policies(client, opts, host, "", &policies).await {
            Err(ErrResp::ErrNotFound(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let mut opts = opts.clone();
        opts.password = "wrong password".to_string();
        match rabbitmq::put_policies(client, &opts, host, "", &policies).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::put_policies(client, &opts, host, "", &policies).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn get_policies(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let policies = rabbitmq::BrokerPolicies {
            ttl: Some(10),
            length: Some(20),
        };
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        if let Err(e) = rabbitmq::put_policies(client, opts, host, user, &policies).await {
            return Err(format!("put_policies error: {}", e));
        }
        match rabbitmq::get_policies(client, opts, host, user).await {
            Err(e) => return Err(format!("get_policies error: {}", e)),
            Ok(value) => {
                if value.ttl != Some(10) {
                    return Err(format!("get TTL {:?}, not 10", value.ttl));
                } else if value.length != Some(20) {
                    return Err(format!("get length {:?}, not 20", value.length));
                }
            }
        }
        let policies = rabbitmq::BrokerPolicies {
            ttl: Some(0),
            length: Some(0),
        };
        if let Err(e) = rabbitmq::put_policies(client, opts, host, user, &policies).await {
            return Err(format!("put_policies to None error: {}", e));
        }
        match rabbitmq::get_policies(client, opts, host, user).await {
            Err(e) => Err(format!("get_policies None error: {}", e)),
            Ok(value) => {
                if value.ttl != Some(0) {
                    return Err(format!("get TTL {:?}, not Some(0)", value.ttl));
                } else if value.length != Some(0) {
                    return Err(format!("get length {:?}, not Some(0)", value.length));
                }
                Ok(())
            }
        }
    })
}

pub fn get_policies_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let mut opts = opts.clone();
        opts.password = "wrong password".to_string();
        match rabbitmq::get_policies(client, &opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::get_policies(client, &opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn publish_message(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        let q_name = format!("broker.{}.uldata", user);
        if let Err(e) = put_queue(client, user, q_name.as_str()).await {
            return Err(format!("put_queue {}/uldata error: {}", user, e));
        }
        if let Err(e) = rabbitmq::publish_message(
            client,
            opts,
            host,
            user,
            "uldata",
            general_purpose::STANDARD.encode("payload"),
        )
        .await
        {
            return Err(format!("publish payload error: {}", e));
        }
        Ok(())
    })
}

pub fn publish_message_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        let q_name = format!("broker.{}.uldata", user);
        if let Err(e) = put_queue(client, user, q_name.as_str()).await {
            return Err(format!("put_queue {}/uldata error: {}", user, e));
        }
        match rabbitmq::publish_message(client, opts, host, user, "uldata", "^".to_string()).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::publish_message(client, opts, host, user, "uldata", "^".to_string()).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn stats(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        let q_name = format!("broker.{}.uldata", user);
        if let Err(e) = put_queue(client, user, q_name.as_str()).await {
            return Err(format!("put_queue {}/uldata error: {}", user, e));
        }
        if let Err(e) = rabbitmq::publish_message(
            client,
            opts,
            host,
            user,
            "uldata",
            general_purpose::STANDARD.encode("payload"),
        )
        .await
        {
            return Err(format!("publish payload error: {}", e));
        }
        for _ in 0..1000 {
            time::sleep(Duration::from_millis(10)).await;
            match rabbitmq::stats(client, opts, host, user, "uldata").await {
                Err(e) => return Err(format!("stats {} error: {}", user, e)),
                Ok(stats) => {
                    if stats.messages == 0 {
                        continue;
                    }
                    if stats.consumers != 0 {
                        return Err(format!("consumers not 0: {}", stats.consumers));
                    } else if stats.messages != 1 {
                        return Err(format!("messages not 1: {}", stats.messages));
                    }
                    return Ok(());
                }
            }
        }
        Err("no valid stats data".to_string())
    })
}

pub fn stats_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        let q_name = format!("broker.{}.uldata", user);
        if let Err(e) = put_queue(client, user, q_name.as_str()).await {
            return Err(format!("put_queue {}/uldata error: {}", user, e));
        }
        match rabbitmq::stats(client, opts, host, user, "").await {
            Err(ErrResp::ErrNotFound(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let mut opts = opts.clone();
        opts.password = "wrong password".to_string();
        match rabbitmq::stats(client, &opts, host, user, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match rabbitmq::stats(client, &opts, host, user, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

/// Test a real scenario with the following steps:
/// 1. Create a new user/vhost with permission.
/// 2. Set TTL: 5 seconds, length: 2.
/// 3. Use new user to connect to the specified vhost/queue.
/// 4. Use new user to connect to the unspecified queue.
/// 5. Test TTL/length. Enlarge TTL to 10 seconds, length to 3 and test again.
/// 6. Wrong user to connect to the new user's vhost/queue.
pub fn scenario(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().0;

    runtime.block_on(async move {
        // Step 1,2.
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let user_wrong = crate::TEST_MQ_USER_WRONG;
        let pass = crate::TEST_MQ_PASS;
        let ttl = 5000;
        let length = 2;
        let policies = rabbitmq::BrokerPolicies {
            ttl: Some(ttl),
            length: Some(length),
        };
        if let Err(e) = rabbitmq::put_user(client, opts, host, user, pass).await {
            return Err(format!("put_user error: {}", e));
        }
        if let Err(e) = rabbitmq::put_user(client, opts, host, user_wrong, pass).await {
            return Err(format!("put_user for wrong user error: {}", e));
        }
        if let Err(e) = rabbitmq::put_vhost(client, opts, host, user).await {
            return Err(format!("put_vhost error: {}", e));
        }
        let q_type = QueueType::Application;
        if let Err(e) = rabbitmq::put_permissions(client, opts, host, q_type, user).await {
            return Err(format!("put_permissions application error: {}", e));
        }
        if let Err(e) = rabbitmq::put_policies(client, opts, host, user, &policies).await {
            return Err(format!("put_policies error: {}", e));
        }

        // Step 3,4.
        let q_opts = AmqpConnectionOptions {
            uri: format!("amqp://{}:{}@{}/{}", user, pass, host, user),
            ..Default::default()
        };
        let mut conn = AmqpConnection::new(q_opts)?;
        if let Err(e) = conn.connect() {
            return Err(format!("connect correct error: {}", e));
        }
        let q_opts = AmqpQueueOptions {
            name: format!("broker.{}.uldata", user),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut queue = AmqpQueue::new(q_opts, &conn)?;
        let q_opts = AmqpQueueOptions {
            name: format!("broker.{}.dldata", user),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut wrong_queue = AmqpQueue::new(q_opts, &conn)?;
        if let Err(e) = queue.connect() {
            return Err(format!("connect queue error: {}", e));
        }
        if let Err(e) = wrong_queue.connect() {
            return Err(format!("connect wrong queue error: {}", e));
        }
        time::sleep(Duration::from_secs(2)).await;
        if conn.status() != ConnStatus::Connected {
            return Err("should connect to vhost".to_string());
        } else if queue.status() != QueueStatus::Connected {
            return Err(format!(
                "should connect to queue, status: {:?}",
                queue.status()
            ));
        } else if wrong_queue.status() == QueueStatus::Connected {
            return Err(format!(
                "should not connect to wrong queue, status: {:?}",
                wrong_queue.status()
            ));
        }
        if let Err(e) = queue.close().await {
            return Err(format!("close queue error: {}", e));
        }
        if let Err(e) = wrong_queue.close().await {
            return Err(format!("close wrong queue error: {}", e));
        }

        // Step 5.
        let name = "uldata";
        let payload = general_purpose::STANDARD.encode("1");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 1 error: {}", e));
        }
        let payload = general_purpose::STANDARD.encode("2");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 2 error: {}", e));
        }
        let payload = general_purpose::STANDARD.encode("3");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 3 error: {}", e));
        }
        let mut is_ok = false;
        for _ in 0..500 {
            time::sleep(Duration::from_millis(10)).await;
            match rabbitmq::stats(client, opts, host, user, name).await {
                Err(e) => return Err(format!("get TTL {} stats error: {}", ttl, e)),
                Ok(stats) => {
                    if stats.messages == 3 {
                        return Err("should not contain 3 messages".to_string());
                    } else if stats.messages == 2 {
                        is_ok = true;
                        break;
                    }
                }
            }
        }
        if !is_ok {
            return Err(format!("queue not TTL to {}", ttl));
        }
        let start = Utc::now();
        let mut end = Utc::now();
        for _ in 0..(ttl / 10 + 5000) {
            time::sleep(Duration::from_millis(10)).await;
            match rabbitmq::stats(client, opts, host, user, name).await {
                Err(e) => return Err(format!("get TTL {} stats error: {}", ttl, e)),
                Ok(stats) => {
                    end = Utc::now();
                    if stats.messages == 0 {
                        break;
                    }
                }
            }
        }
        let diff = end.timestamp_millis() - start.timestamp_millis();
        if diff >= ((ttl + 5000) as i64) {
            return Err(format!("TTL {} has something wrong: {}", ttl, diff));
        }
        let ttl_prev = ttl;
        let ttl = 10000;
        let length = 3;
        let policies = rabbitmq::BrokerPolicies {
            ttl: Some(ttl),
            length: Some(length),
        };
        if let Err(e) = rabbitmq::put_policies(client, opts, host, user, &policies).await {
            return Err(format!("put_policies error: {}", e));
        }
        let payload = general_purpose::STANDARD.encode("1");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 1 error: {}", e));
        }
        let payload = general_purpose::STANDARD.encode("2");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 2 error: {}", e));
        }
        let payload = general_purpose::STANDARD.encode("3");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 3 error: {}", e));
        }
        let payload = general_purpose::STANDARD.encode("4");
        if let Err(e) = rabbitmq::publish_message(client, opts, host, user, name, payload).await {
            return Err(format!("publish message 3 error: {}", e));
        }
        let mut messages = 100;
        is_ok = false;
        for _ in 0..500 {
            time::sleep(Duration::from_millis(10)).await;
            match rabbitmq::stats(client, opts, host, user, name).await {
                Err(e) => return Err(format!("get TTL {} stats error: {}", ttl, e)),
                Ok(stats) => {
                    messages = stats.messages;
                    if stats.messages == 3 {
                        is_ok = true;
                        break;
                    }
                }
            }
        }
        if !is_ok {
            return Err(format!("queue not TTL to {}: {}", ttl, messages));
        }
        let start = Utc::now();
        let mut end = Utc::now();
        for _ in 0..(ttl / 10 + 5000) {
            time::sleep(Duration::from_millis(10)).await;
            match rabbitmq::stats(client, opts, host, user, name).await {
                Err(e) => return Err(format!("get TTL {} stats error: {}", ttl, e)),
                Ok(stats) => {
                    end = Utc::now();
                    if stats.messages == 0 {
                        break;
                    }
                }
            }
        }
        let diff = end.timestamp_millis() - start.timestamp_millis();
        if diff >= ((ttl + 5000) as i64) || diff <= (ttl_prev as i64) {
            return Err(format!(
                "TTL {}/{} has something wrong: {}",
                ttl, ttl_prev, diff
            ));
        }

        // Step 6.
        let q_opts = AmqpConnectionOptions {
            uri: format!("amqp://{}:{}@{}/{}", user_wrong, pass, host, user),
            ..Default::default()
        };
        let mut conn = AmqpConnection::new(q_opts)?;
        if let Err(e) = conn.connect() {
            return Err(format!("connect correct error: {}", e));
        }
        time::sleep(Duration::from_secs(2)).await;
        if conn.status() == ConnStatus::Connected {
            let _ = conn.close().await;
            return Err("wrong user should connect to vhost".to_string());
        }
        let _ = conn.close().await;

        Ok(())
    })
}

async fn get_object(
    client: &Client,
    resource: &str,
    name: &str,
) -> Result<(StatusCode, String), String> {
    let uri = format!(
        "http://{}:15672/api/{}/{}",
        crate::TEST_MQ_HOST,
        resource,
        name
    );
    match client
        .request(Method::GET, uri)
        .basic_auth(crate::TEST_RABBITMQ_USER, Some(crate::TEST_RABBITMQ_PASS))
        .build()
    {
        Err(e) => Err(format!(
            "create request for get {} {} error: {}",
            resource, name, e
        )),
        Ok(req) => match client.execute(req).await {
            Err(e) => Err(format!(
                "execute get {} {} request error: {}",
                resource, name, e
            )),
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Err(e) => Err(format!(
                        "read get {} {} response error: {}",
                        resource, name, e
                    )),
                    Ok(body) => Ok((status, body)),
                }
            }
        },
    }
}

async fn put_queue(
    client: &Client,
    vhost: &str,
    name: &str,
) -> Result<(StatusCode, String), String> {
    let uri = format!(
        "http://{}:15672/api/queues/{}/{}",
        crate::TEST_MQ_HOST,
        vhost,
        name
    );
    match client
        .request(Method::PUT, uri)
        .basic_auth(crate::TEST_RABBITMQ_USER, Some(crate::TEST_RABBITMQ_PASS))
        .build()
    {
        Err(e) => Err(format!(
            "create request for put {}/{} error: {}",
            vhost, name, e
        )),
        Ok(req) => match client.execute(req).await {
            Err(e) => Err(format!(
                "execute put {}/{} request error: {}",
                vhost, name, e
            )),
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Err(e) => Err(format!("read put {}/{} response error: {}", vhost, name, e)),
                    Ok(body) => Ok((status, body)),
                }
            }
        },
    }
}
