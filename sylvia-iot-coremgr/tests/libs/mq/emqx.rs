use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use laboratory::SpecContext;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::time;

use general_mq::{
    connection::{GmqConnection, Status as ConnStatus},
    queue::{GmqQueue, Message, MessageHandler, Status as QueueStatus},
    MqttConnection, MqttConnectionOptions, MqttQueue, MqttQueueOptions,
};
use sylvia_iot_corelib::err::ErrResp;
use sylvia_iot_coremgr::libs::mq::{
    emqx::{self, ManagementOpts},
    QueueType,
};

use super::STATE;
use crate::TestState;

struct TestDummyHandler;

#[async_trait]
impl MessageHandler for TestDummyHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, _msg: Box<dyn Message>) {}
}

#[derive(Serialize)]
struct PostLoginReqBody<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct PostLoginResBody {
    token: String,
}

#[derive(Serialize)]
struct PostUsersReqBody<'a> {
    user_id: &'a str,
    password: &'a str,
    is_superuser: bool,
}

#[derive(Serialize)]
struct PostApiKeyReqBody<'a> {
    name: &'a str,
    desc: &'a str,
    enable: bool,
}

#[derive(Deserialize)]
struct PostApiKeyResBody {
    api_key: String,
    api_secret: String,
}

#[derive(Deserialize)]
struct GetAuthUsersResp {
    user_id: String,
}

#[derive(Deserialize)]
struct GetAclBody {
    rules: Vec<GetAclRulesItem>,
    #[serde(rename = "username")]
    _username: String,
}

#[derive(Deserialize)]
struct GetAclRulesItem {
    #[serde(rename = "topic")]
    _topic: String,
    #[serde(rename = "action")]
    _action: String,
    #[serde(rename = "permission")]
    _permission: String,
}

/// Authenticator ID.
const AUTH_ID: &'static str = "password_based:built_in_database";

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let emqx_opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;

        let names = vec![
            crate::TEST_MQ_USER,
            crate::TEST_MQ_USER_SUPER,
            crate::TEST_MQ_USER_WRONG,
        ];
        for name in names {
            let uri = format!(
                "http://{}:18083/api/v5/authentication/{}/users/{}",
                host, AUTH_ID, name
            );
            match client
                .request(Method::DELETE, uri)
                .basic_auth(
                    emqx_opts.api_key.as_str(),
                    Some(emqx_opts.api_secret.as_str()),
                )
                .build()
            {
                Err(e) => println!("create request for delete user {} error: {}", name, e),
                Ok(req) => match client.execute(req).await {
                    Err(e) => println!("execute user {} request error: {}", name, e),
                    Ok(resp) => match resp.status() {
                        StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                        _ => println!(
                            "execute user {} request with status: {}",
                            name,
                            resp.status()
                        ),
                    },
                },
            }

            let uri = format!(
                "http://{}:18083/api/v5/authorization/sources/built_in_database/rules/users/{}",
                host, name
            );
            match client
                .request(Method::DELETE, uri)
                .basic_auth(
                    emqx_opts.api_key.as_str(),
                    Some(emqx_opts.api_secret.as_str()),
                )
                .build()
            {
                Err(e) => println!("create request for delete acl {}/# error: {}", name, e),
                Ok(req) => match client.execute(req).await {
                    Err(e) => println!("execute acl request error: {}", e),
                    Ok(resp) => match resp.status() {
                        StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => (),
                        _ => println!("execute acl request with status: {}", resp.status()),
                    },
                },
            }
        }
    })
}

