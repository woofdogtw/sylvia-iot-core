use std::collections::HashMap;

use chrono::Utc;
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};

use sylvia_iot_broker::models::device::{
    DelCacheQueryCond, Device, GetCacheQueryCond, QueryCond, QueryOneCond,
};

use super::{TestState, STATE};

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
        let _ = model.device().del(&cond).await;
        let _ = cache.device().clear().await;
    });
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device();
    let cache = state.cache.as_ref().unwrap().device();

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
        return Err(format!("create device error: {}", e));
    }
    device.device_id = "device_id_pub".to_string();
    device.unit_code = None;
    device.network_id = "network_id_pub".to_string();
    device.network_code = "network_code_pub".to_string();
    device.network_addr = "network_addr_pub".to_string();
    device.profile = "pub".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device pub error: {}", e));
    }

    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: Some("unit_code"),
        network_code: "network_code",
        network_addr: "network_addr",
    });
    let device = match runtime.block_on(async { cache.get(&cond).await }) {
        Err(e) => return Err(format!("get() get correct error: {}", e)),
        Ok(device) => device,
    };
    expect(device.is_some()).to_equal(true)?;
    let device = device.unwrap();
    expect(device.device_id.as_str()).to_equal("device_id")?;
    expect(device.profile.as_str()).to_equal("")?;

    // Fetch again to get data from cache.
    let device = match runtime.block_on(async { cache.get(&cond).await }) {
        Err(e) => return Err(format!("get() get again correct error: {}", e)),
        Ok(device) => device,
    };
    expect(device.is_some()).to_equal(true)?;
    let device = device.unwrap();
    expect(device.device_id.as_str()).to_equal("device_id")?;
    expect(device.profile.as_str()).to_equal("")?;

    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: None,
        network_code: "network_code_pub",
        network_addr: "network_addr_pub",
    });
    let device = match runtime.block_on(async { cache.get(&cond).await }) {
        Err(e) => return Err(format!("get() get correct pub error: {}", e)),
        Ok(device) => device,
    };
    expect(device.is_some()).to_equal(true)?;
    let device = device.unwrap();
    expect(device.device_id.as_str()).to_equal("device_id_pub")?;
    expect(device.profile.as_str()).to_equal("pub")?;

    // Fetch again to get data from cache.
    let device = match runtime.block_on(async { cache.get(&cond).await }) {
        Err(e) => return Err(format!("get() get again correct pub error: {}", e)),
        Ok(device) => device,
    };
    expect(device.is_some()).to_equal(true)?;
    let device = device.unwrap();
    expect(device.device_id.as_str()).to_equal("device_id_pub")?;
    expect(device.profile.as_str()).to_equal("pub")?;

    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: None,
        network_code: "network_code",
        network_addr: "network_addr",
    });
    let device = match runtime.block_on(async { cache.get(&cond).await }) {
        Err(e) => return Err(format!("get() get not-match error: {}", e)),
        Ok(device) => device,
    };
    expect(device.is_none()).to_equal(true)?;
    let device = match runtime.block_on(async { cache.get(&cond).await }) {
        Err(e) => return Err(format!("get() get again not-match error: {}", e)),
        Ok(device) => device,
    };
    expect(device.is_none()).to_equal(true)
}

/// Test `del()`.
pub fn del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.cache_model.as_ref().unwrap().device();
    let cache = state.cache.as_ref().unwrap().device();

    let mut device = Device {
        device_id: "device_id".to_string(),
        unit_id: "unit_id".to_string(),
        unit_code: Some("unit_code".to_string()),
        network_id: "network_id".to_string(),
        network_code: "network_code".to_string(),
        network_addr: "network_addr1".to_string(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        profile: "".to_string(),
        name: "".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 1 error: {}", e));
    }
    device.device_id = "device_id2".to_string();
    device.network_addr = "network_addr2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 2 error: {}", e));
    }
    device.device_id = "device_id3".to_string();
    device.network_id = "network_id3".to_string();
    device.network_code = "network_code3".to_string();
    device.network_addr = "network_addr3".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device 3 error: {}", e));
    }
    device.device_id = "device_id_pub".to_string();
    device.unit_code = None;
    device.network_id = "network_id_pub".to_string();
    device.network_code = "network_code_pub".to_string();
    device.network_addr = "network_addr_pub".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device pub error: {}", e));
    }
    device.device_id = "device_id_pub2".to_string();
    device.network_addr = "network_addr_pub2".to_string();
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("create device pub 2 error: {}", e));
    }

    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: Some("unit_code"),
        network_code: "network_code",
        network_addr: "network_addr1",
    });
    if let Err(e) = runtime.block_on(async { cache.get(&cond).await }) {
        return Err(format!("get() get correct 1 error: {}", e));
    }
    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: Some("unit_code"),
        network_code: "network_code",
        network_addr: "network_addr2",
    });
    if let Err(e) = runtime.block_on(async { cache.get(&cond).await }) {
        return Err(format!("get() get correct 2 error: {}", e));
    }
    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: Some("unit_code"),
        network_code: "network_code3",
        network_addr: "network_addr3",
    });
    if let Err(e) = runtime.block_on(async { cache.get(&cond).await }) {
        return Err(format!("get() get correct 3 error: {}", e));
    }
    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: None,
        network_code: "network_code_pub",
        network_addr: "network_addr_pub",
    });
    if let Err(e) = runtime.block_on(async { cache.get(&cond).await }) {
        return Err(format!("get() get correct pub error: {}", e));
    }
    let cond = GetCacheQueryCond::CodeAddr(QueryOneCond {
        unit_code: None,
        network_code: "network_code_pub",
        network_addr: "network_addr_pub2",
    });
    if let Err(e) = runtime.block_on(async { cache.get(&cond).await }) {
        return Err(format!("get() get correct pub 2 error: {}", e));
    }

    let cond = DelCacheQueryCond {
        unit_code: "unit_code",
        network_code: Some("network_code"),
        network_addr: Some("network_addr1"),
    };
    if let Err(e) = runtime.block_on(async { cache.del(&cond).await }) {
        return Err(format!("del() network_code_addr1 error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "unit_code",
        network_code: Some("network_code"),
        network_addr: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del(&cond).await }) {
        return Err(format!("del() network_code error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "unit_code",
        network_code: None,
        network_addr: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del(&cond).await }) {
        return Err(format!("del() unit_code error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "",
        network_code: Some("network_code_pub"),
        network_addr: Some("network_addr_pub"),
    };
    if let Err(e) = runtime.block_on(async { cache.del(&cond).await }) {
        return Err(format!("del() pub_network_code_addr error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "",
        network_code: Some("network_code_pub"),
        network_addr: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del(&cond).await }) {
        return Err(format!("del() pub_network_code error: {}", e));
    }
    let cond = DelCacheQueryCond {
        unit_code: "",
        network_code: None,
        network_addr: None,
    };
    if let Err(e) = runtime.block_on(async { cache.del(&cond).await }) {
        return Err(format!("del() pub error: {}", e));
    }
    Ok(())
}
