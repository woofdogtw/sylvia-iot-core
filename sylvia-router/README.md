![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-router

A simple router application with full Sylvia-IoT core components:
- sylvia-iot-auth
- sylvia-iot-broker
- sylvia-iot-coremgr
- sylvia-iot-coremgr-cli
- sylvia-iot-data

**Note**: This is a demonstration how to develop your own system that integrates Sylvia-IoT core components.

Features:
- Multiple WAN
- One LAN using a bridge
- One optional wireless LAN shared with the ethernet LAN bridge
- One optional wireless WAN

# Configuration file notes

Because `sylvia-router-cli` includes `sylvia-iot-coremgr-cli` by simply using `sylvia-iot-coremgr-cli/src/lib` module,
please provides `coremgrCli` key/values in the JSON5 configuration file.

This project only support a JSON5 configuration file without parsing other command line arguments and environment variables.

# Documentation

- [HTTP APIs](doc/api.md)

# Mount sylvia-router in your axum App

You can simply mount sylvia-router into your axum App:

```rust
use axum::Router;
use clap::App as ClapApp;
use std::net::SocketAddr;
use sylvia_router::{libs, routes};
use tokio::{self, net::TcpListener};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = ClapApp::new("your-project-name").get_matches();

    let conf = libs::config::read_args(&args);
    let router_state = match routes::new_state("/router", &conf).await {
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        },
        Ok(state) => state,
    };
    let app = Router::new().merge(routes::new_service(&router_state));
    let listener = match TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
}
```

Please see [`src/bin/sylvia-router.rs`](src/bin/sylvia-router.rs) to get the real world example.
