use laboratory::{describe, expect, SpecContext, Suite};
use reqwest::Client;
use tokio::runtime::Runtime;

use sylvia_iot_coremgr::libs::{
    config::Rumqttd as RumqttdOpts,
    mq::{
        self, emqx::ManagementOpts as EmqxOpts, rabbitmq::ManagementOpts as RabbitMqOpts, QueueType,
    },
};

use crate::TestState;

pub mod emqx; // public for routes test suites.
mod rabbitmq;
mod rumqttd;

pub const STATE: &'static str = "libs/mq";

pub fn suite() -> Suite<TestState> {
    describe("libs.mq", move |context| {
        context.it("to_username", to_username);

        context.describe("rabbitmq", |context| {
            context.it("put_user", rabbitmq::put_user);
            context.it("put_user with error", rabbitmq::put_user_error);
            context.it("delete_user", rabbitmq::delete_user);
            context.it("delete_user with error", rabbitmq::delete_user_error);
            context.it("put_vhost", rabbitmq::put_vhost);
            context.it("put_vhost with error", rabbitmq::put_vhost_error);
            context.it("delete_vhost", rabbitmq::delete_vhost);
            context.it("delete_vhost with error", rabbitmq::delete_vhost_error);
            context.it("put_permissions", rabbitmq::put_permissions);
            context.it(
                "put_permissions with error",
                rabbitmq::put_permissions_error,
            );
            context.it("put_policies", rabbitmq::put_policies);
            context.it("put_policies with error", rabbitmq::put_policies_error);
            context.it("get_policies", rabbitmq::get_policies);
            context.it("get_policies with error", rabbitmq::get_policies_error);
            context.it("publish_message", rabbitmq::publish_message);
            context.it(
                "publish_message with error",
                rabbitmq::publish_message_error,
            );
            context.it("stats", rabbitmq::stats);
            context.it("stats with error", rabbitmq::stats_error);

            context.it("scenario", rabbitmq::scenario);

            context.after_each(rabbitmq::after_each_fn);
        });

        context.describe("emqx", |context| {
            context.it("post_user", emqx::post_user);
            context.it("post_user with error", emqx::post_user_error);
            context.it("put_user", emqx::put_user);
            context.it("put_user with error", emqx::put_user_error);
            context.it("delete_user", emqx::delete_user);
            context.it("delete_user with error", emqx::delete_user_error);
            context.it("post_acl", emqx::post_acl);
            context.it("post_acl with error", emqx::post_acl_error);
            context.it("delete_acl", emqx::delete_acl);
            context.it("delete_acl with error", emqx::delete_acl_error);
            context.it("publish_message", emqx::publish_message);
            context.it("publish_message with error", emqx::publish_message_error);
            context.it("post_topic_metrics", emqx::post_topic_metrics);
            context.it(
                "post_topic_metrics with error",
                emqx::post_topic_metrics_error,
            );
            context.it("delete_topic_metrics", emqx::delete_topic_metrics);
            context.it(
                "delete_topic_metrics with error",
                emqx::delete_topic_metrics_error,
            );
            context.it("stats", emqx::stats);
            context.it("stats with error", emqx::stats_error);
            context.it("add_superuser", emqx::add_superuser);
            context.it("add_superuser with error", emqx::add_superuser_error);

            context.it("scenario", emqx::scenario);

            context.after_each(emqx::after_each_fn);
        });

        context.describe("rumqttd", |context| {
            context.it("start_rumqttd", rumqttd::start_rumqttd);

            context.after_each(rumqttd::after_each_fn);
        });

        context
            .before_all(|state| {
                state.insert(STATE, new_state());
            })
            .after_all(|state| {
                let state = state.get_mut(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();
                runtime.block_on(async {
                    if let Err(e) = emqx::after_del_api_key().await {
                        println!("delete EMQX API key error: {}", e);
                    }
                });
            });
    })
}

/// Test [`mq::to_username`].
pub fn to_username(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let name = mq::to_username(QueueType::Application, "unit", "code");
    expect(name.as_str()).equals("application.unit.code")?;

    let name = mq::to_username(QueueType::Network, "unit", "code");
    expect(name.as_str()).equals("network.unit.code")?;

    let name = mq::to_username(QueueType::Network, "", "code");
    expect(name.as_str()).equals("network._.code")
}

pub fn new_state() -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    let (api_key, api_secret) = match runtime.block_on(async {
        let _ = emqx::after_del_api_key().await;
        emqx::before_add_api_key().await
    }) {
        Err(e) => panic!("create API key error: {}", e),
        Ok(result) => (result.0, result.1),
    };

    let rabbitmq_opts = RabbitMqOpts {
        username: crate::TEST_RABBITMQ_USER.to_string(),
        password: crate::TEST_RABBITMQ_PASS.to_string(),
        ttl: Some(1000),
        length: Some(10),
    };
    let emqx_opts = EmqxOpts {
        api_key,
        api_secret,
    };
    let rumqttd_opts = RumqttdOpts {
        mqtt_port: Some(crate::TEST_RUMQTTD_MQTT_PORT),
        mqtts_port: Some(crate::TEST_RUMQTTD_MQTTS_PORT),
        console_port: Some(crate::TEST_RUMQTTD_CONSOLE_PORT),
    };

    TestState {
        runtime: Some(runtime),
        client: Some(Client::new()),
        mq_opts: Some((rabbitmq_opts, emqx_opts, rumqttd_opts)),
        ..Default::default()
    }
}
