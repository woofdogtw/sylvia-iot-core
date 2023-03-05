//! Redis model implementation.
//!
//! Currently we do not use Redis because it uses `&mut self` that is not compatible with model
//! trait interface. Redis connection does not implement [`Clone`] so we cannot use
//! `Arc<Mutex<Connection>>` to implement.

pub mod access_token;
pub mod authorization_code;
pub mod conn;
pub mod refresh_token;
