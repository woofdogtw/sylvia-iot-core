use axum::{routing, Router};

use super::super::{middleware::AuthService, State};

mod api;
mod request;
mod response;

pub fn new_service(scope_path: &str, state: &State) -> Router {
    let auth_uri = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    Router::new().nest(
        scope_path,
        Router::new()
            .route(
                "/count",
                routing::get(api::get_count).layer(AuthService::new(auth_uri.clone())),
            )
            .route(
                "/list",
                routing::get(api::get_list).layer(AuthService::new(auth_uri.clone())),
            )
            .with_state(state.clone()),
    )
}
