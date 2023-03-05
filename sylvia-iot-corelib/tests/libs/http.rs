use actix_web::{http::header, test::TestRequest};
use laboratory::{expect, SpecContext};

use sylvia_iot_corelib::http;

use crate::TestState;

/// Test [`http::parse_header_auth`].
pub fn parse_header_auth(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let req = TestRequest::default().to_http_request();
    let result = http::parse_header_auth(&req);
    match result {
        Err(e) => return Err(format!("empty Authorization header error: {}", e)),
        Ok(content) => expect(content).to_equal(None)?,
    }

    let req = TestRequest::default()
        .append_header((header::AUTHORIZATION, "test"))
        .append_header((header::AUTHORIZATION, "test"))
        .to_http_request();
    let result = http::parse_header_auth(&req);
    if result.is_ok() {
        return Err("multiple Authorization header not error".to_string());
    }

    let req = TestRequest::default()
        .append_header((header::AUTHORIZATION, "test"))
        .to_http_request();
    let result = http::parse_header_auth(&req);
    match result {
        Err(e) => return Err(format!("Authorization header error: {}", e)),
        Ok(content) => expect(content).to_equal(Some("test".to_string())),
    }
}
