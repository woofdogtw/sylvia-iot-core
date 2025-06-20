use axum::{Router, routing};
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
                "/usage",
                routing::get(api::get_usage).layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/time",
                routing::get(api::get_time).layer(AuthService::new(auth_uri.clone())),
            )
            .with_state(state.clone()),
    )
}
