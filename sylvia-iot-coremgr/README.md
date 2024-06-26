[![crates.io](https://img.shields.io/crates/v/sylvia-iot-coremgr)](https://crates.io/crates/sylvia-iot-coremgr)
[![Documentation](https://docs.rs/sylvia-iot-coremgr/badge.svg)](https://docs.rs/sylvia-iot-coremgr)
![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![Coverage](https://raw.githubusercontent.com/woofdogtw/sylvia-iot-core/gh-pages/docs/coverage/sylvia-iot-coremgr/badges/flat.svg)](https://woofdogtw.github.io/sylvia-iot-core/coverage/sylvia-iot-coremgr/)
[![Docker](https://img.shields.io/docker/v/woofdogtw/sylvia-iot-coremgr?label=docker&logo=docker)](https://hub.docker.com/r/woofdogtw/sylvia-iot-coremgr)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-iot-coremgr

The manager of Sylvia-IoT core modules.

This module:

- Integrate APIs of `sylvia-iot-auth` and `sylvia-iot-broker`.
- Management the underlying brokers for applications and networks. Now we support:
    - AMQP: RabbitMQ
    - MQTT: EMQX
- Records all POST, PUT, PATCH, DELETE APIs.

# Documentation

- [HTTP APIs](doc/api.md)
- [Messages](doc/message.md)

# Mount sylvia-iot-coremgr in your axum App

You can simply mount sylvia-iot-coremgr into your axum App:

```rust
use axum::Router;
use clap::App as ClapApp;
use std::net::SocketAddr;
use sylvia_iot_coremgr::{libs, routes};
use tokio::{self, net::TcpListener};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = ClapApp::new("your-project-name").get_matches();

    let conf = libs::config::read_args(&args);
    let coremgr_state = match routes::new_state("/coremgr", &conf).await {
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        },
        Ok(state) => state,
    };
    let app = Router::new().merge(routes::new_service(&coremgr_state));
    let listener = match TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
}
```

Please see [`src/bin/sylvia-iot-coremgr.rs`](src/bin/sylvia-iot-coremgr.rs) to get the real world example.

## All-in-one binary

You can use [`src/bin/sylvia-iot-core.rs`](src/bin/sylvia-iot-core.rs) as a single binary to run the whole sylvia-iot platform.
