use std::collections::HashMap;

use axum::{http::Method, routing, Router};

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
    let mut role_scopes_bulk: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_bulk_del: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_count: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_list: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_param: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("device-route.post") {
        None => {
            role_scopes_root.insert(Method::POST, (vec![], vec![]));
            role_scopes_bulk.insert(Method::POST, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::POST, (vec![], scopes.clone()));
            role_scopes_bulk.insert(Method::POST, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("device-route.get") {
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
    match state.api_scopes.get("device-route.patch") {
        None => {
            role_scopes_param.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("device-route.delete") {
        None => {
            role_scopes_bulk_del.insert(Method::POST, (vec![], vec![]));
            role_scopes_param.insert(Method::DELETE, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_bulk_del.insert(Method::POST, (vec![], scopes.clone()));
            role_scopes_param.insert(Method::DELETE, (vec![], scopes.clone()));
        }
    }

    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/",
                routing::post(api::post_device_route)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_root)),
            )
            .route(
                "/bulk",
                routing::post(api::post_device_route_bulk)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_bulk.clone())),
            )
            .route(
                "/bulk-delete",
                routing::post(api::post_device_route_bulk_del).layer(AuthService::new(
                    auth_uri.clone(),
                    role_scopes_bulk_del.clone(),
                )),
            )
            .route(
                "/range",
                routing::post(api::post_device_route_range)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_bulk)),
            )
            .route(
                "/range-delete",
                routing::post(api::post_device_route_range_del)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_bulk_del)),
            )
            .route(
                "/count",
                routing::get(api::get_device_route_count)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_count)),
            )
            .route(
                "/list",
                routing::get(api::get_device_route_list)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_list)),
            )
            .route(
                "/{route_id}",
                routing::delete(api::delete_device_route)
                    .layer(AuthService::new(auth_uri.clone(), role_scopes_param)),
            )
            .with_state(state.clone()),
    )
}
