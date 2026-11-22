//! String libraries.

use std::{num::NonZeroU32, sync::LazyLock};

use aws_lc_rs::pbkdf2;
use chrono::{DateTime, SecondsFormat, Utc};
use hex;
use rand::{RngExt, distr::Alphanumeric};
use regex::Regex;
use sha2::{Digest, Sha256};
use url::Url;

/// PBKDF2 rounds for hashing passwords.
const PASSWORD_ROUNDS: u32 = 600000;
/// PBKDF2 rounds of the legacy password hash format (hex string without the prefix).
const PASSWORD_ROUNDS_LEGACY: u32 = 10000;
/// The prefix of the password hash format: `$pbkdf2-sha256$i=[rounds]$[hex]`.
const PASSWORD_HASH_PREFIX: &'static str = "$pbkdf2-sha256$i=";

static CODE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]{1}[a-z0-9_-]*$").unwrap());
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})$",
    )
    .unwrap()
});
static NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]{1}[a-z0-9_-]*$").unwrap());
static SCOPE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+([\.]{1}[a-z0-9]+)*$").unwrap());

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
    NAME_REGEX.is_match(account) || EMAIL_REGEX.is_match(account)
}

/// To check if the (unit/application/network) code is valid.
pub fn is_code(code: &str) -> bool {
    CODE_REGEX.is_match(code)
}

/// To check if the (client) scope is valid.
pub fn is_scope(scope: &str) -> bool {
    SCOPE_REGEX.is_match(scope)
}

/// To check if the (redirect) URI is valid.
pub fn is_uri(uri: &str) -> bool {
    Url::parse(uri).is_ok()
}

/// To hash the password with PBKDF2-HMAC-SHA256 in `$pbkdf2-sha256$i=[rounds]$[hex]` format.
///
/// **Note**: this takes a long time, please use async utility to call this function.
pub fn password_hash(password: &str, salt: &str) -> String {
    let mut res: [u8; 32] = [0; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PASSWORD_ROUNDS).unwrap(),
        salt.as_bytes(),
        password.as_bytes(),
        &mut res,
    );
    format!(
        "{}{}${}",
        PASSWORD_HASH_PREFIX,
        PASSWORD_ROUNDS,
        hex::encode(res)
    )
}

/// To verify the password with the hash from [`password_hash`] or the legacy format.
///
/// **Note**: this takes a long time, please use async utility to call this function.
pub fn password_verify(password: &str, salt: &str, hash: &str) -> bool {
    let (rounds, hash_hex) = match hash.strip_prefix(PASSWORD_HASH_PREFIX) {
        None => (PASSWORD_ROUNDS_LEGACY, hash),
        Some(value) => match value.split_once('$') {
            None => return false,
            Some((rounds, hash_hex)) => match rounds.parse::<u32>() {
                Err(_) => return false,
                Ok(rounds) => (rounds, hash_hex),
            },
        },
    };
    let rounds = match NonZeroU32::new(rounds) {
        None => return false,
        Some(rounds) => rounds,
    };
    let derived = match hex::decode(hash_hex) {
        Err(_) => return false,
        Ok(derived) => derived,
    };
    if derived.len() == 0 {
        return false;
    }
    pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA256,
        rounds,
        salt.as_bytes(),
        password.as_bytes(),
        derived.as_slice(),
    )
    .is_ok()
}

/// To check if the password hash is not generated with the current rounds by [`password_hash`].
/// Re-hash the password after verified to upgrade it.
pub fn password_need_rehash(hash: &str) -> bool {
    let current = format!("{}{}$", PASSWORD_HASH_PREFIX, PASSWORD_ROUNDS);
    !hash.starts_with(current.as_str())
}

/// To generate item ID in `[timestamp-milliseconds]-[random-alphanumeric]` format.
pub fn random_id(time: &DateTime<Utc>, len: usize) -> String {
    format!("{}-{}", time.timestamp_millis(), randomstring(len))
}

/// To generate hex-string item ID using [`random_id`] and additional hash.
///
/// **Note**: the result only has the entropy of the `len` random characters (the timestamp is
/// predictable), so do not use this for secrets such as tokens. Use [`randomstring`] instead.
pub fn random_id_sha(time: &DateTime<Utc>, len: usize) -> String {
    let str = random_id(time, len);
    let mut hasher = Sha256::new();
    hasher.update(str.as_bytes());
    hex::encode(hasher.finalize())
}

/// To generate random alphanumeric string with the specified length.
///
/// This uses a cryptographically secure random number generator, so it can be used for secrets.
pub fn randomstring(len: usize) -> String {
    let mut rng = rand::rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}

/// To convert time to RFC 3339 format with milliseconds precision (`YYYY-MM-DDThh:mm:ss.SSSZ`).
pub fn time_str(time: &DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Escapes regex special characters in a string so it can be used as a literal match pattern.
pub fn escape_regex_str(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' | '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
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
