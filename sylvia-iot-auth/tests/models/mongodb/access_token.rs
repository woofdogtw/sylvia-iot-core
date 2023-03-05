use std::collections::HashMap;

use chrono::{SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_auth::models::{
    access_token::{AccessToken, QueryCond},
    Model,
};

use super::{TestState, STATE};

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken", skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    scope: Option<String>,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
}

const COL_NAME: &'static str = "accessToken";

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
    let model = state.mongodb.as_ref().unwrap().access_token();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().access_token();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        access_token: "token_get_none".to_string(),
        refresh_token: None,
        expires_at: now.into(),
        scope: None,
        client_id: "client_id_get".to_string(),
        redirect_uri: "redirect_uri_get".to_string(),
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

    match runtime.block_on(async { model.get("token_get_not_exist").await }) {
        Err(e) => return Err(format!("model.get() not-exist error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let token = match runtime.block_on(async { model.get("token_get_none").await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get none one".to_string()),
            Some(token) => token,
        },
    };
    expect(token.access_token).to_equal("token_get_none".to_string())?;
    expect(token.refresh_token).to_equal(None)?;
    expect(token.expires_at).to_equal(now)?;
    expect(token.scope).to_equal(None)?;
    expect(token.client_id).to_equal("client_id_get".to_string())?;
    expect(token.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(token.user_id).to_equal("user_id_get".to_string())?;

    let item = Schema {
        access_token: "token_get_some".to_string(),
        refresh_token: Some("token_get".to_string()),
        expires_at: now.into(),
        scope: Some("scope_get".to_string()),
        client_id: "client_id_get".to_string(),
        redirect_uri: "redirect_uri_get".to_string(),
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

    let token = match runtime.block_on(async { model.get("token_get_some").await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get some one".to_string()),
            Some(token) => token,
        },
    };
    expect(token.access_token).to_equal("token_get_some".to_string())?;
    expect(token.refresh_token).to_equal(Some("token_get".to_string()))?;
    expect(token.expires_at).to_equal(now)?;
    expect(token.scope).to_equal(Some("scope_get".to_string()))?;
    expect(token.client_id).to_equal("client_id_get".to_string())?;
    expect(token.redirect_uri).to_equal("redirect_uri_get".to_string())?;
    expect(token.user_id).to_equal("user_id_get".to_string())
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token = AccessToken {
        access_token: "token_add_none".to_string(),
        refresh_token: None,
        expires_at: Utc::now().trunc_subsecs(3),
        scope: None,
        client_id: "client_id_add".to_string(),
        redirect_uri: "redirect_uri_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&token).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let get_token = match runtime.block_on(async { model.get(&token.access_token).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get none one".to_string()),
            Some(token) => token,
        },
    };
    expect(get_token).to_equal(token)?;

    let token = AccessToken {
        access_token: "token_add_some".to_string(),
        refresh_token: Some("token_add_some".to_string()),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_add".to_string()),
        client_id: "client_id_add".to_string(),
        redirect_uri: "redirect_uri_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&token).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let get_token = match runtime.block_on(async { model.get(&token.access_token).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(token) => match token {
            None => return Err("should get some one".to_string()),
            Some(token) => token,
        },
    };
    expect(get_token).to_equal(token)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token = AccessToken {
        access_token: "token_add".to_string(),
        refresh_token: Some("token_add".to_string()),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_add".to_string()),
        client_id: "client_id_add".to_string(),
        redirect_uri: "redirect_uri_add".to_string(),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&token).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&token).await }) {
        return Err("model.add() duplicate should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying an access token.
pub fn del_by_access_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token_del = "token_del";
    let token_not_del = "token_not_del";
    let mut token = AccessToken {
        access_token: token_del.to_string(),
        refresh_token: Some(token_not_del.to_string()),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        access_token: Some(token_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&token).await?;
        token.access_token = token_not_del.to_string();
        token.refresh_token = Some(token_del.to_string());
        model.add(&token).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(token_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(token) => match token {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a refresh token.
pub fn del_by_refresh_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token_del = "token_del";
    let token_not_del = "token_not_del";
    let mut token = AccessToken {
        access_token: token_not_del.to_string(),
        refresh_token: Some(token_del.to_string()),
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        refresh_token: Some(token_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&token).await?;
        token.access_token = token_del.to_string();
        token.refresh_token = Some(token_not_del.to_string());
        model.add(&token).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(token_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete one fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_del).await }) {
        Err(e) => Err(format!("model.get() delete one error: {}", e)),
        Ok(token) => match token {
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
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token_del = "token_del";
    let token = AccessToken {
        access_token: token_del.to_string(),
        refresh_token: None,
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        access_token: Some(token_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&token).await?;
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
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token_del1 = "token_del1";
    let token_del2 = "token_del2";
    let token_not_del = "token_not_del";
    let mut token = AccessToken {
        access_token: token_del1.to_string(),
        refresh_token: None,
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        client_id: Some("client_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&token).await?;
        token.access_token = token_del2.to_string();
        model.add(&token).await?;
        token.access_token = token_not_del.to_string();
        token.client_id = "client_id_not_del".to_string();
        model.add(&token).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(token_del1).await }) {
        Err(e) => return Err(format!("model.get() delete token1 error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete token1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_del2).await }) {
        Err(e) => return Err(format!("model.get() delete token2 error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete token2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(token) => match token {
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
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token_del1 = "token_del1";
    let token_del2 = "token_del2";
    let token_not_del = "token_not_del";
    let mut token = AccessToken {
        access_token: token_del1.to_string(),
        refresh_token: None,
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&token).await?;
        token.access_token = token_del2.to_string();
        model.add(&token).await?;
        token.access_token = token_not_del.to_string();
        token.user_id = "user_id_not_del".to_string();
        model.add(&token).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(token_del1).await }) {
        Err(e) => return Err(format!("model.get() delete token1 error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete token1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_del2).await }) {
        Err(e) => return Err(format!("model.get() delete token2 error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete token2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(token) => match token {
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
    let model = state.mongodb.as_ref().unwrap().access_token();

    let token_del = "token_del";
    let token_not_del = "token_not_del";
    let mut token = AccessToken {
        access_token: token_del.to_string(),
        refresh_token: None,
        expires_at: Utc::now().trunc_subsecs(3),
        scope: Some("scope_del".to_string()),
        client_id: "client_id_del".to_string(),
        redirect_uri: "redirect_uri_del".to_string(),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        client_id: Some("client_id_del"),
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&token).await?;
        token.access_token = token_not_del.to_string();
        token.user_id = "user_id_not_del".to_string();
        model.add(&token).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(token_del).await }) {
        Err(e) => return Err(format!("model.get() delete one error: {}", e)),
        Ok(token) => match token {
            None => (),
            Some(_) => return Err("delete one fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(token_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(token) => match token {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}
