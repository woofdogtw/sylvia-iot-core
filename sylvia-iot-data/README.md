[![crates.io](https://img.shields.io/crates/v/sylvia-iot-data)](https://crates.io/crates/sylvia-iot-data)
[![Documentation](https://docs.rs/sylvia-iot-data/badge.svg)](https://docs.rs/sylvia-iot-data)
![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![Coverage](https://raw.githubusercontent.com/woofdogtw/sylvia-iot-core/gh-pages/docs/coverage/sylvia-iot-data/badges/flat.svg)](https://woofdogtw.github.io/sylvia-iot-core/coverage/sylvia-iot-data/)
[![Docker](https://img.shields.io/docker/v/woofdogtw/sylvia-iot-data?label=docker&logo=docker)](https://hub.docker.com/r/woofdogtw/sylvia-iot-data)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-iot-data

The data storage of Sylvia-IoT core modules.

# Documentation

- [HTTP APIs](doc/api.md)

# Mount sylvia-iot-data in your axum App

You can simply mount sylvia-iot-data into your axum App:

```rust
use axum::Router;
use clap::App as ClapApp;
use std::net::SocketAddr;
use sylvia_iot_data::{libs, routes};
use tokio::{self, net::TcpListener};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = ClapApp::new("your-project-name").get_matches();

    let conf = libs::config::read_args(&args);
    let data_state = match routes::new_state("/data", &conf).await {
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        },
        Ok(state) => state,
    };
    let app = Router::new().merge(routes::new_service(&data_state));
    let listener = match TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
}
```

Please see [`src/bin/sylvia-iot-data.rs`](src/bin/sylvia-iot-data.rs) to get the real world example.

## All-in-one binary

You can use [`src/bin/sylvia-iot-core.rs`](src/bin/sylvia-iot-core.rs) as a single binary to run the whole sylvia-iot platform.
