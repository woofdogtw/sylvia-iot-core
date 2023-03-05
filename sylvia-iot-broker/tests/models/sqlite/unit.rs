use std::collections::HashMap;

use chrono::{Duration, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use sql_builder::{quote, SqlBuilder};

use sylvia_iot_broker::models::{
    unit::{
        ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, Unit, UpdateQueryCond, Updates,
    },
    Model,
};

use super::{TestState, STATE};

const TABLE_NAME: &'static str = "unit";
const FIELDS: &'static [&'static str] = &[
    "unit_id",
    "code",
    "created_at",
    "modified_at",
    "owner_id",
    "member_ids",
    "name",
    "info",
];

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let sql = SqlBuilder::delete_from(TABLE_NAME).sql().unwrap();
    let _ = runtime.block_on(async { sqlx::query(sql.as_str()).execute(conn).await });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a unit ID.
pub fn get_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("unit_id_get_none"),
            quote("code_get_none"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_none".to_string()),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() none error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() none error: {}", e.to_string()));
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

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("unit_id_get_some"),
            quote("code_get_some"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_some".to_string()),
            quote("member_id1 member_id2"),
            quote("name_get"),
            quote("{\"boolean\":false,\"string\":\"string\",\"number\":1,\"object\":{\"array\":[\"array\"]}}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() some error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() some error: {}", e.to_string()));
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
    expect(unit.member_ids.len()).to_equal(2)?;
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
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("unit_id_get"),
            quote("code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() error: {}", e.to_string()));
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
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_get = "unit_id_get";
    let unit_id_not_get = "unit_id_not_get";
    let unit_id_get_other = "unit_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get),
            quote("code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_not_get),
            quote("code_not_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get_other),
            quote("code_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_other".to_string()),
            quote(""),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
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
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_get = "unit_id_get";
    let unit_id_not_get = "unit_id_not_get";
    let unit_id_get_other = "unit_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get),
            quote("code_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote("member_id_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_not_get),
            quote("code_not_get"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get"),
            quote("member_id_get"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(unit_id_get_other),
            quote("code_get_other"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("owner_id_get_other".to_string()),
            quote("member_id__get_other"),
            quote("name_get"),
            quote("{}"),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
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
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit = Unit {
        unit_id: "unit_id_add_none".to_string(),
        code: "code_add_none".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_add_none".to_string(),
        member_ids: vec![],
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&unit).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let cond = QueryCond {
        unit_id: Some(&unit.unit_id),
        ..Default::default()
    };
    let get_unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get none one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(get_unit).to_equal(unit)?;

    let mut info = Map::<String, Value>::new();
    info.insert("boolean".to_string(), Value::Bool(true));
    info.insert("string".to_string(), Value::String("string".to_string()));
    info.insert("number".to_string(), Value::Number(1.into()));
    let info_object_array = vec![Value::String("array".to_string())];
    let mut info_object = Map::<String, Value>::new();
    info_object.insert("array".to_string(), Value::Array(info_object_array));
    info.insert("object".to_string(), Value::Object(info_object));
    let unit = Unit {
        unit_id: "unit_id_add_some".to_string(),
        code: "code_add_some".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_add_some".to_string(),
        member_ids: vec!["member_id1".to_string(), "member_id2".to_string()],
        name: "name_add".to_string(),
        info: info.clone(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&unit).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let cond = QueryCond {
        unit_id: Some(&unit.unit_id),
        ..Default::default()
    };
    let get_unit = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(unit) => match unit {
            None => return Err("should get some one".to_string()),
            Some(unit) => unit,
        },
    };
    expect(get_unit).to_equal(unit)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let mut unit = Unit {
        unit_id: "unit_id_add".to_string(),
        code: "code_add".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_add".to_string(),
        member_ids: vec![],
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&unit).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    unit.code = "code_not_exist".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&unit).await }) {
        return Err("model.add() duplicate unit_id should error".to_string());
    }
    unit.unit_id = "unit_id_not_exist".to_string();
    unit.code = "code_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&unit).await }) {
        return Err("model.add() duplicate code should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_del = "unit_id_del";
    let unit_id_not_del = "unit_id_not_del";
    let mut unit = Unit {
        unit_id: unit_id_del.to_string(),
        code: "code_del".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_del".to_string(),
        member_ids: vec![],
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        unit_id: Some(unit_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = unit_id_not_del.to_string();
        unit.code = "code_not_del".to_string();
        model.add(&unit).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.unit_id = Some(unit_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(unit) => match unit {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_del = "unit_id_del";
    let unit = Unit {
        unit_id: unit_id_del.to_string(),
        code: "code_del".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_del".to_string(),
        member_ids: vec![],
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        unit_id: Some(unit_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a owner ID.
pub fn del_by_owner_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_del1 = "unit_id_del1";
    let unit_id_del2 = "unit_id_del2";
    let unit_id_not_del = "unit_id_not_del";
    let mut unit = Unit {
        unit_id: unit_id_del1.to_string(),
        code: "code_del1".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_del".to_string(),
        member_ids: vec![],
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        owner_id: Some("owner_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = unit_id_del2.to_string();
        unit.code = "code_del2".to_string();
        model.add(&unit).await?;
        unit.unit_id = unit_id_not_del.to_string();
        unit.code = "code_not_del".to_string();
        unit.owner_id = "owner_id_not_del".to_string();
        model.add(&unit).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        unit_id: Some(unit_id_del1),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete unit1 error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("delete unit1 fail".to_string()),
        },
    }
    cond.unit_id = Some(unit_id_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete unit2 error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("delete unit2 fail".to_string()),
        },
    }
    cond.unit_id = Some(unit_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(unit) => match unit {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a pair of owner ID and unit ID.
pub fn del_by_owner_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_del = "unit_id_del";
    let unit_id_not_del = "unit_id_not_del";
    let mut unit = Unit {
        unit_id: unit_id_del.to_string(),
        code: "code_del".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_del".to_string(),
        member_ids: vec![],
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        unit_id: Some(unit_id_del),
        owner_id: Some("owner_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = unit_id_not_del.to_string();
        unit.code = "code_not_del".to_string();
        model.add(&unit).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(unit) => match unit {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.unit_id = Some(unit_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(unit) => match unit {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let unit_id_update = "unit_id_update";
    let owner_id_update = "owner_id_update";
    let unit = Unit {
        unit_id: unit_id_update.to_string(),
        code: "code_update".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: owner_id_update.to_string(),
        member_ids: vec!["member_id1".to_string()],
        name: "name_update".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&unit).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let mut get_cond = QueryCond {
        unit_id: Some(unit_id_update),
        owner_id: Some(owner_id_update),
        ..Default::default()
    };
    let update_cond = UpdateQueryCond {
        unit_id: unit_id_update,
    };

    // Update only one field.
    let now = now + Duration::milliseconds(1);
    let updates = Updates {
        modified_at: Some(now),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() one error: {}", e));
    }
    let get_unit = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() one error: {}", e)),
        Ok(unit) => match unit {
            None => return Err(format!("model.get() one should get one")),
            Some(unit) => unit,
        },
    };
    expect(get_unit.unit_id.as_str()).to_equal(unit.unit_id.as_str())?;
    expect(get_unit.code.as_str()).to_equal(unit.code.as_str())?;
    expect(get_unit.created_at).to_equal(unit.created_at)?;
    expect(get_unit.modified_at).to_equal(now)?;
    expect(get_unit.owner_id.as_str()).to_equal(unit.owner_id.as_str())?;
    expect(get_unit.member_ids.as_slice()).to_equal(unit.member_ids.as_slice())?;
    expect(get_unit.name.as_str()).to_equal(unit.name.as_str())?;
    expect(get_unit.info).to_equal(unit.info.clone())?;

    // Update all fields.
    let now = now + Duration::milliseconds(1);
    let member_ids = vec!["member_id_all1".to_string(), "member_id_all2".to_string()];
    let mut info = Map::<String, Value>::new();
    info.insert("key".to_string(), Value::String("value".to_string()));
    get_cond.owner_id = Some("owner_id_update_all");
    let updates = Updates {
        modified_at: Some(now),
        owner_id: Some("owner_id_update_all"),
        member_ids: Some(&member_ids),
        name: Some("name_update_all"),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() all error: {}", e));
    }
    let get_unit = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() all error: {}", e)),
        Ok(unit) => match unit {
            None => return Err(format!("model.get() all should get one")),
            Some(unit) => unit,
        },
    };
    expect(get_unit.unit_id.as_str()).to_equal(unit.unit_id.as_str())?;
    expect(get_unit.code.as_str()).to_equal(unit.code.as_str())?;
    expect(get_unit.created_at).to_equal(unit.created_at)?;
    expect(get_unit.modified_at).to_equal(now)?;
    expect(get_unit.owner_id.as_str()).to_equal("owner_id_update_all")?;
    expect(get_unit.member_ids.as_slice()).to_equal(member_ids.as_slice())?;
    expect(get_unit.name.as_str()).to_equal("name_update_all")?;
    expect(get_unit.info).to_equal(info)?;

    // Update all fields back to None.
    let now = now + Duration::milliseconds(1);
    let member_ids = vec![];
    let info = Map::<String, Value>::new();
    get_cond.owner_id = Some(owner_id_update);
    let updates = Updates {
        modified_at: Some(now),
        owner_id: Some(owner_id_update),
        member_ids: Some(&member_ids),
        name: Some(""),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() none error: {}", e));
    }
    let get_unit = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(unit) => match unit {
            None => return Err(format!("model.get() none should get one")),
            Some(unit) => unit,
        },
    };
    expect(get_unit.unit_id.as_str()).to_equal(unit.unit_id.as_str())?;
    expect(get_unit.code.as_str()).to_equal(unit.code.as_str())?;
    expect(get_unit.created_at).to_equal(unit.created_at)?;
    expect(get_unit.modified_at).to_equal(now)?;
    expect(get_unit.owner_id.as_str()).to_equal(owner_id_update)?;
    expect(get_unit.member_ids.as_slice()).to_equal(member_ids.as_slice())?;
    expect(get_unit.name.as_str()).to_equal("")?;
    expect(get_unit.info).to_equal(info)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let cond = UpdateQueryCond {
        unit_id: "unit_id_not_exist",
    };
    let updates = Updates {
        modified_at: Some(Utc::now()),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let cond = UpdateQueryCond { unit_id: "unit_id" };
    let updates = Updates {
        modified_at: None,
        owner_id: None,
        member_ids: None,
        name: None,
        info: None,
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let mut unit = Unit {
        unit_id: "unit_id_count1_1".to_string(),
        code: "code_count1_1".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_count".to_string(),
        member_ids: vec!["member_id1".to_string(), "member_id2".to_string()],
        name: "name_count_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = "unit_id_count1_2".to_string();
        unit.code = "code_count1_2".to_string();
        unit.member_ids = vec![];
        unit.name = "name_count1_2".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_count2_1".to_string();
        unit.code = "code_count2_1".to_string();
        unit.name = "name_count2_1".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_count3_1".to_string();
        unit.code = "code_count3_1".to_string();
        unit.owner_id = "owner_id_count3".to_string();
        unit.member_ids = vec!["member_id1".to_string()];
        unit.name = "name_count_1".to_string();
        model.add(&unit).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count all result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(4)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        owner_id: Some("owner_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count owner_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_count1_1"),
        owner_id: Some("owner_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit-owner result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_count3_1"),
        owner_id: Some("owner_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit3-owner result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        owner_id: Some("owner_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code-owner result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("count_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("count_1"),
        owner_id: Some("owner_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-owner result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        owner_id: Some("owner_id_count3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-owner3 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        member_id: Some("member_id1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count member result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        member_id: Some("member_id2"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count member2 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        member_id: Some("member_id1"),
        code_contains: Some("count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count member-name result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let mut unit = Unit {
        unit_id: "unit_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_list".to_string(),
        member_ids: vec!["member_id1".to_string(), "member_id2".to_string()],
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list1_2".to_string();
        unit.code = "code_list1_2".to_string();
        unit.member_ids = vec![];
        unit.name = "name_list1_2".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list2_1".to_string();
        unit.code = "code_list2_1".to_string();
        unit.name = "name_list2_1".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list3_1".to_string();
        unit.code = "code_list3_1".to_string();
        unit.owner_id = "owner_id_list3".to_string();
        unit.member_ids = vec!["member_id1".to_string()];
        unit.name = "name\\\\%%''_list_1".to_string();
        model.add(&unit).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list all result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        owner_id: Some("owner_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list owner_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_list1_1"),
        owner_id: Some("owner_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit-owner result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_list3_1"),
        owner_id: Some("owner_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit3-owner result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        owner_id: Some("owner_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-owner result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("list_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("list_1"),
        owner_id: Some("owner_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-owner result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        owner_id: Some("owner_id_list3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-owner3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        member_id: Some("member_id1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list member result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        member_id: Some("member_id2"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list member2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        member_id: Some("member_id1"),
        code_contains: Some("list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list member-name result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        code_contains: Some("lIsT1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-case result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("lIsT_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-case result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("\\\\%%''"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-escape result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let mut now = Utc::now().trunc_subsecs(3);
    let mut unit = Unit {
        unit_id: "unit_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_list".to_string(),
        member_ids: vec![],
        name: "name_list1_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        now = now + Duration::seconds(1);
        unit.unit_id = "unit_id_list1_2".to_string();
        unit.code = "code_list1_2".to_string();
        unit.created_at = now;
        unit.modified_at = now;
        unit.name = "name_list1_2".to_string();
        model.add(&unit).await?;
        now = now + Duration::seconds(1);
        unit.unit_id = "unit_id_list2_1".to_string();
        unit.code = "code_list2_1".to_string();
        unit.created_at = now;
        unit.modified_at = now;
        unit.name = "name_list2_1".to_string();
        model.add(&unit).await?;
        now = now + Duration::seconds(1);
        unit.unit_id = "unit_id_list3_1".to_string();
        unit.code = "code_list3_1".to_string();
        unit.created_at = now;
        unit.modified_at = now;
        unit.name = "name_list2_1".to_string();
        model.add(&unit).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Code,
        asc: true,
    }];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Code,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(list[1].code.as_str()).to_equal("code_list2_1")?;
    expect(list[2].code.as_str()).to_equal("code_list1_2")?;
    expect(list[3].code.as_str()).to_equal("code_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(list[1].code.as_str()).to_equal("code_list2_1")?;
    expect(list[2].code.as_str()).to_equal("code_list1_2")?;
    expect(list[3].code.as_str()).to_equal("code_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list modified-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list modified-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(list[1].code.as_str()).to_equal("code_list2_1")?;
    expect(list[2].code.as_str()).to_equal("code_list1_2")?;
    expect(list[3].code.as_str()).to_equal("code_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Name,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].name.as_str()).to_equal("name_list1_1")?;
    expect(list[1].name.as_str()).to_equal("name_list1_2")?;
    expect(list[2].name.as_str()).to_equal("name_list2_1")?;
    expect(list[3].name.as_str()).to_equal("name_list2_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Name,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].name.as_str()).to_equal("name_list2_1")?;
    expect(list[1].name.as_str()).to_equal("name_list2_1")?;
    expect(list[2].name.as_str()).to_equal("name_list1_2")?;
    expect(list[3].name.as_str()).to_equal("name_list1_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::Name,
            asc: true,
        },
        SortCond {
            key: SortKey::CreatedAt,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-created-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::Name,
            asc: true,
        },
        SortCond {
            key: SortKey::CreatedAt,
            asc: false,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-created-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list3_1")?;
    expect(list[3].code.as_str()).to_equal("code_list2_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let mut unit = Unit {
        unit_id: "unit_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_list".to_string(),
        member_ids: vec![],
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list1_2".to_string();
        unit.code = "code_list1_2".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list2_1".to_string();
        unit.code = "code_list2_1".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list3_1".to_string();
        unit.code = "code_list3_1".to_string();
        model.add(&unit).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Code,
        asc: true,
    }];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: Some(3),
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(list[1].code.as_str()).to_equal("code_list3_1")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().unit();

    let now = Utc::now().trunc_subsecs(3);
    let mut unit = Unit {
        unit_id: "unit_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        owner_id: "owner_id_list".to_string(),
        member_ids: vec![],
        name: "name_list".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list1_2".to_string();
        unit.code = "code_list1_2".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list2_1".to_string();
        unit.code = "code_list2_1".to_string();
        model.add(&unit).await?;
        unit.unit_id = "unit_id_list3_1".to_string();
        unit.code = "code_list3_1".to_string();
        model.add(&unit).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Code,
        asc: true,
    }];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: Some(3),
    };
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-3-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(3)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(list[1].code.as_str()).to_equal("code_list3_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(4)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-3 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(0)?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.offset = Some(2);
    opts.limit = Some(3);
    opts.cursor_max = Some(5);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-5 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(list[1].code.as_str()).to_equal("code_list3_1")?;
    expect(cursor.is_none()).to_equal(true)
}
