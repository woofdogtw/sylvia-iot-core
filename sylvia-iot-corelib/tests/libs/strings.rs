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

/// The password hash of `password` with salt `salt`.
const PASSWORD_HASH: &'static str =
    "$pbkdf2-sha256$i=600000$669cfe52482116fda1aa2cbe409b2f56c8e4563752b7a28f6eaab614ee005178";
/// The legacy password hash (10000 rounds) of `password` with salt `salt`.
const PASSWORD_HASH_LEGACY: &'static str =
    "5ec02b91a4b59c6f59dd5fbe4ca649ece4fa8568cdb8ba36cf41426e8805522b";

/// Test [`strings::password_hash`].
pub fn password_hash(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::password_hash("password", "salt")).to_equal(PASSWORD_HASH.to_string())
}

/// Test [`strings::password_verify`].
pub fn password_verify(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::password_verify("password", "salt", PASSWORD_HASH)).to_equal(true)?;
    expect(strings::password_verify(
        "password",
        "salt",
        PASSWORD_HASH_LEGACY,
    ))
    .to_equal(true)?;
    expect(strings::password_verify("wrong", "salt", PASSWORD_HASH)).to_equal(false)?;
    expect(strings::password_verify(
        "wrong",
        "salt",
        PASSWORD_HASH_LEGACY,
    ))
    .to_equal(false)?;
    expect(strings::password_verify("password", "wrong", PASSWORD_HASH)).to_equal(false)?;

    // Other rounds.
    let hash =
        "$pbkdf2-sha256$i=10000$5ec02b91a4b59c6f59dd5fbe4ca649ece4fa8568cdb8ba36cf41426e8805522b";
    expect(strings::password_verify("password", "salt", hash)).to_equal(true)?;

    // Invalid formats.
    let hashes = [
        "",
        "$pbkdf2-sha256$i=",
        "$pbkdf2-sha256$i=600000",
        "$pbkdf2-sha256$i=600000$",
        "$pbkdf2-sha256$i=0$669cfe52482116fda1aa2cbe409b2f56c8e4563752b7a28f6eaab614ee005178",
        "$pbkdf2-sha256$i=abc$669cfe52482116fda1aa2cbe409b2f56c8e4563752b7a28f6eaab614ee005178",
        "$pbkdf2-sha256$i=600000$zz",
        "zz",
    ];
    for hash in hashes {
        if strings::password_verify("password", "salt", hash) {
            return Err(format!("hash `{}` should be invalid", hash));
        }
    }
    Ok(())
}

/// Test [`strings::password_need_rehash`].
pub fn password_need_rehash(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::password_need_rehash(PASSWORD_HASH)).to_equal(false)?;
    expect(strings::password_need_rehash(PASSWORD_HASH_LEGACY)).to_equal(true)?;
    let hash =
        "$pbkdf2-sha256$i=10000$5ec02b91a4b59c6f59dd5fbe4ca649ece4fa8568cdb8ba36cf41426e8805522b";
    expect(strings::password_need_rehash(hash)).to_equal(true)?;
    expect(strings::password_need_rehash("$pbkdf2-sha256$i=6000000$00")).to_equal(true)
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

/// Test [`strings::escape_regex_str`].
pub fn escape_regex_str(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    expect(strings::escape_regex_str("hello")).to_equal("hello".to_string())?;
    expect(strings::escape_regex_str("a.b")).to_equal("a\\.b".to_string())?;
    expect(strings::escape_regex_str(".*+?^${}()|[]\\"))
        .to_equal("\\.\\*\\+\\?\\^\\$\\{\\}\\(\\)\\|\\[\\]\\\\".to_string())?;
    expect(strings::escape_regex_str("")).to_equal("".to_string())?;
    expect(strings::escape_regex_str("abc123")).to_equal("abc123".to_string())
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
