use std::collections::HashMap;

use chrono::{Duration, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{doc, DateTime, Document};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use sylvia_iot_broker::models::{
    application::{
        Application, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, UpdateQueryCond,
        Updates,
    },
    Model,
};

use super::{TestState, STATE};

/// MongoDB schema.
#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "applicationId")]
    application_id: String,
    code: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    #[serde(rename = "hostUri")]
    host_uri: String,
    name: String,
    info: Document,
}

const COL_NAME: &'static str = "application";

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
    let model = state.mongodb.as_ref().unwrap().application();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a application ID.
pub fn get_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        application_id: "application_id_get_none".to_string(),
        code: "code_get_none".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        host_uri: "host_uri_get".to_string(),
        name: "".to_string(),
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
        application_id: Some("application_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        application_id: Some("application_id_get_none"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get none one".to_string()),
            Some(application) => application,
        },
    };
    expect(application.application_id).to_equal("application_id_get_none".to_string())?;
    expect(application.code).to_equal("code_get_none".to_string())?;
    expect(application.unit_id).to_equal("unit_id_get".to_string())?;
    expect(application.unit_code).to_equal("unit_code_get".to_string())?;
    expect(application.created_at).to_equal(now)?;
    expect(application.modified_at).to_equal(now)?;
    expect(application.host_uri).to_equal("host_uri_get".to_string())?;
    expect(application.name).to_equal("".to_string())?;
    expect(application.info).to_equal(Map::<String, Value>::new())?;

    let item = Schema {
        application_id: "application_id_get_some".to_string(),
        code: "code_get_some".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        host_uri: "host_uri_get".to_string(),
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
        application_id: Some("application_id_get_some"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get some one".to_string()),
            Some(application) => application,
        },
    };
    expect(application.application_id).to_equal("application_id_get_some".to_string())?;
    expect(application.code).to_equal("code_get_some".to_string())?;
    expect(application.unit_id).to_equal("unit_id_get".to_string())?;
    expect(application.unit_code).to_equal("unit_code_get".to_string())?;
    expect(application.created_at).to_equal(now)?;
    expect(application.modified_at).to_equal(now)?;
    expect(application.host_uri).to_equal("host_uri_get".to_string())?;
    expect(application.name).to_equal("name_get".to_string())?;

    match application.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match application.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match application.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match application.info.get("object") {
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

/// Test `get()` by specifying a application code.
pub fn get_by_code(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        application_id: "application_id_get".to_string(),
        code: "code_get".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        host_uri: "host_uri_get".to_string(),
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
        Ok(application) => match application {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        code: Some("code_get"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get one".to_string()),
            Some(application) => application,
        },
    };
    expect(application.application_id).to_equal("application_id_get".to_string())?;
    expect(application.code).to_equal("code_get".to_string())?;
    expect(application.unit_id).to_equal("unit_id_get".to_string())?;
    expect(application.unit_code).to_equal("unit_code_get".to_string())?;
    expect(application.created_at).to_equal(now)?;
    expect(application.modified_at).to_equal(now)?;
    expect(application.host_uri).to_equal("host_uri_get".to_string())?;
    expect(application.name).to_equal("name_get".to_string())?;
    expect(application.info).to_equal(Map::<String, Value>::new())
}

/// Test `get()` by specifying a pair of unit ID and application ID.
pub fn get_by_unit_application(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_get = "application_id_get";
    let application_id_not_get = "application_id_not_get";
    let application_id_get_other = "application_id_get_other";
    let item = Schema {
        application_id: application_id_get.to_string(),
        code: "code_get".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        host_uri: "host_uri_get".to_string(),
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
        application_id: application_id_not_get.to_string(),
        code: "code_not_get".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        host_uri: "host_uri_get".to_string(),
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
        application_id: application_id_get_other.to_string(),
        code: "code_get".to_string(),
        unit_id: "unit_id_get_other".to_string(),
        unit_code: "unit_code_get_other".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        host_uri: "host_uri_get".to_string(),
        name: "name_get".to_string(),
        info: Document::new(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() get other error: {}", e));
    }

    let cond = QueryCond {
        application_id: Some(application_id_get),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    let application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get one".to_string()),
            Some(application) => application,
        },
    };
    if application.application_id.as_str() != application_id_get {
        return Err("get wrong application".to_string());
    }

    let cond = QueryCond {
        application_id: Some(application_id_get_other),
        unit_id: Some("unit_id_get"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err("should not get other one".to_string()),
        },
    }
    Ok(())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application = Application {
        application_id: "application_id_add_none".to_string(),
        code: "code_add_none".to_string(),
        unit_id: "unit_id_add_none".to_string(),
        unit_code: "unit_code_add_none".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&application).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let cond = QueryCond {
        application_id: Some(&application.application_id),
        ..Default::default()
    };
    let get_application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get none one".to_string()),
            Some(application) => application,
        },
    };
    expect(get_application).to_equal(application)?;

    let mut info = Map::<String, Value>::new();
    info.insert("boolean".to_string(), Value::Bool(true));
    info.insert("string".to_string(), Value::String("string".to_string()));
    info.insert("number".to_string(), Value::Number(1.into()));
    let info_object_array = vec![Value::String("array".to_string())];
    let mut info_object = Map::<String, Value>::new();
    info_object.insert("array".to_string(), Value::Array(info_object_array));
    info.insert("object".to_string(), Value::Object(info_object));
    let application = Application {
        application_id: "application_id_add_some".to_string(),
        code: "code_add_some".to_string(),
        unit_id: "unit_id_add_some".to_string(),
        unit_code: "unit_code_add_some".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_add".to_string(),
        name: "name_add".to_string(),
        info: info.clone(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&application).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let cond = QueryCond {
        application_id: Some(&application.application_id),
        ..Default::default()
    };
    let get_application = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(application) => match application {
            None => return Err("should get some one".to_string()),
            Some(application) => application,
        },
    };
    expect(get_application).to_equal(application)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let mut application = Application {
        application_id: "application_id_add".to_string(),
        code: "code_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&application).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    application.code = "code_not_exist".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&application).await }) {
        return Err("model.add() duplicate application_id should error".to_string());
    }
    application.application_id = "application_id_not_exist".to_string();
    application.code = "code_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&application).await }) {
        return Err("model.add() duplicate code should error".to_string());
    }
    application.application_id = "application_id_not_exist_another".to_string();
    application.unit_id = "unit_another".to_string();
    application.unit_code = "unit_code_another".to_string();
    if let Err(_) = runtime.block_on(async { model.add(&application).await }) {
        return Err("model.add() should not duplicate in another unit".to_string());
    }
    application.application_id = "application_id_not_exist_another2".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&application).await }) {
        return Err("model.add() duplicate code in another unit should error".to_string());
    }
    application.application_id = "application_id_not_exist_more".to_string();
    application.unit_id = "unit_more".to_string();
    application.unit_code = "unit_code_more".to_string();
    if let Err(_) = runtime.block_on(async { model.add(&application).await }) {
        return Err("model.add() should not duplicate in more another unit".to_string());
    }
    application.application_id = "application_id_not_exist_more2".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&application).await }) {
        return Err("model.add() duplicate code in more another unit should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying an application ID.
pub fn del_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_del = "application_id_del";
    let application_id_not_del = "application_id_not_del";
    let mut application = Application {
        application_id: application_id_del.to_string(),
        code: "code_del".to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        application_id: Some(application_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = application_id_not_del.to_string();
        application.code = "code_not_del".to_string();
        model.add(&application).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.application_id = Some(application_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(application) => match application {
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
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_del = "application_id_del";
    let application = Application {
        application_id: application_id_del.to_string(),
        code: "code_del".to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        application_id: Some(application_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_del1 = "application_id_del1";
    let application_id_del2 = "application_id_del2";
    let application_id_not_del = "application_id_not_del";
    let mut application = Application {
        application_id: application_id_del1.to_string(),
        code: "code_del".to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = application_id_del2.to_string();
        application.code = "code_del2".to_string();
        model.add(&application).await?;
        application.application_id = application_id_not_del.to_string();
        application.code = "code_not_del".to_string();
        application.unit_id = "unit_id_not_del".to_string();
        application.unit_code = "unit_code_not_del".to_string();
        model.add(&application).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        application_id: Some(application_id_del1),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete application1 error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err("delete application1 fail".to_string()),
        },
    }
    cond.application_id = Some(application_id_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete application2 error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err("delete application2 fail".to_string()),
        },
    }
    cond.application_id = Some(application_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(application) => match application {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a pair of unit ID and application ID.
pub fn del_by_unit_application(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_del = "application_id_del";
    let application_id_not_del = "application_id_not_del";
    let mut application = Application {
        application_id: application_id_del.to_string(),
        code: "code_del".to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        application_id: Some(application_id_del),
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = application_id_not_del.to_string();
        application.code = "code_not_del".to_string();
        model.add(&application).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(application) => match application {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.application_id = Some(application_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(application) => match application {
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
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let application_id_update = "application_id_update";
    let unit_id_update = "unit_id_update";
    let application = Application {
        application_id: application_id_update.to_string(),
        code: "code_update".to_string(),
        unit_id: unit_id_update.to_string(),
        unit_code: "unit_code_update".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_update".to_string(),
        name: "name_update".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&application).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_cond = QueryCond {
        application_id: Some(application_id_update),
        unit_id: Some(unit_id_update),
        ..Default::default()
    };
    let update_cond = UpdateQueryCond {
        application_id: application_id_update,
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
    let get_application = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() one error: {}", e)),
        Ok(application) => match application {
            None => return Err(format!("model.get() one should get one")),
            Some(application) => application,
        },
    };
    expect(get_application.application_id.as_str())
        .to_equal(application.application_id.as_str())?;
    expect(get_application.code.as_str()).to_equal(application.code.as_str())?;
    expect(get_application.unit_id.as_str()).to_equal(application.unit_id.as_str())?;
    expect(get_application.unit_code.as_str()).to_equal(application.unit_code.as_str())?;
    expect(get_application.created_at).to_equal(application.created_at)?;
    expect(get_application.modified_at).to_equal(now)?;
    expect(get_application.host_uri.as_str()).to_equal(application.host_uri.as_str())?;
    expect(get_application.name.as_str()).to_equal(application.name.as_str())?;
    expect(get_application.info).to_equal(application.info.clone())?;

    // Update all fields.
    let now = now + Duration::milliseconds(1);
    let mut info = Map::<String, Value>::new();
    info.insert("key".to_string(), Value::String("value".to_string()));
    let updates = Updates {
        modified_at: Some(now),
        host_uri: Some("host_uri_update_all"),
        name: Some("name_update_all"),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() all error: {}", e));
    }
    let get_application = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() all error: {}", e)),
        Ok(application) => match application {
            None => return Err(format!("model.get() all should get one")),
            Some(application) => application,
        },
    };
    expect(get_application.application_id.as_str())
        .to_equal(application.application_id.as_str())?;
    expect(get_application.code.as_str()).to_equal(application.code.as_str())?;
    expect(get_application.unit_id.as_str()).to_equal(application.unit_id.as_str())?;
    expect(get_application.unit_code.as_str()).to_equal(application.unit_code.as_str())?;
    expect(get_application.created_at).to_equal(application.created_at)?;
    expect(get_application.modified_at).to_equal(now)?;
    expect(get_application.host_uri.as_str()).to_equal("host_uri_update_all")?;
    expect(get_application.name.as_str()).to_equal("name_update_all")?;
    expect(get_application.info).to_equal(info)?;

    // Update all fields back to None.
    let now = now + Duration::milliseconds(1);
    let info = Map::<String, Value>::new();
    let updates = Updates {
        modified_at: Some(now),
        host_uri: Some("host_uri_update"),
        name: Some(""),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() none error: {}", e));
    }
    let get_application = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(application) => match application {
            None => return Err(format!("model.get() none should get one")),
            Some(application) => application,
        },
    };
    expect(get_application.application_id.as_str())
        .to_equal(application.application_id.as_str())?;
    expect(get_application.code.as_str()).to_equal(application.code.as_str())?;
    expect(get_application.unit_id.as_str()).to_equal(application.unit_id.as_str())?;
    expect(get_application.unit_code.as_str()).to_equal(application.unit_code.as_str())?;
    expect(get_application.created_at).to_equal(application.created_at)?;
    expect(get_application.modified_at).to_equal(now)?;
    expect(get_application.host_uri.as_str()).to_equal(application.host_uri.as_str())?;
    expect(get_application.name.as_str()).to_equal("")?;
    expect(get_application.info).to_equal(info)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let cond = UpdateQueryCond {
        application_id: "application_id_not_exist",
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
    let model = state.mongodb.as_ref().unwrap().application();

    let cond = UpdateQueryCond {
        application_id: "application_id",
    };
    let updates = Updates {
        modified_at: None,
        host_uri: None,
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
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let mut application = Application {
        application_id: "application_id_count1_1".to_string(),
        code: "code_count1_1".to_string(),
        unit_id: "unit_id_count".to_string(),
        unit_code: "unit_code_count".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_count_1".to_string(),
        name: "name_count_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = "application_id_count1_2".to_string();
        application.code = "code_count1_2".to_string();
        application.name = "name_count1_2".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_count2_1".to_string();
        application.code = "code_count2_1".to_string();
        application.name = "name_count2_1".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_count3_1".to_string();
        application.code = "code_count3_1".to_string();
        application.unit_id = "unit_id_count3".to_string();
        application.unit_code = "unit_code_count3".to_string();
        application.name = "name_count_1".to_string();
        model.add(&application).await
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
        application_id: Some("application_id_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count application_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        application_id: Some("application_id_count1_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count application-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        application_id: Some("application_id_count3_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count application3-unit result error: {}", e)),
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
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code-unit result error: {}", e)),
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
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        unit_id: Some("unit_id_count3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-unit3 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let mut application = Application {
        application_id: "application_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list_1".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = "application_id_list1_2".to_string();
        application.code = "code_list1_2".to_string();
        application.name = "name_list1_2".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_list2_1".to_string();
        application.code = "code_list2_1".to_string();
        application.name = "name_list2_1".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_list3_1".to_string();
        application.code = "code_list3_1".to_string();
        application.unit_id = "unit_id_list3".to_string();
        application.unit_code = "unit_code_list3".to_string();
        application.name = "name\\\\%%''_list_1".to_string();
        model.add(&application).await
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
        application_id: Some("application_id_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list application_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        application_id: Some("application_id_list1_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list application-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        application_id: Some("application_id_list3_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list application3-unit result error: {}", e)),
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
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-unit result error: {}", e)),
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
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        unit_id: Some("unit_id_list3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-unit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

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
    let model = state.mongodb.as_ref().unwrap().application();

    let mut now = Utc::now().trunc_subsecs(3);
    let mut application = Application {
        application_id: "application_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list1_1".to_string(),
        name: "name_list1_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        now = now + Duration::seconds(1);
        application.application_id = "application_id_list1_2".to_string();
        application.code = "code_list1_2".to_string();
        application.created_at = now;
        application.modified_at = now;
        application.name = "name_list1_2".to_string();
        model.add(&application).await?;
        now = now + Duration::seconds(1);
        application.application_id = "application_id_list2_1".to_string();
        application.code = "code_list2_1".to_string();
        application.created_at = now;
        application.modified_at = now;
        application.name = "name_list2_1".to_string();
        model.add(&application).await?;
        now = now + Duration::seconds(1);
        application.application_id = "application_id_list3_1".to_string();
        application.code = "code_list3_1".to_string();
        application.created_at = now;
        application.modified_at = now;
        application.name = "name_list2_1".to_string();
        model.add(&application).await
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
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let mut application = Application {
        application_id: "application_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list_1".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = "application_id_list1_2".to_string();
        application.code = "code_list1_2".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_list2_1".to_string();
        application.code = "code_list2_1".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_list3_1".to_string();
        application.code = "code_list3_1".to_string();
        model.add(&application).await
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
    let model = state.mongodb.as_ref().unwrap().application();

    let now = Utc::now().trunc_subsecs(3);
    let mut application = Application {
        application_id: "application_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list".to_string(),
        name: "name_list".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&application).await?;
        application.application_id = "application_id_list1_2".to_string();
        application.code = "code_list1_2".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_list2_1".to_string();
        application.code = "code_list2_1".to_string();
        model.add(&application).await?;
        application.application_id = "application_id_list3_1".to_string();
        application.code = "code_list3_1".to_string();
        model.add(&application).await
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
