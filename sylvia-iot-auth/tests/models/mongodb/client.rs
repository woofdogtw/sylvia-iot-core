use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{SpecContext, expect};
use mongodb::bson::{DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_auth::models::{Model, client::QueryCond};

use super::{super::common::client as common_test, STATE, TestState};

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "modifiedAt")]
    modified_at: DateTime,
    #[serde(rename = "clientSecret")]
    client_secret: Option<String>,
    #[serde(rename = "redirectUris")]
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
    #[serde(rename = "userId")]
    user_id: String,
    name: String,
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

const COL_NAME: &'static str = "client";

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
    let model = state.mongodb.as_ref().unwrap().client();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a client ID.
pub fn get_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        client_id: "client_id_get_none".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_get".to_string(),
        name: "".to_string(),
        image_url: None,
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() none error: {}", e));
    }

    let cond = QueryCond {
        client_id: Some("client_id_get_not_exist"),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(client) => match client {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let cond = QueryCond {
        client_id: Some("client_id_get_none"),
        ..Default::default()
    };
    let client = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(client) => match client {
            None => return Err("should get none one".to_string()),
            Some(client) => client,
        },
    };
    expect(client.client_id).to_equal("client_id_get_none".to_string())?;
    expect(client.created_at).to_equal(now)?;
    expect(client.modified_at).to_equal(now)?;
    expect(client.client_secret).to_equal(None)?;
    expect(client.redirect_uris.len()).to_equal(0)?;
    expect(client.scopes.len()).to_equal(0)?;
    expect(client.user_id).to_equal("user_id_get".to_string())?;
    expect(client.name).to_equal("".to_string())?;
    expect(client.image_url).to_equal(None)?;

    let item = Schema {
        client_id: "client_id_get_some".to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        client_secret: Some("secret_get".to_string()),
        redirect_uris: vec!["uri1".to_string(), "uri2".to_string()],
        scopes: vec!["scope1".to_string(), "scope2".to_string()],
        user_id: "user_id_get".to_string(),
        name: "name_get".to_string(),
        image_url: Some("image_url_get".to_string()),
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() some error: {}", e));
    }

    let cond = QueryCond {
        client_id: Some("client_id_get_some"),
        ..Default::default()
    };
    let client = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(client) => match client {
            None => return Err("should get some one".to_string()),
            Some(client) => client,
        },
    };
    expect(client.client_id).to_equal("client_id_get_some".to_string())?;
    expect(client.created_at).to_equal(now)?;
    expect(client.modified_at).to_equal(now)?;
    expect(client.client_secret).to_equal(Some("secret_get".to_string()))?;
    expect(client.redirect_uris).to_equal(vec!["uri1".to_string(), "uri2".to_string()])?;
    expect(client.scopes).to_equal(vec!["scope1".to_string(), "scope2".to_string()])?;
    expect(client.user_id).to_equal("user_id_get".to_string())?;
    expect(client.name).to_equal("name_get".to_string())?;
    expect(client.image_url).to_equal(Some("image_url_get".to_string()))
}

/// Test `get()` by specifying a pair of user ID and client ID.
pub fn get_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_get = "client_id_get";
    let client_id_not_get = "client_id_not_get";
    let client_id_get_other = "client_id_get_other";
    let item = Schema {
        client_id: client_id_get.to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_get".to_string(),
        name: "name_get".to_string(),
        image_url: None,
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() get error: {}", e));
    }
    let item = Schema {
        client_id: client_id_not_get.to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_get".to_string(),
        name: "name_get".to_string(),
        image_url: None,
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() not-get error: {}", e));
    }
    let item = Schema {
        client_id: client_id_get_other.to_string(),
        created_at: now.into(),
        modified_at: now.into(),
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_get_other".to_string(),
        name: "name_get".to_string(),
        image_url: None,
    };
    if let Err(e) =
        runtime.block_on(async { conn.collection::<Schema>(COL_NAME).insert_one(item).await })
    {
        return Err(format!("insert_one() other error: {}", e));
    }

    let cond = QueryCond {
        client_id: Some(client_id_get),
        user_id: Some("user_id_get"),
    };
    let client = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(client) => match client {
            None => return Err("should get one".to_string()),
            Some(client) => client,
        },
    };
    if client.client_id.as_str() != client_id_get {
        return Err("get wrong client".to_string());
    }

    let cond = QueryCond {
        client_id: Some(client_id_get_other),
        user_id: Some("user_id_get"),
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() other error: {}", e)),
        Ok(client) => match client {
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
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::del_by_client_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::del_by_user_id(runtime, model)
}

/// Test `del()` by specifying a pair of user ID and client ID.
pub fn del_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::del_by_user_client(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().client();

    common_test::list_cursor(runtime, model)
}
