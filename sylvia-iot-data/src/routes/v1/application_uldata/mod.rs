use actix_web::{dev::HttpServiceFactory, web};

use super::super::{middleware::AuthService, State};

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> impl HttpServiceFactory {
    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    web::scope(scope_path)
        .service(
            web::resource("/count")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_count)),
        )
        .service(
            web::resource("/list")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_list)),
        )
}
