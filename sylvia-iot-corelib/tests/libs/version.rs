use axum::{Router, http::StatusCode, routing};
use axum_test::TestServer;
use laboratory::{SpecContext, expect};
use tokio::runtime::Runtime;

use sylvia_iot_corelib::version;

use crate::TestState;

const TEST_NAME: &'static str = "test-name";
const TEST_VER: &'static str = "1.2.3";

/// Test [`version::gen_get_version`].
pub fn gen_get_version(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let runtime = Runtime::new().unwrap();

    let app = Router::new().route(
        "/version",
        routing::get(version::gen_get_version(TEST_NAME, TEST_VER)),
    );
    let server = TestServer::new(app);

    let expect_full = format!(
        "{{\"data\":{{\"name\":\"{}\",\"version\":\"{}\"}}}}",
        TEST_NAME, TEST_VER
    );

    // Default: no query parameter.
    let resp = runtime.block_on(async { server.get("/version").await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    expect(resp.text().as_ref()).to_equal(expect_full.as_str().as_bytes())?;

    // Invalid query parameter: returns full JSON.
    let resp =
        runtime.block_on(async { server.get("/version").add_query_param("q", "test").await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    expect(resp.text().as_ref()).to_equal(expect_full.as_str().as_bytes())?;

    // Query name.
    let resp =
        runtime.block_on(async { server.get("/version").add_query_param("q", "name").await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    expect(resp.text().as_ref()).to_equal(TEST_NAME.as_bytes())?;

    // Query version.
    let resp =
        runtime.block_on(async { server.get("/version").add_query_param("q", "version").await });
    expect(resp.status_code()).to_equal(StatusCode::OK)?;
    expect(resp.text().as_ref()).to_equal(TEST_VER.as_bytes())
}
