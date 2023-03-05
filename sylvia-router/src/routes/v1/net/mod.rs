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
            web::resource("/wan")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_wan)),
        )
        .service(
            web::resource("/wan/{wan_id}")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::put().to(api::put_wan)),
        )
        .service(
            web::resource("/lan")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_lan))
                .route(web::put().to(api::put_lan)),
        )
        .service(
            web::resource("/lan/leases")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_lan_leases)),
        )
        .service(
            web::resource("/wlan")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_wlan))
                .route(web::put().to(api::put_wlan)),
        )
        .service(
            web::resource("/wwan")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_wwan))
                .route(web::put().to(api::put_wwan)),
        )
        .service(
            web::resource("/wwan/list")
                .wrap(AuthService::new(auth_uri.clone()))
                .route(web::get().to(api::get_wwan_list)),
        )
}
