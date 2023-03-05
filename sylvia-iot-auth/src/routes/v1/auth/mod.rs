use std::collections::HashMap;

use actix_web::{dev::HttpServiceFactory, http::Method, web};

use super::super::{
    oauth2::middleware::{AuthService, RoleScopeType},
    State,
};

mod api;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> impl HttpServiceFactory {
    let mut role_scopes_tokeninfo: HashMap<Method, RoleScopeType> = HashMap::new();
    let mut role_scopes_logout: HashMap<Method, RoleScopeType> = HashMap::new();

    match state.api_scopes.get("auth.tokeninfo.get") {
        None => {
            role_scopes_tokeninfo.insert(Method::GET, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_tokeninfo.insert(Method::GET, (vec![], scopes.clone()));
        }
    }
    match state.api_scopes.get("auth.logout.post") {
        None => {
            role_scopes_logout.insert(Method::POST, (vec![], vec![]));
        }
        Some(scopes) => {
            role_scopes_logout.insert(Method::POST, (vec![], scopes.clone()));
        }
    }

    web::scope(scope_path)
        .service(
            web::resource("/tokeninfo")
                .wrap(AuthService::new(&state.model, role_scopes_tokeninfo))
                .route(web::get().to(api::get_tokeninfo)),
        )
        .service(
            web::resource("/logout")
                .wrap(AuthService::new(&state.model, role_scopes_logout))
                .route(web::post().to(api::post_logout)),
        )
}
