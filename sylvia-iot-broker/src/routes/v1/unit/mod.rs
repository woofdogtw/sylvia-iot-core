use std::collections::HashMap;

use actix_web::{dev::HttpServiceFactory, http::Method, web};

use sylvia_iot_corelib::role::Role;

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
            role_scopes_params.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_count.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_list.insert(Method::GET, (vec![], scopes.clone()));
            role_scopes_params.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("unit.patch") {
        None => {
            role_scopes_params.insert(Method::PATCH, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_params.insert(Method::PATCH, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("unit.delete") {
        None => {
            role_scopes_params.insert(Method::DELETE, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_params.insert(Method::DELETE, (vec![], scopes.clone()));
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
    web::scope(scope_path)
        .service(
            web::resource("")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_root))
                .route(web::post().to(api::post_unit)),
        )
        .service(
            web::resource("/count")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_count))
                .route(web::get().to(api::get_unit_count)),
        )
        .service(
            web::resource("/list")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_list))
                .route(web::get().to(api::get_unit_list)),
        )
        .service(
            web::resource("/{unit_id}")
                .wrap(AuthService::new(auth_uri.clone(), role_scopes_params))
                .route(web::get().to(api::get_unit))
                .route(web::patch().to(api::patch_unit))
                .route(web::delete().to(api::delete_unit)),
        )
        .service(
            web::resource("/user/{user_id}")
                .wrap(AuthService::new(auth_uri, role_scopes_user))
                .route(web::delete().to(api::delete_unit_user)),
        )
}
