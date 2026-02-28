use axum::{
    Router,
    extract::Request,
    http::{StatusCode, header},
    routing,
};
use axum_test::TestServer;
use laboratory::{SpecContext, expect};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use sylvia_iot_corelib::http::{self, Json, Path, Query};

use crate::TestState;

#[derive(Deserialize, Serialize)]
struct TestParam {
    param: isize,
}

#[derive(Serialize)]
struct TestWrongParam {
    param: &'static str,
}

#[derive(Deserialize)]
struct ErrBody {
    code: String,
}

/// Test [`http::parse_header_auth`].
pub fn parse_header_auth(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let req = Request::default();
    let result = http::parse_header_auth(&req);
    match result {
        Err(e) => return Err(format!("empty Authorization header error: {}", e)),
        Ok(content) => expect(content).to_equal(None)?,
    }

    let mut req = Request::default();
    req.headers_mut()
        .append(header::AUTHORIZATION, "test".parse().unwrap());
    req.headers_mut()
        .append(header::AUTHORIZATION, "test".parse().unwrap());
    let result = http::parse_header_auth(&req);
    if result.is_ok() {
        return Err("multiple Authorization header not error".to_string());
    }

    let mut req = Request::default();
    req.headers_mut()
        .append(header::AUTHORIZATION, "test".parse().unwrap());
    let result = http::parse_header_auth(&req);
    match result {
        Err(e) => return Err(format!("Authorization header error: {}", e)),
        Ok(content) => expect(content).to_equal(Some("test".to_string())),
    }
}

/// Test [`http::Json`].
pub fn test_json(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let runtime = Runtime::new().unwrap();

    let app = Router::new().route(
        "/",
        routing::post(|Json(_): Json<TestParam>| async { Json(TestParam { param: 2 }) }),
    );
    let server = TestServer::new(app);

    let req = server.post("/").json(&TestParam { param: 1 });
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    let body: TestParam = resp.json();
    expect(body.param).to_equal(2)?;

    let req = server.post("/").json(&TestWrongParam { param: "a" });
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ErrBody = resp.json();
    expect(body.code.as_str()).to_equal("err_param")
}

/// Test [`http::Path`].
pub fn test_path(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let runtime = Runtime::new().unwrap();

    let app = Router::new().route(
        "/{param}",
        routing::get(|Path(_): Path<TestParam>| async { "" }),
    );
    let server = TestServer::new(app);

    let req = server.get("/1");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;

    let req = server.get("/a");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ErrBody = resp.json();
    expect(body.code.as_str()).to_equal("err_param")
}

/// Test [`http::Query`].
pub fn test_query(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let runtime = Runtime::new().unwrap();

    let app = Router::new().route("/", routing::get(|Query(_): Query<TestParam>| async { "" }));
    let server = TestServer::new(app);

    let req = server.get("/").add_query_param("param", 1);
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;

    let req = server.get("/").add_query_param("param", "a");
    let resp = runtime.block_on(async { req.await });
    expect(resp.status_code()).to_equal(StatusCode::BAD_REQUEST)?;
    let body: ErrBody = resp.json();
    expect(body.code.as_str()).to_equal("err_param")
}
