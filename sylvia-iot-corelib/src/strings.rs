//! String libraries.

use chrono::{DateTime, SecondsFormat, Utc};
use hex;
use hmac::Hmac;
use pbkdf2;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::Regex;
use sha2::{Digest, Sha256};
use url::Url;

const PASSWORD_ROUNDS: u32 = 10000;

/// To transfer hex address string to 128-bit integer.
pub fn hex_addr_to_u128(addr: &str) -> Result<u128, &'static str> {
    if addr.len() == 0 || addr.len() > 32 || addr.len() % 2 != 0 {
        return Err("invalid address format");
    }

    match u128::from_str_radix(addr, 16) {
        Err(_) => Err("invalid address format"),
        Ok(value) => Ok(value),
    }
}

/// To check if the account is valid.
pub fn is_account(account: &str) -> bool {
    let name_regex = Regex::new(r"^[a-z0-9]{1}[a-z0-9_-]*$").unwrap();
    let email_regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .unwrap();

    name_regex.is_match(account) || email_regex.is_match(account)
}

/// To check if the (unit/application/network) code is valid.
pub fn is_code(code: &str) -> bool {
    let regex = Regex::new(r"^[a-z0-9]{1}[a-z0-9_-]*$").unwrap();
    regex.is_match(code)
}

/// To check if the (client) scope is valid.
pub fn is_scope(scope: &str) -> bool {
    let regex = Regex::new(r"^[a-z0-9]+([\.]{1}[a-z0-9]+)*$").unwrap();
    regex.is_match(scope)
}

/// To check if the (redirect) URI is valid.
pub fn is_uri(uri: &str) -> bool {
    Url::parse(uri).is_ok()
}

/// To hash the password.
pub fn password_hash(password: &str, salt: &str) -> String {
    let mut res: [u8; 32] = [0; 32];
    let _ = pbkdf2::pbkdf2::<Hmac<Sha256>>(
        password.as_bytes(),
        salt.as_bytes(),
        PASSWORD_ROUNDS,
        &mut res,
    );
    hex::encode(res)
}

/// To generate item ID in `[timestamp-milliseconds]-[random-alphanumeric]` format.
pub fn random_id(time: &DateTime<Utc>, len: usize) -> String {
    format!("{}-{}", time.timestamp_millis(), randomstring(len))
}

/// To generate hex-string item ID using [`random_id`] and additional hash.
pub fn random_id_sha(time: &DateTime<Utc>, len: usize) -> String {
    let str = random_id(time, len);
    let mut hasher = Sha256::new();
    hasher.update(str.as_bytes());
    hex::encode(hasher.finalize())
}

/// To generate random alphanumeric string with the specified length.
pub fn randomstring(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}

/// To convert time to ISO8601 format with milliseconds precision (`YYYY-MM-DDThh:mm:ss.SSSZ`).
pub fn time_str(time: &DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// To generate hex address string with the specified length (hex string length).
pub fn u128_to_addr(value: u128, len: usize) -> String {
    match len {
        0 | 1 | 2 => format!("{:02x}", value & 0xff),
        3 | 4 => format!("{:04x}", value & 0xffff),
        5 | 6 => format!("{:06x}", value & 0xff_ffff),
        7 | 8 => format!("{:08x}", value & 0xffff_ffff),
        9 | 10 => format!("{:010x}", value & 0xff_ffff_ffff),
        11 | 12 => format!("{:012x}", value & 0xffff_ffff_ffff),
        13 | 14 => format!("{:014x}", value & 0xff_ffff_ffff_ffff),
        15 | 16 => format!("{:016x}", value & 0xffff_ffff_ffff_ffff),
        17 | 18 => format!("{:018x}", value & 0xff_ffff_ffff_ffff_ffff),
        19 | 20 => format!("{:020x}", value & 0xffff_ffff_ffff_ffff_ffff),
        21 | 22 => format!("{:022x}", value & 0xff_ffff_ffff_ffff_ffff_ffff),
        23 | 24 => format!("{:024x}", value & 0xffff_ffff_ffff_ffff_ffff_ffff),
        25 | 26 => format!("{:026x}", value & 0xff_ffff_ffff_ffff_ffff_ffff_ffff),
        27 | 28 => format!("{:028x}", value & 0xffff_ffff_ffff_ffff_ffff_ffff_ffff),
        29 | 30 => format!("{:030x}", value & 0xff_ffff_ffff_ffff_ffff_ffff_ffff_ffff),
        _ => format!("{:032x}", value),
    }
}