/// This is a utility function for getting user token to operate key/secret APIs.
/// The return value is the Bearer token.
pub async fn login() -> Result<String, String> {
    let uri = format!("http://{}:18083/api/v5/login", crate::TEST_MQ_HOST);
    let client = reqwest::Client::new();

    let resp = match client
        .request(Method::POST, uri)
        .json(&PostLoginReqBody {
            username: crate::TEST_EMQX_USER,
            password: crate::TEST_EMQX_PASS,
        })
        .build()
    {
        Err(e) => return Err(format!("create request for login error: {}", e)),
        Ok(req) => match client.execute(req).await {
            Err(e) => return Err(format!("execute login request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::OK => resp,
                _ => {
                    return Err(format!(
                        "execute login request with status: {}",
                        resp.status()
                    ))
                }
            },
        },
    };
    match resp.json::<PostLoginResBody>().await {
        Err(e) => Err(format!("read login body error: {}", e)),
        Ok(body) => Ok(body.token),
    }
}

/// This is a utility function for `before()` functions to create a super user for testing.
pub async fn before_add_superuser() -> Result<(), String> {
    let uri = format!(
        "http://{}:18083/api/v5/authentication/password_based:built_in_database/users",
        crate::TEST_MQ_HOST
    );
    let client = reqwest::Client::new();

    let token = login().await?;
    match client
        .request(Method::POST, uri)
        .header("Authorization", format!("Bearer {}", token))
        .json(&PostUsersReqBody {
            user_id: crate::TEST_EMQX_USER,
            password: crate::TEST_EMQX_PASS,
            is_superuser: true,
        })
        .build()
    {
        Err(e) => Err(format!(
            "create request for creating super user error: {}",
            e
        )),
        Ok(req) => match client.execute(req).await {
            Err(e) => Err(format!("execute creating super user request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::CREATED | StatusCode::CONFLICT => Ok(()),
                _ => {
                    let status = resp.status();
                    let body = resp.bytes().await.unwrap().to_vec();
                    let body = std::str::from_utf8(body.as_slice()).unwrap();
                    Err(format!(
                        "execute creating super user request with status: {}, body: {}",
                        status, body
                    ))
                }
            },
        },
    }
}

/// This is a utility function for `after()` functions to delete the super user for testing.
pub async fn after_del_superuser() -> Result<(), String> {
    let uri = format!(
        "http://{}:18083/api/v5/authentication/password_based:built_in_database/users/{}",
        crate::TEST_MQ_HOST,
        crate::TEST_EMQX_USER
    );
    let client = reqwest::Client::new();

    let token = login().await?;
    match client
        .request(Method::DELETE, uri)
        .header("Authorization", format!("Bearer {}", token))
        .build()
    {
        Err(e) => Err(format!(
            "create request for deleting super user error: {}",
            e
        )),
        Ok(req) => match client.execute(req).await {
            Err(e) => Err(format!("execute deleting super user request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
                _ => Err(format!(
                    "execute deleting super user request with status: {}",
                    resp.status()
                )),
            },
        },
    }
}

/// This is a utility function for `before()` functions to create an API key for testing.
/// The return tuple is (api_key, api_secret).
pub async fn before_add_api_key() -> Result<(String, String), String> {
    let uri = format!("http://{}:18083/api/v5/api_key", crate::TEST_MQ_HOST);
    let client = reqwest::Client::new();

    let token = login().await?;
    let resp = match client
        .request(Method::POST, uri)
        .header("Authorization", format!("Bearer {}", token))
        .json(&PostApiKeyReqBody {
            name: crate::TEST_EMQX_KEY_NAME,
            desc: "",
            enable: true,
        })
        .build()
    {
        Err(e) => return Err(format!("create request for creating api_key error: {}", e)),
        Ok(req) => match client.execute(req).await {
            Err(e) => return Err(format!("execute creating api_key request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::OK => resp,
                _ => {
                    let status = resp.status();
                    let body = resp.bytes().await.unwrap().to_vec();
                    let body = std::str::from_utf8(body.as_slice()).unwrap();
                    return Err(format!(
                        "execute creating api_key request with status: {}, body: {}",
                        status, body
                    ));
                }
            },
        },
    };
    match resp.json::<PostApiKeyResBody>().await {
        Err(e) => Err(format!("read login body error: {}", e)),
        Ok(body) => Ok((body.api_key, body.api_secret)),
    }
}

/// This is a utility function for `after()` functions to clear the API key.
pub async fn after_del_api_key() -> Result<(), String> {
    let uri = format!(
        "http://{}:18083/api/v5/api_key/{}",
        crate::TEST_MQ_HOST,
        crate::TEST_EMQX_KEY_NAME
    );
    let client = reqwest::Client::new();

    let token = login().await?;
    match client
        .request(Method::DELETE, uri)
        .header("Authorization", format!("Bearer {}", token))
        .build()
    {
        Err(e) => Err(format!("create request for deleting api_key error: {}", e)),
        Ok(req) => match client.execute(req).await {
            Err(e) => Err(format!("execute deleting api_key request error: {}", e)),
            Ok(resp) => match resp.status() {
                StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
                _ => Err(format!(
                    "execute deleting api_key request with status: {}",
                    resp.status()
                )),
            },
        },
    }
}

pub fn post_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = emqx::post_user(client, opts, host, user, pass, false).await {
            return Err(format!("post_user error: {}", e));
        }
        match get_object(
            client,
            &format!("authentication/{}/users", AUTH_ID),
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::OK, body) => {
                match serde_json::from_str::<GetAuthUsersResp>(body.as_str()) {
                    Err(e) => return Err(format!("unexpected response: {}, body: {}", e, body)),
                    Ok(resp) => match resp.user_id.as_str() {
                        crate::TEST_MQ_USER => (),
                        _ => return Err(format!("get unexpected user: {}", resp.user_id)),
                    },
                }
            }
            (status, body) => {
                return Err(format!(
                    "post_user get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        }
        match emqx::post_user(client, opts, host, user, pass, false).await {
            Err(e) => Err(format!("post_user exist user error: {}", e)),
            Ok(_) => Ok(()),
        }
    })
}

pub fn post_user_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::post_user(client, &opts, host, user, "", false).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::post_user(client, &opts, host, user, "", false).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn put_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = emqx::post_user(client, opts, host, user, pass, false).await {
            return Err(format!("post_user error: {}", e));
        }
        if let Err(e) = emqx::put_user(client, opts, host, user, "changed password").await {
            return Err(format!("put_user error: {}", e));
        }
        Ok(())
    })
}

pub fn put_user_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = "test1";
        let pass = crate::TEST_MQ_PASS;
        match emqx::put_user(client, opts, host, user, pass).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::put_user(client, &opts, host, user, pass).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::put_user(client, &opts, host, user, pass).await {
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
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = emqx::post_user(client, opts, host, user, pass, false).await {
            return Err(format!("post_user error: {}", e));
        }
        match get_object(
            client,
            &format!("authentication/{}/users", AUTH_ID),
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::OK, body) => {
                match serde_json::from_str::<GetAuthUsersResp>(body.as_str()) {
                    Err(e) => return Err(format!("unexpected response: {}, body: {}", e, body)),
                    Ok(resp) => match resp.user_id.as_str() {
                        crate::TEST_MQ_USER => (),
                        _ => return Err(format!("get unexpected user: {}", resp.user_id)),
                    },
                }
            }
            (status, body) => {
                return Err(format!(
                    "post_user get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        }
        if let Err(e) = emqx::delete_user(client, opts, host, user).await {
            return Err(format!("delete_user error: {}", e));
        }
        match get_object(
            client,
            &format!("authentication/{}/users", AUTH_ID),
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::OK, body) => {
                match serde_json::from_str::<GetAuthUsersResp>(body.as_str()) {
                    Err(e) => return Err(format!("unexpected response: {}, body: {}", e, body)),
                    Ok(_) => return Err("delete user not work".to_string()),
                }
            }
            (StatusCode::NOT_FOUND, _) => (),
            (status, body) => {
                return Err(format!(
                    "post_user get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        }
        if let Err(e) = emqx::delete_user(client, opts, host, user).await {
            return Err(format!("delete_user error: {}", e));
        }
        Ok(())
    })
}

pub fn delete_user_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        match emqx::delete_user(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::delete_user(client, opts, host, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn post_acl(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl application error: {}", e));
        }
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl application error: {}", e));
        }
        match get_object(
            client,
            "authorization/sources/built_in_database/rules/users",
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::OK, body) => match serde_json::from_str::<GetAclBody>(body.as_str()) {
                Err(e) => return Err(format!("unexpected response: {}, body: {}", e, body)),
                Ok(resp) => match resp.rules.len() {
                    4 => (),
                    _ => return Err(format!("post_acl number wrong: 4/{}", resp.rules.len())),
                },
            },
            (status, body) => {
                return Err(format!(
                    "post_acl get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        }
        let q_type = QueueType::Network;
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl network error: {}", e));
        }
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl network error: {}", e));
        }
        match get_object(
            client,
            "authorization/sources/built_in_database/rules/users",
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::OK, body) => match serde_json::from_str::<GetAclBody>(body.as_str()) {
                Err(e) => return Err(format!("unexpected response: {}, body: {}", e, body)),
                Ok(resp) => match resp.rules.len() {
                    4 => Ok(()),
                    _ => Err(format!("post_acl number wrong: 4/{}", resp.rules.len())),
                },
            },
            (status, body) => Err(format!(
                "post_acl get wrong result status: {}, body: {}",
                status, body
            )),
        }
    })
}

pub fn post_acl_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let q_type = QueueType::Application;
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::post_acl(client, &opts, host, q_type, "").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::post_acl(client, &opts, host, q_type, "").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn delete_acl(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl application error: {}", e));
        }
        if let Err(e) = emqx::delete_acl(client, opts, host, user).await {
            return Err(format!("delete_acl application error: {}", e));
        }
        match get_object(
            client,
            "authorization/sources/built_in_database/rules/users",
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::NOT_FOUND, _) => (),
            (status, body) => {
                return Err(format!(
                    "delete_acl get wrong result status: {}, body: {}",
                    status, body
                ))
            }
        }
        let q_type = QueueType::Network;
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl network error: {}", e));
        }
        if let Err(e) = emqx::delete_acl(client, opts, host, user).await {
            return Err(format!("delete_acl network error: {}", e));
        }
        match get_object(
            client,
            "authorization/sources/built_in_database/rules/users",
            user,
            &opts,
        )
        .await?
        {
            (StatusCode::NOT_FOUND, _) => Ok(()),
            (status, body) => Err(format!(
                "delete_acl get wrong result status: {}, body: {}",
                status, body
            )),
        }
    })
}

