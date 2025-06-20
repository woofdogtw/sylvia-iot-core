use std::collections::HashMap;

use chrono::Utc;
use laboratory::{Suite, describe};
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;

use sylvia_iot_auth::models::Model as AuthModel;
use sylvia_iot_broker::models::Model as BrokerModel;

use super::{
    libs::{
        TOKEN_MANAGER, create_application, create_client, create_device, create_device_route,
        create_dldata_buffer, create_network, create_network_route, create_unit, create_user,
        create_users_tokens, new_state,
    },
    stop_auth_broker_svc,
};
use crate::{TEST_BROKER_BASE, TestState, libs::mq::emqx};

mod application;
mod auth;
mod client;
mod device;
mod device_route;
mod dldata_buffer;
mod network;
mod network_route;
mod unit;
mod user;

#[derive(Deserialize)]
struct Stats {
    #[serde(rename = "consumers")]
    _consumers: usize,
    messages: usize,
    #[serde(rename = "publishRate")]
    publish_rate: f64,
    #[serde(rename = "deliverRate")]
    _deliver_rate: f64,
}

pub const STATE: &'static str = "routes/v1";

pub fn suite(mqtt_engine: &'static str) -> Suite<TestState> {
    describe(format!("routes.v1 - {}", mqtt_engine), move |context| {
        context.describe("pure bridge", |context| {
            context.describe("auth", |context| {
                context.it("GET /tokeninfo", auth::tokeninfo);
                context.it("POST /logout", auth::logout);
            });

            context.describe("user", |context| {
                context.it("GET /user", user::get);
                context.it("PATCH /user", user::patch);
                context.it("POST /user", user::post_admin);
                context.it("GET /user/count", user::get_admin_count);
                context.it("GET /user/list", user::get_admin_list);
                context.it("GET /user/{userId}", user::get_admin);
                context.it("PATCH /user/{userId}", user::patch_admin);
                context.it("DELETE /user/{userId}", user::delete_admin);
            });

            context.describe("client", |context| {
                context.it("POST /client", client::post);
                context.it("GET /client/count", client::get_count);
                context.it("GET /client/list", client::get_list);
                context.it("GET /client/{clientId}", client::get);
                context.it("PATCH /client/{clientId}", client::patch);
                context.it("DELETE /client/{clientId}", client::delete);
                context.it("DELETE /client/user/{userId}", client::delete_user);
            });

            context.describe("application", |context| {
                context.it("GET /application/count", application::get_count);
                context.it("GET /application/list", application::get_list);
            });

            context.describe("device", |context| {
                context.it("POST /device", device::post);
                context.it("POST /device/bulk", device::post_bulk);
                context.it("POST /device/bulk-delete", device::post_bulk_del);
                context.it("POST /device/range", device::post_range);
                context.it("POST /device/range-delete", device::post_range_del);
                context.it("GET /device/count", device::get_count);
                context.it("GET /device/list", device::get_list);
                context.it("GET /device/{deviceId}", device::get);
                context.it("PATCH /device/{deviceId}", device::patch);
                context.it("DELETE /device/{deviceId}", device::delete);
            });

            context.describe("device_route", |context| {
                context.it("POST /device-route", device_route::post);
                context.it("POST /device-route/bulk", device_route::post_bulk);
                context.it(
                    "POST /device-route/bulk-delete",
                    device_route::post_bulk_del,
                );
                context.it("POST /device-route/range", device_route::post_range);
                context.it(
                    "POST /device-route/range-delete",
                    device_route::post_range_del,
                );
                context.it("GET /device-route/count", device_route::get_count);
                context.it("GET /device-route/list", device_route::get_list);
                context.it("GET /device-route/list", device_route::get_list);
                context.it("DELETE /device-route/{routeId}", device_route::delete);
            });

            context.describe("dldata_buffer", |context| {
                context.it("GET /dldata-buffer/count", dldata_buffer::get_count);
                context.it("GET /dldata-buffer/list", dldata_buffer::get_list);
                context.it("DELETE /dldata-buffer/{routeId}", dldata_buffer::delete);
            });

            context.describe("network", |context| {
                context.it("GET /network/count", network::get_count);
                context.it("GET /network/list", network::get_list);
            });

            context.describe("network_route", |context| {
                context.it("POST /network-route", network_route::post);
                context.it("GET /network-route/count", network_route::get_count);
                context.it("GET /network-route/list", network_route::get_list);
                context.it("DELETE /network-route/{routeId}", network_route::delete);
            });

            context.describe("unit", |context| {
                context.it("POST /unit", unit::post);
                context.it("GET /unit/count", unit::get_count);
                context.it("GET /unit/list", unit::get_list);
                context.it("GET /unit/{unitId}", unit::get);
                context.it("PATCH /unit/{unitId}", unit::patch);
            });

            context.before_all(move |state| {
                create_list_rsc(state);
            });
        });

        context.describe("application", |context| {
            context.it("POST /application", application::post);
            context.it(
                "POST /application with invalid parameters",
                application::post_invalid,
            );
            context.it("GET /application/{applicationId}", application::get);
            context.it(
                "GET /application/{applicationId} with invalid parameters",
                application::get_invalid,
            );
            context.it("PATCH /application/{applicationId}", application::patch);
            context.it(
                "PATCH /application/{applicationId} with invalid parameters",
                application::patch_invalid,
            );
            context.it("DELETE /application/{applicationId}", application::delete);
            context.it(
                "DELETE /application/{applicationId} with invalid parameters",
                application::delete_invalid,
            );
            context.it("GET /application/{applicationId}/stats", application::stats);
            context.it(
                "GET /application/{applicationId}/stats with invalid parameters",
                application::stats_invalid,
            );
            context.it(
                "POST /application/{applicationId}/dldata",
                application::dldata,
            );
            context.it(
                "POST /application/{applicationId}/dldata with invalid parameters",
                application::dldata_invalid,
            );

            context.after_each(application::after_each_fn);
        });

        context.describe("network", |context| {
            context.it("POST /network", network::post);
            context.it(
                "POST /network with invalid parameters",
                network::post_invalid,
            );
            context.it("GET /network/{networkId}", network::get);
            context.it(
                "GET /network/{networkId} with invalid parameters",
                network::get_invalid,
            );
            context.it("PATCH /network/{networkId}", network::patch);
            context.it(
                "PATCH /network/{networkId} with invalid parameters",
                network::patch_invalid,
            );
            context.it("DELETE /network/{networkId}", network::delete);
            context.it(
                "DELETE /network/{networkId} with invalid parameters",
                network::delete_invalid,
            );
            context.it("GET /network/{networkId}/stats", network::stats);
            context.it(
                "GET /network/{networkId}/stats with invalid parameters",
                network::stats_invalid,
            );
            context.it("POST /network/{networkId}/uldata", network::uldata);
            context.it(
                "POST /network/{networkId}/uldata with invalid parameters",
                network::uldata_invalid,
            );

            context.after_each(network::after_each_fn);
        });

        context.describe("unit", |context| {
            context.it("DELETE /unit/{unitId}", unit::delete);
            context.it(
                "DELETE /unit/{unitId} with invalid parameters",
                unit::delete_invalid,
            );

            context.before_all(move |state| {
                unit::create_delete_rsc(state);
            });
        });

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(mqtt_engine), None));

                let state = state.get(STATE).unwrap();
                create_users_tokens(state);
            })
            .after_all(after_all_fn);
    })
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let _ = state.rumqttd_handles.take();
    let runtime = state.runtime.as_ref().unwrap();

    if let Err(e) = runtime.block_on(async { emqx::after_del_api_key().await }) {
        println!("delete EMQX API key error: {}", e);
    }

    stop_auth_broker_svc(state);
}

