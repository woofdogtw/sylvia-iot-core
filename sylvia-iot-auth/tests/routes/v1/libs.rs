use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use laboratory::expect;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_auth::routes;
use sylvia_iot_corelib::err;

use super::super::read_location;

#[derive(Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
struct PostAuthorizeRequest<'a> {
    response_type: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
    user_id: &'a str,
    allow: &'a str,
}

#[derive(Deserialize)]
struct PostAuthorizeLocation {
    code: String,
}

#[derive(Debug, Serialize)]
struct PostTokenRequest<'a> {
    grant_type: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
}

#[derive(Deserialize)]
struct OAuth2Error {
    error: String,
    #[serde(rename = "error_description")]
    _error_description: Option<String>,
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    #[serde(rename = "refresh_token")]
    _refresh_token: String,
    #[serde(rename = "token_type")]
    _token_type: String,
    #[serde(rename = "expires_in")]
    _expires_in: u64,
}

pub fn get_token(
    runtime: &Runtime,
    state: &routes::State,
    user_id: &str,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    // Get authorization code.
    let params = PostAuthorizeRequest {
        response_type: "code",
        client_id: "public",
        redirect_uri: crate::TEST_REDIRECT_URI,
        user_id,
        allow: "yes",
    };
    let req = TestRequest::post()
        .uri("/auth/oauth2/authorize")
        .set_form(&params)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::FOUND {
        return Err(format!(
            "post authorize response not 302, {}",
            resp.status()
        ));
    }
    let location = read_location(&resp)?;
    let code = match location.query() {
        None => return Err("302 with no query content".to_string()),
        Some(query) => {
            if let Ok(resp) = serde_urlencoded::from_str::<OAuth2Error>(query) {
                if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                    return Err(format!("redirect wrong URI: {}", location.as_str()));
                }
                return Err(format!("error: {}", resp.error));
            } else if let Ok(resp) = serde_urlencoded::from_str::<PostAuthorizeLocation>(query) {
                if !location.as_str().starts_with(crate::TEST_REDIRECT_URI) {
                    return Err(format!("redirect wrong URI: {}", location.as_str()));
                } else if resp.code.len() == 0 {
                    return Err("code length zero".to_string());
                }
                resp.code
            } else {
                return Err(format!("unexpected 302 query: {}", query));
            }
        }
    };

    // Get access token.
    let params = PostTokenRequest {
        grant_type: "authorization_code",
        code: code.as_str(),
        redirect_uri: crate::TEST_REDIRECT_URI,
        client_id: "public",
    };
    let req = TestRequest::post()
        .uri("/auth/oauth2/token")
        .set_form(&params)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    let status = resp.status();
    if status != StatusCode::OK {
        let body: OAuth2Error = runtime.block_on(async { test::read_body_json(resp).await });
        if body.error.len() == 0 {
            return Err(format!("post token response not 200, {}", status));
        }
        return Err(format!("get token error: {}", body.error));
    }
    let body: AccessToken = runtime.block_on(async { test::read_body_json(resp).await });
    if body.access_token.len() == 0 {
        return Err("post token unexpected 200".to_string());
    }
    return Ok(body.access_token);
}

pub fn test_invalid_perm(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    req: TestRequest,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let req = req
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::FORBIDDEN)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_PERM {
        return Err(format!("unexpected 403 error: {}", body.code.as_str()));
    }
    Ok(())
}

pub fn test_invalid_token(
    runtime: &Runtime,
    state: &routes::State,
    req: TestRequest,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(&state)),
        )
        .await
    });

    let req = req
        .insert_header((header::AUTHORIZATION, "Bearer token"))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::UNAUTHORIZED)
}

pub fn test_get_list_invalid_param(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    uri: &str,
    param: &Map<String, Value>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = format!("{}?{}", uri, serde_urlencoded::to_string(param).unwrap());
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
    if body.code.as_str() != err::E_PARAM {
        return Err(format!("unexpected 400 error: {}", body.code.as_str()));
    }
    Ok(())
}
