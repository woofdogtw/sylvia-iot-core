use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use mongodb::bson::{DateTime, Document, doc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use sylvia_iot_broker::models::{
    Model,
    device::{QueryCond, QueryOneCond},
};

use super::{super::common::device as common_test, STATE, TestState};

/// MongoDB schema.
#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: Option<String>,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    profile: String,
    name: String,
    info: Document,
}

const COL_NAME: &'static str = "device";

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let _ = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .delete_many(Document::new())
            .await
    });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a device ID.
pub fn get_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        device_id: "device_id_get_none".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: None,
        network_id: "network_id_get_none".to_string(),
        network_code: "network_code_get_none".to_string(),
        network_addr: "network_addr_get_none".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "".to_string(),
        name: "".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() none error: {}", e));
    }

    let cond = QueryCond {
        device_id: Some("device_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        device_id: Some("device_id_get_none"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get none one".to_string()),
            Some(device) => device,
        },
    };
    expect(device.device_id).to_equal("device_id_get_none".to_string())?;
    expect(device.unit_id).to_equal("unit_id_get".to_string())?;
    expect(device.unit_code).to_equal(None)?;
    expect(device.network_id).to_equal("network_id_get_none".to_string())?;
    expect(device.network_code).to_equal("network_code_get_none".to_string())?;
    expect(device.network_addr).to_equal("network_addr_get_none".to_string())?;
    expect(device.created_at).to_equal(now)?;
    expect(device.modified_at).to_equal(now)?;
    expect(device.profile).to_equal("".to_string())?;
    expect(device.name).to_equal("".to_string())?;
    expect(device.info).to_equal(Map::<String, Value>::new())?;

    let item = Schema {
        device_id: "device_id_get_some".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: Some("unit_code_get".to_string()),
        network_id: "network_id_get_some".to_string(),
        network_code: "network_code_get_some".to_string(),
        network_addr: "network_addr_get_some".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: doc! {
            "boolean": false,
            "string": "string",
            "number": 1,
            "object": {
                "array": ["array"]
            }
        },
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() some error: {}", e));
    }

    let cond = QueryCond {
        device_id: Some("device_id_get_some"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get some one".to_string()),
            Some(device) => device,
        },
    };
    expect(device.device_id).to_equal("device_id_get_some".to_string())?;
    expect(device.unit_id).to_equal("unit_id_get".to_string())?;
    expect(device.unit_code).to_equal(Some("unit_code_get".to_string()))?;
    expect(device.network_id).to_equal("network_id_get_some".to_string())?;
    expect(device.network_code).to_equal("network_code_get_some".to_string())?;
    expect(device.network_addr).to_equal("network_addr_get_some".to_string())?;
    expect(device.created_at).to_equal(now)?;
    expect(device.modified_at).to_equal(now)?;
    expect(device.profile).to_equal("profile_get".to_string())?;
    expect(device.name).to_equal("name_get".to_string())?;

    match device.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match device.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match device.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match device.info.get("object") {
        Some(Value::Object(v)) => {
            if v.len() != 1 {
                return Err("wrong info.object key length not 1".to_string());
            }
            match v.get("array") {
                Some(Value::Array(v)) => {
                    if v.len() != 1 {
                        return Err("wrong info.object.array length not 1".to_string());
                    }
                    match v.get(0) {
                        Some(Value::String(v)) => match v.as_str() {
                            "array" => (),
                            _ => return Err("wrong info.object.array content value".to_string()),
                        },
                        _ => return Err("wrong info.object.array content type".to_string()),
                    }
                }
                _ => return Err("wrong info.object.array type".to_string()),
            }
        }
        _ => return Err("wrong info.object type".to_string()),
    }
    Ok(())
}

