![GitHub Actions](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
![Coverage](https://raw.githubusercontent.com/woofdogtw/sylvia-iot-core/gh-pages/docs/coverage/sylvia-iot-broker/badges/flat.svg)

# sylvia-broker

The message broker module of the Sylvia-IoT platform.

This module provides:

- Unit management for each owner, including
    - Applications
    - Private networks
    - Devices
    - Device routes to applications
    - Private network routes to applications
- Public network management

# Mount sylvia-broker in your Actix-Web App

You can simply mount sylvia-broker into your Actix-Web App:

    use actix_web::{self, App, HttpServer};
    use clap::App as ClapApp;
    use sylvia_iot_broker::{libs, routes};

    #[tokio::main]
    async fn main() -> std::io::Result<()> {
        let args = ClapApp::new("your-project-name").get_matches();

        let conf = libs::config::read_args(&args);
        let broker_state = match routes::new_state("/broker", &conf).await {
            Err(e) => {
                println!("Error: {}", e);
                return Ok(());
            },
            Ok(state) => state,
        };
        HttpServer::new(move || App::new().service(routes::new_service(&broker_state)))
            .bind("0.0.0.0:1080")?
            .run()
            .await
    }

Please see [`src/bin/sylvia-iot-broker.rs`](src/bin/sylvia-iot-broker.rs) to get the real world example.
