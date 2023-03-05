use actix_web::{dev::HttpServiceFactory, web, HttpRequest, Responder};

use super::{super::State, api_bridge};

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("/tokeninfo").route(web::get().to(get_tokeninfo)))
        .service(web::resource("/logout").route(web::post().to(post_logout)))
}

/// `GET /{base}/api/v1/auth/tokeninfo`
async fn get_tokeninfo(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "get_tokeninfo";
    let api_path = format!("{}/api/v1/auth/tokeninfo", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}

/// `POST /{base}/api/v1/auth/logout`
async fn post_logout(mut req: HttpRequest, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_logout";
    let api_path = format!("{}/api/v1/auth/logout", state.auth_base.as_str());
    let client = state.client.clone();

    api_bridge(FN_NAME, &client, &mut req, api_path.as_str(), None).await
}
