use std::collections::HashMap;

use axum::{http::Method, routing, Router};

use sylvia_iot_corelib::role::Role;

use super::super::{
    oauth2::middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> Router {
    let mut role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_count: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_list: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_param: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("user.get") {
        None => {
            role_scopes_root.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("user.patch") {
        None => {
            role_scopes_root.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("user.post.admin") {
        None => {
            role_scopes_root.insert(Method::POST, (vec![Role::ADMIN], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::POST, (vec![Role::ADMIN], scopes.clone()));
        }
    }
    match state.api_scopes.get("user.get.admin") {
        None => {
            role_scopes_count.insert(Method::GET, (vec![Role::ADMIN, Role::MANAGER], vec![]));
            role_scopes_list.insert(Method::GET, (vec![Role::ADMIN, Role::MANAGER], vec![]));
            role_scopes_param.insert(Method::GET, (vec![Role::ADMIN, Role::MANAGER], vec![]));
        }
        Some(scopes) => {
            role_scopes_count.insert(
                Method::GET,
                (vec![Role::ADMIN, Role::MANAGER], scopes.clone()),
            );
            role_scopes_list.insert(
                Method::GET,
                (vec![Role::ADMIN, Role::MANAGER], scopes.clone()),
            );
            role_scopes_param.insert(
                Method::GET,
                (vec![Role::ADMIN, Role::MANAGER], scopes.clone()),
            );
        }
    }
    match state.api_scopes.get("user.patch.admin") {
        None => {
            role_scopes_param.insert(Method::PATCH, (vec![Role::ADMIN, Role::MANAGER], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(
                Method::PATCH,
                (vec![Role::ADMIN, Role::MANAGER], scopes.clone()),
            );
        }
    }
    match state.api_scopes.get("user.delete.admin") {
        None => {
            role_scopes_param.insert(Method::DELETE, (vec![Role::ADMIN], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::DELETE, (vec![Role::ADMIN], scopes.clone()));
        }
    }

    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/",
                routing::get(api::get_user)
                    .patch(api::patch_user)
                    .post(api::post_admin_user)
                    .layer(AuthService::new(&state.model, role_scopes_root)),
            )
            .route(
                "/count",
                routing::get(api::get_admin_user_count)
                    .layer(AuthService::new(&state.model, role_scopes_count)),
            )
            .route(
                "/list",
                routing::get(api::get_admin_user_list)
                    .layer(AuthService::new(&state.model, role_scopes_list)),
            )
            .route(
                "/{user_id}",
                routing::get(api::get_admin_user)
                    .patch(api::patch_admin_user)
                    .delete(api::delete_admin_user)
                    .layer(AuthService::new(&state.model, role_scopes_param)),
            )
            .with_state(state.clone()),
    )
}
