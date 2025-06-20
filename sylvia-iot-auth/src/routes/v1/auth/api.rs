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
use crate::models::{access_token::QueryCond as AccessTokenQueryCond, client::Client, user::User};

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

    let cond = AccessTokenQueryCond {
        access_token: Some(token.as_str()),
        ..Default::default()
    };
    if let Err(e) = state.model.access_token().del(&cond).await {
        error!("[{}] clear access token error: {}", FN_NAME, e);
        let e = ErrResp::ErrDb(Some(format!("clear access token error: {}", e)));
        return Err(e);
    }

    Ok(StatusCode::NO_CONTENT)
}
