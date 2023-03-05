//! Handlers of all OAuth2 functions.

use actix_web::{web, HttpResponse, Responder};
use tera::Tera;

use super::State;

mod api;
pub(crate) mod endpoint;
pub mod middleware;
mod primitive;
pub(crate) mod request;
pub(crate) mod response;
mod template;

/// To register all OAuth2 relative URIs.
pub fn gen_configure(state: &State) -> impl FnOnce(&mut web::ServiceConfig) {
    let templates = state.templates.clone();
    move |cfg: &mut web::ServiceConfig| {
        let mut tera = Tera::default();
        let _ = match templates.get("login") {
            None => match tera.add_raw_template(api::TMPL_LOGIN, template::LOGIN) {
                Err(e) => panic!("login default template error: {}", e),
                Ok(_) => (),
            },
            Some(template) => {
                match tera.add_template_file(template.as_str(), Some(api::TMPL_LOGIN)) {
                    Err(e) => panic!("login template file {} error: {}", template.as_str(), e),
                    Ok(_) => (),
                }
            }
        };
        let _ = match templates.get("grant") {
            None => match tera.add_raw_template(api::TMPL_GRANT, template::GRANT) {
                Err(e) => panic!("grant default template error: {}", e),
                Ok(_) => (),
            },
            Some(template) => {
                match tera.add_template_file(template.as_str(), Some(api::TMPL_GRANT)) {
                    Err(e) => panic!("grant template file {} error: {}", template.as_str(), e),
                    Ok(_) => (),
                }
            }
        };

        cfg.app_data(web::Data::new(tera))
            .service(web::resource("/auth").route(web::get().to(api::get_auth)))
            .service(
                web::resource("/login")
                    .route(web::get().to(api::get_login))
                    .route(web::post().to(api::post_login)),
            )
            .service(
                web::resource("/authorize")
                    .route(web::get().to(api::authorize))
                    .route(web::post().to(api::authorize)),
            )
            .route("/token", web::post().to(api::post_token))
            .route("/refresh", web::post().to(api::post_refresh))
            .route("/redirect", web::get().to(redirect));
    }
}

/// The built-in redirect path for getting authorization codes.
async fn redirect() -> impl Responder {
    HttpResponse::Ok().finish()
}
