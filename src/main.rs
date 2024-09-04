use std::collections::HashMap;
use axum::{extract::Path, routing::{delete, get, patch, post, put}, Router};
use config::{Endpoint, HttpMethod, parse_config};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub(crate) mod config;

// TODO: Add error handling
#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    // TODO: Use config to determine output + better error handling
    tracing::subscriber::set_global_default(subscriber).expect("setting default logger failed");

    // TODO: add flag to specify the config file vs. hardcoded
    let user_config = parse_config("example_config.yaml");
    let mut endpoint_routes: Router = Router::new();

    for endpoint in user_config.endpoints {
        let path = endpoint.path.clone();
        let func = if let Some(method) = endpoint.method {
            match method {
                HttpMethod::GET => get(move |params| handler(params, endpoint)),
                HttpMethod::POST => post(move |params| handler(params, endpoint)),
                HttpMethod::PUT => put(move |params| handler(params, endpoint)),
                HttpMethod::PATCH => patch(move |params| handler(params, endpoint)),
                HttpMethod::DELETE => delete(move |params| handler(params, endpoint)),
            }
            } else {
                get(move |params| handler(params, endpoint))
            };
            endpoint_routes = endpoint_routes.route(&path, func);

        }


    let prefix = if let Some(path_prefix) = &user_config.common.path_prefix {
        path_prefix
    } else {
        "/api/v1"
    };

    let app = Router::new().nest(&prefix, endpoint_routes);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("🚀 Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

// TODO: Actually implement the handler
async fn handler(Path(params): Path<HashMap<String, String>>, endpoint: Endpoint) -> &'static str {
    info!("Params: {:?}", params);
    info!("Endpoint: {:?}", endpoint);
    "Hello, User!"
}
