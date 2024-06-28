use std::collections::HashMap;

use laboratory::{describe, Suite};
use tokio::runtime::Runtime;

use sylvia_iot_data::models::{MongoDbModel, MongoDbOptions};

use crate::TestState;

mod application_dldata;
mod application_uldata;
mod conn;
mod coremgr_opdata;
mod network_dldata;
mod network_uldata;

pub const STATE: &'static str = "models/mongodb";

pub fn suite() -> Suite<TestState> {
    describe("models.mongodb", |context| {
        context.describe("conn", |context| {
            context.it("connect", conn::conn);
            context.it("models::new()", conn::models_new);
        });

        context.describe_import(describe("collections", |context| {
            context.describe("application_dldata", |context| {
                context.it("init()", application_dldata::init);
                context.it("add()", application_dldata::add);
                context.it("add() with duplicate ID", application_dldata::add_dup);
                context.it("del() by unit_id", application_dldata::del_by_unit);
                context.it("del() twice", application_dldata::del_twice);
                context.it("del() by device_id", application_dldata::del_by_device_id);
                context.it(
                    "del() by network_addr",
                    application_dldata::del_by_network_addr,
                );
                context.it("del() by proc", application_dldata::del_by_proc);
                context.it("update() with zero", application_dldata::update_with_zero);
                context.it(
                    "update() with positive",
                    application_dldata::update_with_positive,
                );
                context.it(
                    "update() with negative",
                    application_dldata::update_with_negative,
                );
                context.it("update() not exist", application_dldata::update_not_exist);
                context.it("count()", application_dldata::count);
                context.it("list()", application_dldata::list);
                context.it("list() sort", application_dldata::list_sort);
                context.it("list() offset limit", application_dldata::list_offset_limit);
                context.it("list() cursor", application_dldata::list_cursor);

                context.after_each(application_dldata::after_each_fn);
            });

            context.describe("application_uldata", |context| {
                context.it("init()", application_uldata::init);
                context.it("add()", application_uldata::add);
                context.it("add() with duplicate ID", application_uldata::add_dup);
                context.it("del() by unit_id", application_uldata::del_by_unit);
                context.it("del() twice", application_uldata::del_twice);
                context.it("del() by device_id", application_uldata::del_by_device_id);
                context.it("del() by proc", application_uldata::del_by_proc);
                context.it("count()", application_uldata::count);
                context.it("list()", application_uldata::list);
                context.it("list() sort", application_uldata::list_sort);
                context.it("list() offset limit", application_uldata::list_offset_limit);
                context.it("list() cursor", application_uldata::list_cursor);

                context.after_each(application_uldata::after_each_fn);
            });

            context.describe("coremgr_opdata", |context| {
                context.it("init()", coremgr_opdata::init);
                context.it("add()", coremgr_opdata::add);
                context.it("add() with duplicate ID", coremgr_opdata::add_dup);
                context.it("del() by user_id", coremgr_opdata::del_by_user);
                context.it("del() twice", coremgr_opdata::del_twice);
                context.it("del() by client_id", coremgr_opdata::del_by_client);
                context.it("del() by req_time", coremgr_opdata::del_by_req);
                context.it("count()", coremgr_opdata::count);
                context.it("list()", coremgr_opdata::list);
                context.it("list() sort", coremgr_opdata::list_sort);
                context.it("list() offset limit", coremgr_opdata::list_offset_limit);
                context.it("list() cursor", coremgr_opdata::list_cursor);

                context.after_each(coremgr_opdata::after_each_fn);
            });

            context.describe("network_dldata", |context| {
                context.it("init()", network_dldata::init);
                context.it("add()", network_dldata::add);
                context.it("add() with duplicate ID", network_dldata::add_dup);
                context.it("del() by unit_id", network_dldata::del_by_unit);
                context.it("del() twice", network_dldata::del_twice);
                context.it("del() by device_id", network_dldata::del_by_device_id);
                context.it("del() by proc", network_dldata::del_by_proc);
                context.it("update() with zero", network_dldata::update_with_zero);
                context.it(
                    "update() with positive",
                    network_dldata::update_with_positive,
                );
                context.it(
                    "update() with negative",
                    network_dldata::update_with_negative,
                );
                context.it("update() not exist", network_dldata::update_not_exist);
                context.it("count()", network_dldata::count);
                context.it("list()", network_dldata::list);
                context.it("list() sort", network_dldata::list_sort);
                context.it("list() offset limit", network_dldata::list_offset_limit);
                context.it("list() cursor", network_dldata::list_cursor);

                context.after_each(network_dldata::after_each_fn);
            });

            context.describe("network_uldata", |context| {
                context.it("init()", network_uldata::init);
                context.it("add()", network_uldata::add);
                context.it("add() with duplicate ID", network_uldata::add_dup);
                context.it("del() by unit_id", network_uldata::del_by_unit);
                context.it("del() twice", network_uldata::del_twice);
                context.it("del() by device_id", network_uldata::del_by_device_id);
                context.it("del() by proc", network_uldata::del_by_proc);
                context.it("count()", network_uldata::count);
                context.it("list()", network_uldata::list);
                context.it("list() sort", network_uldata::list_sort);
                context.it("list() offset limit", network_uldata::list_offset_limit);
                context.it("list() cursor", network_uldata::list_cursor);

                context.after_each(network_uldata::after_each_fn);
            });

            context
                .before_all(|state| {
                    state.insert(STATE, new_state(true));
                })
                .after_all(collections_after_all);
        }));

        context.before_all(|state| {
            state.insert(STATE, new_state(false));
        });
    })
}

fn collections_after_all(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let _ = state.runtime.as_ref().unwrap().block_on(async {
        state
            .mongodb
            .as_ref()
            .unwrap()
            .get_connection()
            .drop()
            .await
    });
}

fn new_state(with_model: bool) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    if !with_model {
        return TestState {
            runtime: Some(runtime),
            ..Default::default()
        };
    }
    let model = match runtime.block_on(async {
        MongoDbModel::new(&MongoDbOptions {
            url: crate::TEST_MONGODB_URL.to_string(),
            db: crate::TEST_MONGODB_DB.to_string(),
            pool_size: None,
        })
        .await
    }) {
        Err(e) => panic!("create model error: {}", e),
        Ok(model) => Some(model),
    };
    TestState {
        runtime: Some(runtime),
        mongodb: model,
        ..Default::default()
    }
}
