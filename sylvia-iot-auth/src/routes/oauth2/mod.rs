//! Handlers of all OAuth2 functions.

use axum::{http::StatusCode, response::IntoResponse, routing, Extension, Router};
use tera::Tera;

use super::State as AppState;

mod api;
pub(crate) mod endpoint;
pub mod middleware;
mod primitive;
pub(crate) mod request;
pub(crate) mod response;
mod template;

pub fn new_service(state: &AppState) -> Router {
    let templates = state.templates.clone();
    let mut tera = Tera::default();
    let _ = match templates.get("login") {
        None => match tera.add_raw_template(api::TMPL_LOGIN, template::LOGIN) {
            Err(e) => panic!("login default template error: {}", e),
            Ok(_) => (),
        },
        Some(template) => match tera.add_template_file(template.as_str(), Some(api::TMPL_LOGIN)) {
            Err(e) => panic!("login template file {} error: {}", template.as_str(), e),
            Ok(_) => (),
        },
    };
    let _ = match templates.get("grant") {
        None => match tera.add_raw_template(api::TMPL_GRANT, template::GRANT) {
            Err(e) => panic!("grant default template error: {}", e),
            Ok(_) => (),
        },
        Some(template) => match tera.add_template_file(template.as_str(), Some(api::TMPL_GRANT)) {
            Err(e) => panic!("grant template file {} error: {}", template.as_str(), e),
            Ok(_) => (),
        },
    };

    Router::new()
        .route("/auth", routing::get(api::get_auth))
        .route("/login", routing::get(api::get_login).post(api::post_login))
        .route(
            "/authorize",
            routing::get(api::authorize).post(api::authorize),
        )
        .route("/token", routing::post(api::post_token))
        .route("/refresh", routing::post(api::post_refresh))
        .route("/redirect", routing::get(redirect))
        .layer(Extension(tera))
        .with_state(state.clone())
}

/// The built-in redirect path for getting authorization codes.
async fn redirect() -> impl IntoResponse {
    StatusCode::OK
}
