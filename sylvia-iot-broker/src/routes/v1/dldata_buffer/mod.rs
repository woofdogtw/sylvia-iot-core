use std::collections::HashMap;

use axum::{http::Method, routing, Router};

use super::super::{
    middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> Router {
    let mut role_scopes_count: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_list: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_param: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("dldata-buffer.get") {
        None => {
            role_scopes_count.insert(Method::GET, (vec![], vec![]));
            role_scopes_list.insert(Method::GET, (vec![], vec![]));
            role_scopes_param.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_count.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_list.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_param.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("dldata-buffer.patch") {
        None => {
            role_scopes_param.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("dldata-buffer.delete") {
        None => {
            role_scopes_param.insert(Method::DELETE, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::DELETE, (vec![], scopes.clone()));
        }
    }

    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/count",
                routing::get(api::get_dldata_buffer_count)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_count)),
            )
            .route(
                "/list",
                routing::get(api::get_dldata_buffer_list)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_list)),
            )
            .route(
                "/:data_id",
                routing::delete(api::delete_dldata_buffer)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_param)),
            )
            .with_state(state.clone()),
    )
}
