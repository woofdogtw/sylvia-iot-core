use std::collections::HashMap;

use axum::{http::Method, routing, Router};

use super::super::{
    oauth2::middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> Router {
    let mut role_scopes_tokeninfo: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_logout: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("auth.tokeninfo.get") {
        None => {
            role_scopes_tokeninfo.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_tokeninfo.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("auth.logout.post") {
        None => {
            role_scopes_logout.insert(Method::POST, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_logout.insert(Method::POST, (vec![], scopes.clone()));
        }
    }

    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/tokeninfo",
                routing::get(api::get_tokeninfo)
                    .layer(AuthService::new(&state.model, role_scopes_tokeninfo)),
            )
            .route(
                "/logout",
                routing::post(api::post_logout)
                    .layer(AuthService::new(&state.model, role_scopes_logout)),
            )
            .with_state(state.clone()),
    )
}