/// Test `get()` by specifying a pair of unit ID and device ID.
pub fn get_by_unit_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let device_id_get = "device_id_get";
    let device_id_not_get = "device_id_not_get";
    let device_id_get_other = "device_id_get_other";
    let item = Schema {
        device_id: device_id_get.to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: None,
        network_id: "network_id_get".to_string(),
        network_code: "network_code_get".to_string(),
        network_addr: "network_addr_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() get error: {}", e));
    }
    let item = Schema {
        device_id: device_id_not_get.to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: None,
        network_id: "network_id_get".to_string(),
        network_code: "network_code_not_get".to_string(),
        network_addr: "network_addr_not_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() not-get error: {}", e));
    }
    let item = Schema {
        device_id: device_id_get_other.to_string(),
        unit_id: "unit_id_get_other".to_string(),
        unit_code: None,
        network_id: "network_id_get".to_string(),
        network_code: "network_code_get_other".to_string(),
        network_addr: "network_addr_get_other".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() get other error: {}", e));
    }

    let cond = QueryCond {
        device_id: Some(device_id_get),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get one".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get {
        return Err("get wrong device".to_string());
    }

    let cond = QueryCond {
        device_id: Some(device_id_get_other),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    }
    Ok(())
}

/// Test `get()` by specifying a pair of network ID and device ID.
pub fn get_by_network_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let device_id_get = "device_id_get";
    let device_id_not_get = "device_id_not_get";
    let device_id_get_other = "device_id_get_other";
    let item = Schema {
        device_id: device_id_get.to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: None,
        network_id: "network_id_get".to_string(),
        network_code: "network_code_get".to_string(),
        network_addr: "network_addr_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() get error: {}", e));
    }
    let item = Schema {
        device_id: device_id_not_get.to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: None,
        network_id: "network_id_get".to_string(),
        network_code: "network_code_not_get".to_string(),
        network_addr: "network_addr_not_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() not-get error: {}", e));
    }
    let item = Schema {
        device_id: device_id_get_other.to_string(),
        unit_id: "unit_id_get_other".to_string(),
        unit_code: None,
        network_id: "network_id_get_other".to_string(),
        network_code: "network_code_get_other".to_string(),
        network_addr: "network_addr_get_other".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() get other error: {}", e));
    }

    let cond = QueryCond {
        device_id: Some(device_id_get),
        network_id: Some("network_id_get"),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get one".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get {
        return Err("get wrong device".to_string());
    }

    let cond = QueryCond {
        device_id: Some(device_id_get_other),
        network_id: Some("network_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    }
    Ok(())
}

/// Test `get()` by specifying a pair of unit code and network code/address.
pub fn get_by_unit_network(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().device();

    let now = Utc::now().trunc_subsecs(3);
    let device_id_get_none = "device_id_get_none";
    let device_id_get_some = "device_id_get_some";
    let item = Schema {
        device_id: device_id_get_none.to_string(),
        unit_id: "unit_id_get_none".to_string(),
        unit_code: None,
        network_id: "network_id_get_none".to_string(),
        network_code: "network_code_get_none".to_string(),
        network_addr: "network_addr_get_none".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() none error: {}", e));
    }
    let item = Schema {
        device_id: device_id_get_some.to_string(),
        unit_id: "unit_id_get_some".to_string(),
        unit_code: Some("unit_code_get_some".to_string()),
        network_id: "network_id_get_some".to_string(),
        network_code: "network_code_get_some".to_string(),
        network_addr: "network_addr_get_some".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        profile: "profile_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() some error: {}", e));
    }

    let cond = QueryCond {
        device: Some(QueryOneCond {
            unit_code: None,
            network_code: "network_code_get_none",
            network_addr: "network_addr_get_none",
        }),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get none".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get_none {
        return Err("get wrong none device".to_string());
    }

    let cond = QueryCond {
        device: Some(QueryOneCond {
            unit_code: Some("unit_code_get_some"),
            network_code: "network_code_get_some",
            network_addr: "network_addr_get_some",
        }),
        ..Default::default()
    };
    let device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get some".to_string()),
            Some(device) => device,
        },
    };
    if device.device_id.as_str() != device_id_get_some {
        return Err("get wrong some device".to_string());
    }
    Ok(())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::add_dup(runtime, model)
}

/// Test `add_bulk()`.
pub fn add_bulk(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::add_bulk(runtime, model)
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::del_by_device_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` by specifying a pair of unit ID and device ID.
pub fn del_by_unit_device(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::del_by_unit_device(runtime, model)
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::del_by_network_id(runtime, model)
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::del_by_network_addrs(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().device();

    common_test::list_cursor(runtime, model)
}
