![GitHub Actions](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
![Coverage](https://raw.githubusercontent.com/woofdogtw/sylvia-iot-core/gh-pages/docs/coverage/sylvia-iot-data/badges/flat.svg)

# sylvia-data

The data storage of Sylvia-IoT core modules.

# Mount sylvia-data in your Actix-Web App

You can simply mount sylvia-data into your Actix-Web App:

    use actix_web::{self, App, HttpServer};
    use clap::App as ClapApp;
    use sylvia_iot_data::{libs, routes};

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
        HttpServer::new(move || App::new().service(routes::new_service(&data_state)))
            .bind("0.0.0.0:1080")?
            .run()
            .await
    }

Please see [`src/bin/sylvia-iot-data.rs`](src/bin/sylvia-iot-data.rs) to get the real world example.

## All-in-one binary

You can use [`src/bin/sylvia-iot-core.rs`](src/bin/sylvia-iot-core.rs) as a single binary to run the whole sylvia-iot platform.
