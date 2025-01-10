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
    let mut role_scopes_user_param: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("client.post") {
        None => {
            role_scopes_root.insert(Method::POST, (vec![Role::ADMIN, Role::DEV], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::POST, (vec![Role::ADMIN, Role::DEV], scopes.clone()));
        }
    }
    match state.api_scopes.get("client.get") {
        None => {
            role_scopes_count.insert(Method::GET, (vec![Role::ADMIN, Role::DEV], vec![]));
            role_scopes_list.insert(Method::GET, (vec![Role::ADMIN, Role::DEV], vec![]));
            role_scopes_param.insert(Method::GET, (vec![Role::ADMIN, Role::DEV], vec![]));
        }
        Some(scopes) => {
            role_scopes_count.insert(Method::GET, (vec![Role::ADMIN, Role::DEV], scopes.clone()));
            role_scopes_list.insert(Method::GET, (vec![Role::ADMIN, Role::DEV], scopes.clone()));
            role_scopes_param.insert(Method::GET, (vec![Role::ADMIN, Role::DEV], scopes.clone()));
        }
    }
    match state.api_scopes.get("client.patch") {
        None => {
            role_scopes_param.insert(Method::PATCH, (vec![Role::ADMIN, Role::DEV], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(
                Method::PATCH,
                (vec![Role::ADMIN, Role::DEV], scopes.clone()),
            );
        }
    }
    match state.api_scopes.get("client.delete") {
        None => {
            role_scopes_param.insert(Method::DELETE, (vec![Role::ADMIN, Role::DEV], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(
                Method::DELETE,
                (vec![Role::ADMIN, Role::DEV], scopes.clone()),
            );
        }
    }
    match state.api_scopes.get("client.delete.user") {
        None => {
            role_scopes_user_param.insert(Method::DELETE, (vec![Role::ADMIN], vec![]));
        }
        Some(scopes) => {
            role_scopes_user_param.insert(Method::DELETE, (vec![Role::ADMIN], scopes.clone()));
        }
    }

    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/",
                routing::post(api::post_client)
                    .layer(AuthService::new(&state.model, role_scopes_root)),
            )
            .route(
                "/count",
                routing::get(api::get_client_count)
                    .layer(AuthService::new(&state.model, role_scopes_count)),
            )
            .route(
                "/list",
                routing::get(api::get_client_list)
                    .layer(AuthService::new(&state.model, role_scopes_list)),
            )
            .route(
                "/{client_id}",
                routing::get(api::get_client)
                    .patch(api::patch_client)
                    .delete(api::delete_client)
                    .layer(AuthService::new(&state.model, role_scopes_param)),
            )
            .route(
                "/user/{user_id}",
                routing::delete(api::delete_client_user)
                    .layer(AuthService::new(&state.model, role_scopes_user_param)),
            )
            .with_state(state.clone()),
    )
}
