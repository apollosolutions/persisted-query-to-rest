use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use axum_macros::debug_handler;
use clap::{Parser, Subcommand};
use config::{generate_schema, parse_config, Endpoint, HttpMethod, LogLevel};
use graphql_request::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;

pub mod config;
pub mod graphql_request;

#[derive(Clone)]
struct EndpointHandler {
    endpoint: Endpoint,
    client: Client,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliFlags {
    /// The configuration file to use; this is required and can be relative.
    #[clap(long = "config-schema", default_value = "config.yaml")]
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

    let listener = tokio::net::TcpListener::bind(user_config.common.listen)
        .await
        .unwrap();
    info!("🚀 Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
/// The handler function for the endpoints.
/// Each endpoint uses the same handler function but uses different states to represent the PQ it is serving along with the configuration.
async fn handler(
    Path(path_parameters): Path<HashMap<String, String>>,
    State(state): State<EndpointHandler>,
    query_parameters: Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut request_parameters = HashMap::<String, Value>::new();
    if let Some(params) = state.endpoint.query_params.clone() {
        for param in params {
            if query_parameters.contains_key(param.from.clone().as_str()) {
                debug!(
                    "Query Key: {:?}, Value: {:?} QV: {:?}",
                    param.from,
                    param.to,
                    query_parameters.get(param.from.as_str()).unwrap()
                );
                let v = query_parameters.get(param.from.as_str()).unwrap();
                match param.kind.clone().from_str(v) {
                    Ok(p) => request_parameters.insert(param.to.unwrap_or(param.from.clone()), p),
                    Err(e) => return build_error_response(StatusCode::BAD_REQUEST, e.to_string()),
                };
            } else if param.required {
                return (
                    StatusCode::BAD_REQUEST,
                    HeaderMap::new(),
                    Json(Value::String(format!(
                        "Missing required parameter: {}",
                        param.from
                    ))),
                );
            }
        }
    }

    if let Some(params) = state.endpoint.path_arguments.clone() {
        for param in params {
            if path_parameters.contains_key(param.from.clone().as_str()) {
                debug!(
                    "Query Key: {:?}, Value: {:?} QV: {:?}",
                    param.from,
                    param.to,
                    path_parameters.get(param.from.as_str()).unwrap()
                );
                let v = path_parameters.get(param.from.as_str()).unwrap();
                match param.kind.clone().from_str(v) {
                    Ok(p) => request_parameters.insert(param.to.unwrap_or(param.from.clone()), p),
                    Err(e) => return build_error_response(StatusCode::BAD_REQUEST, e.to_string()),
                };
            } else if param.required {
                return (
                    StatusCode::BAD_REQUEST,
                    HeaderMap::new(),
                    Json(Value::String(format!(
                        "Missing required parameter: {}",
                        param.from
                    ))),
                );
            }
        }
    }

    debug!("Request Parameters: {:?}", request_parameters);
    let response = state
        .client
        .make_request(state.endpoint.clone(), Some(request_parameters))
        .await;
    debug!("Endpoint: {:?}", state.endpoint);
    match response {
        Ok(resp) => {
            debug!("Response: {:?}", resp);
            let status = resp.status();
            let mut headers = resp.headers().clone();
            // Remove content-length header to prevent issues with axum
            headers.remove("content-length");
            let json = resp.json::<Value>().await;
            match json {
                Ok(json) => (status, headers, Json(json)),
                Err(e) => build_error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            }
        }
        Err(e) => build_error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

fn build_error_response(
    status: StatusCode,
    message: String,
) -> (StatusCode, HeaderMap, Json<Value>) {
    (
        status,
        HeaderMap::new(),
        Json(json!({
            "errors": vec![message],
            "data":null,
        })),
    )
}
