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
//! # Mount sylvia-iot-auth in your Actix-Web App
//!
//! You can simply mount sylvia-iot-auth into your Actix-Web App:
//!
//! ```
//! use actix_web::{self, App, HttpServer};
//! use clap::App as ClapApp;
//! use sylvia_iot_auth::{libs, routes};
//!
//! #[actix_web::main]
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
//!     HttpServer::new(move || App::new().service(routes::new_service(&auth_state)))
//!         .bind("0.0.0.0:1080")?
//!         .run()
//!         .await
//! }
//! ```
//!
//! Please see `main.rs` to get the real world example.

pub mod libs;
pub mod models;
pub mod routes;