pub fn delete_acl_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = "";
        match emqx::delete_acl(client, opts, host, user).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::delete_acl(client, opts, host, user).await {
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
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        if let Err(e) = emqx::publish_message(
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
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        //let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        // FIXME: Valid from EMQX 4.4.4.
        //match emqx::publish_message(client, opts, host, user, "uldata", "^".to_string()).await {
        //    Err(ErrResp::ErrIntMsg(_)) => (),
        //    _ => return Err("unexpected response".to_string()),
        //}
        let host = "localhost1";
        match emqx::publish_message(client, opts, host, user, "uldata", "^".to_string()).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn post_topic_metrics(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        if let Err(e) = emqx::post_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("post_topic_metrics error: {}", e));
        }
        if let Err(e) = emqx::post_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("post_topic_metrics again error: {}", e));
        }
        let q_type = QueueType::Network;
        if let Err(e) = emqx::post_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("post_topic_metrics network error: {}", e));
        }
        Ok(())
    })
}

pub fn post_topic_metrics_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::post_topic_metrics(client, &opts, host, q_type, user).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::post_topic_metrics(client, &opts, host, q_type, user).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn delete_topic_metrics(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        if let Err(e) = emqx::delete_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("delete_topic_metrics error: {}", e));
        }
        if let Err(e) = emqx::delete_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("delete_topic_metrics again error: {}", e));
        }
        let q_type = QueueType::Network;
        if let Err(e) = emqx::delete_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("delete_topic_metrics network error: {}", e));
        }
        Ok(())
    })
}

