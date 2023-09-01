[![crates.io](https://img.shields.io/crates/v/sylvia-iot-auth)](https://crates.io/crates/sylvia-iot-auth)
[![Documentation](https://docs.rs/sylvia-iot-auth/badge.svg)](https://docs.rs/sylvia-iot-auth)
![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![Coverage](https://raw.githubusercontent.com/woofdogtw/sylvia-iot-core/gh-pages/docs/coverage/sylvia-iot-auth/badges/flat.svg)](https://woofdogtw.github.io/sylvia-iot-core/coverage/sylvia-iot-auth/)
[![Docker](https://img.shields.io/docker/v/woofdogtw/sylvia-iot-auth?label=docker&logo=docker)](https://hub.docker.com/r/woofdogtw/sylvia-iot-auth)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-iot-auth

The authentication/authorization module of the Sylvia-IoT platform.

This module provides:

- OAuth2 authorization that supports the following grant flows:
    - Authorization code
    - Client credentials (to be implemented)
- User management.
- Client management.

# Documentation

- [HTTP APIs](doc/api.md)

# Mount sylvia-iot-auth in your Actix-Web App

You can simply mount sylvia-iot-auth into your Actix-Web App:

```rust
use actix_web::{self, App, HttpServer};
use clap::App as ClapApp;
use sylvia_iot_auth::{libs, routes};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = ClapApp::new("your-project-name").get_matches();

    let conf = libs::config::read_args(&args);
    let auth_state = match routes::new_state("/auth", &conf).await {
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        },
        Ok(state) => state,
    };
    HttpServer::new(move || App::new().service(routes::new_service(&auth_state)))
        .bind("0.0.0.0:1080")?
        .run()
        .await
}
```

Please see [`src/bin/sylvia-iot-auth.rs`](src/bin/sylvia-iot-auth.rs) to get the real world example.
