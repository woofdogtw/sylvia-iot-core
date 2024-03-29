use std::collections::HashMap;

use chrono::{SubsecRound, TimeDelta, Utc};
use laboratory::expect;
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_auth::models::user::{
    ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, Updates, User, UserModel,
};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let user = User {
        user_id: "user_id_add_none".to_string(),
        account: "account_add_none".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_add".to_string(),
        salt: "salt_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&user).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let cond = QueryCond {
        user_id: Some(&user.user_id),
        ..Default::default()
    };
    let get_user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get none one".to_string()),
            Some(user) => user,
        },
    };
    expect(get_user).to_equal(user)?;

    let mut roles = HashMap::<String, bool>::new();
    roles.insert("role1".to_string(), true);
    roles.insert("role2".to_string(), false);
    let mut info = Map::<String, Value>::new();
    info.insert("boolean".to_string(), Value::Bool(true));
    info.insert("string".to_string(), Value::String("string".to_string()));
    info.insert("number".to_string(), Value::Number(1.into()));
    let info_object_array = vec![Value::String("array".to_string())];
    let mut info_object = Map::<String, Value>::new();
    info_object.insert("array".to_string(), Value::Array(info_object_array));
    info.insert("object".to_string(), Value::Object(info_object));
    let user = User {
        user_id: "user_id_add_some".to_string(),
        account: "account_add_some".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: Some(now),
        expired_at: Some(now),
        disabled_at: Some(now),
        roles: roles.clone(),
        password: "password_add".to_string(),
        salt: "salt_add".to_string(),
        name: "name_add".to_string(),
        info: info.clone(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&user).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let cond = QueryCond {
        user_id: Some(&user.user_id),
        ..Default::default()
    };
    let get_user = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(user) => match user {
            None => return Err("should get some one".to_string()),
            Some(user) => user,
        },
    };
    expect(get_user).to_equal(user)
}

/// Test `add()` with duplicate key.
pub fn add_dup(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut user = User {
        user_id: "user_id_add".to_string(),
        account: "account_add".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_add".to_string(),
        salt: "salt_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&user).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    user.account = "account_not_exist".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&user).await }) {
        return Err("model.add() duplicate user_id should error".to_string());
    }
    user.user_id = "user_id_not_exist".to_string();
    user.account = "account_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&user).await }) {
        return Err("model.add() duplicate account should error".to_string());
    }
    Ok(())
}

