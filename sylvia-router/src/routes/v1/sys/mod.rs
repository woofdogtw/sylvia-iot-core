use actix_web::{dev::HttpServiceFactory, web};
use sylvia_iot_sdk::middlewares::auth::AuthService;

use super::super::State;

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> impl HttpServiceFactory {
    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.config.auth.as_str());
    web::scope(scope_path)
        .service(
            web::resource("/usage")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_usage)),
        )
        .service(
            web::resource("/time")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_time)),
        )
}
