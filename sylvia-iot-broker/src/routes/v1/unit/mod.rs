use std::collections::HashMap;

use axum::{http::Method, routing, Router};

use sylvia_iot_corelib::role::Role;

use super::super::{
    middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod request;
mod response;
pub use api::{init, new_ctrl_receiver, new_ctrl_sender};

pub fn new_service(scope_path: &str, state: &State) -> Router {
    let mut role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_count: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_list: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_param: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_user: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("unit.post") {
        None => {
            role_scopes_root.insert(Method::POST, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::POST, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("unit.get") {
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
    match state.api_scopes.get("unit.patch") {
        None => {
            role_scopes_param.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("unit.delete") {
        None => {
            role_scopes_param.insert(Method::DELETE, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::DELETE, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("unit.delete.user") {
        None => {
            role_scopes_user.insert(Method::DELETE, (vec![Role::ADMIN, Role::MANAGER], vec![]));
        }
        Some(scopes) => {
            role_scopes_user.insert(
                Method::DELETE,
                (vec![Role::ADMIN, Role::MANAGER], scopes.clone()),
            );
        }
    }

    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/",
                routing::post(api::post_unit)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_root)),
            )
            .route(
                "/count",
                routing::get(api::get_unit_count)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_count)),
            )
            .route(
                "/list",
                routing::get(api::get_unit_list)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_list)),
            )
            .route(
                "/:unit_id",
                routing::get(api::get_unit)
                    .patch(api::patch_unit)
                    .delete(api::delete_unit)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_param)),
            )
            .route(
                "/user/:user_id",
                routing::delete(api::delete_unit_user)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_user)),
            )
            .with_state(state.clone()),
    )
}
