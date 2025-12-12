use std::collections::HashMap;

use chrono::Utc;
use laboratory::{SpecContext, expect};
use serde_json::{Map, Value};

use sylvia_iot_broker::models::{
    device::{Device, QueryCond as DeviceQueryCond},
    device_route::{
        DelCachePubQueryCond, DelCacheQueryCond, DeviceRoute, GetCachePubQueryCond,
        GetCacheQueryCond, QueryCond,
    },
};

use super::{STATE, TestState};

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap();
    let cache = state.cache.as_ref().unwrap();

    runtime.block_on(async {
        let cond = DeviceQueryCond {
            unit_id: Some("unit_id"),
            ..Default::default()
        };
        let _ = model.device().del(&cond).await;
        let cond = QueryCond {
            unit_id: Some("unit_id"),
            ..Default::default()
        };
        let _ = model.device_route().del(&cond).await;
        let _ = cache.device_route().clear().await;
    });
}

/// Test `get_uldata()`.
pub fn get_uldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device_route();
    let cache = state.cache.as_ref().unwrap().device_route();

    let now = Utc::now();
    let mut route = DeviceRoute {
        route_id: "route_id1".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        application_id: "application_id1".to_string(),
        application_code: "application_code1".to_string(),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        device_id: "device_id".to_string(),
        profile: "".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create device_route 1 error: {}", e));
    }
    route.route_id = "route_id2".to_string();
    route.application_id = "application_id2".to_string();
    route.application_code = "application_code2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create device_route 2 error: {}", e));
    }

    // Fetch data from database.
    let route = match runtime.block_on(async { cache.get_uldata("device_id").await }) {
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
    let route = match runtime.block_on(async { cache.get_uldata("device_id").await }) {
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
    let route = match runtime.block_on(async { cache.get_uldata("device_id1").await }) {
        Err(e) => return Err(format!("get_uldata() get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_uldata("device_id1").await }) {
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
    let model = state.cache_model.as_ref().unwrap().device_route();
    let cache = state.cache.as_ref().unwrap().device_route();

    let now = Utc::now();
    let mut route = DeviceRoute {
        route_id: "route_id1".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: "unit_code".to_string(),
        application_id: "application_id1".to_string(),
        application_code: "application_code1".to_string(),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        device_id: "device_id".to_string(),
        profile: "".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create device_route 1 error: {}", e));
    }
    route.route_id = "route_id2".to_string();
    route.application_id = "application_id2".to_string();
    route.application_code = "application_code2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create device_route 2 error: {}", e));
    }
    route.route_id = "route_id3".to_string();
    route.application_id = "application_id1".to_string();
    route.application_code = "application_code1".to_string();
    route.network_id = "network_id1".to_string();
    route.network_code = "network_code1".to_string();
    route.network_addr = "network_addr1".to_string();
    route.device_id = "device_id1".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create device_route 3 error: {}", e));
    }
    route.route_id = "route_id4".to_string();
    route.network_addr = "network_addr2".to_string();
    route.device_id = "device_id2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("create device_route 4 error: {}", e));
    }

    if let Err(e) = runtime.block_on(async { cache.get_uldata("device_id").await }) {
        return Err(format!("get_uldata() get correct 1 error: {}", e));
    }
    if let Err(e) = runtime.block_on(async { cache.get_uldata("device_id1").await }) {
        return Err(format!("get_uldata() get correct 2 error: {}", e));
    }
    if let Err(e) = runtime.block_on(async { cache.get_uldata("device_id2").await }) {
        return Err(format!("get_uldata() get correct 3 error: {}", e));
    }

    if let Err(e) = runtime.block_on(async { cache.del_uldata("device_id1").await }) {
        return Err(format!("del_uldata() device_id1 error: {}", e));
    }
    Ok(())
}

