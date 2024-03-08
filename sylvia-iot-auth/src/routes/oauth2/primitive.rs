use std::{borrow::Cow, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeDelta, Utc};
use log::error;
use oxide_auth::primitives::{
    grant::{Extensions, Grant},
    issuer::{IssuedToken, RefreshedToken, TokenType},
    registrar::{BoundClient, ClientUrl, ExactUrl, PreGrant, RegisteredUrl, RegistrarError},
    scope::Scope,
};
use oxide_auth_async::primitives::{Authorizer, Issuer, Registrar};

use sylvia_iot_corelib::{err::E_UNKNOWN, strings};

use crate::models::{
    access_token::{self, AccessToken, QueryCond as AccessTokenQuery},
    authorization_code::{self, AuthorizationCode, QueryCond as AuthorizationCodeQuery},
    client::QueryCond,
    refresh_token::{self, QueryCond as RefreshTokenQuery, RefreshToken},
    Model,
};

#[derive(Clone)]
pub struct Primitive {
    model: Arc<dyn Model>,
}

impl Primitive {
    pub fn new(model: Arc<dyn Model>) -> Self {
        Primitive {
            model: model.clone(),
        }
    }
}

#[async_trait]
impl Authorizer for Primitive {
    async fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        const FN_NAME: &'static str = "authorize";

        let scope = grant.scope.to_string();
        let code = AuthorizationCode {
            code: strings::random_id_sha(&grant.until, 4),
            expires_at: match TimeDelta::try_seconds(authorization_code::EXPIRES) {
                None => panic!("{}", E_UNKNOWN),
                Some(t) => Utc::now() + t,
            },
            redirect_uri: grant.redirect_uri.to_string(),
            scope: match scope.len() {
                0 => None,
                _ => Some(scope),
            },
            client_id: grant.client_id,
            user_id: grant.owner_id,
        };

        match self.model.authorization_code().add(&code).await {
            Err(e) => {
                error!("[{}] add authorization code error: {}", FN_NAME, e);
                Err(())
            }
            Ok(()) => Ok(code.code),
        }
    }

    async fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        const FN_NAME: &'static str = "extract";

        let auth_code = match self.model.authorization_code().get(code).await {
            Err(_) => return Err(()),
            Ok(code) => match code {
                None => return Ok(None),
                Some(code) => code,
            },
        };
        {
            let query = AuthorizationCodeQuery {
                code: Some(code),
                ..Default::default()
            };
            if let Err(e) = self.model.authorization_code().del(&query).await {
                error!("[{}] delete authorization code error: {}", FN_NAME, e);
                return Err(());
            }
        }
        if auth_code.expires_at < Utc::now() {
            return Ok(None);
        }

        Ok(Some(Grant {
            owner_id: auth_code.user_id,
            client_id: auth_code.client_id,
            scope: match auth_code.scope {
                None => "".parse().unwrap(),
                Some(scope) => match scope.as_str().parse() {
                    Err(e) => {
                        error!("[{}] parse authorization code scope error: {}", FN_NAME, e);
                        return Err(());
                    }
                    Ok(scope) => scope,
                },
            },
            redirect_uri: match auth_code.redirect_uri.parse() {
                Err(e) => {
                    error!(
                        "[{}] parse authorization code redirect_uri error: {}",
                        FN_NAME, e
                    );
                    return Err(());
                }
                Ok(uri) => uri,
            },
            until: auth_code.expires_at,
            extensions: Extensions::new(),
        }))
    }
}

#[async_trait]
impl Issuer for Primitive {
    async fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        const FN_NAME: &'static str = "issue";

        let now = Utc::now();
        let refresh_token = strings::random_id_sha(&now, 8);
        let access = AccessToken {
            access_token: strings::random_id_sha(&now, 8),
            refresh_token: Some(refresh_token.clone()),
            expires_at: match TimeDelta::try_seconds(access_token::EXPIRES) {
                None => panic!("{}", E_UNKNOWN),
                Some(t) => now + t,
            },
            scope: Some(grant.scope.to_string()),
            client_id: grant.client_id.clone(),
            redirect_uri: grant.redirect_uri.to_string(),
            user_id: grant.owner_id.clone(),
        };
        let refresh = RefreshToken {
            refresh_token,
            expires_at: match TimeDelta::try_seconds(refresh_token::EXPIRES) {
                None => panic!("{}", E_UNKNOWN),
                Some(t) => now + t,
            },
            scope: Some(grant.scope.to_string()),
            client_id: grant.client_id,
            redirect_uri: grant.redirect_uri.to_string(),
            user_id: grant.owner_id,
        };

