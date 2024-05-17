use axum::{http::StatusCode, response::IntoResponse};
use laboratory::{expect, SpecContext};

use sylvia_iot_corelib::err::{self, ErrResp};

use crate::TestState;

/// Test [`err::to_json`].
pub fn to_json(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(err::to_json("without_message", None))
        .to_equal("{\"code\":\"without_message\"}".to_string())?;
    expect(err::to_json("with_message", Some("text")))
        .to_equal("{\"code\":\"with_message\",\"message\":\"text\"}".to_string())
}

/// Test `err::ErrResp::fmt` implementations.
pub fn fmt(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(format!("{}", ErrResp::ErrAuth(None)))
        .to_equal("{\"code\":\"err_auth\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrAuth(Some("auth".to_string()))))
        .to_equal("{\"code\":\"err_auth\",\"message\":\"auth\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrDb(None))).to_equal("{\"code\":\"err_db\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrDb(Some("db".to_string()))))
        .to_equal("{\"code\":\"err_db\",\"message\":\"db\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrIntMsg(None)))
        .to_equal("{\"code\":\"err_int_msg\"}".to_string())?;
    expect(format!(
        "{}",
        ErrResp::ErrIntMsg(Some("int_msg".to_string()))
    ))
    .to_equal("{\"code\":\"err_int_msg\",\"message\":\"int_msg\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrNotFound(None)))
        .to_equal("{\"code\":\"err_not_found\"}".to_string())?;
    expect(format!(
        "{}",
        ErrResp::ErrNotFound(Some("not_found".to_string()))
    ))
    .to_equal("{\"code\":\"err_not_found\",\"message\":\"not_found\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrParam(None)))
        .to_equal("{\"code\":\"err_param\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrParam(Some("param".to_string()))))
        .to_equal("{\"code\":\"err_param\",\"message\":\"param\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrPerm(None)))
        .to_equal("{\"code\":\"err_perm\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrPerm(Some("perm".to_string()))))
        .to_equal("{\"code\":\"err_perm\",\"message\":\"perm\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrRsc(None))).to_equal("{\"code\":\"err_rsc\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrRsc(Some("rsc".to_string()))))
        .to_equal("{\"code\":\"err_rsc\",\"message\":\"rsc\"}".to_string())?;
    expect(format!("{}", ErrResp::ErrUnknown(None)))
        .to_equal("{\"code\":\"err_unknown\"}".to_string())?;
    expect(format!(
        "{}",
        ErrResp::ErrUnknown(Some("unknown".to_string()))
    ))
    .to_equal("{\"code\":\"err_unknown\",\"message\":\"unknown\"}".to_string())?;
    expect(format!(
        "{}",
        ErrResp::Custom(123, "error_custom_code", None)
    ))
    .to_equal("{\"code\":\"error_custom_code\"}".to_string())?;
    expect(format!(
        "{}",
        ErrResp::Custom(123, "error_custom_code", Some("custom_code".to_string()))
    ))
    .to_equal("{\"code\":\"error_custom_code\",\"message\":\"custom_code\"}".to_string())
}

/// Test `err::ErrResp::into_response` implementations.
pub fn into_response(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(ErrResp::ErrAuth(None).into_response().status()).to_equal(StatusCode::UNAUTHORIZED)?;
    expect(ErrResp::ErrDb(None).into_response().status())
        .to_equal(StatusCode::SERVICE_UNAVAILABLE)?;
    expect(ErrResp::ErrIntMsg(None).into_response().status())
        .to_equal(StatusCode::SERVICE_UNAVAILABLE)?;
    expect(ErrResp::ErrNotFound(None).into_response().status()).to_equal(StatusCode::NOT_FOUND)?;
    expect(ErrResp::ErrParam(None).into_response().status()).to_equal(StatusCode::BAD_REQUEST)?;
    expect(ErrResp::ErrPerm(None).into_response().status()).to_equal(StatusCode::FORBIDDEN)?;
    expect(ErrResp::ErrRsc(None).into_response().status())
        .to_equal(StatusCode::SERVICE_UNAVAILABLE)?;
    expect(ErrResp::ErrUnknown(None).into_response().status())
        .to_equal(StatusCode::INTERNAL_SERVER_ERROR)?;
    expect(ErrResp::Custom(123, "", None).into_response().status())
        .to_equal(StatusCode::from_u16(123).unwrap())
}
