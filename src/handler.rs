use crate::config::Parameter;
use crate::{config::Endpoint, graphql_request::Client};
use axum::http::StatusCode;
use axum::{
    extract::{Json as ExtractJson, Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

#[derive(Clone)]
pub struct EndpointHandler {
    pub endpoint: Endpoint,
    pub client: Client,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ClientResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<ClientResponseError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extensions: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ClientResponseError {
    message: String,
}

#[debug_handler]
/// The handler function for the endpoints.
/// Each endpoint uses the same handler function but uses different states to represent the PQ it is serving along with the configuration.
pub async fn handler(
    headers: HeaderMap,
    Path(path_parameters): Path<HashMap<String, String>>,
    State(state): State<EndpointHandler>,
    Query(query_parameters): Query<HashMap<String, String>>,
    body: Option<ExtractJson<Value>>,
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

    // If the endpoint is configured to use the request body as variables, parse the body
    let bp = match body {
        Some(body) => {
            let mut m = HashMap::<String, String>::new();
            // convert the body to a hashmap of strings since we really only care about top level keys at the moment
            for (key, val) in body.as_object().unwrap() {
                let value = val.as_str();
                match value {
                    Some(v) => {
                        m.insert(key.clone(), v.to_string());
                    }
                    None => {
                        m.insert(key.clone(), val.to_string());
                    }
                }
            }
            m
        }
        None => HashMap::<String, String>::new(),
    };
    let body_params = match parse_parameters(bp, state.endpoint.body_params.clone()) {
        Ok(p) => p,
        Err(e) => return build_error_response(StatusCode::BAD_REQUEST, e),
    };
    request_variables.extend(body_params);

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

            let mut status = resp.status();
            let mut headers = resp.headers().clone();

            // Remove transfer-encoding header to prevent issues with gzip responses
            headers.remove("transfer-encoding");

            let json = resp.json::<ClientResponse>().await;
            match json {
                Ok(json) => {
                    debug!("JSON: {:?}", json);
                    if let Some(ref errors) = json.errors {
                        // If there are errors in the response, set the status to 500 if the response is 200 or 400; this prioritizes the status returned by the router in non-compliant situations
                        if status == StatusCode::OK && !errors.is_empty() {
                            status = StatusCode::INTERNAL_SERVER_ERROR;
                            // If there is data in the response, set the status to 206 to indicate partial content per RFC
                            if json.data.is_some() {
                                status = StatusCode::PARTIAL_CONTENT;
                            }
                        }
                    }
                    (status, headers, Json(json!(json)))
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
    request_parameters: HashMap<String, String>,
    config_parameters: Option<Vec<Parameter>>,
) -> Result<HashMap<String, Value>, String> {
    let mut parameters = HashMap::<String, Value>::new();
    if let Some(params) = config_parameters {
        for param in params {
            if request_parameters.contains_key(param.from.clone().as_str()) {
                if let Some(value) = request_parameters.get(param.from.as_str()) {
                    match param.kind.clone().from_str(value) {
                        Ok(p) => parameters.insert(param.to.unwrap_or(param.from.clone()), p),
                        Err(e) => return Err(e.to_string()),
                    };
                }
            } else if param.required {
                return Err(format!("Missing required parameter: {}", param.from));
            }
        }
    }
    Ok(parameters)
}
#[cfg(test)]
mod tests {
    use std::vec;

    use axum::body::to_bytes;

    use super::*;
    use crate::config::{Endpoint, ParamKind, Parameter};
    use crate::Client;

    #[tokio::test]
    async fn test_parse_parameters() {
        let request_parameters = vec![("param1".to_string(), "value1".to_string())]
            .into_iter()
            .collect();

        let config_parameters = Some(vec![Parameter {
            from: "param1".to_string(),
            to: Some("param1".to_string()),
            kind: ParamKind::STRING,
            required: true,
        }]);

        let result = parse_parameters(request_parameters, config_parameters);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("param1").unwrap(),
            &Value::String("value1".to_string())
        );
    }

    #[tokio::test]
    async fn test_parse_parameters_with_missing_required_parameter() {
        let request_parameters = HashMap::new();

        let config_parameters = Some(vec![Parameter {
            from: "param1".to_string(),
            to: Some("param1".to_string()),
            kind: ParamKind::STRING,
            required: true,
        }]);

        let result = parse_parameters(request_parameters, config_parameters);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Missing required parameter: param1");
    }

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
            body_params: None,
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
            None,
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
    async fn test_handler_returns_500() {
        let server_body = json!({
            "errors": [{"message": "test"}]
        });
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
            .with_body(server_body.to_string())
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

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        mock_endpoint.assert();
        assert_eq!(body_string, server_body.to_string());
    }

    #[tokio::test]
    async fn test_handler_returns_206() {
        let server_body = json!({
            "data": "test",
            "errors": [{"message": "test"}]
        });
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
            .with_body(server_body.to_string())
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

        assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
        mock_endpoint.assert();
        assert_eq!(body_string, server_body.to_string());
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
            body_params: None,
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
            None,
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
            body_params: None,
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
            None,
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

    #[tokio::test]
    async fn test_handler_with_body_params() {
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
            method: crate::config::HttpMethod::POST,
            path: "/test".to_string(),
            pq_id: "test".to_string(),
            path_arguments: None,
            query_params: None,
            body_params: Some(vec![Parameter {
                from: "param1".to_string(),
                to: Some("param1".to_string()),
                kind: ParamKind::STRING,
                required: true,
            }]),
        };

        let client = Client::new(server.url().as_str());
        let state = EndpointHandler { endpoint, client };
        let query_parameters = HashMap::new();

        let (response, body) = handler(
            HeaderMap::new(),
            Path(vec![].into_iter().collect()),
            State(state),
            Query(query_parameters),
            Some(Json(json!({"param1": "value1"}))),
        )
        .await
        .into_response()
        .into_parts();

        mock_endpoint.assert();
        assert_eq!(response.status, StatusCode::OK);

        let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
        let body_string = std::str::from_utf8(&body_bytes).unwrap();
        assert_eq!(
            body_string,
            json!({
                "data": "test"
            })
            .to_string()
        );
    }

    #[tokio::test]
    async fn test_handler_with_missing_body_params() {
        let endpoint = Endpoint {
            method: crate::config::HttpMethod::POST,
            path: "/test".to_string(),
            pq_id: "test".to_string(),
            path_arguments: None,
            query_params: None,
            body_params: Some(vec![Parameter {
                from: "param1".to_string(),
                to: Some("param1".to_string()),
                kind: ParamKind::STRING,
                required: true,
            }]),
        };

        let client = Client::new("");
        let state = EndpointHandler { endpoint, client };
        let query_parameters = HashMap::new();

        let (response, body) = handler(
            HeaderMap::new(),
            Path(vec![].into_iter().collect()),
            State(state),
            Query(query_parameters),
            None,
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
                "errors": [{"message": "Missing required parameter: param1"}],
                "data": null
            })
            .to_string()
        );
    }
}
