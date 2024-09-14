use crate::config::Parameter;
use crate::{config::Endpoint, graphql_request::Client};
use axum::http::StatusCode;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use axum_macros::debug_handler;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

#[derive(Clone)]
pub struct EndpointHandler {
    pub endpoint: Endpoint,
    pub client: Client,
}

#[debug_handler]
/// The handler function for the endpoints.
/// Each endpoint uses the same handler function but uses different states to represent the PQ it is serving along with the configuration.
pub async fn handler(
    headers: HeaderMap,
    Path(path_parameters): Path<HashMap<String, String>>,
    State(state): State<EndpointHandler>,
    Query(query_parameters): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut request_variables = HashMap::<String, Value>::new();

    // If there are query parameters defined within the endpoint configuration, iterate through them
    let query_variables =
        match parse_parameters(query_parameters, state.endpoint.query_params.clone()) {
            Ok(p) => p,
            Err(e) => return build_error_response(StatusCode::BAD_REQUEST, e),
        };

    request_variables.extend(query_variables);

    // Repeat the same process for path parameters
    let path_variables =
        match parse_parameters(path_parameters, state.endpoint.path_arguments.clone()) {
            Ok(p) => p,
            Err(e) => return build_error_response(StatusCode::BAD_REQUEST, e),
        };

    request_variables.extend(path_variables);

    debug!("Request Parameters: {:?}", request_variables);
    let response = state
        .client
        .make_request(headers, state.endpoint.clone(), Some(request_variables))
        .await;
    debug!("Endpoint: {:?}", state.endpoint);
    match response {
        Ok(resp) => {
            debug!("Response: {:?}", resp);
            debug!("Response headers: {:?}", resp.headers());

            let status = resp.status();
            let mut headers = resp.headers().clone();

            // Remove transfer-encoding header to prevent issues with gzip responses
            headers.remove("transfer-encoding");

            let json = resp.json::<Value>().await;
            match json {
                Ok(json) => {
                    debug!("JSON: {:?}", json);
                    (status, headers, Json(json))
                }
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
            "errors": [
                {
                    "message": message
                }
            ],
            "data":null,
        })),
    )
}

fn parse_parameters(
    parameters: HashMap<String, String>,
    config_parameters: Option<Vec<Parameter>>,
) -> Result<HashMap<String, Value>, String> {
    let mut request_parameters = HashMap::<String, Value>::new();
    if let Some(params) = config_parameters {
        for param in params {
            if parameters.contains_key(param.from.clone().as_str()) {
                if let Some(value) = parameters.get(param.from.as_str()) {
                    match param.kind.clone().from_str(value) {
                        Ok(p) => {
                            request_parameters.insert(param.to.unwrap_or(param.from.clone()), p)
                        }
                        Err(e) => return Err(e.to_string()),
                    };
                }
            } else if param.required {
                return Err(format!("Missing required parameter: {}", param.from));
            }
        }
    }
    Ok(request_parameters)
}
#[cfg(test)]
mod tests {
    use std::vec;

    use axum::body::to_bytes;

    use super::*;
    use crate::config::{Endpoint, ParamKind, Parameter};
    use crate::Client;

    #[tokio::test]
    async fn test_handler_with_valid_parameters() {
        let mut server = mockito::Server::new_async().await;
        let mock_endpoint = server
            .mock("POST", "/")
            .with_header("content-type", "application/json")
            .match_body(mockito::Matcher::Json(json!({
                "variables": {
                    "param1": "value1"
                },
                "extensions": {
                    "persistedQuery": {
                        "sha256Hash": "test",
                        "version": 1
                    }
                }
            })))
            .with_body(json!({"data": "test"}).to_string())
            .create();

        let endpoint = Endpoint {
            path: "/test".to_string(),
            pq_id: "test".to_string(),
            method: crate::config::HttpMethod::GET,
            path_arguments: None,
            query_params: Some(vec![Parameter {
                from: "param1".to_string(),
                to: Some("param1".to_string()),
                kind: ParamKind::STRING,
                required: true,
            }]),
        };

        let client = Client::new(server.url().as_str());
        let state = EndpointHandler { endpoint, client };

        let query_parameters = vec![("param1".to_string(), "value1".to_string())]
            .into_iter()
            .collect();

        let (response, body) = handler(
            HeaderMap::new(),
            Path(vec![].into_iter().collect()),
            State(state),
            Query(query_parameters),
        )
        .await
        .into_response()
        .into_parts();

        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_string = std::str::from_utf8(&body_bytes).unwrap();

        assert_eq!(response.status, StatusCode::OK);
        mock_endpoint.assert();
        assert_eq!(
            body_string,
            json!({
                "data": "test"
            })
            .to_string()
        );
    }

    #[tokio::test]
    async fn test_handler_with_missing_required_parameter() {
        let endpoint = Endpoint {
            path: "/test".to_string(),
            pq_id: "test".to_string(),
            method: crate::config::HttpMethod::GET,
            path_arguments: None,
            query_params: Some(vec![Parameter {
                from: "param1".to_string(),
                to: Some("param1".to_string()),
                kind: ParamKind::STRING,
                required: true,
            }]),
        };

        let client = Client::new("");
        let state = EndpointHandler { endpoint, client };

        let path_parameters = vec![("param1".to_string(), "value1".to_string())]
            .into_iter()
            .collect();

        let query_parameters = HashMap::new();

        let (response, body) = handler(
            HeaderMap::new(),
            Path(path_parameters),
            State(state),
            Query(query_parameters),
        )
        .await
        .into_response()
        .into_parts();

        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_string = std::str::from_utf8(&body_bytes).unwrap();
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            body_string,
            json!({
                "errors": [{"message": "Missing required parameter: param1"}],
                "data": null
            })
            .to_string()
        );
    }

    #[tokio::test]
    async fn test_handler_with_invalid_parameter_value() {
        let endpoint = Endpoint {
            path: "/test".to_string(),
            pq_id: "test".to_string(),
            method: crate::config::HttpMethod::GET,
            path_arguments: Some(vec![Parameter {
                from: "param1".to_string(),
                to: Some("param1".to_string()),
                kind: ParamKind::INT,
                required: true,
            }]),
            query_params: None,
        };

        let client = Client::new("");
        let state = EndpointHandler { endpoint, client };

        let path_parameters = vec![("param1".to_string(), "value1".to_string())]
            .into_iter()
            .collect();

        let query_parameters = HashMap::new();

        let (response, body) = handler(
            HeaderMap::new(),
            Path(path_parameters),
            State(state),
            Query(query_parameters),
        )
        .await
        .into_response()
        .into_parts();

        assert_eq!(response.status, StatusCode::BAD_REQUEST);

        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_string = std::str::from_utf8(&body_bytes).unwrap();
        assert_eq!(
            body_string,
            json!({
                "errors": [{"message":"invalid digit found in string"}],
                "data": null
            })
            .to_string()
        );
    }
}
