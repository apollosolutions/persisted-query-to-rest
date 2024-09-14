use crate::{
    config::{generate_schema, parse_config, HttpMethod, LogLevel},
    handler::handler,
};
use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use clap::{Parser, Subcommand};
use graphql_request::Client;
use handler::EndpointHandler;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
pub mod config;
pub mod graphql_request;
pub mod handler;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliFlags {
    /// The configuration file to use; this is required and can be relative.
    #[clap(long = "config", short, default_value = "config.yaml")]
    config_path: String,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    ConfigSchema,
}

// TODO: Add error handling
#[tokio::main]
/// Entrypoint into the application.
async fn main() {
    // Parse the command line arguments
    let opt = CliFlags::parse();

    match opt.command {
        Some(Commands::ConfigSchema) => {
            generate_schema();
        }
        None => start_proxy(opt).await,
    }
}

async fn start_proxy(args: CliFlags) {
    // Parse the configuration file and load it
    let user_config = parse_config(args.config_path.as_str());

    // Set up logging
    let level = if let Some(logging) = user_config.common.logging.clone() {
        match logging.level {
            LogLevel::TRACE => Level::TRACE,
            LogLevel::DEBUG => Level::DEBUG,
            LogLevel::INFO => Level::INFO,
            LogLevel::WARN => Level::WARN,
            LogLevel::ERROR => Level::ERROR,
        }
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();

    // This shouldn't fail, hence the .expect()
    tracing::subscriber::set_global_default(subscriber).expect("setting default logger failed");

    let mut endpoint_routes: Router = Router::new();
    for endpoint in user_config.clone().endpoints {
        let endpoint_handler = EndpointHandler {
            endpoint: endpoint.clone(),
            client: Client::new(user_config.clone().common.graphql_endpoint.as_str()),
        };
        let path = endpoint.path.clone();
        let func = match endpoint.method {
            HttpMethod::GET => get(handler).with_state(endpoint_handler),
            HttpMethod::POST => post(handler).with_state(endpoint_handler),
            HttpMethod::PUT => put(handler).with_state(endpoint_handler),
            HttpMethod::PATCH => patch(handler).with_state(endpoint_handler),
            HttpMethod::DELETE => delete(handler).with_state(endpoint_handler),
        };

        endpoint_routes = endpoint_routes.route(&path, func);
    }

    let app = Router::new().nest(&user_config.common.path_prefix, endpoint_routes);

    // Attempt to start the listener on the provided address
    let listener = match tokio::net::TcpListener::bind(user_config.common.listen).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Error binding to address: {:?}", e);
            return;
        }
    };

    info!("🚀 Listening on {}", listener.local_addr().unwrap());
    match axum::serve(listener, app).await {
        Ok(_) => (),
        Err(e) => {
            error!("Error starting server: {:?}", e);
        }
    }
}
