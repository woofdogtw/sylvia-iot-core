use axum::{routing, Router};
use sylvia_iot_sdk::middlewares::auth::AuthService;

use super::super::State;

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> Router {
    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.config.auth.as_str());
    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/wan",
                routing::get(api::get_wan).layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/wan/:wan_id",
                routing::put(api::put_wan).layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/lan",
                routing::get(api::get_lan)
                    .put(api::put_lan)
                    .layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/lan/leases",
                routing::put(api::get_lan_leases).layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/wlan",
                routing::get(api::get_wlan)
                    .put(api::put_wlan)
                    .layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/wwan",
                routing::get(api::get_wwan)
                    .put(api::put_wwan)
                    .layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/wwan/list",
                routing::put(api::get_wwan_list).layer(AuthService::new(auth_uri.clone())),
            )
            .with_state(state.clone()),
    )
}
