use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use log::error;

use sylvia_iot_corelib::err::ErrResp;

use super::{super::super::State, response};
use crate::models::{
    access_token::QueryCond as AccessTokenQueryCond,
    authorization_code::QueryCond as AuthCodeQueryCond, client::Client,
    refresh_token::QueryCond as RefreshTokenQueryCond, user::User,
};

/// `GET /{base}/api/v1/auth/tokeninfo`
pub async fn get_tokeninfo(req: HttpRequest) -> impl Responder {
    const FN_NAME: &'static str = "get_tokeninfo";

    let user_id;
    let account;
    let name;
    let roles;
    let client_id;
    let scopes;

    match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => {
            user_id = user.user_id.clone();
            account = user.account.clone();
            name = user.name.clone();
            roles = user.roles.clone();
        }
    }
    match req.extensions_mut().get::<Client>() {
        None => {
            error!("[{}] client not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("client not found".to_string())));
        }
        Some(client) => {
            client_id = client.client_id.clone();
            scopes = client.scopes.clone();
        }
    }

    Ok(HttpResponse::Ok().json(response::GetTokenInfo {
        data: response::GetTokenInfoData {
            user_id,
            account,
            name,
            roles,
            client_id,
            scopes,
        },
    }))
}

/// `POST /{base}/api/v1/auth/logout`
pub async fn post_logout(req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_logout";

    let user_id = match req.extensions_mut().get::<User>() {
        None => {
            error!("[{}] user not found", FN_NAME);
            return Err(ErrResp::ErrUnknown(Some("user not found".to_string())));
        }
        Some(user) => user.user_id.clone(),
    };

    let cond = AuthCodeQueryCond {
        user_id: Some(user_id.as_str()),
        ..Default::default()
    };
    if let Err(e) = state.model.authorization_code().del(&cond).await {
        error!("[{}] clear authorization code error: {}", FN_NAME, e);
        let e = ErrResp::ErrDb(Some(format!("clear authorization code error: {}", e)));
        return Err(e);
    }
    let cond = RefreshTokenQueryCond {
        user_id: Some(user_id.as_str()),
        ..Default::default()
    };
    if let Err(e) = state.model.refresh_token().del(&cond).await {
        error!("[{}] clear refresh token error: {}", FN_NAME, e);
        let e = ErrResp::ErrDb(Some(format!("clear refresh token error: {}", e)));
        return Err(e);
    }
    let cond = AccessTokenQueryCond {
        user_id: Some(user_id.as_str()),
        ..Default::default()
    };
    if let Err(e) = state.model.access_token().del(&cond).await {
        error!("[{}] clear access token error: {}", FN_NAME, e);
        let e = ErrResp::ErrDb(Some(format!("clear access token error: {}", e)));
        return Err(e);
    }

    Ok(HttpResponse::NoContent().finish())
}