        if let Err(e) = self.model.access_token().add(&access).await {
            error!("[{}] add access token error: {}", FN_NAME, e);
            return Err(());
        }
        if let Err(e) = self.model.refresh_token().add(&refresh).await {
            error!("[{}] add refresh token error: {}", FN_NAME, e);
            return Err(());
        }
        Ok(IssuedToken {
            token: access.access_token,
            refresh: Some(refresh.refresh_token),
            until: access.expires_at,
            token_type: TokenType::Bearer,
        })
    }

    async fn refresh(&mut self, token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        const FN_NAME: &'static str = "refresh";

        let query = AccessTokenQuery {
            refresh_token: Some(token),
            ..Default::default()
        };
        if let Err(e) = self.model.access_token().del(&query).await {
            error!("[{}] delete access token error: {}", FN_NAME, e);
            return Err(());
        }
        let query = RefreshTokenQuery {
            refresh_token: Some(token),
            ..Default::default()
        };
        if let Err(e) = self.model.refresh_token().del(&query).await {
            error!("[{}] delete refresh token error: {}", FN_NAME, e);
            return Err(());
        }

        match self.issue(grant).await {
            Err(_) => Err(()),
            Ok(token) => Ok(RefreshedToken {
                token: token.token,
                refresh: token.refresh,
                until: token.until,
                token_type: token.token_type,
            }),
        }
    }

    async fn recover_token(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        const FN_NAME: &'static str = "recover_token";

        let access = match self.model.access_token().get(token).await {
            Err(e) => {
                error!("[{}] get access token error: {}", FN_NAME, e);
                return Err(());
            }
            Ok(token) => match token {
                None => return Ok(None),
                Some(token) => token,
            },
        };
        if access.expires_at < Utc::now() {
            return Ok(None);
        }

        Ok(Some(Grant {
            owner_id: access.user_id,
            client_id: access.client_id,
            scope: match access.scope {
                None => "".parse().unwrap(),
                Some(scope) => match scope.as_str().parse() {
                    Err(e) => {
                        error!("[{}] parse access token scope error: {}", FN_NAME, e);
                        return Err(());
                    }
                    Ok(scope) => scope,
                },
            },
            redirect_uri: match access.redirect_uri.parse() {
                Err(e) => {
                    error!("[{}] parse access token redirect_uri error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(uri) => uri,
            },
            until: access.expires_at,
            extensions: Extensions::new(),
        }))
    }

    async fn recover_refresh(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        const FN_NAME: &'static str = "recover_refresh";

        let refresh = match self.model.refresh_token().get(token).await {
            Err(e) => {
                error!("[{}] get refresh token error: {}", FN_NAME, e);
                return Err(());
            }
            Ok(token) => match token {
                None => return Ok(None),
                Some(token) => token,
            },
        };
        if refresh.expires_at < Utc::now() {
            return Ok(None);
        }

        Ok(Some(Grant {
            owner_id: refresh.user_id,
            client_id: refresh.client_id,
            scope: match refresh.scope {
                None => "".parse().unwrap(),
                Some(scope) => match scope.as_str().parse() {
                    Err(e) => {
                        error!("[{}] parse access token scope error: {}", FN_NAME, e);
                        return Err(());
                    }
                    Ok(scope) => scope,
                },
            },
            redirect_uri: match refresh.redirect_uri.parse() {
                Err(e) => {
                    error!("[{}] parse access token redirect_uri error: {}", FN_NAME, e);
                    return Err(());
                }
                Ok(uri) => uri,
            },
            until: refresh.expires_at,
            extensions: Extensions::new(),
        }))
    }
}

#[async_trait]
impl Registrar for Primitive {
    async fn bound_redirect<'a>(
        &self,
        bound: ClientUrl<'a>,
    ) -> Result<BoundClient<'a>, RegistrarError> {
        const FN_NAME: &'static str = "bound_redirect";

        let cond = QueryCond {
            client_id: Some(&bound.client_id),
            ..Default::default()
        };
        let redirect_uris = match self.model.client().get(&cond).await {
            Err(e) => {
                error!("[{}] get client error: {}", FN_NAME, e);
                return Err(RegistrarError::PrimitiveError);
            }
            Ok(client) => match client {
                None => return Err(RegistrarError::Unspecified),
                Some(client) => client.redirect_uris,
            },
        };

        let redirect_uri = match bound.redirect_uri {
            None => match redirect_uris.len() {
                0 => return Err(RegistrarError::Unspecified),
                _ => redirect_uris.get(0).unwrap(),
            },
            Some(url) => match redirect_uris
                .iter()
                .find(|uri| uri.as_str() == url.as_str())
            {
                None => return Err(RegistrarError::Unspecified),
                Some(uri) => uri,
            },
        };
        let redirect_uri = match ExactUrl::new(redirect_uri.clone()) {
            Err(_) => return Err(RegistrarError::Unspecified),
            Ok(url) => url,
        };

        Ok(BoundClient {
            client_id: bound.client_id,
            redirect_uri: Cow::Owned(RegisteredUrl::Exact(redirect_uri)),
        })
    }

    async fn negotiate<'a>(
        &self,
        bound: BoundClient<'a>,
        scope: Option<Scope>,
    ) -> Result<PreGrant, RegistrarError> {
        const FN_NAME: &'static str = "negotiate";

        let cond = QueryCond {
            client_id: Some(&bound.client_id),
            ..Default::default()
        };
        let client = match self.model.client().get(&cond).await {
            Err(e) => {
                return {
                    error!("[{}] get client error: {}", FN_NAME, e);
                    Err(RegistrarError::PrimitiveError)
                }
            }
            Ok(client) => match client {
                None => return Err(RegistrarError::Unspecified),
                Some(client) => client,
            },
        };

        if client.scopes.len() > 0 {
            match scope {
                None => return Err(RegistrarError::Unspecified),
                Some(scope) => {
                    let client_scopes = match client.scopes.join(" ").parse::<Scope>() {
                        Err(e) => {
                            error!("[{}] parse client scope error: {}", FN_NAME, e);
                            return Err(RegistrarError::PrimitiveError);
                        }
                        Ok(scopes) => scopes,
                    };
                    if !scope.allow_access(&client_scopes) {
                        return Err(RegistrarError::Unspecified);
                    }
                }
            }
        }

        Ok(PreGrant {
            client_id: bound.client_id.into_owned(),
            redirect_uri: bound.redirect_uri.into_owned(),
            scope: match client.scopes.join(" ").parse() {
                Err(e) => {
                    error!("[{}] parse client scope error: {}", FN_NAME, e);
                    return Err(RegistrarError::PrimitiveError);
                }
                Ok(scopes) => scopes,
            },
        })
    }

    async fn check(
        &self,
        client_id: &str,
        passphrase: Option<&[u8]>,
    ) -> Result<(), RegistrarError> {
        const FN_NAME: &'static str = "check";

        let cond = QueryCond {
            client_id: Some(client_id),
            ..Default::default()
        };
        let client = match self.model.client().get(&cond).await {
            Err(e) => {
                error!("[{}] get client error: {}", FN_NAME, e);
                return Err(RegistrarError::PrimitiveError);
            }
            Ok(client) => match client {
                None => return Err(RegistrarError::Unspecified),
                Some(client) => client,
            },
        };

        match (passphrase, client.client_secret) {
            (None, None) => Ok(()),
            (Some(passphrase), Some(client_secret)) => {
                match passphrase == client_secret.as_bytes() {
                    true => Ok(()),
                    false => Err(RegistrarError::Unspecified),
                }
            }
            _ => Err(RegistrarError::Unspecified),
        }
    }
}
