use std::collections::HashMap;

use chrono::Utc;
use laboratory::{SpecContext, expect};

use sylvia_iot_broker::models::network_route::{NetworkRoute, QueryCond};

use super::{STATE, TestState};

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap();
    let cache = state.cache.as_ref().unwrap();

    runtime.block_on(async {
        let cond = QueryCond {
            unit_id: Some("unit_id"),
            ..Default::default()
        };
        let _ = model.network_route().del(&cond).await;
        let _ = cache.network_route().clear().await;
    });
}

/// Test `get_uldata()`.
pub fn get_uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().network_route();
    let cache = state.cache.as_ref().unwrap().network_route();

    let mut route = NetworkRoute {
        route_id: "route_id1".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        application_id: "application_id1".to_string(),
        application_code: "application_code1".to_string(),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        created_at: Utc::now(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create network_route 1 error: {}", e));
    }
    route.route_id = "route_id2".to_string();
    route.application_id = "application_id2".to_string();
    route.application_code = "application_code2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create network_route 2 error: {}", e));
    }

    // Fetch data from database.
    let route = match runtime.block_on(async { cache.get_uldata("network_id").await }) {
        Err(e) => return Err(format!("get_uldata() get correct error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let keys = route.as_ref().unwrap().app_mgr_keys.as_slice();
    expect(keys.len()).to_equal(2)?;
    if keys[0].as_str().eq("unit_code.application_code1") {
        expect(keys[1].as_str()).to_equal("unit_code.application_code2")?;
    } else {
        expect(keys[0].as_str()).to_equal("unit_code.application_code2")?;
        expect(keys[1].as_str()).to_equal("unit_code.application_code1")?;
    }

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_uldata("network_id").await }) {
        Err(e) => return Err(format!("get_uldata() again get correct error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let keys = route.as_ref().unwrap().app_mgr_keys.as_slice();
    expect(keys.len()).to_equal(2)?;
    if keys[0].as_str().eq("unit_code.application_code1") {
        expect(keys[1].as_str()).to_equal("unit_code.application_code2")?;
    } else {
        expect(keys[0].as_str()).to_equal("unit_code.application_code2")?;
        expect(keys[1].as_str()).to_equal("unit_code.application_code1")?;
    }

    // Fetch data from database.
    let route = match runtime.block_on(async { cache.get_uldata("network_id1").await }) {
        Err(e) => return Err(format!("get_uldata() get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_uldata("network_id1").await }) {
        Err(e) => return Err(format!("get_uldata() get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)
}

/// Test `del_uldata()`.
pub fn del_uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().network_route();
    let cache = state.cache.as_ref().unwrap().network_route();

    let mut route = NetworkRoute {
        route_id: "route_id1".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        application_id: "application_id1".to_string(),
        application_code: "application_code1".to_string(),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        created_at: Utc::now(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create network_route 1 error: {}", e));
    }
    route.route_id = "route_id2".to_string();
    route.application_id = "application_id2".to_string();
    route.application_code = "application_code2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create network_route 2 error: {}", e));
    }
    route.route_id = "route_id3".to_string();
    route.application_id = "application_id1".to_string();
    route.application_code = "application_code1".to_string();
    route.network_id = "network_id1".to_string();
    route.network_code = "network_code1".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create network_route 3 error: {}", e));
    }

    if let Err(e) = runtime.block_on(async { cache.get_uldata("network_id").await }) {
        return Err(format!("get_uldata() get correct error: {}", e));
    }

    if let Err(e) = runtime.block_on(async { cache.del_uldata("network_id1").await }) {
        return Err(format!("del_uldata() network_code1 error: {}", e));
    }
    Ok(())
}
