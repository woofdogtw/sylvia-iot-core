//! The authentication/authorization module of the Sylvia-IoT platform.
//!
//! This module provides:
//!
//! - OAuth2 authorization that supports the following grant flows:
//!     - Authorization code
//!     - Client credentials (to be implemented)
//! - User management.
//! - Client management.
//!
//!
//! # Mount sylvia-iot-auth in your axum App
//!
//! You can simply mount sylvia-iot-auth into your axum App:
//!
//! ```
//! use axum::Router;
//! use clap::App as ClapApp;
//! use std::net::SocketAddr;
//! use sylvia_iot_auth::{libs, routes};
//! use tokio::{self, net::TcpListener};
//!
//! #[tokio::main]
//! async fn main() -> std::io::Result<()> {
//!     let args = ClapApp::new("your-project-name").get_matches();
//!
//!     let conf = libs::config::read_args(&args);
//!     let auth_state = match routes::new_state("/auth", &conf).await {
//!         Err(e) => {
//!             println!("Error: {}", e);
//!             return Ok(());
//!         },
//!         Ok(state) => state,
//!     };
//!     let app = Router::new().merge(routes::new_service(&auth_state));
//!     let listener = match TcpListener::bind(http_addr).await.unwrap();
//!     axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
//! }
//! ```
//!
//! Please see `main.rs` to get the real world example.

pub mod libs;
pub mod models;
pub mod routes;
