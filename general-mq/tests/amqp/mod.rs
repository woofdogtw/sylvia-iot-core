use laboratory::{Suite, describe};
use reqwest::{self, Method, StatusCode};
use serde::Deserialize;

use crate::{STATE, TestState, clear_state, new_state};

mod connection;
mod queue;

#[derive(Deserialize)]
struct QueueInfo {
    name: String,
}

pub fn suite() -> Suite<TestState> {
    describe("amqp", |context| {
        context.describe("AmqpConnection", |context| {
            context.it("new() with default", connection::new_default);
            context.it("new() with zero", connection::new_zero);
            context.it("new() with wrong opts", connection::new_wrong_opts);

            context.it("status()", connection::properties);

            context.it("connect() without handler", connection::connect_no_handler);
            context.it("connect() with handler", connection::connect_with_handler);
            context.it(
                "connect() after connect()",
                connection::connect_after_connect,
            );

            context.it("remove_handler()", connection::remove_handler);

            context.it("close()", connection::close);
            context.it("close() after close()", connection::close_after_close);

            context.after_each(clear_state);
        });

        context.describe("AmqpQueue", |context| {
            context.it("new() with default", queue::new_default);
            context.it("new() with wrong opts", queue::new_wrong_opts);

            context.it("name(), is_recv(), status()", queue::properties);

            context.it("connect() without handler", queue::connect_no_handler);
            context.it("connect() with handler", queue::connect_with_handler);
            context.it("connect() after connect()", queue::connect_after_connect);

            context.it("clear_handler()", queue::clear_handler);

            context.it("close()", queue::close);
            context.it("close() after close()", queue::close_after_close);

            context.it("send_msg() with error conditions", queue::send_error);

            context.after_each(clear_state);
        });

        context.describe("General Queue", |context| {
            context.it("new() with default", queue::mq_new_default);
            context.it("new() with wrong opts", queue::mq_new_wrong_opts);

            context.it("name(), is_recv(), status()", queue::mq_properties);

            context.it("connect() without handler", queue::mq_connect_no_handler);
            context.it("connect() with handler", queue::mq_connect_with_handler);
            context.it("connect() after connect()", queue::mq_connect_after_connect);

            context.it("clear_handler()", queue::mq_clear_handler);

            context.it("close()", queue::mq_close);
            context.it("close() after close()", queue::mq_close_after_close);

            context.it("send_msg() with error conditions", queue::mq_send_error);

            context.after_each(clear_state);
        });

        context.describe("Scenarios", |context| {
            context.it("reconnect", queue::reconnect);

            context.it("unicast 1 to 1", queue::data_unicast_1to1);
            context.it("unicast 1 to 3", queue::data_unicast_1to3);

            context.it("broadcast 1 to 1", queue::data_broadcast_1to1);
            context.it("broadcast 1 to 3", queue::data_broadcast_1to3);

            context.it("reliable", queue::data_reliable);
            context.it("best effort", queue::data_best_effort);

            context.it("persistent", queue::data_persistent);
            context.it("nack", queue::data_nack);

            context.after_each(clear_state);
        });

        context
            .before_all(|state| {
                state.insert(STATE, new_state());
            })
            .after_all(|state| {
                let state = state.get(STATE).unwrap();
                remove_rabbitmq_queues(state);
            });
    })
}

fn remove_rabbitmq_queues(state: &TestState) {
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
    if let Err(e) = state.runtime.block_on(async {
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
