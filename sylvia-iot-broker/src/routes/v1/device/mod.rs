use std::collections::HashMap;

use axum::{Router, http::Method, routing};

use super::super::{
    State,
    middleware::{AuthService, RoleScopeType},
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

    match state.api_scopes.get("device.post") {
        None => {
            role_scopes_root.insert(Method::POST, (vec![], vec![]));
            role_scopes_bulk.insert(Method::POST, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::POST, (vec![], scopes.clone()));
            role_scopes_bulk.insert(Method::POST, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("device.get") {
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
    match state.api_scopes.get("device.patch") {
        None => {
            role_scopes_param.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_param.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("device.delete") {
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
                routing::post(api::post_device).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_root,
                )),
            )
            .route(
                "/bulk",
                routing::post(api::post_device_bulk).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_bulk.clone(),
                )),
            )
            .route(
                "/bulk-delete",
                routing::post(api::post_device_bulk_del).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_bulk_del.clone(),
                )),
            )
            .route(
                "/range",
                routing::post(api::post_device_range).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_bulk,
                )),
            )
            .route(
                "/range-delete",
                routing::post(api::post_device_range_del).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_bulk_del,
                )),
            )
            .route(
                "/count",
                routing::get(api::get_device_count).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_count,
                )),
            )
            .route(
                "/list",
                routing::get(api::get_device_list).layer(AuthService::new(
                    state.client.clone(),
                    auth_uri.clone(),
                    role_scopes_list,
                )),
            )
            .route(
                "/{device_id}",
                routing::get(api::get_device)
                    .patch(api::patch_device)
                    .delete(api::delete_device)
                    .layer(AuthService::new(
                        state.client.clone(),
                        auth_uri.clone(),
                        role_scopes_param,
                    )),
            )
            .with_state(state.clone()),
    )
}
