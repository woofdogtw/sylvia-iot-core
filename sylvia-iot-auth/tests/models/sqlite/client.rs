use std::collections::HashMap;

use chrono::{Duration, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use sql_builder::{quote, SqlBuilder};

use sylvia_iot_auth::models::{
    client::{
        Client, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, UpdateQueryCond, Updates,
    },
    Model,
};

use super::{TestState, STATE};

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

    let now = Utc::now().trunc_subsecs(3);
    let client = Client {
        client_id: "client_id_add_none".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_add".to_string(),
        name: "name_add".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&client).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let cond = QueryCond {
        client_id: Some(&client.client_id),
        ..Default::default()
    };
    let get_client = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(client) => match client {
            None => return Err("should get none one".to_string()),
            Some(client) => client,
        },
    };
    expect(get_client).to_equal(client)?;

    let now = Utc::now().trunc_subsecs(3);
    let client = Client {
        client_id: "client_id_add_some".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: Some("secret_add".to_string()),
        redirect_uris: vec!["uri1".to_string(), "uri2".to_string()],
        scopes: vec!["scope1".to_string(), "scope2".to_string()],
        user_id: "user_id_add".to_string(),
        name: "name_add".to_string(),
        image_url: Some("image_url_add".to_string()),
    };
    if let Err(e) = runtime.block_on(async { model.add(&client).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let cond = QueryCond {
        client_id: Some(&client.client_id),
        ..Default::default()
    };
    let get_client = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(client) => match client {
            None => return Err("should get some one".to_string()),
            Some(client) => client,
        },
    };
    expect(get_client).to_equal(client)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client = Client {
        client_id: "client_id_add".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_add".to_string(),
        name: "name_add".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&client).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&client).await }) {
        return Err("model.add() duplicate should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_del = "client_id_del";
    let client_id_not_del = "client_id_not_del";
    let mut client = Client {
        client_id: client_id_del.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_del".to_string(),
        name: "name_del".to_string(),
        image_url: None,
    };
    let mut cond = QueryCond {
        client_id: Some(client_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = client_id_not_del.to_string();
        model.add(&client).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(client) => match client {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.client_id = Some(client_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(client) => match client {
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
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_del = "client_id_del";
    let client = Client {
        client_id: client_id_del.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_del".to_string(),
        name: "name_del".to_string(),
        image_url: None,
    };
    let cond = QueryCond {
        client_id: Some(client_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_del1 = "client_id_del1";
    let client_id_del2 = "client_id_del2";
    let client_id_not_del = "client_id_not_del";
    let mut client = Client {
        client_id: client_id_del1.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_del".to_string(),
        name: "name_del".to_string(),
        image_url: None,
    };
    let cond = QueryCond {
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = client_id_del2.to_string();
        model.add(&client).await?;
        client.client_id = client_id_not_del.to_string();
        client.user_id = "user_id_not_del".to_string();
        model.add(&client).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        client_id: Some(client_id_del1),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete client1 error: {}", e)),
        Ok(client) => match client {
            None => (),
            Some(_) => return Err("delete client1 fail".to_string()),
        },
    }
    cond.client_id = Some(client_id_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete client2 error: {}", e)),
        Ok(client) => match client {
            None => (),
            Some(_) => return Err("delete client2 fail".to_string()),
        },
    }
    cond.client_id = Some(client_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(client) => match client {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a pair of user ID and client ID.
pub fn del_by_user_client(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_del = "client_id_del";
    let client_id_not_del = "client_id_not_del";
    let mut client = Client {
        client_id: client_id_del.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_del".to_string(),
        name: "name_del".to_string(),
        image_url: None,
    };
    let cond = QueryCond {
        client_id: Some(client_id_del),
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = client_id_not_del.to_string();
        model.add(&client).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        client_id: Some(client_id_del),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(client) => match client {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.client_id = Some(client_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(client) => match client {
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
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let client_id_update = "client_id_update";
    let user_id_update = "user_id_update";
    let client = Client {
        client_id: client_id_update.to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: user_id_update.to_string(),
        name: "name_update".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&client).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_cond = QueryCond {
        client_id: Some(client_id_update),
        user_id: Some(user_id_update),
    };
    let update_cond = UpdateQueryCond {
        user_id: user_id_update,
        client_id: client_id_update,
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
    let get_client = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() one error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("model.get() one should get one")),
            Some(client) => client,
        },
    };
    expect(get_client.client_id.as_str()).to_equal(client.client_id.as_str())?;
    expect(get_client.created_at).to_equal(client.created_at)?;
    expect(get_client.modified_at).to_equal(now)?;
    expect(get_client.client_secret).to_equal(client.client_secret.clone())?;
    expect(get_client.redirect_uris.as_slice()).to_equal(client.redirect_uris.as_slice())?;
    expect(get_client.scopes.as_slice()).to_equal(client.scopes.as_slice())?;
    expect(get_client.user_id.as_str()).to_equal(client.user_id.as_str())?;
    expect(get_client.name.as_str()).to_equal(client.name.as_str())?;
    expect(get_client.image_url.as_ref()).to_equal(client.image_url.as_ref())?;

    // Update all fields.
    let now = now + Duration::milliseconds(1);
    let redirect_uris = vec!["url_update_all1".to_string(), "url_update_all2".to_string()];
    let scopes = vec!["scope_update1".to_string(), "scope_update2".to_string()];
    let updates = Updates {
        modified_at: Some(now),
        client_secret: Some(Some("secret_update_all".to_string())),
        redirect_uris: Some(&redirect_uris),
        scopes: Some(&scopes),
        name: Some("name_update_all"),
        image_url: Some(Some("image_update_all")),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() all error: {}", e));
    }
    let get_client = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() all error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("model.get() all should get one")),
            Some(client) => client,
        },
    };
    expect(get_client.client_id.as_str()).to_equal(client.client_id.as_str())?;
    expect(get_client.created_at).to_equal(client.created_at)?;
    expect(get_client.modified_at).to_equal(now)?;
    expect(get_client.client_secret).to_equal(Some("secret_update_all".to_string()))?;
    expect(get_client.redirect_uris).to_equal(redirect_uris)?;
    expect(get_client.scopes).to_equal(scopes)?;
    expect(get_client.user_id).to_equal(client.user_id.clone())?;
    expect(get_client.name.as_str()).to_equal("name_update_all")?;
    expect(get_client.image_url).to_equal(Some("image_update_all".to_string()))?;

    // Update all fields back to None.
    let now = now + Duration::milliseconds(1);
    let redirect_uris = vec![];
    let scopes = vec![];
    let updates = Updates {
        modified_at: Some(now),
        client_secret: Some(None),
        redirect_uris: Some(&redirect_uris),
        scopes: Some(&scopes),
        name: Some(""),
        image_url: Some(None),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() none error: {}", e));
    }
    let get_client = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("model.get() none should get one")),
            Some(client) => client,
        },
    };
    expect(get_client.client_id.as_str()).to_equal(client.client_id.as_str())?;
    expect(get_client.created_at).to_equal(client.created_at)?;
    expect(get_client.modified_at).to_equal(now)?;
    expect(get_client.client_secret).to_equal(None)?;
    expect(get_client.redirect_uris).to_equal(redirect_uris)?;
    expect(get_client.scopes).to_equal(scopes)?;
    expect(get_client.user_id.as_str()).to_equal(client.user_id.as_str())?;
    expect(get_client.name.as_str()).to_equal("")?;
    expect(get_client.image_url).to_equal(None)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let cond = UpdateQueryCond {
        user_id: "user_id_not_exist",
        ..Default::default()
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
    let model = state.sqlite.as_ref().unwrap().client();

    let cond = UpdateQueryCond {
        user_id: "user_id",
        ..Default::default()
    };
    let updates = Updates {
        modified_at: None,
        client_secret: None,
        redirect_uris: None,
        scopes: None,
        name: None,
        image_url: None,
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
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let mut client = Client {
        client_id: "client_id_count1_1".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_count1".to_string(),
        name: "name_count1_1".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = "client_id_count1_2".to_string();
        client.name = "name_count1_2".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_count2_1".to_string();
        client.user_id = "user_id_count2".to_string();
        client.name = "name_count2_1".to_string();
        model.add(&client).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        user_id: Some("user_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        client_id: Some("client_id_count1_2"),
        user_id: Some("user_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user-client1 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        client_id: Some("client_id_count2_1"),
        user_id: Some("user_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user-client2 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        name_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        user_id: Some("user_id_count2"),
        name_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user-name1 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        user_id: Some("user_id_count2"),
        name_contains: Some("_2"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user-name2 result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let mut client = Client {
        client_id: "client_id_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_list1".to_string(),
        name: "name_list1_1".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = "client_id_list1_2".to_string();
        client.name = "name_list1_2".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_1".to_string();
        client.user_id = "user_id_list2".to_string();
        client.name = "name_\\\\%%''_list2_1".to_string();
        model.add(&client).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        user_id: Some("user_id_list1"),
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
        Err(e) => return Err(format!("list user_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        client_id: Some("client_id_list1_2"),
        user_id: Some("user_id_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list user-client1 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        client_id: Some("client_id_list2_1"),
        user_id: Some("user_id_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list user-client2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        name_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        user_id: Some("user_id_list2"),
        name_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list user-name1 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        user_id: Some("user_id_list2"),
        name_contains: Some("_2"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list user-name2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        name_contains: Some("lIsT1"),
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
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let mut client = Client {
        client_id: "client_id_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_list1".to_string(),
        name: "name_list1_1".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = "client_id_list1_2".to_string();
        client.created_at = now + Duration::seconds(2);
        client.modified_at = now - Duration::seconds(2);
        client.name = "name_list1_2".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_1".to_string();
        client.created_at = now + Duration::seconds(1);
        client.modified_at = now - Duration::seconds(1);
        client.user_id = "user_id_list2".to_string();
        client.name = "name_list2_1".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_2".to_string();
        client.created_at = now + Duration::seconds(3);
        client.modified_at = now - Duration::seconds(3);
        client.name = "name_list2_1".to_string(); // for sort testing
        model.add(&client).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Name,
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
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list2_2")?;

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
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_2")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list1_1")?;

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
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_2")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list2_2")?;

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
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list2_2")?;

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
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_2")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list2_1")?;

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
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let mut client = Client {
        client_id: "client_id_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_list1".to_string(),
        name: "name_list1_1".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = "client_id_list1_2".to_string();
        client.name = "name_list1_2".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_1".to_string();
        client.user_id = "user_id_list2".to_string();
        client.name = "name_list2_1".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_2".to_string();
        client.name = "name_list2_2".to_string();
        model.add(&client).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
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
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list2_2")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list2_2")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[3].client_id.as_str()).to_equal("client_id_list2_2")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_2")
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().client();

    let now = Utc::now().trunc_subsecs(3);
    let mut client = Client {
        client_id: "client_id_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        client_secret: None,
        redirect_uris: vec![],
        scopes: vec![],
        user_id: "user_id_list1".to_string(),
        name: "name_list1_1".to_string(),
        image_url: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&client).await?;
        client.client_id = "client_id_list1_2".to_string();
        client.name = "name_list1_2".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_1".to_string();
        client.user_id = "user_id_list2".to_string();
        client.name = "name_list2_1".to_string();
        model.add(&client).await?;
        client.client_id = "client_id_list2_2".to_string();
        client.name = "name_list2_2".to_string();
        model.add(&client).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Name,
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
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(list[2].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_2")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list1_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list2_2")?;
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
    expect(list[0].client_id.as_str()).to_equal("client_id_list2_1")?;
    expect(list[1].client_id.as_str()).to_equal("client_id_list2_2")?;
    expect(cursor.is_none()).to_equal(true)
}
