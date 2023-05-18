use chrono::{SubsecRound, Utc};
use laboratory::expect;
use tokio::runtime::Runtime;

use sylvia_iot_auth::models::login_session::{LoginSession, LoginSessionModel, QueryCond};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn LoginSessionModel) -> Result<(), String> {
    let session = LoginSession {
        session_id: "session_id_add_none".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&session).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let get_session = match runtime.block_on(async { model.get(&session.session_id).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(session) => match session {
            None => return Err("should get none one".to_string()),
            Some(session) => session,
        },
    };
    expect(get_session).to_equal(session)?;

    let session = LoginSession {
        session_id: "session_id_add_some".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&session).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let get_session = match runtime.block_on(async { model.get(&session.session_id).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(session) => match session {
            None => return Err("should get some one".to_string()),
            Some(session) => session,
        },
    };
    expect(get_session).to_equal(session)
}

/// Test `add()` with duplicate key.
pub fn add_dup(runtime: &Runtime, model: &dyn LoginSessionModel) -> Result<(), String> {
    let session = LoginSession {
        session_id: "session_id_add".to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        user_id: "user_id_add".to_string(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&session).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&session).await }) {
        return Err("model.add() duplicate should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying a session ID.
pub fn del_by_session(runtime: &Runtime, model: &dyn LoginSessionModel) -> Result<(), String> {
    let session_id_del = "session_id_del";
    let session_id_not_del = "session_id_not_del";
    let mut session = LoginSession {
        session_id: session_id_del.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        session_id: Some(session_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&session).await?;
        session.session_id = session_id_not_del.to_string();
        model.add(&session).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(session_id_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(session) => match session {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(session_id_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(session) => match session {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn LoginSessionModel) -> Result<(), String> {
    let session_id_del = "session_id_del";
    let session = LoginSession {
        session_id: session_id_del.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        session_id: Some(session_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&session).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a user ID.
pub fn del_by_user_id(runtime: &Runtime, model: &dyn LoginSessionModel) -> Result<(), String> {
    let session_id_del1 = "session_id_del1";
    let session_id_del2 = "session_id_del2";
    let session_id_not_del = "session_id_not_del";
    let mut session = LoginSession {
        session_id: session_id_del1.to_string(),
        expires_at: Utc::now().trunc_subsecs(3),
        user_id: "user_id_del".to_string(),
    };
    let cond = QueryCond {
        user_id: Some("user_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&session).await?;
        session.session_id = session_id_del2.to_string();
        model.add(&session).await?;
        session.session_id = session_id_not_del.to_string();
        session.user_id = "user_id_not_del".to_string();
        model.add(&session).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(session_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete session_id1 error: {}", e)),
        Ok(session) => match session {
            None => (),
            Some(_) => return Err("delete session_id1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(session_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete session_id2 error: {}", e)),
        Ok(session) => match session {
            None => (),
            Some(_) => return Err("delete session_id2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(session_id_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(session) => match session {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}
