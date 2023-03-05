use std::collections::HashMap;

use laboratory::{describe, Suite};
use tokio::runtime::Runtime;

use sylvia_iot_broker::models::{SqliteModel, SqliteOptions};

use crate::TestState;

mod application;
mod conn;
mod device;
mod device_route;
mod dldata_buffer;
mod network;
mod network_route;
mod unit;

pub const STATE: &'static str = "models/sqlite";

pub fn suite() -> Suite<TestState> {
    describe("models.sqlite", |context| {
        context.describe("conn", |context| {
            context.it("connect", conn::conn);
            context.it("models::new()", conn::models_new);
        });

        context.describe_import(describe("tables", |context| {
            context.describe("application", |context| {
                context.it("init()", application::init);
                context.it(
                    "get() by application_id",
                    application::get_by_application_id,
                );
                context.it("get() by code", application::get_by_code);
                context.it(
                    "get() by unit and application",
                    application::get_by_unit_application,
                );
                context.it("add()", application::add);
                context.it("add() with duplicate ID and code", application::add_dup);
                context.it(
                    "del() by application_id",
                    application::del_by_application_id,
                );
                context.it("del() twice", application::del_twice);
                context.it("del() by unit_id", application::del_by_unit_id);
                context.it(
                    "del() by unit and application",
                    application::del_by_unit_application,
                );
                context.it("update()", application::update);
                context.it("update() not exist", application::update_not_exist);
                context.it("update() with invalid options", application::update_invalid);
                context.it("count()", application::count);
                context.it("list()", application::list);
                context.it("list() sort", application::list_sort);
                context.it("list() offset limit", application::list_offset_limit);
                context.it("list() cursor", application::list_cursor);

                context.after_each(application::after_each_fn);
            });

            context.describe("device", |context| {
                context.it("init()", device::init);
                context.it("get() by device_id", device::get_by_device_id);
                context.it("get() by unit and device", device::get_by_unit_device);
                context.it("get() by network and device", device::get_by_network_device);
                context.it("get() by unit and network", device::get_by_unit_network);
                context.it("add()", device::add);
                context.it("add() with duplicate ID and code", device::add_dup);
                context.it("add_bulk()", device::add_bulk);
                context.it("del() by device_id", device::del_by_device_id);
                context.it("del() twice", device::del_twice);
                context.it("del() by unit_id", device::del_by_unit_id);
                context.it("del() by unit and device", device::del_by_unit_device);
                context.it("del() by network_id", device::del_by_network_id);
                context.it("del() by network_addrs", device::del_by_network_addrs);
                context.it("update()", device::update);
                context.it("update() not exist", device::update_not_exist);
                context.it("update() with invalid options", device::update_invalid);
                context.it("count()", device::count);
                context.it("list()", device::list);
                context.it("list() sort", device::list_sort);
                context.it("list() offset limit", device::list_offset_limit);
                context.it("list() cursor", device::list_cursor);

                context.after_each(device::after_each_fn);
            });

            context.describe("device_route", |context| {
                context.it("init()", device_route::init);
                context.it("get()", device_route::get);
                context.it("add()", device_route::add);
                context.it("add() with duplicate ID", device_route::add_dup);
                context.it("add_bulk()", device_route::add_bulk);
                context.it("del() by route_id", device_route::del_by_route_id);
                context.it("del() twice", device_route::del_twice);
                context.it("del() by unit_id", device_route::del_by_unit_id);
                context.it("del() by unit and route", device_route::del_by_unit_route);
                context.it(
                    "del() by application_id",
                    device_route::del_by_application_id,
                );
                context.it("del() by network_id", device_route::del_by_network_id);
                context.it("del() by device_id", device_route::del_by_device_id);
                context.it("del() by network_addrs", device_route::del_by_network_addrs);
                context.it("count()", device_route::count);
                context.it("list()", device_route::list);
                context.it("list() sort", device_route::list_sort);
                context.it("list() offset limit", device_route::list_offset_limit);
                context.it("list() cursor", device_route::list_cursor);

                context.after_each(device_route::after_each_fn);
            });

            context.describe("dldata_buffer", |context| {
                context.it("init()", dldata_buffer::init);
                context.it("get()", dldata_buffer::get);
                context.it("add()", dldata_buffer::add);
                context.it("add() with duplicate ID", dldata_buffer::add_dup);
                context.it("del() by data_id", dldata_buffer::del_by_data_id);
                context.it("del() twice", dldata_buffer::del_twice);
                context.it("del() by unit_id", dldata_buffer::del_by_unit_id);
                context.it("del() by unit and data", dldata_buffer::del_by_unit_data);
                context.it(
                    "del() by application_id",
                    dldata_buffer::del_by_application_id,
                );
                context.it("del() by network_id", dldata_buffer::del_by_network_id);
                context.it("del() by device_id", dldata_buffer::del_by_device_id);
                context.it(
                    "del() by network_addrs",
                    dldata_buffer::del_by_network_addrs,
                );
                context.it("count()", dldata_buffer::count);
                context.it("list()", dldata_buffer::list);
                context.it("list() sort", dldata_buffer::list_sort);
                context.it("list() offset limit", dldata_buffer::list_offset_limit);
                context.it("list() cursor", dldata_buffer::list_cursor);

                context.after_each(dldata_buffer::after_each_fn);
            });

            context.describe("network", |context| {
                context.it("init()", network::init);
                context.it("get() by network_id", network::get_by_network_id);
                context.it("get() by code", network::get_by_code);
                context.it("get() by unit and network", network::get_by_unit_network);
                context.it("add()", network::add);
                context.it("add() with duplicate ID and code", network::add_dup);
                context.it("del() by network_id", network::del_by_network_id);
                context.it("del() twice", network::del_twice);
                context.it("del() by unit_id", network::del_by_unit_id);
                context.it("del() by unit and network", network::del_by_unit_network);
                context.it("update()", network::update);
                context.it("update() not exist", network::update_not_exist);
                context.it("update() with invalid options", network::update_invalid);
                context.it("count()", network::count);
                context.it("list()", network::list);
                context.it("list() sort", network::list_sort);
                context.it("list() offset limit", network::list_offset_limit);
                context.it("list() cursor", network::list_cursor);

                context.after_each(network::after_each_fn);
            });

            context.describe("network_route", |context| {
                context.it("init()", network_route::init);
                context.it("get()", network_route::get);
                context.it("add()", network_route::add);
                context.it("add() with duplicate ID", network_route::add_dup);
                context.it("del() by route_id", network_route::del_by_route_id);
                context.it("del() twice", network_route::del_twice);
                context.it("del() by unit_id", network_route::del_by_unit_id);
                context.it("del() by unit and route", network_route::del_by_unit_route);
                context.it(
                    "del() by application_id",
                    network_route::del_by_application_id,
                );
                context.it("del() by network_id", network_route::del_by_network_id);
                context.it("count()", network_route::count);
                context.it("list()", network_route::list);
                context.it("list() sort", network_route::list_sort);
                context.it("list() offset limit", network_route::list_offset_limit);
                context.it("list() cursor", network_route::list_cursor);

                context.after_each(network_route::after_each_fn);
            });

            context.describe("unit", |context| {
                context.it("init()", unit::init);
                context.it("get() by unti_id", unit::get_by_unit_id);
                context.it("get() by code", unit::get_by_code);
                context.it("get() by owner and unit", unit::get_by_owner_unit);
                context.it("get() by member and unit", unit::get_by_member_unit);
                context.it("add()", unit::add);
                context.it("add() with duplicate ID and code", unit::add_dup);
                context.it("del() by unit_id", unit::del_by_unit_id);
                context.it("del() twice", unit::del_twice);
                context.it("del() by owner_id", unit::del_by_owner_id);
                context.it("del() by owner and unit", unit::del_by_owner_unit);
                context.it("update()", unit::update);
                context.it("update() not exist", unit::update_not_exist);
                context.it("update() with invalid options", unit::update_invalid);
                context.it("count()", unit::count);
                context.it("list()", unit::list);
                context.it("list() sort", unit::list_sort);
                context.it("list() offset limit", unit::list_offset_limit);
                context.it("list() cursor", unit::list_cursor);

                context.after_each(unit::after_each_fn);
            });

            context
                .before_all(|state| {
                    state.insert(STATE, new_state(true));
                })
                .after_all(tables_after_all);
        }));

        context
            .before_all(|state| {
                state.insert(STATE, new_state(false));
            })
            .after_all(|state| {
                let state = state.get_mut(STATE).unwrap();
                let runtime = state.runtime.as_ref().unwrap();
                if let Some(pool) = state.sqlite.as_ref() {
                    runtime.block_on(async { pool.get_connection().close().await });
                }
                let file = crate::TEST_SQLITE_PATH;
                let mut path = std::env::temp_dir();
                path.push(file);
                if let Err(e) = std::fs::remove_file(path.as_path()) {
                    println!("remove file {} error: {}", file, e);
                }
                let file = format!("{}-shm", crate::TEST_SQLITE_PATH);
                let mut path = std::env::temp_dir();
                path.push(file.as_str());
                if let Err(e) = std::fs::remove_file(path.as_path()) {
                    println!("remove file {} error: {}", file, e);
                }
                let file = format!("{}-wal", crate::TEST_SQLITE_PATH);
                let mut path = std::env::temp_dir();
                path.push(file.as_str());
                if let Err(e) = std::fs::remove_file(path.as_path()) {
                    println!("remove file {} error: {}", file, e);
                }
            });
    })
}

fn tables_after_all(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    if let Some(pool) = state.sqlite.as_ref() {
        runtime.block_on(async { pool.get_connection().close().await });
    }
    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    if let Err(e) = std::fs::remove_file(path.as_path()) {
        println!("remove file error: {}", e);
    }
}

fn new_state(with_pool: bool) -> TestState {
    let runtime = match Runtime::new() {
        Err(e) => panic!("create runtime error: {}", e),
        Ok(runtime) => runtime,
    };

    if !with_pool {
        return TestState {
            runtime: Some(runtime),
            ..Default::default()
        };
    }
    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        SqliteModel::new(&SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        })
        .await
    }) {
        Err(e) => panic!("create model error: {}", e),
        Ok(model) => Some(model),
    };
    TestState {
        runtime: Some(runtime),
        sqlite: model,
        ..Default::default()
    }
}
