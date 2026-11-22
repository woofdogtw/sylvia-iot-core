//! Password hashing utilities.
//!
//! Hashing passwords takes a long time, so these functions run hashing in blocking threads and
//! limit the number of concurrent hashing to the number of CPUs.

use std::{sync::LazyLock, thread};

use tokio::{sync::Semaphore, task};

use sylvia_iot_corelib::strings;

/// The length of salts.
pub const SALT_LEN: usize = 8;

/// Limits the number of concurrent password hashing.
static HASH_PERMITS: LazyLock<Semaphore> = LazyLock::new(|| {
    Semaphore::new(match thread::available_parallelism() {
        Err(_) => 1,
        Ok(count) => count.get(),
    })
});

/// The hash to verify passwords of nonexistent users, so that the response time does not reveal
/// whether the user exists.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| strings::password_hash("", ""));

/// To hash the password.
pub async fn hash(password: &str, salt: &str) -> Result<String, String> {
    let password = password.to_string();
    let salt = salt.to_string();
    let _permit = match HASH_PERMITS.acquire().await {
        Err(e) => return Err(format!("acquire hash permit error: {}", e)),
        Ok(permit) => permit,
    };
    match task::spawn_blocking(move || strings::password_hash(password.as_str(), salt.as_str()))
        .await
    {
        Err(e) => Err(format!("hash password error: {}", e)),
        Ok(hash) => Ok(hash),
    }
}

/// To verify the password with the user's `(salt, hash)`. Use [`None`] for nonexistent users to
/// verify with a dummy hash.
///
/// Returns `(matched, new_salt_hash)`. `new_salt_hash` is [`Some`] with the new `(salt, hash)` if
/// the hash should be upgraded.
pub async fn verify(
    password: &str,
    user: Option<(&str, &str)>,
) -> Result<(bool, Option<(String, String)>), String> {
    let password = password.to_string();
    let user = user.map(|(salt, hash)| (salt.to_string(), hash.to_string()));
    let _permit = match HASH_PERMITS.acquire().await {
        Err(e) => return Err(format!("acquire hash permit error: {}", e)),
        Ok(permit) => permit,
    };
    let result = task::spawn_blocking(move || match user {
        None => {
            let _ = strings::password_verify(password.as_str(), "", DUMMY_HASH.as_str());
            (false, None)
        }
        Some((salt, hash)) => {
            match strings::password_verify(password.as_str(), salt.as_str(), hash.as_str()) {
                false => (false, None),
                true => match strings::password_need_rehash(hash.as_str()) {
                    false => (true, None),
                    true => {
                        let salt = strings::randomstring(SALT_LEN);
                        let hash = strings::password_hash(password.as_str(), salt.as_str());
                        (true, Some((salt, hash)))
                    }
                },
            }
        }
    })
    .await;
    match result {
        Err(e) => Err(format!("verify password error: {}", e)),
        Ok(result) => Ok(result),
    }
}
