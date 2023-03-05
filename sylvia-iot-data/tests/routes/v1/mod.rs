use std::collections::HashMap;

use chrono::Utc;
use laboratory::{describe, Suite};

use sylvia_iot_auth::models::Model as AuthModel;
use sylvia_iot_broker::models::Model as BrokerModel;
use sylvia_iot_corelib::role::Role;

use super::{
    clear_state,
    libs::{create_client, create_token, create_unit, create_user, new_state},
    remove_sqlite, stop_auth_broker_svc,
};
use crate::TestState;

mod application_dldata;
mod application_uldata;
mod coremgr_opdata;
mod libs;
mod network_dldata;
mod network_uldata;

use application_dldata::api as application_dldata_api;
use application_uldata::api as application_uldata_api;
use coremgr_opdata::api as coremgr_opdata_api;
use network_dldata::api as network_dldata_api;
use network_uldata::api as network_uldata_api;

pub const STATE: &'static str = "routes/v1";
const TOKEN_MANAGER: &'static str = "TOKEN_MANAGER";
const TOKEN_OWNER: &'static str = "TOKEN_OWNER";
const TOKEN_MEMBER: &'static str = "TOKEN_MEMBER";
const UNIT_OWNER: &'static str = "owner";

pub fn suite(db_engine: &'static str) -> Suite<TestState> {
    let suite_name = format!("routes.v1 - {}", db_engine);
    describe(suite_name, move |context| {
        context.describe("application_dldata", |context| {
            context.it(
                "GET /application-dldata/count",
                application_dldata_api::get_count,
            );
            context.it(
                "GET /application-dldata/count with not exist unit",
                application_dldata_api::get_count_not_exist_unit,
            );
            context.it(
                "GET /application-dldata/count with invalid parameters",
                application_dldata_api::get_count_invalid_param,
            );
            context.it(
                "GET /application-dldata/count with invalid token",
                application_dldata_api::get_count_invalid_token,
            );
            context.it(
                "GET /application-dldata/list",
                application_dldata_api::get_list,
            );
            context.it(
                "GET /application-dldata/list sort",
                application_dldata_api::get_list_sort,
            );
            context.it(
                "GET /application-dldata/list offset limit",
                application_dldata_api::get_list_offset_limit,
            );
            context.it(
                "GET /application-dldata/list array/CSV format",
                application_dldata_api::get_list_format_array_csv,
            );
            context.it(
                "GET /application-dldata/list with not exist unit",
                application_dldata_api::get_list_not_exist_unit,
            );
            context.it(
                "GET /application-dldata/list with invalid parameters",
                application_dldata_api::get_list_invalid_param,
            );
            context.it(
                "GET /application-dldata/list with invalid token",
                application_dldata_api::get_list_invalid_token,
            );

            context.after_each(after_each_fn);
        });
        context.describe("application_uldata", |context| {
            context.it(
                "GET /application-uldata/count",
                application_uldata_api::get_count,
            );
            context.it(
                "GET /application-uldata/count with not exist unit",
                application_uldata_api::get_count_not_exist_unit,
            );
            context.it(
                "GET /application-uldata/count with invalid parameters",
                application_uldata_api::get_count_invalid_param,
            );
            context.it(
                "GET /application-uldata/count with invalid token",
                application_uldata_api::get_count_invalid_token,
            );
            context.it(
                "GET /application-uldata/list",
                application_uldata_api::get_list,
            );
            context.it(
                "GET /application-uldata/list sort",
                application_uldata_api::get_list_sort,
            );
            context.it(
                "GET /application-uldata/list offset limit",
                application_uldata_api::get_list_offset_limit,
            );
            context.it(
                "GET /application-uldata/list array/CSV format",
                application_uldata_api::get_list_format_array_csv,
            );
            context.it(
                "GET /application-uldata/list with not exist unit",
                application_uldata_api::get_list_not_exist_unit,
            );
            context.it(
                "GET /application-uldata/list with invalid parameters",
                application_uldata_api::get_list_invalid_param,
            );
            context.it(
                "GET /application-uldata/list with invalid token",
                application_uldata_api::get_list_invalid_token,
            );

            context.after_each(after_each_fn);
        });
        context.describe("coremgr_opdata", |context| {
            context.it("GET /coremgr-opdata/count", coremgr_opdata_api::get_count);
            context.it(
                "GET /coremgr-opdata/count with not exist user",
                coremgr_opdata_api::get_count_not_exist_user,
            );
            context.it(
                "GET /coremgr-opdata/count with invalid parameters",
                coremgr_opdata_api::get_count_invalid_param,
            );
            context.it(
                "GET /coremgr-opdata/count with invalid token",
                coremgr_opdata_api::get_count_invalid_token,
            );
            context.it("GET /coremgr-opdata/list", coremgr_opdata_api::get_list);
            context.it(
                "GET /coremgr-opdata/list sort",
                coremgr_opdata_api::get_list_sort,
            );
            context.it(
                "GET /coremgr-opdata/list offset limit",
                coremgr_opdata_api::get_list_offset_limit,
            );
            context.it(
                "GET /coremgr-opdata/list array/CSV format",
                coremgr_opdata_api::get_list_format_array_csv,
            );
            context.it(
                "GET /coremgr-opdata/list with not exist user",
                coremgr_opdata_api::get_list_not_exist_user,
            );
            context.it(
                "GET /coremgr-opdata/list with invalid parameters",
                coremgr_opdata_api::get_list_invalid_param,
            );
            context.it(
                "GET /coremgr-opdata/list with invalid token",
                coremgr_opdata_api::get_list_invalid_token,
            );

            context.after_each(after_each_fn);
        });
        context.describe("network_dldata", |context| {
            context.it("GET /network-dldata/count", network_dldata_api::get_count);
            context.it(
                "GET /network-dldata/count with not exist unit",
                network_dldata_api::get_count_not_exist_unit,
            );
            context.it(
                "GET /network-dldata/count with invalid parameters",
                network_dldata_api::get_count_invalid_param,
            );
            context.it(
                "GET /network-dldata/count with invalid token",
                network_dldata_api::get_count_invalid_token,
            );
            context.it("GET /network-dldata/list", network_dldata_api::get_list);
            context.it(
                "GET /network-dldata/list sort",
                network_dldata_api::get_list_sort,
            );
            context.it(
                "GET /network-dldata/list offset limit",
                network_dldata_api::get_list_offset_limit,
            );
            context.it(
                "GET /network-dldata/list array/CSV format",
                network_dldata_api::get_list_format_array_csv,
            );
            context.it(
                "GET /network-dldata/list with not exist unit",
                network_dldata_api::get_list_not_exist_unit,
            );
            context.it(
                "GET /network-dldata/list with invalid parameters",
                network_dldata_api::get_list_invalid_param,
            );
            context.it(
                "GET /network-dldata/list with invalid token",
                network_dldata_api::get_list_invalid_token,
            );

            context.after_each(after_each_fn);
        });
        context.describe("network_uldata", |context| {
            context.it("GET /network-uldata/count", network_uldata_api::get_count);
            context.it(
                "GET /network-uldata/count with not exist unit",
                network_uldata_api::get_count_not_exist_unit,
            );
            context.it(
                "GET /network-uldata/count with invalid parameters",
                network_uldata_api::get_count_invalid_param,
            );
            context.it(
                "GET /network-uldata/count with invalid token",
                network_uldata_api::get_count_invalid_token,
            );
            context.it("GET /network-uldata/list", network_uldata_api::get_list);
            context.it(
                "GET /network-uldata/list sort",
                network_uldata_api::get_list_sort,
            );
            context.it(
                "GET /network-uldata/list offset limit",
                network_uldata_api::get_list_offset_limit,
            );
            context.it(
                "GET /network-uldata/list array/CSV format",
                network_uldata_api::get_list_format_array_csv,
            );
            context.it(
                "GET /network-uldata/list with not exist unit",
                network_uldata_api::get_list_not_exist_unit,
            );
            context.it(
                "GET /network-uldata/list with invalid parameters",
                network_uldata_api::get_list_invalid_param,
            );
            context.it(
                "GET /network-uldata/list with invalid token",
                network_uldata_api::get_list_invalid_token,
            );

            context.after_each(after_each_fn);
        });

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(db_engine)));
                create_users_tokens(state);
                create_units(state);
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

    stop_auth_broker_svc(state);

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

    let user = create_user("user_id2", now, HashMap::<String, bool>::new());
    if let Err(e) = runtime.block_on(async { auth_db.user().add(&user).await }) {
        panic!("create user_id2 error: {}", e);
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

fn create_units(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let broker_db = state.broker_db.as_ref().unwrap();

    let mut unit = create_unit(UNIT_OWNER, "owner");
    unit.member_ids.push("member".to_string());
    if let Err(e) = runtime.block_on(async { broker_db.unit().add(&unit).await }) {
        panic!("create unit error: {}", e);
    }
}
