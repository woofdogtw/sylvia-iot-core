use std::collections::HashMap;

use laboratory::{describe, SpecContext, Suite};
use tokio::runtime::Runtime;

use sylvia_iot_broker::models::{
    self, CacheConnOptions, ConnOptions, DeviceOptions, DeviceRouteOptions, NetworkRouteOptions,
    SqliteOptions,
};

use crate::TestState;

mod device;
mod device_route;
mod network_route;

pub const STATE: &'static str = "models/memory";

pub fn suite() -> Suite<TestState> {
    describe("models.memory", |context| {
        context.describe("cache new/close", |context| {
            context.it("new", fn_new);
            context.it("new_cache", fn_new_cache);
            context.it("close", fn_close);

            context
                .before_all(|state| {
                    state.insert(STATE, new_state(false));
                })
                .after_each(after_each)
                .after_all(after_all);
        });

        context.describe_import(describe("cache", |context| {
            context.describe("device", |context| {
                context.it("get()", device::get);
                context.it("del()", device::del);

                context
                    .before_all(|state| {
                        state.insert(STATE, new_state(true));
                    })
                    .after_each(device::after_each_fn);
            });

            context.describe("device_route", |context| {
                context.it("get_uldata()", device_route::get_uldata);
                context.it("del_uldata()", device_route::del_uldata);
                context.it("get_dldata()", device_route::get_dldata);
                context.it("del_dldata()", device_route::del_dldata);
                context.it("get_dldata_pub()", device_route::get_dldata_pub);
                context.it("del_dldata_pub()", device_route::del_dldata_pub);

                context
                    .before_all(|state| {
                        state.insert(STATE, new_state(true));
                    })
                    .after_each(device_route::after_each_fn);
            });

            context.describe("network_route", |context| {
                context.it("get_uldata()", network_route::get_uldata);
                context.it("del_uldata()", network_route::del_uldata);

                context
                    .before_all(|state| {
                        state.insert(STATE, new_state(true));
                    })
                    .after_each(network_route::after_each_fn);
            });
        }));

        context.after_all(after_all);
    })
}

fn after_all(_state: &mut HashMap<&'static str, TestState>) -> () {
    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    if let Err(e) = std::fs::remove_file(path.as_path()) {
        println!("remove file error: {}", e);
    }
}

fn after_each(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(model) = state.cache_model.take() {
        let _ = runtime.block_on(async { model.close().await });
    }
    if let Some(cache) = state.cache.take() {
        let _ = runtime.block_on(async { cache.close().await });
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
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => panic!("create model error: {}", e),
        Ok(model) => model,
    };
    let cache = match runtime.block_on(async {
        let opts = CacheConnOptions::Memory {
            device: DeviceOptions::default(),
            device_route: DeviceRouteOptions::default(),
            network_route: NetworkRouteOptions::default(),
        };
        models::new_cache(&opts, &model).await
    }) {
        Err(e) => panic!("create cache error: {}", e),
        Ok(cache) => cache,
    };
    TestState {
        runtime: Some(runtime),
        cache_model: Some(model),
        cache: Some(cache),
        ..Default::default()
    }
}

fn fn_new(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => return Err(format!("create model error: {}", e)),
        Ok(model) => model,
    };
    state.cache_model = Some(model);
    state.cache = None;
    Ok(())
}

fn fn_new_cache(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => return Err(format!("create model error: {}", e)),
        Ok(model) => model,
    };
    let cache = match runtime.block_on(async {
        let opts = CacheConnOptions::Memory {
            device: DeviceOptions::default(),
            device_route: DeviceRouteOptions::default(),
            network_route: NetworkRouteOptions::default(),
        };
        models::new_cache(&opts, &model).await
    }) {
        Err(e) => panic!("create cache error: {}", e),
        Ok(cache) => cache,
    };
    state.cache_model = Some(model);
    state.cache = Some(cache);
    Ok(())
}

fn fn_close(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        let opts = ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        });
        models::new(&opts).await
    }) {
        Err(e) => return Err(format!("create model error: {}", e)),
        Ok(model) => model,
    };
    let cache = match runtime.block_on(async {
        let opts = CacheConnOptions::Memory {
            device: DeviceOptions::default(),
            device_route: DeviceRouteOptions::default(),
            network_route: NetworkRouteOptions::default(),
        };
        models::new_cache(&opts, &model).await
    }) {
        Err(e) => panic!("create cache error: {}", e),
        Ok(cache) => cache,
    };
    state.cache_model = Some(model);
    if let Err(e) = runtime.block_on(async { cache.close().await }) {
        return Err(format!("close cache error: {}", e));
    }
    Ok(())
}
