[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-router

A simple router application with full Sylvia-IoT core components:
- sylvia-auth
- sylvia-broker
- sylvia-coremgr
- sylvia-coremgr-cli
- sylvia-data

**Note**: This is a demonstration how to develop your own system that integrates Sylvia-IoT core components.

Features:
- Multiple WAN
- One LAN using a bridge
- One optional wireless LAN shared with the ethernet LAN bridge
- One optional wireless WAN

# Configuration file notes

Because `sylvia-router-cli` includes `sylvia-coremgr-cli` by simply using `sylvia-coremgr-cli/src/lib` module,
please provides `coremgrCli` key/values in the JSON5 configuration file.

This project only support a JSON5 configuration file without parsing other command line arguments and environment variables.

# Mount sylvia-router in your Actix-Web App

You can simply mount sylvia-router into your Actix-Web App:

    use actix_web::{self, App, HttpServer};
    use clap::App as ClapApp;
    use sylvia_router::{libs, routes};

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
        HttpServer::new(move || App::new().service(routes::new_service(&router_state)))
            .bind("0.0.0.0:1080")?
            .run()
            .await
    }

Please see [`src/bin/sylvia-router.rs`](src/bin/sylvia-router.rs) to get the real world example.
