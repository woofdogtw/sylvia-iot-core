use std::collections::HashMap;

use actix_web::{dev::HttpServiceFactory, http::Method, web};

use super::super::{
    middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod request;
mod response;
pub use api::{init, new_ctrl_receiver, new_ctrl_sender};

pub fn new_service(scope_path: &str, state: &State) -> impl HttpServiceFactory {
    let mut role_scopes_root: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_count: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_list: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_params: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("network-route.post") {
        None => {
            role_scopes_root.insert(Method::POST, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_root.insert(Method::POST, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("network-route.get") {
        None => {
            role_scopes_count.insert(Method::GET, (vec![], vec![]));
            role_scopes_list.insert(Method::GET, (vec![], vec![]));
            role_scopes_params.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_count.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_list.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_params.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("network-route.patch") {
        None => {
            role_scopes_params.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_params.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("network-route.delete") {
        None => {
            role_scopes_params.insert(Method::DELETE, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_params.insert(Method::DELETE, (vec![], scopes.clone()));
        }
    }

    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    web::scope(scope_path)
        .service(
            web::resource("")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_root))
                .route(web::post().to(api::post_network_route)),
        )
        .service(
            web::resource("/count")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_count))
                .route(web::get().to(api::get_network_route_count)),
        )
        .service(
            web::resource("/list")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_list))
                .route(web::get().to(api::get_network_route_list)),
        )
        .service(
            web::resource("/{route_id}")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_params))
                .route(web::delete().to(api::delete_network_route)),
        )
}
