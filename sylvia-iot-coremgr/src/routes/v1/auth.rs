use axum::{
    extract::{Request, State},
    response::IntoResponse,
    routing, Router,
};

use super::{super::State as AppState, api_bridge};

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/tokeninfo", routing::get(get_tokeninfo))
            .route("/logout", routing::post(post_logout))
            .with_state(state.clone()),
    )
}

/// `GET /{base}/api/v1/auth/tokeninfo`
async fn get_tokeninfo(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_tokeninfo";
    let api_path = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}

/// `POST /{base}/api/v1/auth/logout`
async fn post_logout(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "post_logout";
    let api_path = format!("{}/api/v1/auth/logout", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, req, api_path.as_str()).await
}
