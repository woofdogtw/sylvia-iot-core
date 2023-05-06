use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use sql_builder::{quote, SqlBuilder};

use sylvia_iot_auth::models::{client::QueryCond, Model};

use super::{super::common::client as common_test, TestState, STATE};

const TABLE_NAME: &'static str = "client";
const FIELDS: &'static [&'static str] = &[
    "client_id",
    "created_at",
    "modified_at",
    "client_secret",
    "redirect_uris",
    "scopes",
    "user_id",
    "name",
    "image_url",
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
    let model = state.sqlite.as_ref().unwrap().client();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()` by specifying a client ID.
pub fn get_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("client_id_get_none"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            quote(""),
            quote(""),
            quote("user_id_get"),
            quote(""),
            "NULL".to_string(),
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

    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote("client_id_get_some"),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            quote("secret_get"),
            quote("uri1 uri2"),
            quote("scope1 scope2"),
            quote("user_id_get"),
            quote("name_get"),
            quote("image_url_get"),
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
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_get = "client_id_get";
    let client_id_not_get = "client_id_not_get";
    let client_id_get_other = "client_id_get_other";
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(client_id_get),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            quote(""),
            quote(""),
            quote("user_id_get"),
            quote("name_get"),
            "NULL".to_string(),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() get error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(client_id_not_get),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            quote(""),
            quote(""),
            quote("user_id_get"),
            quote("name_get"),
            "NULL".to_string(),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() not-get error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() not-get error: {}", e.to_string()));
    }
    let sql = match SqlBuilder::insert_into(TABLE_NAME)
        .fields(FIELDS)
        .values(&vec![
            quote(client_id_get_other),
            now.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
            "NULL".to_string(),
            quote(""),
            quote(""),
            quote("user_id_get_other"),
            quote("name_get"),
            "NULL".to_string(),
        ])
        .sql()
    {
        Err(e) => return Err(format!("sql() other error: {}", e.to_string())),
        Ok(sql) => sql,
    };
    if let Err(e) = runtime.block_on(async { sqlx::query(&sql).execute(conn).await }) {
        return Err(format!("insert_into() other error: {}", e.to_string()));
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
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::add(runtime, model)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::add_dup(runtime, model)
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::del_by_client_id(runtime, model)
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::del_twice(runtime, model)
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::del_by_user_id(runtime, model)
}

/// Test `del()` by specifying a pair of user ID and client ID.
pub fn del_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::del_by_user_client(runtime, model)
}

/// Test `update()`.
pub fn update(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::update(runtime, model)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::update_not_exist(runtime, model)
}

/// Test `update()` with invalid update content.
pub fn update_invalid(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::update_invalid(runtime, model)
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::count(runtime, model)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::list(runtime, model)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::list_sort(runtime, model)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::list_offset_limit(runtime, model)
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    common_test::list_cursor(runtime, model)
}
