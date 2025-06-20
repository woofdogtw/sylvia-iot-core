use std::{env, ffi::OsStr};

pub mod err;
pub mod http;
pub mod logger;
pub mod role;
pub mod server_config;
pub mod strings;

fn set_env_var(key: &str, val: &str) {
    unsafe {
        env::set_var(&OsStr::new(key), val);
    }
}
