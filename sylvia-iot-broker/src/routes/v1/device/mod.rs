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
    let mut role_scopes_bulk: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_bulk_del: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_count: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_list: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_params: HashMap<Method, RoleScopeType> = HashMap::new();

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
            role_scopes_params.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_count.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_list.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_params.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("device.patch") {
        None => {
            role_scopes_params.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_params.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("device.delete") {
        None => {
            role_scopes_bulk_del.insert(Method::POST, (vec![], vec![]));
            role_scopes_params.insert(Method::DELETE, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_bulk_del.insert(Method::POST, (vec![], scopes.clone()));
            role_scopes_params.insert(Method::DELETE, (vec![], scopes.clone()));
        }
    }

    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    web::scope(scope_path)
        .service(
            web::resource("")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_root))
                .route(web::post().to(api::post_device)),
        )
        .service(
            web::resource("/bulk")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_bulk.clone()))
                .route(web::post().to(api::post_device_bulk)),
        )
        .service(
            web::resource("/bulk-delete")
                .wrap(AuthService::new(
                    auth_uri.clone(),
                    role_scopes_bulk_del.clone(),
                ))
                .route(web::post().to(api::post_device_bulk_del)),
        )
        .service(
            web::resource("/range")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_bulk))
                .route(web::post().to(api::post_device_range)),
        )
        .service(
            web::resource("/range-delete")
                .wrap(AuthService::new(
                    auth_uri.clone(),
                    role_scopes_bulk_del.clone(),
                ))
                .route(web::post().to(api::post_device_range_del)),
        )
        .service(
            web::resource("/count")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_count))
                .route(web::get().to(api::get_device_count)),
        )
        .service(
            web::resource("/list")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_list))
                .route(web::get().to(api::get_device_list)),
        )
        .service(
            web::resource("/{device_id}")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_params))
                .route(web::get().to(api::get_device))
                .route(web::patch().to(api::patch_device))
                .route(web::delete().to(api::delete_device)),
        )
}
