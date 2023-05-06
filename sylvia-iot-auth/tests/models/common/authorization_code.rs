use chrono::{SubsecRound, Utc};
use laboratory::expect;
use tokio::runtime::Runtime;

use sylvia_iot_auth::models::authorization_code::{
    AuthorizationCode, AuthorizationCodeModel, QueryCond,
};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn AuthorizationCodeModel) -> Result<(), String> {
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
pub fn add_dup(runtime: &Runtime, model: &dyn AuthorizationCodeModel) -> Result<(), String> {
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
pub fn del_by_code(runtime: &Runtime, model: &dyn AuthorizationCodeModel) -> Result<(), String> {
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
pub fn del_twice(runtime: &Runtime, model: &dyn AuthorizationCodeModel) -> Result<(), String> {
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
pub fn del_by_client_id(
    runtime: &Runtime,
    model: &dyn AuthorizationCodeModel,
) -> Result<(), String> {
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
pub fn del_by_user_id(runtime: &Runtime, model: &dyn AuthorizationCodeModel) -> Result<(), String> {
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
pub fn del_by_user_client(
    runtime: &Runtime,
    model: &dyn AuthorizationCodeModel,
) -> Result<(), String> {
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
