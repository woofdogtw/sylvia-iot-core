use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{doc, DateTime, Document};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use sylvia_iot_broker::models::{unit::QueryCond, Model};

use super::{super::common::unit as common_test, TestState, STATE};

/// MongoDB schema.
#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "unitId")]
    unit_id: String,
    code: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    #[serde(rename = "ownerId")]
    owner_id: String,
    #[serde(rename = "memberIds")]
    member_ids: Vec<String>,
    name: String,
    info: Document,
}

const COL_NAME: &'static str = "unit";

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let _ = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .delete_many(Document::new(), None)
            .await
    });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a unit ID.
pub fn get_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        unit_id: "unit_id_get_none".to_string(),
        code: "code_get_none".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get_none".to_string(),
        member_ids: vec![],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() none error: {}", e));
    }

    let cond = QueryCond {
        unit_id: Some("unit_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        unit_id: Some("unit_id_get_none"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get none one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(unit.unit_id).to_equal("unit_id_get_none".to_string())?;
    expect(unit.code).to_equal("code_get_none".to_string())?;
    expect(unit.created_at).to_equal(now)?;
    expect(unit.modified_at).to_equal(now)?;
    expect(unit.owner_id).to_equal("owner_id_get_none".to_string())?;
    expect(unit.member_ids.len()).to_equal(0)?;
    expect(unit.name).to_equal("name_get".to_string())?;
    expect(unit.info).to_equal(Map::<String, Value>::new())?;

    let item = Schema {
        unit_id: "unit_id_get_some".to_string(),
        code: "code_get_some".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get_some".to_string(),
        member_ids: vec!["member_id1".to_string(), "member_id2".to_string()],
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
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() some error: {}", e));
    }

    let cond = QueryCond {
        unit_id: Some("unit_id_get_some"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get some one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(unit.unit_id).to_equal("unit_id_get_some".to_string())?;
    expect(unit.code).to_equal("code_get_some".to_string())?;
    expect(unit.created_at).to_equal(now)?;
    expect(unit.modified_at).to_equal(now)?;
    expect(unit.owner_id).to_equal("owner_id_get_some".to_string())?;
    expect(unit.member_ids).to_equal(vec!["member_id1".to_string(), "member_id2".to_string()])?;
    expect(unit.name).to_equal("name_get".to_string())?;

    match unit.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match unit.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match unit.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match unit.info.get("object") {
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

/// Test `get()` by specifying a unit code.
pub fn get_by_code(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        unit_id: "unit_id_get".to_string(),
        code: "code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get".to_string(),
        member_ids: vec![],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() error: {}", e));
    }

    let cond = QueryCond {
        code: Some("code_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        code: Some("code_get"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(unit.unit_id).to_equal("unit_id_get".to_string())?;
    expect(unit.code).to_equal("code_get".to_string())?;
    expect(unit.created_at).to_equal(now)?;
    expect(unit.modified_at).to_equal(now)?;
    expect(unit.owner_id).to_equal("owner_id_get".to_string())?;
    expect(unit.member_ids.len()).to_equal(0)?;
    expect(unit.name).to_equal("name_get".to_string())?;
    expect(unit.info).to_equal(Map::<String, Value>::new())
}

/// Test `get()` by specifying a pair of owner ID and unit ID.
pub fn get_by_owner_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_get = "unit_id_get";
    let unit_id_not_get = "unit_id_not_get";
    let unit_id_get_other = "unit_id_get_other";
    let item = Schema {
        unit_id: unit_id_get.to_string(),
        code: "code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get".to_string(),
        member_ids: vec![],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() get error: {}", e));
    }
    let item = Schema {
        unit_id: unit_id_not_get.to_string(),
        code: "code_not_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get".to_string(),
        member_ids: vec![],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() not-get error: {}", e));
    }
    let item = Schema {
        unit_id: unit_id_get_other.to_string(),
        code: "code_get_other".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get_other".to_string(),
        member_ids: vec![],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() other error: {}", e));
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get),
        owner_id: Some("owner_id_get"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get one".to_string()),
            Some(unit) => unit,
        },
    };
    if unit.unit_id.as_str() != unit_id_get {
        return Err("get wrong unit".to_string());
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get_other),
        owner_id: Some("owner_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    };
    Ok(())
}

/// Test `get()` by specifying a pair of member ID and unit ID.
pub fn get_by_member_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_get = "unit_id_get";
    let unit_id_not_get = "unit_id_not_get";
    let unit_id_get_other = "unit_id_get_other";
    let item = Schema {
        unit_id: unit_id_get.to_string(),
        code: "code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get".to_string(),
        member_ids: vec!["member_id_get".to_string()],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() get error: {}", e));
    }
    let item = Schema {
        unit_id: unit_id_not_get.to_string(),
        code: "code_not_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get".to_string(),
        member_ids: vec!["member_id_get".to_string()],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() not-get error: {}", e));
    }
    let item = Schema {
        unit_id: unit_id_get_other.to_string(),
        code: "code_get_other".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        owner_id: "owner_id_get_other".to_string(),
        member_ids: vec!["member_id_get_other".to_string()],
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() other error: {}", e));
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get),
        member_id: Some("member_id_get"),
        ..Default::default()
    };
    let unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get one".to_string()),
            Some(unit) => unit,
        },
    };
    if unit.unit_id.as_str() != unit_id_get {
        return Err("get wrong unit".to_string());
    }

    let cond = QueryCond {
        unit_id: Some(unit_id_get_other),
        member_id: Some("member_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    };
    Ok(())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::del_by_unit_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a owner ID.
pub fn del_by_owner_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::del_by_owner_id(runtime, model)
}

/// Test `del()` by specifying a pair of owner ID and unit ID.
pub fn del_by_owner_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::del_by_owner_unit(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().unit();

    common_test::list_cursor(runtime, model)
}