fn create_list_rsc(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_db = state.auth_db.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();

    let now = Utc::now();
    let role = HashMap::<String, bool>::new();
    let user_id = "manager";

    for i in 100..201 {
        let name = format!("list{}", i);
        let name = name.as_str();
        let item = create_user(name, now, role.clone());
        if let Err(e) = runtime.block_on(async { auth_db.user().add(&item).await }) {
            panic!("create list user error: {}", e);
        }
        let item = create_client(name, user_id, None);
        if let Err(e) = runtime.block_on(async { auth_db.client().add(&item).await }) {
            panic!("create list client error: {}", e);
        }
        let item = create_unit(name, user_id);
        if let Err(e) = runtime.block_on(async { broker_db.unit().add(&item).await }) {
            panic!("create list unit error: {}", e);
        }
        let item = create_application(name, "mqtt://localhost", name);
        if let Err(e) = runtime.block_on(async { broker_db.application().add(&item).await }) {
            panic!("create list application error: {}", e);
        }
        let item = create_network(name, "mqtt://localhost", name);
        if let Err(e) = runtime.block_on(async { broker_db.network().add(&item).await }) {
            panic!("create list network error: {}", e);
        }
        let item = create_device(name, name, name, false);
        if let Err(e) = runtime.block_on(async { broker_db.device().add(&item).await }) {
            panic!("create list device error: {}", e);
        }
        let item = create_device_route(name, name, name, name, name);
        if let Err(e) = runtime.block_on(async { broker_db.device_route().add(&item).await }) {
            panic!("create list device route error: {}", e);
        }
        let item = create_network_route(name, name, name, name);
        if let Err(e) = runtime.block_on(async { broker_db.network_route().add(&item).await }) {
            panic!("create list network route error: {}", e);
        }
        let item = create_dldata_buffer(name, name, name, name, name);
        if let Err(e) = runtime.block_on(async { broker_db.dldata_buffer().add(&item).await }) {
            panic!("create list dldata buffer error: {}", e);
        }
    }
}

async fn remove_unit(client: &Client, unit_id: &str) -> Result<(), String> {
    let uri = format!("{}/api/v1/unit/{}", TEST_BROKER_BASE, unit_id);
    let req = match client
        .request(Method::DELETE, uri.as_str())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", TOKEN_MANAGER),
        )
        .build()
    {
        Err(e) => return Err(format!("generate request error: {}", e)),
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => return Err(format!("execute request error: {}", e)),
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT => Ok(()),
            _ => Err(format!("execute request with status {}", resp.status())),
        },
    }
}

async fn remove_network(client: &Client, network_id: &str) -> Result<(), String> {
    let uri = format!("{}/api/v1/network/{}", TEST_BROKER_BASE, network_id);
    let req = match client
        .request(Method::DELETE, uri.as_str())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", TOKEN_MANAGER),
        )
        .build()
    {
        Err(e) => return Err(format!("generate request error: {}", e)),
        Ok(req) => req,
    };
    match client.execute(req).await {
        Err(e) => return Err(format!("execute request error: {}", e)),
        Ok(resp) => match resp.status() {
            StatusCode::NO_CONTENT => Ok(()),
            _ => Err(format!("execute request with status {}", resp.status())),
        },
    }
}
