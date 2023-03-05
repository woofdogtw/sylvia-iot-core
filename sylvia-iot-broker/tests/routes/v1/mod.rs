use std::collections::HashMap;

use chrono::Utc;
use laboratory::{describe, Suite};

use sylvia_iot_auth::models::Model;
use sylvia_iot_corelib::role::Role;

use super::{
    clear_state,
    libs::{create_client, create_token, create_user, new_state},
    remove_rabbitmq_queues, remove_sqlite, stop_auth_svc,
};
use crate::TestState;

mod application;
mod control;
mod data;
mod device;
mod device_route;
mod dldata_buffer;
mod libs;
mod network;
mod network_route;
mod routing;
mod unit;

use application::api as applicationapi;
use device::api as deviceapi;
use device_route::api as devicerouteapi;
use dldata_buffer::api as dldatabufferapi;
use network::api as networkapi;
use network_route::api as networkrouteapi;
use unit::api as unitapi;

pub const STATE: &'static str = "routes/v1";
const TOKEN_MANAGER: &'static str = "TOKEN_MANAGER";
const TOKEN_OWNER: &'static str = "TOKEN_OWNER";
const TOKEN_MEMBER: &'static str = "TOKEN_MEMBER";

pub fn suite(db_engine: &'static str, cache_engine: &'static str) -> Suite<TestState> {
    let suite_name = format!("routes.v1 - {}/{}", db_engine, cache_engine);
    describe(suite_name, move |context| {
        context.describe("application", |context| {
            context.it("POST /application", applicationapi::post);
            context.it(
                "POST /application with duplicate code",
                applicationapi::post_dup,
            );
            context.it(
                "POST /application with not exist unit",
                applicationapi::post_not_exist_unit,
            );
            context.it(
                "POST /application with invalid parameters",
                applicationapi::post_invalid_param,
            );
            context.it(
                "POST /application with invalid token",
                applicationapi::post_invalid_token,
            );
            context.it("GET /application/count", applicationapi::get_count);
            context.it(
                "GET /application/count with not exist unit",
                applicationapi::get_count_not_exist_unit,
            );
            context.it(
                "GET /application/count with invalid parameters",
                applicationapi::get_count_invalid_param,
            );
            context.it(
                "GET /application/count with invalid token",
                applicationapi::get_count_invalid_token,
            );
            context.it("GET /application/list", applicationapi::get_list);
            context.it("GET /application/list sort", applicationapi::get_list_sort);
            context.it(
                "GET /application/list offset limit",
                applicationapi::get_list_offset_limit,
            );
            context.it(
                "GET /application/list array format",
                applicationapi::get_list_format_array,
            );
            context.it(
                "GET /application/list with not exist unit",
                applicationapi::get_list_not_exist_unit,
            );
            context.it(
                "GET /application/list with invalid parameters",
                applicationapi::get_list_invalid_param,
            );
            context.it(
                "GET /application/list with invalid token",
                applicationapi::get_list_invalid_token,
            );
            context.it("GET /application/{applicationId}", applicationapi::get);
            context.it(
                "GET /application/{applicationId} with wrong ID",
                applicationapi::get_wrong_id,
            );
            context.it(
                "GET /application/{applicationId} with invalid token",
                applicationapi::get_invalid_token,
            );
            context.it("PATCH /application/{applicationId}", applicationapi::patch);
            context.it(
                "PATCH /application/{applicationId} with wrong ID",
                applicationapi::patch_wrong_id,
            );
            context.it(
                "PATCH /application/{applicationId} with invalid parameters",
                applicationapi::patch_invalid_param,
            );
            context.it(
                "PATCH /application/{applicationId} with invalid token",
                applicationapi::patch_invalid_token,
            );
            context.it(
                "DELETE /application/{applicationId}",
                applicationapi::delete,
            );
            context.it(
                "DELETE /application/{applicationId} with invalid token",
                applicationapi::delete_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context.describe("device", |context| {
            context.it("POST /device", deviceapi::post);
            context.it("POST /device with duplicate address", deviceapi::post_dup);
            context.it(
                "POST /device with not exist unit or network",
                deviceapi::post_not_exist,
            );
            context.it(
                "POST /device with invalid parameters",
                deviceapi::post_invalid_param,
            );
            context.it(
                "POST /device with invalid token",
                deviceapi::post_invalid_token,
            );
            context.it("POST /device/bulk", deviceapi::post_bulk);
            context.it(
                "POST /device/bulk with not exist unit or network",
                deviceapi::post_bulk_not_exist,
            );
            context.it(
                "POST /device/bulk with invalid parameters",
                deviceapi::post_bulk_invalid_param,
            );
            context.it(
                "POST /device/bulk with invalid token",
                deviceapi::post_bulk_invalid_token,
            );
            context.it("POST /device/bulk-delete", deviceapi::post_bulk_del);
            context.it(
                "POST /device/bulk-delete with not exist unit or network",
                deviceapi::post_bulk_del_not_exist,
            );
            context.it(
                "POST /device/bulk-delete with invalid parameters",
                deviceapi::post_bulk_del_invalid_param,
            );
            context.it(
                "POST /device/bulk-delete with invalid token",
                deviceapi::post_bulk_del_invalid_token,
            );
            context.it("POST /device/range", deviceapi::post_range);
            context.it(
                "POST /device/range with not exist unit or network",
                deviceapi::post_range_not_exist,
            );
            context.it(
                "POST /device/range with invalid parameters",
                deviceapi::post_range_invalid_param,
            );
            context.it(
                "POST /device/range with invalid token",
                deviceapi::post_range_invalid_token,
            );
            context.it("POST /device/range-delete", deviceapi::post_range_del);
            context.it(
                "POST /device/range-delete with not exist unit or network",
                deviceapi::post_range_del_not_exist,
            );
            context.it(
                "POST /device/range-delete with invalid parameters",
                deviceapi::post_range_del_invalid_param,
            );
            context.it(
                "POST /device/range-delete with invalid token",
                deviceapi::post_range_del_invalid_token,
            );
            context.it("GET /device/count", deviceapi::get_count);
            context.it(
                "GET /device/count with not exist unit",
                deviceapi::get_count_not_exist_unit,
            );
            context.it(
                "GET /device/count with invalid parameters",
                deviceapi::get_count_invalid_param,
            );
            context.it(
                "GET /device/count with invalid token",
                deviceapi::get_count_invalid_token,
            );
            context.it("GET /device/list", deviceapi::get_list);
            context.it("GET /device/list sort", deviceapi::get_list_sort);
            context.it(
                "GET /device/list offset limit",
                deviceapi::get_list_offset_limit,
            );
            context.it(
                "GET /device/list array format",
                deviceapi::get_list_format_array,
            );
            context.it(
                "GET /device/list with not exist unit",
                deviceapi::get_list_not_exist_unit,
            );
            context.it(
                "GET /device/list with invalid parameters",
                deviceapi::get_list_invalid_param,
            );
            context.it(
                "GET /device/list with invalid token",
                deviceapi::get_list_invalid_token,
            );
            context.it("GET /device/{deviceId}", deviceapi::get);
            context.it(
                "GET /device/{deviceId} with wrong ID",
                deviceapi::get_wrong_id,
            );
            context.it(
                "GET /device/{deviceId} with invalid token",
                deviceapi::get_invalid_token,
            );
            context.it("PATCH /device/{deviceId}", deviceapi::patch);
            context.it(
                "PATCH /device/{deviceId} with wrong ID",
                deviceapi::patch_wrong_id,
            );
            context.it(
                "PATCH /device/{deviceId} with invalid parameters",
                deviceapi::patch_invalid_param,
            );
            context.it(
                "PATCH /device/{deviceId} with invalid token",
                deviceapi::patch_invalid_token,
            );
            context.it("DELETE /device/{deviceId}", deviceapi::delete);
            context.it(
                "DELETE /device/{deviceId} with invalid token",
                deviceapi::delete_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context.describe("device_route", |context| {
            context.it("POST /device-route", devicerouteapi::post);
            context.it(
                "POST /device-route with duplicate route",
                devicerouteapi::post_dup,
            );
            context.it(
                "POST /device-route with not exist device or application",
                devicerouteapi::post_not_exist,
            );
            context.it(
                "POST /device-route with not match unit",
                devicerouteapi::post_not_match_unit,
            );
            context.it(
                "POST /device-route with invalid parameters",
                devicerouteapi::post_invalid_param,
            );
            context.it(
                "POST /device-route with invalid token",
                devicerouteapi::post_invalid_token,
            );
            context.it("POST /device-route/bulk", devicerouteapi::post_bulk);
            context.it(
                "POST /device-route/bulk with not exist application/network/device",
                devicerouteapi::post_bulk_not_exist,
            );
            context.it(
                "POST /device-rout/bulk with invalid parameters",
                devicerouteapi::post_bulk_invalid_param,
            );
            context.it(
                "POST /device-route/bulk with invalid token",
                devicerouteapi::post_bulk_invalid_token,
            );
            context.it(
                "POST /device-route/bulk-delete",
                devicerouteapi::post_bulk_del,
            );
            context.it(
                "POST /device-route/bulk-delete with not exist application/network",
                devicerouteapi::post_bulk_del_not_exist,
            );
            context.it(
                "POST /device-rout/bulk-delete with invalid parameters",
                devicerouteapi::post_bulk_del_invalid_param,
            );
            context.it(
                "POST /device-route/bulk-delete with invalid token",
                devicerouteapi::post_bulk_del_invalid_token,
            );
            context.it("POST /device-route/range", devicerouteapi::post_range);
            context.it(
                "POST /device-route/range with not exist application/network/device",
                devicerouteapi::post_range_not_exist,
            );
            context.it(
                "POST /device-rout/range with invalid parameters",
                devicerouteapi::post_range_invalid_param,
            );
            context.it(
                "POST /device-route/range with invalid token",
                devicerouteapi::post_range_invalid_token,
            );
            context.it(
                "POST /device-route/range-delete",
                devicerouteapi::post_range_del,
            );
            context.it(
                "POST /device-route/range-delete with not exist application/network",
                devicerouteapi::post_range_del_not_exist,
            );
            context.it(
                "POST /device-rout/range-delete with invalid parameters",
                devicerouteapi::post_range_del_invalid_param,
            );
            context.it(
                "POST /device-route/range-delete with invalid token",
                devicerouteapi::post_range_del_invalid_token,
            );
            context.it("GET /device-route/count", devicerouteapi::get_count);
            context.it(
                "GET /device-route/count with not exist unit",
                devicerouteapi::get_count_not_exist_unit,
            );
            context.it(
                "GET /device-route/count with invalid parameters",
                devicerouteapi::get_count_invalid_param,
            );
            context.it(
                "GET /device-route/count with invalid token",
                devicerouteapi::get_count_invalid_token,
            );
            context.it("GET /device-route/list", devicerouteapi::get_list);
            context.it("GET /device-route/list sort", devicerouteapi::get_list_sort);
            context.it(
                "GET /device-route/list offset limit",
                devicerouteapi::get_list_offset_limit,
            );
            context.it(
                "GET /device-route/list array format",
                devicerouteapi::get_list_format_array,
            );
            context.it(
                "GET /device-route/list with not exist unit",
                devicerouteapi::get_list_not_exist_unit,
            );
            context.it(
                "GET /device-route/list with invalid parameters",
                devicerouteapi::get_list_invalid_param,
            );
            context.it(
                "GET /device-route/list with invalid token",
                devicerouteapi::get_list_invalid_token,
            );
            context.it("DELETE /device-route/{routeId}", devicerouteapi::delete);
            context.it(
                "DELETE /device-route/{routeId} with invalid token",
                devicerouteapi::delete_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context.describe("dldata_buffer", |context| {
            context.it("GET /dldata-buffer/count", dldatabufferapi::get_count);
            context.it(
                "GET /dldata-buffer/count with not exist unit",
                dldatabufferapi::get_count_not_exist_unit,
            );
            context.it(
                "GET /dldata-buffer/count with invalid parameters",
                dldatabufferapi::get_count_invalid_param,
            );
            context.it(
                "GET /dldata-buffer/count with invalid token",
                dldatabufferapi::get_count_invalid_token,
            );
            context.it("GET /dldata-buffer/list", dldatabufferapi::get_list);
            context.it(
                "GET /dldata-buffer/list sort",
                dldatabufferapi::get_list_sort,
            );
            context.it(
                "GET /dldata-buffer/list offset limit",
                dldatabufferapi::get_list_offset_limit,
            );
            context.it(
                "GET /dldata-buffer/list array format",
                dldatabufferapi::get_list_format_array,
            );
            context.it(
                "GET /dldata-buffer/list with not exist unit",
                dldatabufferapi::get_list_not_exist_unit,
            );
            context.it(
                "GET /dldata-buffer/list with invalid parameters",
                dldatabufferapi::get_list_invalid_param,
            );
            context.it(
                "GET /dldata-buffer/list with invalid token",
                dldatabufferapi::get_list_invalid_token,
            );
            context.it("DELETE /dldata-buffer/{dataId}", dldatabufferapi::delete);
            context.it(
                "DELETE /dldata-buffer/{dataId} with invalid token",
                dldatabufferapi::delete_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context.describe("network", |context| {
            context.it("POST /network", networkapi::post);
            context.it("POST /network with duplicate code", networkapi::post_dup);
            context.it(
                "POST /network with not exist unit",
                networkapi::post_not_exist_unit,
            );
            context.it(
                "POST /network with invalid parameters",
                networkapi::post_invalid_param,
            );
            context.it(
                "POST /network with invalid token",
                networkapi::post_invalid_token,
            );
            context.it("GET /network/count", networkapi::get_count);
            context.it(
                "GET /network/count with not exist unit",
                networkapi::get_count_not_exist_unit,
            );
            context.it(
                "GET /network/count with invalid parameters",
                networkapi::get_count_invalid_param,
            );
            context.it(
                "GET /network/count with invalid token",
                networkapi::get_count_invalid_token,
            );
            context.it("GET /network/list", networkapi::get_list);
            context.it("GET /network/list sort", networkapi::get_list_sort);
            context.it(
                "GET /network/list offset limit",
                networkapi::get_list_offset_limit,
            );
            context.it(
                "GET /network/list array format",
                networkapi::get_list_format_array,
            );
            context.it(
                "GET /network/list with not exist unit",
                networkapi::get_list_not_exist_unit,
            );
            context.it(
                "GET /network/list with invalid parameters",
                networkapi::get_list_invalid_param,
            );
            context.it(
                "GET /network/list with invalid token",
                networkapi::get_list_invalid_token,
            );
            context.it("GET /network/{networkId}", networkapi::get);
            context.it(
                "GET /network/{networkId} with wrong ID",
                networkapi::get_wrong_id,
            );
            context.it(
                "GET /network/{networkId} with invalid token",
                networkapi::get_invalid_token,
            );
            context.it("PATCH /network/{networkId}", networkapi::patch);
            context.it(
                "PATCH /network/{networkId} with wrong ID",
                networkapi::patch_wrong_id,
            );
            context.it(
                "PATCH /network/{networkId} with invalid parameters",
                networkapi::patch_invalid_param,
            );
            context.it(
                "PATCH /network/{networkId} with invalid token",
                networkapi::patch_invalid_token,
            );
            context.it("DELETE /network/{networkId}", networkapi::delete);
            context.it(
                "DELETE /network/{networkId} with invalid token",
                networkapi::delete_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context.describe("network_route", |context| {
            context.it("POST /network-route", networkrouteapi::post);
            context.it(
                "POST /network-route with duplicate route",
                networkrouteapi::post_dup,
            );
            context.it(
                "POST /network-route with not exist network or application",
                networkrouteapi::post_not_exist,
            );
            context.it(
                "POST /network-route with not match unit",
                networkrouteapi::post_not_match_unit,
            );
            context.it(
                "POST /network-route with invalid parameters",
                networkrouteapi::post_invalid_param,
            );
            context.it(
                "POST /network-route with invalid token",
                networkrouteapi::post_invalid_token,
            );
            context.it("GET /network-route/count", networkrouteapi::get_count);
            context.it(
                "GET /network-route/count with not exist unit",
                networkrouteapi::get_count_not_exist_unit,
            );
            context.it(
                "GET /network-route/count with invalid parameters",
                networkrouteapi::get_count_invalid_param,
            );
            context.it(
                "GET /network-route/count with invalid token",
                networkrouteapi::get_count_invalid_token,
            );
            context.it("GET /network-route/list", networkrouteapi::get_list);
            context.it(
                "GET /network-route/list sort",
                networkrouteapi::get_list_sort,
            );
            context.it(
                "GET /network-route/list offset limit",
                networkrouteapi::get_list_offset_limit,
            );
            context.it(
                "GET /network-route/list array format",
                networkrouteapi::get_list_format_array,
            );
            context.it(
                "GET /network-route/list with not exist unit",
                networkrouteapi::get_list_not_exist_unit,
            );
            context.it(
                "GET /network-route/list with invalid parameters",
                networkrouteapi::get_list_invalid_param,
            );
            context.it(
                "GET /network-route/list with invalid token",
                networkrouteapi::get_list_invalid_token,
            );
            context.it("DELETE /network-route/{routeId}", networkrouteapi::delete);
            context.it(
                "DELETE /network-route/{routeId} with invalid token",
                networkrouteapi::delete_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context.describe("unit", |context| {
            context.it("POST /unit", unitapi::post);
            context.it("POST /unit with duplicate code", unitapi::post_dup);
            context.it(
                "POST /unit with not exist user",
                unitapi::post_not_exist_user,
            );
            context.it(
                "POST /unit with invalid parameters",
                unitapi::post_invalid_param,
            );
            context.it("POST /unit with invalid token", unitapi::post_invalid_token);
            context.it("GET /unit/count", unitapi::get_count);
            context.it(
                "GET /unit/count with invalid token",
                unitapi::get_count_invalid_token,
            );
            context.it("GET /unit/list", unitapi::get_list);
            context.it("GET /unit/list sort", unitapi::get_list_sort);
            context.it(
                "GET /unit/list offset limit",
                unitapi::get_list_offset_limit,
            );
            context.it(
                "GET /unit/list array format",
                unitapi::get_list_format_array,
            );
            context.it(
                "GET /unit/list with invalid parameters",
                unitapi::get_list_invalid_param,
            );
            context.it(
                "GET /unit/list with invalid token",
                unitapi::get_list_invalid_token,
            );
            context.it("GET /unit/{unitId}", unitapi::get);
            context.it("GET /unit/{unitId} with wrong ID", unitapi::get_wrong_id);
            context.it(
                "GET /unit/{unitId} with invalid token",
                unitapi::get_invalid_token,
            );
            context.it("PATCH /unit/{unitId}", unitapi::patch);
            context.it(
                "PATCH /unit/{unitId} with wrong ID",
                unitapi::patch_wrong_id,
            );
            context.it(
                "PATCH /unit/{unitId} with not exist user",
                unitapi::patch_not_exist_user,
            );
            context.it(
                "PATCH /unit/{unitId} with invalid parameters",
                unitapi::patch_invalid_param,
            );
            context.it(
                "PATCH /unit/{unitId} with invalid token",
                unitapi::patch_invalid_token,
            );
            context.it("DELETE /unit/{unitId}", unitapi::delete);
            context.it(
                "DELETE /unit/{unitId} with invalid token",
                unitapi::delete_invalid_token,
            );
            context.it("DELETE /unit/user/{userId}", unitapi::delete_user);
            context.it(
                "DELETE /unit/user/{userId} with invalid token",
                unitapi::delete_user_invalid_token,
            );
            context.it(
                "DELETE /unit/user/{userId} with invalid permission",
                unitapi::delete_user_invalid_perm,
            );

            context.after_each(after_each_fn);
        });

        context.describe("route integration", |context| {
            context.it("uplink", routing::uplink);
            context.it("downlink", routing::downlink);
            context.it("downlink with not exist", routing::downlink_not_exist);
            context.it(
                "uplink with off/on/off routes",
                routing::uplink_route_on_off,
            );

            context
                .before_all(routing::before_all_fn)
                .after_all(routing::after_all_fn)
                .after_each(routing::after_each_fn);
        });

        context.describe("control channel", |context| {
            context.it("wrong payload format", control::test_wrong_data);

            context.after_each(control::after_each_fn);
        });

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(db_engine), Some(cache_engine), None));
                create_users_tokens(state);
            })
            .after_all(after_all_fn);
    })
}

pub fn suite_data(
    db_engine: &'static str,
    cache_engine: &'static str,
    data_host: &'static str,
) -> Suite<TestState> {
    let suite_name = format!(
        "routes.v1 - data channel - {}/{}/{}",
        db_engine, cache_engine, data_host
    );
    describe(suite_name, move |context| {
        context.describe("data channel", |context| {
            context.it("uplink", data::uplink);
            context.it("downlink", data::downlink);

            context
                .before_all(data::before_all_fn)
                .after_all(data::after_all_fn)
                .after_each(data::after_each_fn);
        });

        context
            .before_all(move |state| {
                state.insert(
                    STATE,
                    new_state(Some(db_engine), Some(cache_engine), Some(data_host)),
                );
                create_users_tokens(state);
            })
            .after_all(after_all_fn);
    })
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(state) = state.routes_state.as_mut() {
        runtime.block_on(async {
            clear_state(state).await;
        });
    }

    stop_auth_svc(state);

    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            if let Err(e) = model.get_connection().drop(None).await {
                println!("remove mongodb database error: {}", e);
            }
        })
    }
    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
    remove_rabbitmq_queues(state);
}

fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    libs::clear_all_data(runtime, state);
}

fn create_users_tokens(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let auth_db = state.auth_db.as_ref().unwrap();

    let now = Utc::now();

    let mut roles = HashMap::<String, bool>::new();
    roles.insert(Role::MANAGER.to_string(), true);
    let user = create_user("manager", now, roles);
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create manager error: {}", e);
    }

    let user = create_user("owner", now, HashMap::<String, bool>::new());
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create owner error: {}", e);
    }

    let user = create_user("member", now, HashMap::<String, bool>::new());
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create member error: {}", e);
    }

    let client = create_client("client", "manager", None);
    if let Err(e) = runtime.block_on(async { auth_db.client().add(&client).await }) {
        panic!("create client error: {}", e);
    }

    let token = create_token(TOKEN_MANAGER, "manager", "client");
    if let Err(e) = runtime.block_on(async { auth_db.access_token().add(&token).await }) {
        panic!("create manager token error: {}", e);
    }

    let token = create_token(TOKEN_OWNER, "owner", "client");
    if let Err(e) = runtime.block_on(async { auth_db.access_token().add(&token).await }) {
        panic!("create owner token error: {}", e);
    }

    let token = create_token(TOKEN_MEMBER, "member", "client");
    if let Err(e) = runtime.block_on(async { auth_db.access_token().add(&token).await }) {
        panic!("create member token error: {}", e);
    }
}