/// Test `del()`.
pub fn del(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let user_id_del = "user_id_del";
    let user_id_not_del = "user_id_not_del";
    let mut user = User {
        user_id: user_id_del.to_string(),
        account: "account_del".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_del".to_string(),
        salt: "salt_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        user.user_id = user_id_not_del.to_string();
        user.account = "account_not_del".to_string();
        model.add(&user).await?;
        model.del(user_id_del).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        user_id: Some(user_id_del),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(client) => match client {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.user_id = Some(user_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(client) => match client {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let user_id_del = "user_id_del";
    let user = User {
        user_id: user_id_del.to_string(),
        account: "account_del".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_del".to_string(),
        salt: "salt_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        model.del(user_id_del).await?;
        model.del(user_id_del).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `update()`.
pub fn update(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let user_id_update = "user_id_update";
    let user = User {
        user_id: user_id_update.to_string(),
        account: "account_update".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_update".to_string(),
        salt: "salt_update".to_string(),
        name: "name_update".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&user).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_cond = QueryCond {
        user_id: Some(user_id_update),
        ..Default::default()
    };

    // Update only one field.
    let now = now + TimeDelta::try_milliseconds(1).unwrap();
    let updates = Updates {
        modified_at: Some(now),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.update(user_id_update, &updates).await }) {
        return Err(format!("model.update() one error: {}", e));
    }
    let get_user = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() one error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("model.get() one should get one")),
            Some(client) => client,
        },
    };
    expect(get_user.user_id.as_str()).to_equal(user.user_id.as_str())?;
    expect(get_user.created_at).to_equal(user.created_at)?;
    expect(get_user.modified_at).to_equal(now)?;
    expect(get_user.verified_at).to_equal(user.verified_at)?;
    expect(get_user.expired_at).to_equal(user.expired_at)?;
    expect(get_user.disabled_at).to_equal(user.disabled_at)?;
    expect(get_user.roles).to_equal(user.roles.clone())?;
    expect(get_user.password.as_str()).to_equal(user.password.as_str())?;
    expect(get_user.salt.as_str()).to_equal(user.salt.as_str())?;
    expect(get_user.name.as_str()).to_equal(user.name.as_str())?;
    expect(get_user.info).to_equal(user.info.clone())?;

    // Update all fields.
    let now = now + TimeDelta::try_milliseconds(1).unwrap();
    let mut roles = HashMap::<String, bool>::new();
    roles.insert("role".to_string(), true);
    let mut info = Map::<String, Value>::new();
    info.insert("key".to_string(), Value::String("value".to_string()));
    let updates = Updates {
        modified_at: Some(now),
        verified_at: Some(now),
        expired_at: Some(Some(now)),
        disabled_at: Some(Some(now)),
        roles: Some(&roles),
        password: Some("password_update_all".to_string()),
        salt: Some("salt_update_all".to_string()),
        name: Some("name_update_all"),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(user_id_update, &updates).await }) {
        return Err(format!("model.update() all error: {}", e));
    }
    let get_user = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() all error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("model.get() all should get one")),
            Some(client) => client,
        },
    };
    expect(get_user.user_id.as_str()).to_equal(user.user_id.as_str())?;
    expect(get_user.created_at).to_equal(user.created_at)?;
    expect(get_user.modified_at).to_equal(now)?;
    expect(get_user.verified_at).to_equal(Some(now))?;
    expect(get_user.expired_at).to_equal(Some(now))?;
    expect(get_user.disabled_at).to_equal(Some(now))?;
    expect(get_user.roles).to_equal(roles)?;
    expect(get_user.password.as_str()).to_equal("password_update_all")?;
    expect(get_user.salt.as_str()).to_equal("salt_update_all")?;
    expect(get_user.name.as_str()).to_equal("name_update_all")?;
    expect(get_user.info).to_equal(info)?;

    // Update all fields back to None.
    let now = now + TimeDelta::try_milliseconds(1).unwrap();
    let roles = HashMap::<String, bool>::new();
    let info = Map::<String, Value>::new();
    let updates = Updates {
        modified_at: Some(now),
        verified_at: Some(now),
        expired_at: Some(None),
        disabled_at: Some(None),
        roles: Some(&roles),
        password: Some("password_update_none".to_string()),
        salt: Some("salt_update_none".to_string()),
        name: Some(""),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(user_id_update, &updates).await }) {
        return Err(format!("model.update() none error: {}", e));
    }
    let get_user = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(client) => match client {
            None => return Err(format!("model.get() none should get one")),
            Some(client) => client,
        },
    };
    expect(get_user.user_id.as_str()).to_equal(user.user_id.as_str())?;
    expect(get_user.created_at).to_equal(user.created_at)?;
    expect(get_user.modified_at).to_equal(now)?;
    expect(get_user.verified_at).to_equal(Some(now))?;
    expect(get_user.expired_at).to_equal(None)?;
    expect(get_user.disabled_at).to_equal(None)?;
    expect(get_user.roles).to_equal(roles)?;
    expect(get_user.password.as_str()).to_equal("password_update_none")?;
    expect(get_user.salt.as_str()).to_equal("salt_update_none")?;
    expect(get_user.name.as_str()).to_equal("")?;
    expect(get_user.info).to_equal(info)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let updates = Updates {
        modified_at: Some(Utc::now()),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.update("user_id_not_exist", &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `update()` with invalid update content.
pub fn update_invalid(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let updates = Updates {
        modified_at: None,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: None,
        password: None,
        salt: None,
        name: None,
        info: None,
    };
    if let Err(e) = runtime.block_on(async { model.update("user_id", &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut user = User {
        user_id: "user_id_count1_1".to_string(),
        account: "account_count1_1".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_count".to_string(),
        salt: "salt_count".to_string(),
        name: "name_count_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        user.user_id = "user_id_count1_2".to_string();
        user.account = "account_count1_2".to_string();
        user.verified_at = Some(now);
        user.name = "name_count1_2".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_count2_1".to_string();
        user.account = "account_count2_1".to_string();
        user.verified_at = Some(now);
        user.name = "name_count2_1".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_count3_1".to_string();
        user.account = "account_count3_1".to_string();
        user.verified_at = None;
        user.disabled_at = Some(now);
        user.name = "name_count_1".to_string();
        model.add(&user).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        user_id: Some("user_id_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        account: Some("account_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count account result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        account_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count account_contains result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        verified_at: Some(true),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count verified_at true result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        verified_at: Some(false),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count verified_at false result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        disabled_at: Some(true),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count disabled_at true result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        disabled_at: Some(false),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count disabled_at false result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        name_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name_contains result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        verified_at: Some(true),
        name_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count verified-name result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)
}

/// Test `list()`.
pub fn list(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut user = User {
        user_id: "user_id_list1_1".to_string(),
        account: "account_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_list".to_string(),
        salt: "salt_list".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        user.user_id = "user_id_list1_2".to_string();
        user.account = "account_list1_2".to_string();
        user.verified_at = Some(now);
        user.name = "name_list1_2".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_list2_1".to_string();
        user.account = "account_list2_1".to_string();
        user.verified_at = Some(now);
        user.name = "name_list2_1".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_list3_1".to_string();
        user.account = "account_list3_1".to_string();
        user.verified_at = None;
        user.disabled_at = Some(now);
        user.name = "name\\\\%%''_list_1".to_string();
        model.add(&user).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        user_id: Some("user_id_list1_1"),
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
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        account: Some("account_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list account result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        account_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list account_contains result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        verified_at: Some(true),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list verified_at true result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        verified_at: Some(false),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list verified_at false result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        disabled_at: Some(true),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list disabled_at true result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        disabled_at: Some(false),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list disabled_at false result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        name_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name_contains result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        verified_at: Some(true),
        name_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list verified-name result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        account_contains: Some("lIsT1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list account-case result error: {}", e)),
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
pub fn list_sort(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let mut now = Utc::now().trunc_subsecs(3);
    let mut user = User {
        user_id: "user_id_list1_1".to_string(),
        account: "account_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_list".to_string(),
        salt: "salt_list".to_string(),
        name: "name_list1_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        user.user_id = "user_id_list1_2".to_string();
        user.account = "account_list1_2".to_string();
        user.created_at = now;
        user.modified_at = now;
        user.verified_at = Some(now);
        user.expired_at = None;
        user.disabled_at = Some(now);
        user.name = "name_list1_2".to_string();
        model.add(&user).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        user.user_id = "user_id_list2_1".to_string();
        user.account = "account_list2_1".to_string();
        user.created_at = now;
        user.modified_at = now;
        user.verified_at = Some(now);
        user.expired_at = Some(now);
        user.disabled_at = None;
        user.name = "name_list2_1".to_string();
        model.add(&user).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        user.user_id = "user_id_list3_1".to_string();
        user.account = "account_list3_1".to_string();
        user.created_at = now;
        user.modified_at = now;
        user.expired_at = None;
        user.verified_at = None;
        user.disabled_at = Some(now);
        user.name = "name_list2_1".to_string(); // for sort testing
        model.add(&user).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Account,
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
        Err(e) => return Err(format!("list account-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Account,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list account-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list3_1")?;
    expect(list[1].account.as_str()).to_equal("account_list2_1")?;
    expect(list[2].account.as_str()).to_equal("account_list1_2")?;
    expect(list[3].account.as_str()).to_equal("account_list1_1")?;

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
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

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
    expect(list[0].account.as_str()).to_equal("account_list3_1")?;
    expect(list[1].account.as_str()).to_equal("account_list2_1")?;
    expect(list[2].account.as_str()).to_equal("account_list1_2")?;
    expect(list[3].account.as_str()).to_equal("account_list1_1")?;

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
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

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
    expect(list[0].account.as_str()).to_equal("account_list3_1")?;
    expect(list[1].account.as_str()).to_equal("account_list2_1")?;
    expect(list[2].account.as_str()).to_equal("account_list1_2")?;
    expect(list[3].account.as_str()).to_equal("account_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::VerifiedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list verified-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].verified_at).to_equal(None)?;
    expect(list[1].verified_at).to_equal(None)?;
    expect(list[2].account.as_str()).to_equal("account_list1_2")?;
    expect(list[3].account.as_str()).to_equal("account_list2_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::VerifiedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list verified-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list2_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].verified_at).to_equal(None)?;
    expect(list[3].verified_at).to_equal(None)?;

    let sort_cond = vec![SortCond {
        key: SortKey::ExpiredAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list expired-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].expired_at).to_equal(None)?;
    expect(list[1].expired_at).to_equal(None)?;
    expect(list[2].expired_at).to_equal(None)?;
    expect(list[3].account.as_str()).to_equal("account_list2_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ExpiredAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list expired-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list2_1")?;
    expect(list[1].expired_at).to_equal(None)?;
    expect(list[2].expired_at).to_equal(None)?;
    expect(list[3].expired_at).to_equal(None)?;

    let sort_cond = vec![SortCond {
        key: SortKey::DisabledAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list disabled-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].disabled_at).to_equal(None)?;
    expect(list[1].disabled_at).to_equal(None)?;
    expect(list[2].account.as_str()).to_equal("account_list1_2")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::DisabledAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list disabled-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list3_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].disabled_at).to_equal(None)?;
    expect(list[3].disabled_at).to_equal(None)?;

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
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

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
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list3_1")?;
    expect(list[3].account.as_str()).to_equal("account_list2_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut user = User {
        user_id: "user_id_list1_1".to_string(),
        account: "account_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_list".to_string(),
        salt: "salt_list".to_string(),
        name: "name_list".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        user.user_id = "user_id_list1_2".to_string();
        user.account = "account_list1_2".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_list2_1".to_string();
        user.account = "account_list2_1".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_list3_1".to_string();
        user.account = "account_list3_1".to_string();
        model.add(&user).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Account,
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
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].account.as_str()).to_equal("account_list2_1")?;
    expect(list[1].account.as_str()).to_equal("account_list3_1")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(list[3].account.as_str()).to_equal("account_list3_1")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].account.as_str()).to_equal("account_list3_1")
}

/// Test `list()` with cursors.
pub fn list_cursor(runtime: &Runtime, model: &dyn UserModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut user = User {
        user_id: "user_id_list1_1".to_string(),
        account: "account_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        verified_at: None,
        expired_at: None,
        disabled_at: None,
        roles: HashMap::<String, bool>::new(),
        password: "password_list".to_string(),
        salt: "salt_list".to_string(),
        name: "name_list".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&user).await?;
        user.user_id = "user_id_list1_2".to_string();
        user.account = "account_list1_2".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_list2_1".to_string();
        user.account = "account_list2_1".to_string();
        model.add(&user).await?;
        user.user_id = "user_id_list3_1".to_string();
        user.account = "account_list3_1".to_string();
        model.add(&user).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Account,
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
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(list[2].account.as_str()).to_equal("account_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].account.as_str()).to_equal("account_list3_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].account.as_str()).to_equal("account_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].account.as_str()).to_equal("account_list1_1")?;
    expect(list[1].account.as_str()).to_equal("account_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].account.as_str()).to_equal("account_list2_1")?;
    expect(list[1].account.as_str()).to_equal("account_list3_1")?;
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
    expect(list[0].account.as_str()).to_equal("account_list2_1")?;
    expect(list[1].account.as_str()).to_equal("account_list3_1")?;
    expect(cursor.is_none()).to_equal(true)
}