/// Test `get_dldata()`.
pub fn get_dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device();
    let cache = state.cache.as_ref().unwrap().device_route();

    let device = Device {
        device_id: "device_id".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: Some("unit_code".to_string()),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        profile: "".to_string(),
        name: "".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 1 error: {}", e));
    }

    // Fetch data from database.
    let cond = GetCacheQueryCond {
        unit_code: "unit_code",
        network_code: "network_code",
        network_addr: "network_addr",
    };
    let route = match runtime.block_on(async { cache.get_dldata(&cond).await }) {
        Err(e) => return Err(format!("get_dldata() get correct error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let route = route.as_ref().unwrap();
    expect(route.net_mgr_key.as_str()).to_equal("unit_code.network_code")?;
    expect(route.network_id.as_str()).to_equal("network_id")?;
    expect(route.network_addr.as_str()).to_equal("network_addr")?;
    expect(route.device_id.as_str()).to_equal("device_id")?;
    expect(route.profile.as_str()).to_equal("")?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_dldata(&cond).await }) {
        Err(e) => return Err(format!("get_dldata() again get correct error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let route = route.as_ref().unwrap();
    expect(route.net_mgr_key.as_str()).to_equal("unit_code.network_code")?;
    expect(route.network_id.as_str()).to_equal("network_id")?;
    expect(route.network_addr.as_str()).to_equal("network_addr")?;
    expect(route.device_id.as_str()).to_equal("device_id")?;
    expect(route.profile.as_str()).to_equal("")?;

    // Fetch data from database.
    let cond = GetCacheQueryCond {
        unit_code: "unit_code",
        network_code: "network_code",
        network_addr: "network_addr1",
    };
    let route = match runtime.block_on(async { cache.get_dldata(&cond).await }) {
        Err(e) => return Err(format!("get_dldata() get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_dldata(&cond).await }) {
        Err(e) => return Err(format!("get_dldata() again get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)
}

/// Test `del_dldata()`.
pub fn del_dldata(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device();
    let cache = state.cache.as_ref().unwrap().device_route();

    let mut device = Device {
        device_id: "device_id".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: Some("unit_code".to_string()),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        profile: "".to_string(),
        name: "".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 1 error: {}", e));
    }
    device.device_id = "device_id1".to_string();
    device.network_id = "network_id1".to_string();
    device.network_code = "network_code1".to_string();
    device.network_addr = "network_addr1".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 2 error: {}", e));
    }
    device.device_id = "device_id2".to_string();
    device.network_addr = "network_addr2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 3 error: {}", e));
    }

    let mut cond = GetCacheQueryCond {
        unit_code: "unit_code",
        network_code: "network_code",
        network_addr: "network_addr",
    };
    if let Err(e) = runtime.block_on(async { cache.get_dldata(&cond).await }) {
        return Err(format!("get_dldata() get correct 1 error: {}", e));
    }
    cond.network_code = "network_code1";
    cond.network_addr = "network_addr1";
    if let Err(e) = runtime.block_on(async { cache.get_dldata(&cond).await }) {
        return Err(format!("get_dldata() get correct 2 error: {}", e));
    }
    cond.network_addr = "network_addr2";
    if let Err(e) = runtime.block_on(async { cache.get_dldata(&cond).await }) {
        return Err(format!("get_dldata() get correct 3 error: {}", e));
    }

    let cond = DelCacheQueryCond {
        unit_code: "unit_code",
        network_code: Some("network_code1"),
        network_addr: Some("network_addr1"),
    };
    if let Err(e) = runtime.block_on(async { cache.del_dldata(&cond).await }) {
        return Err(format!("del_uldata() network_code1_addr1 error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "unit_code",
        network_code: Some("network_code1"),
        network_addr: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del_dldata(&cond).await }) {
        return Err(format!("del_uldata() network_code1 error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "unit_code",
        network_code: None,
        network_addr: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del_dldata(&cond).await }) {
        return Err(format!("del_uldata() unit_code error: {}", e));
    }
    Ok(())
}

/// Test `get_dldata_pub()`.
pub fn get_dldata_pub(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device();
    let cache = state.cache.as_ref().unwrap().device_route();

    let mut device = Device {
        device_id: "device_id".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: Some("unit_code".to_string()),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        profile: "".to_string(),
        name: "".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 1 error: {}", e));
    }
    device.device_id = "device_id_pub".to_string();
    device.unit_code = None;
    device.network_id = "network_id_pub".to_string();
    device.network_code = "network_code_pub".to_string();
    device.network_addr = "network_addr_pub".to_string();
    device.profile = "pub".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 2 error: {}", e));
    }

    // Fetch data from database.
    let cond = GetCachePubQueryCond {
        unit_id: "unit_id",
        device_id: "device_id",
    };
    let route = match runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        Err(e) => return Err(format!("get_dldata_pub() get correct error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let route = route.as_ref().unwrap();
    expect(route.net_mgr_key.as_str()).to_equal("unit_code.network_code")?;
    expect(route.network_id.as_str()).to_equal("network_id")?;
    expect(route.network_addr.as_str()).to_equal("network_addr")?;
    expect(route.device_id.as_str()).to_equal("device_id")?;
    expect(route.profile.as_str()).to_equal("")?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        Err(e) => return Err(format!("get_dldata_pub() again get correct error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let route = route.as_ref().unwrap();
    expect(route.net_mgr_key.as_str()).to_equal("unit_code.network_code")?;
    expect(route.network_id.as_str()).to_equal("network_id")?;
    expect(route.network_addr.as_str()).to_equal("network_addr")?;
    expect(route.device_id.as_str()).to_equal("device_id")?;
    expect(route.profile.as_str()).to_equal("")?;

    // Fetch data from database.
    let cond = GetCachePubQueryCond {
        unit_id: "unit_id",
        device_id: "device_id_pub",
    };
    let route = match runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        Err(e) => return Err(format!("get_dldata_pub() get correct pub error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let route = route.as_ref().unwrap();
    expect(route.net_mgr_key.as_str()).to_equal(".network_code_pub")?;
    expect(route.network_id.as_str()).to_equal("network_id_pub")?;
    expect(route.network_addr.as_str()).to_equal("network_addr_pub")?;
    expect(route.device_id.as_str()).to_equal("device_id_pub")?;
    expect(route.profile.as_str()).to_equal("pub")?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        Err(e) => {
            return Err(format!(
                "get_dldata_pub() again get correct pub error: {}",
                e
            ));
        }
        Ok(route) => route,
    };
    expect(route.is_some()).to_equal(true)?;
    let route = route.as_ref().unwrap();
    expect(route.net_mgr_key.as_str()).to_equal(".network_code_pub")?;
    expect(route.network_id.as_str()).to_equal("network_id_pub")?;
    expect(route.network_addr.as_str()).to_equal("network_addr_pub")?;
    expect(route.device_id.as_str()).to_equal("device_id_pub")?;
    expect(route.profile.as_str()).to_equal("pub")?;

    // Fetch data from database.
    let cond = GetCachePubQueryCond {
        unit_id: "unit_id",
        device_id: "device_id1",
    };
    let route = match runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        Err(e) => return Err(format!("get_dldata_pub() get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)?;

    // Fetch again to get data from cache.
    let route = match runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        Err(e) => return Err(format!("get_dldata_pub() again get not-match error: {}", e)),
        Ok(route) => route,
    };
    expect(route.is_none()).to_equal(true)
}

/// Test `del_dldata_pub()`.
pub fn del_dldata_pub(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device();
    let cache = state.cache.as_ref().unwrap().device_route();

    let mut device = Device {
        device_id: "device_id".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: Some("unit_code".to_string()),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        profile: "".to_string(),
        name: "".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 1 error: {}", e));
    }
    device.device_id = "device_id1".to_string();
    device.network_id = "network_id1".to_string();
    device.network_code = "network_code1".to_string();
    device.network_addr = "network_addr1".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 2 error: {}", e));
    }
    device.device_id = "device_id2".to_string();
    device.network_addr = "network_addr2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 3 error: {}", e));
    }

    let mut cond = GetCachePubQueryCond {
        unit_id: "unit_id",
        device_id: "device_id",
    };
    if let Err(e) = runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        return Err(format!("get_dldata() get correct 1 error: {}", e));
    }
    cond.device_id = "device_id1";
    if let Err(e) = runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        return Err(format!("get_dldata() get correct 2 error: {}", e));
    }
    cond.device_id = "device_id2";
    if let Err(e) = runtime.block_on(async { cache.get_dldata_pub(&cond).await }) {
        return Err(format!("get_dldata() get correct 3 error: {}", e));
    }

    let cond = DelCachePubQueryCond {
        unit_id: "unit_id",
        device_id: Some("device_id1"),
    };
    if let Err(e) = runtime.block_on(async { cache.del_dldata_pub(&cond).await }) {
        return Err(format!("del_uldata() unit_id_device_id1 error: {}", e));
    }
    let cond = DelCachePubQueryCond {
        unit_id: "unit_id",
        device_id: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del_dldata_pub(&cond).await }) {
        return Err(format!("del_uldata() unit_id error: {}", e));
    }
    Ok(())
}
