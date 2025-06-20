use chrono::{TimeZone, Utc};
use laboratory::{SpecContext, expect};

use sylvia_iot_corelib::strings;

use crate::TestState;

/// Test [`strings::hex_addr_to_u128`].
pub fn hex_addr_to_u128(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::hex_addr_to_u128("").is_err()).to_equal(true)?;
    expect(strings::hex_addr_to_u128("0123456789abcdef0123456789abcdef01").is_err())
        .to_equal(true)?;
    expect(strings::hex_addr_to_u128("0").is_err()).to_equal(true)?;
    expect(strings::hex_addr_to_u128("gh").is_err()).to_equal(true)?;
    expect(strings::hex_addr_to_u128("0123456789abcdef0123456789abcdef").is_ok()).to_equal(true)
}

/// Test [`strings::is_account`].
pub fn is_account(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::is_account("abc-_")).to_equal(true)?;
    expect(strings::is_account("email@example.com")).to_equal(true)?;
    expect(strings::is_account("_abc")).to_equal(false)?;
    expect(strings::is_account("email@example.com@")).to_equal(false)
}

/// Test [`strings::is_code`].
pub fn is_code(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::is_code("abc-_")).to_equal(true)?;
    expect(strings::is_code("_abc")).to_equal(false)
}

/// Test [`strings::is_scope`].
pub fn is_scope(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::is_scope("abc.def")).to_equal(true)?;
    expect(strings::is_scope("abc..abc")).to_equal(false)
}

/// Test [`strings::is_uri`].
pub fn is_uri(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::is_uri("http://localhost/redirect")).to_equal(true)?;
    expect(strings::is_uri(":://")).to_equal(false)
}

/// Test [`strings::password_hash`].
pub fn password_hash(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::password_hash("password", "salt"))
        .to_equal("5ec02b91a4b59c6f59dd5fbe4ca649ece4fa8568cdb8ba36cf41426e8805522b".to_string())
}

/// Test [`strings::random_id`].
pub fn random_id(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let now = Utc::now();
    expect(strings::random_id(&now, 10)).to_not_equal(strings::random_id(&now, 10))
}

/// Test [`strings::random_id_sha`].
pub fn random_id_sha(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let now = Utc::now();
    expect(strings::random_id_sha(&now, 10)).to_not_equal(strings::random_id_sha(&now, 10))
}

/// Test [`strings::randomstring`].
pub fn randomstring(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::randomstring(10)).to_not_equal(strings::randomstring(10))
}

/// Test [`strings::time_str`].
pub fn time_str(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let time = Utc.timestamp_nanos(1629469195228_000000);
    expect(strings::time_str(&time)).to_equal("2021-08-20T14:19:55.228Z".to_string())
}

/// Test [`strings::u128_to_addr`].
pub fn u128_to_addr(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let addr: u128 = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef;
    expect(strings::u128_to_addr(addr, 2)).to_equal("ef".to_string())?;
    expect(strings::u128_to_addr(addr, 4)).to_equal("cdef".to_string())?;
    expect(strings::u128_to_addr(addr, 6)).to_equal("abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 8)).to_equal("89abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 10)).to_equal("6789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 12)).to_equal("456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 14)).to_equal("23456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 16)).to_equal("0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 18)).to_equal("ef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 20)).to_equal("cdef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 22)).to_equal("abcdef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 24)).to_equal("89abcdef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 26)).to_equal("6789abcdef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 28)).to_equal("456789abcdef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 30))
        .to_equal("23456789abcdef0123456789abcdef".to_string())?;
    expect(strings::u128_to_addr(addr, 32)).to_equal("0123456789abcdef0123456789abcdef".to_string())
}
