use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_auth::models::{
    authorization_code::{AuthorizationCode, QueryCond},
    Model,
};

use super::{TestState, STATE};

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    code: String,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    scope: Option<String>,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "authorizationCode";

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
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        code: "code_get_none".to_string(),
        expires_at: now.into(),
        redirect_uri: "redirect_uri_get".to_string(),
        scope: None,
        client_id: "client_id_get".to_string(),
        user_id: "user_id_get".to_string(),
        created_at: now.into(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() none error: {}", e));
    }

    match runtime.block_on(async { model.get("code_get_not_exist").await }) {
        Err(e) => return Err(format!("model.get() not-exist error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let code = match runtime.block_on(async { model.get("code_get_none").await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(code) => match code {
            None => return Err("should get none one".to_string()),
            Some(code) => code,
        },
    };
    expect(code.code).to_equal("code_get_none".to_string())?;
    expect(code.expires_at).to_equal(now)?;
    expect(code.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(code.scope).to_equal(None)?;
    expect(code.client_id).to_equal("client_id_get".to_string())?;
    expect(code.user_id).to_equal("user_id_get".to_string())?;

    let item = Schema {
        code: "code_get_some".to_string(),
        expires_at: now.into(),
        redirect_uri: "redirect_uri_get".to_string(),
        scope: Some("scope_get".to_string()),
        client_id: "client_id_get".to_string(),
        user_id: "user_id_get".to_string(),
        created_at: now.into(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() some error: {}", e));
    }

    let code = match runtime.block_on(async { model.get("code_get_some").await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(code) => match code {
            None => return Err("should get some one".to_string()),
            Some(code) => code,
        },
    };
    expect(code.code).to_equal("code_get_some".to_string())?;
    expect(code.expires_at).to_equal(now)?;
    expect(code.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(code.scope).to_equal(Some("scope_get".to_string()))?;
    expect(code.client_id).to_equal("client_id_get".to_string())?;
    expect(code.user_id).to_equal("user_id_get".to_string())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code = AuthorizationCode {
        code: "code_add_none".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_add".to_string(),
        scope: None,
        client_id: "client_id_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&code).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let get_code = match runtime.block_on(async { model.get(&code.code).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(code) => match code {
            None => return Err("should get none one".to_string()),
            Some(code) => code,
        },
    };
    expect(get_code).to_equal(code)?;

    let code = AuthorizationCode {
        code: "code_add_some".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_add".to_string(),
        scope: Some("scope".to_string()),
        client_id: "client_id_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&code).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let get_code = match runtime.block_on(async { model.get(&code.code).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(code) => match code {
            None => return Err("should get some one".to_string()),
            Some(code) => code,
        },
    };
    expect(get_code).to_equal(code)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code = AuthorizationCode {
        code: "code_add".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_add".to_string(),
        scope: None,
        client_id: "client_id_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&code).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&code).await }) {
        return Err("model.add() duplicate should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying an authorization code.
pub fn del_by_code(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code_del = "code_del";
    let code_not_del = "code_not_del";
    let mut code = AuthorizationCode {
        code: code_del.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_del".to_string(),
        scope: None,
        client_id: "client_id_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        code: Some(code_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&code).await?;
        code.code = code_not_del.to_string();
        model.add(&code).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(code_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(code_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(code) => match code {
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
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code_del = "code_del";
    let code = AuthorizationCode {
        code: code_del.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_del".to_string(),
        scope: None,
        client_id: "client_id_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        code: Some(code_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&code).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a client ID.
pub fn del_by_client_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code_del1 = "code_del1";
    let code_del2 = "code_del2";
    let code_not_del = "code_not_del";
    let mut code = AuthorizationCode {
        code: code_del1.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_del".to_string(),
        scope: None,
        client_id: "client_id_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        client_id: Some("client_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&code).await?;
        code.code = code_del2.to_string();
        model.add(&code).await?;
        code.code = code_not_del.to_string();
        code.client_id = "client_id_not_del".to_string();
        model.add(&code).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(code_del1).await }) {
        Err(e) => return Err(format!("model.get() delete code1 error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err("delete code1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(code_del2).await }) {
        Err(e) => return Err(format!("model.get() delete code2 error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err("delete code2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(code_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(code) => match code {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code_del1 = "code_del1";
    let code_del2 = "code_del2";
    let code_not_del = "code_not_del";
    let mut code = AuthorizationCode {
        code: code_del1.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_del".to_string(),
        scope: None,
        client_id: "client_id_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&code).await?;
        code.code = code_del2.to_string();
        model.add(&code).await?;
        code.code = code_not_del.to_string();
        code.user_id = "user_id_not_del".to_string();
        model.add(&code).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(code_del1).await }) {
        Err(e) => return Err(format!("model.get() delete code1 error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err("delete code1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(code_del2).await }) {
        Err(e) => return Err(format!("model.get() delete code2 error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err("delete code2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(code_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(code) => match code {
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
    let model = state.mongodb.as_ref().unwrap().authorization_code();

    let code_del = "code_del";
    let code_not_del = "code_not_del";
    let mut code = AuthorizationCode {
        code: code_del.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        redirect_uri: "redirect_uri_del".to_string(),
        scope: None,
        client_id: "client_id_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        client_id: Some("client_id_del"),
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&code).await?;
        code.code = code_not_del.to_string();
        code.user_id = "user_id_not_del".to_string();
        model.add(&code).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(code_del).await }) {
        Err(e) => return Err(format!("model.get() delete one error: {}", e)),
        Ok(code) => match code {
            None => (),
            Some(_) => return Err("delete one fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(code_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(code) => match code {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}
