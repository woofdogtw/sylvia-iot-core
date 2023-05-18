//! This module provides common module/functions for different kind of databases.
//!
//! Most of model test cases only use [`tokio::runtime::Runtime`] and [`sylvia_iot_auth::models`]
//! arguments, this module can reduce code size and lines.

pub mod access_token;
pub mod authorization_code;
pub mod client;
pub mod login_session;
pub mod refresh_token;
pub mod user;
