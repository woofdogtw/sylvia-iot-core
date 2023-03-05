use std::collections::HashMap;

use actix_web::{dev::HttpServiceFactory, http::Method, web};

use sylvia_iot_corelib::role::Role;

use super::super::{
    oauth2::middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> impl HttpServiceFactory {
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

    web::scope(scope_path)
        .service(
            web::resource("")
                .wrap(AuthService::new(&state.model, role_scopes_root))
                .route(web::post().to(api::post_client)),
        )
        .service(
            web::resource("/count")
                .wrap(AuthService::new(&state.model, role_scopes_count))
                .route(web::get().to(api::get_client_count)),
        )
        .service(
            web::resource("/list")
                .wrap(AuthService::new(&state.model, role_scopes_list))
                .route(web::get().to(api::get_client_list)),
        )
        .service(
            web::resource("/{client_id}")
                .wrap(AuthService::new(&state.model, role_scopes_param))
                .route(web::get().to(api::get_client))
                .route(web::patch().to(api::patch_client))
                .route(web::delete().to(api::delete_client)),
        )
        .service(
            web::resource("/user/{user_id}")
                .wrap(AuthService::new(&state.model, role_scopes_user_param))
                .route(web::delete().to(api::delete_client_user)),
        )
}