pub fn delete_topic_metrics_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::delete_topic_metrics(client, &opts, host, q_type, user).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::delete_topic_metrics(client, &opts, host, q_type, user).await {
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
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let q_type = QueueType::Application;
        if let Err(e) = emqx::publish_message(
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
        for _ in 0..500 {
            time::sleep(Duration::from_millis(10)).await;
            match emqx::stats(client, opts, host, user, "uldata").await {
                Err(e) => return Err(format!("stats {} error: {}", user, e)),
                Ok(stats) => {
                    if stats.publish_rate != 0.0 {
                        return Err("publish rate should be 0".to_string());
                    }
                    if stats.consumers != 0 {
                        return Err(format!("consumers not 0: {}", stats.consumers));
                    }
                }
            }
        }
        if let Err(e) = emqx::post_topic_metrics(client, opts, host, q_type, user).await {
            return Err(format!("post_topic_metrics error: {}", e));
        }
        if let Err(e) = emqx::publish_message(
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
        for _ in 0..500 {
            time::sleep(Duration::from_millis(10)).await;
            match emqx::stats(client, opts, host, user, "uldata").await {
                Err(e) => return Err(format!("stats {} error: {}", user, e)),
                Ok(stats) => {
                    if stats.publish_rate == 0.0 {
                        continue;
                    }
                    if stats.consumers != 0 {
                        return Err(format!("consumers not 0: {}", stats.consumers));
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
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::stats(client, &opts, host, user, "uldata").await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::stats(client, &opts, host, user, "uldata").await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

pub fn add_superuser(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER_SUPER;
        let pass = crate::TEST_MQ_PASS;
        if let Err(e) = emqx::post_user(client, opts, host, user, pass, true).await {
            return Err(format!("add_superuser error: {}", e));
        }
        Ok(())
    })
}

pub fn add_superuser_error(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let pass = crate::TEST_MQ_PASS;
        let mut opts = opts.clone();
        opts.api_secret = "wrong secret".to_string();
        match emqx::post_user(client, &opts, host, user, pass, true).await {
            Err(ErrResp::ErrIntMsg(_)) => (),
            _ => return Err("unexpected response".to_string()),
        }
        let host = "localhost1";
        match emqx::post_user(client, &opts, host, user, pass, true).await {
            Err(ErrResp::ErrIntMsg(_)) => Ok(()),
            _ => Err("unexpected response".to_string()),
        }
    })
}

/// Test a real scenario with the following steps:
/// 1. Create a superuser and a new user with permission.
/// 2. Use new user to connect to the specified queue.
/// 3. Use new user to connect to the unspecified queue.
pub fn scenario(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let client = state.client.as_ref().unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().1;

    runtime.block_on(async move {
        // Step 1.
        let host = crate::TEST_MQ_HOST;
        let user = crate::TEST_MQ_USER;
        let user_super = crate::TEST_MQ_USER_SUPER;
        let user_wrong = crate::TEST_MQ_USER_WRONG;
        let pass = crate::TEST_MQ_PASS;

        if let Err(e) = emqx::post_user(client, opts, host, user_super, pass, true).await {
            return Err(format!("add_superuser error: {}", e));
        }
        if let Err(e) = emqx::post_user(client, opts, host, user, pass, false).await {
            return Err(format!("post_user {} error: {}", user, e));
        }
        if let Err(e) = emqx::post_user(client, opts, host, user_wrong, pass, false).await {
            return Err(format!("put_user {} error: {}", user_wrong, e));
        }
        let q_type = QueueType::Application;
        if let Err(e) = emqx::post_acl(client, opts, host, q_type, user).await {
            return Err(format!("post_acl application error: {}", e));
        }

        // Step 2,3.
        let q_opts = MqttConnectionOptions {
            uri: format!("mqtt://{}:{}@{}", user, pass, host),
            ..Default::default()
        };
        let mut conn = MqttConnection::new(q_opts)?;
        if let Err(e) = conn.connect() {
            return Err(format!("connect correct error: {}", e));
        }
        let q_opts = MqttQueueOptions {
            name: format!("broker.{}.uldata", user),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut queue = MqttQueue::new(q_opts, &conn)?;
        queue.set_msg_handler(Arc::new(TestDummyHandler {}));
        let q_opts = MqttQueueOptions {
            name: format!("test.{}.dldata", user),
            is_recv: true,
            reliable: true,
            broadcast: false,
            ..Default::default()
        };
        let mut wrong_queue = MqttQueue::new(q_opts, &conn)?;
        wrong_queue.set_msg_handler(Arc::new(TestDummyHandler {}));
        if let Err(e) = queue.connect() {
            return Err(format!("connect queue error: {}", e));
        }
        if let Err(e) = wrong_queue.connect() {
            return Err(format!("connect wrong queue error: {}", e));
        }
        time::sleep(Duration::from_secs(2)).await;
        if conn.status() != ConnStatus::Connected {
            return Err("should connect to host".to_string());
        } else if queue.status() != QueueStatus::Connected {
            return Err(format!(
                "should connect to queue, status: {:?}",
                queue.status()
            ));
        } else if wrong_queue.status() != QueueStatus::Connected {
            // TODO: modify general-mq mqtt module to handle SUBACK.
            return Err(format!(
                "should connect to wrong queue, status: {:?}", // TODO: should not
                wrong_queue.status()
            ));
        }
        if let Err(e) = queue.close().await {
            return Err(format!("close queue error: {}", e));
        }
        if let Err(e) = wrong_queue.close().await {
            return Err(format!("close wrong queue error: {}", e));
        }

        Ok(())
    })
}

async fn get_object(
    client: &Client,
    resource: &str,
    name: &str,
    opts: &ManagementOpts,
) -> Result<(StatusCode, String), String> {
    let uri = match name.len() {
        0 => format!("http://{}:18083/api/v5/{}", crate::TEST_MQ_HOST, resource,),
        _ => format!(
            "http://{}:18083/api/v5/{}/{}",
            crate::TEST_MQ_HOST,
            resource,
            name
        ),
    };
    match client
        .request(Method::GET, uri)
        .basic_auth(opts.api_key.as_str(), Some(opts.api_secret.as_str()))
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
