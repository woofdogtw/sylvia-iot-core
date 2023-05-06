use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{doc, DateTime, Document};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use sylvia_iot_auth::models::{user::QueryCond, Model};

use super::{super::common::user as common_test, TestState, STATE};

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub account: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    pub modified_at: DateTime,
    #[serde(rename = "verifiedAt")]
    pub verified_at: Option<DateTime>,
    #[serde(rename = "expiredAt")]
    pub expired_at: Option<DateTime>,
    #[serde(rename = "disabledAt")]
    pub disabled_at: Option<DateTime>,
    pub roles: HashMap<String, bool>,
    pub password: String,
    pub salt: String,
    pub name: String,
    info: Document,
}

const COL_NAME: &'static str = "user";

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
    let model = state.mongodb.as_ref().unwrap().user();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a user ID.
pub fn get_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().user();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        user_id: "user_id_get_none".to_string(),
        account: "account_get_none".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_get".to_string(),
        salt: "salt_get".to_string(),
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
        user_id: Some("user_id_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(user) => match user {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        user_id: Some("user_id_get_none"),
        ..Default::default()
    };
    let user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get none one".to_string()),
            Some(user) => user,
        },
    };
    expect(user.user_id).to_equal("user_id_get_none".to_string())?;
    expect(user.account).to_equal("account_get_none".to_string())?;
    expect(user.created_at).to_equal(now)?;
    expect(user.modified_at).to_equal(now)?;
    expect(user.verified_at).to_equal(None)?;
    expect(user.expired_at).to_equal(None)?;
    expect(user.disabled_at).to_equal(None)?;
    expect(user.roles).to_equal(HashMap::<String, bool>::new())?;
    expect(user.password).to_equal("password_get".to_string())?;
    expect(user.salt).to_equal("salt_get".to_string())?;
    expect(user.name).to_equal("".to_string())?;
    expect(user.info).to_equal(Map::<String, Value>::new())?;

    let mut roles = HashMap::<String, bool>::new();
    roles.insert("role1".to_string(), true);
    roles.insert("role2".to_string(), false);
    let item = Schema {
        user_id: "user_id_get_some".to_string(),
        account: "account_get_some".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        verified_at: Some(now.into()),
        expired_at: Some(now.into()),
        disabled_at: Some(now.into()),
        roles: roles.clone(),
        password: "password_get".to_string(),
        salt: "salt_get".to_string(),
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
        user_id: Some("user_id_get_some"),
        ..Default::default()
    };
    let user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get some one".to_string()),
            Some(user) => user,
        },
    };
    expect(user.user_id).to_equal("user_id_get_some".to_string())?;
    expect(user.account).to_equal("account_get_some".to_string())?;
    expect(user.created_at).to_equal(now)?;
    expect(user.modified_at).to_equal(now)?;
    expect(user.verified_at).to_equal(Some(now))?;
    expect(user.expired_at).to_equal(Some(now))?;
    expect(user.disabled_at).to_equal(Some(now))?;
    expect(user.roles).to_equal(roles)?;
    expect(user.password).to_equal("password_get".to_string())?;
    expect(user.salt).to_equal("salt_get".to_string())?;
    expect(user.name).to_equal("name_get".to_string())?;

    match user.info.get("boolean") {
        Some(Value::Bool(v)) => match *v {
            false => (),
            true => return Err("wrong info.boolean value".to_string()),
        },
        _ => return Err("wrong info.boolean type".to_string()),
    }
    match user.info.get("string") {
        Some(Value::String(v)) => match v.as_str() {
            "string" => (),
            _ => return Err("wrong info.string value".to_string()),
        },
        _ => return Err("wrong info.string type".to_string()),
    }
    match user.info.get("number") {
        Some(Value::Number(v)) => match v.as_i64() {
            Some(1) => (),
            _ => return Err("wrong info.number value".to_string()),
        },
        _ => return Err("wrong info.number type".to_string()),
    }
    match user.info.get("object") {
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

/// Test `get()` by specifying an account.
pub fn get_by_account(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().user();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        user_id: "user_id_get".to_string(),
        account: "account_get".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_get".to_string(),
        salt: "salt_get".to_string(),
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
        account: Some("account_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(user) => match user {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        account: Some("account_get"),
        ..Default::default()
    };
    let user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get one".to_string()),
            Some(user) => user,
        },
    };
    expect(user.user_id).to_equal("user_id_get".to_string())?;
    expect(user.account).to_equal("account_get".to_string())?;
    expect(user.created_at).to_equal(now)?;
    expect(user.modified_at).to_equal(now)?;
    expect(user.verified_at).to_equal(None)?;
    expect(user.expired_at).to_equal(None)?;
    expect(user.disabled_at).to_equal(None)?;
    expect(user.roles).to_equal(HashMap::<String, bool>::new())?;
    expect(user.password).to_equal("password_get".to_string())?;
    expect(user.salt).to_equal("salt_get".to_string())?;
    expect(user.name).to_equal("name_get".to_string())?;
    expect(user.info).to_equal(Map::<String, Value>::new())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::add_dup(runtime, model)
}

/// Test `del()`.
pub fn del(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::del(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::del_twice(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().user();

    common_test::list_cursor(runtime, model)
}
