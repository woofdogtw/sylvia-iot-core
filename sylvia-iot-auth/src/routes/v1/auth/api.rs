use axum::{
    Extension,
    extract::{Request, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_extra::headers::authorization::{Bearer, Credentials};
use log::error;

use sylvia_iot_corelib::{err::ErrResp, http::Json};

use super::{super::super::State as AppState, response};
use crate::models::{
    access_token::QueryCond as AccessTokenQueryCond, client::Client,
    refresh_token::QueryCond as RefreshTokenQueryCond, user::User,
};

/// `GET /{base}/api/v1/auth/tokeninfo`
pub async fn get_tokeninfo(
    Extension(user): Extension<User>,
    Extension(client): Extension<Client>,
) -> impl IntoResponse {
    Json(response::GetTokenInfo {
        data: response::GetTokenInfoData {
            user_id: user.user_id,
            account: user.account,
            name: user.name,
            roles: user.roles,
            client_id: client.client_id,
            scopes: client.scopes,
        },
    })
}

/// `POST /{base}/api/v1/auth/logout`
pub async fn post_logout(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_logout";

    let token = match req.headers().get(header::AUTHORIZATION) {
        None => {
            return Err(ErrResp::ErrUnknown(Some(
                "no Authorization header".to_string(),
            )));
        }
        Some(auth) => match Bearer::decode(auth) {
            None => return Err(ErrResp::ErrUnknown(Some("no Bearer token".to_string()))),
            Some(token) => token.token().to_string(),
        },
    };

    let refresh_token = match state.model.access_token().get(token.as_str()).await {
        Err(e) => {
            error!("[{}] pre-clear access token error: {}", FN_NAME, e);
            let e = ErrResp::ErrDb(Some(format!("pre-clear access token error: {}", e)));
            return Err(e);
        }
        Ok(token) => match token {
            None => None,
            Some(token) => token.refresh_token,
        },
    };
    let cond = AccessTokenQueryCond {
        access_token: Some(token.as_str()),
        ..Default::default()
    };
    if let Err(e) = state.model.access_token().del(&cond).await {
        error!("[{}] clear access token error: {}", FN_NAME, e);
        let e = ErrResp::ErrDb(Some(format!("clear access token error: {}", e)));
        return Err(e);
    }
    if let Some(token) = refresh_token {
        let cond = RefreshTokenQueryCond {
            refresh_token: Some(token.as_str()),
            ..Default::default()
        };
        if let Err(e) = state.model.refresh_token().del(&cond).await {
            error!("[{}] clear refresh token error: {}", FN_NAME, e);
            let e = ErrResp::ErrDb(Some(format!("clear refresh token error: {}", e)));
            return Err(e);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
