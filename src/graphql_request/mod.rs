use axum::http::HeaderMap;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

use crate::config::{self};

#[derive(Serialize, Deserialize)]
struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<HashMap<String, Value>>,
    extensions: RequestBodyExtensions,
}
#[derive(Serialize, Deserialize)]
struct RequestBodyExtensions {
    #[serde(rename = "persistedQuery")]
    persisted_query: RequestBodyPersistedQuery,
}

#[derive(Serialize, Deserialize)]
struct RequestBodyPersistedQuery {
    #[serde(rename = "sha256Hash")]
    sha_256_hash: String,
    version: i32,
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    url: String,
}

impl Client {
    pub fn new(url: &str) -> Self {
        let client = match reqwest::Client::builder().build() {
            Ok(c) => c,
            Err(e) => panic!("Failed to create client: {:?}", e),
        };
        Self {
            client: client,
            url: url.to_string(),
        }
    }

    pub async fn make_request(
        &self,
        mut request_headers: HeaderMap,
        endpoint: config::Endpoint,
        parameters: Option<HashMap<String, Value>>,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self.client.request(reqwest::Method::POST, &self.url);

        // Usage of unwrap is safe here because the headers are hardcoded and will always be valid
        request_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        request_headers.insert("apollographql-client-name", "rest_bridge".parse().unwrap());
        request_headers.insert("accept", "*/*".parse().unwrap());

        // Remove the host header to prevent issues with the proxy
        request_headers.remove("host");

        request = request.headers(request_headers.clone());
        debug!("Request Headers: {:?}", request_headers);
        debug!("Making request to: {}", &self.url);
        let variables = if let Some(params) = parameters {
            Some(params)
        } else {
            None
        };

        let body = RequestBody {
            variables: variables,
            extensions: RequestBodyExtensions {
                persisted_query: RequestBodyPersistedQuery {
                    sha_256_hash: endpoint.pq_id.clone(),
                    version: 1,
                },
            },
        };

        match serde_json::to_string(&body) {
            Ok(json) => {
                debug!("JSON: {:?}", json);
                request = request.body(json);
            }
            Err(e) => return Err(Box::from(e.to_string().as_str())),
        }

        match request.send().await {
            Ok(resp) => Ok(resp),
            Err(e) => Err(Box::from(e.to_string().as_str())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use serde_json::json;

    #[tokio::test]
    async fn test_make_request() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();

        let mock_endpoint = server
            .mock("POST", "/")
            .with_header("content-type", "application/json")
            .match_header("test", "test")
            .match_body(mockito::Matcher::Json(json!({
                "extensions": {
                    "persistedQuery": {
                        "sha256Hash": "test",
                        "version": 1
                    }
                }
            })))
            .create();
        let client = Client::new(url.as_str());
        let mut headers = HeaderMap::new();
        headers.insert("test", "test".parse().unwrap());
        let endpoint = config::Endpoint {
            method: config::HttpMethod::GET,
            path: "/test".to_string(),
            pq_id: "test".to_string(),
            path_arguments: None,
            query_params: None,
        };

        let response = client
            .make_request(headers, endpoint, None)
            .await
            .expect("Failed to make request");
        mock_endpoint.assert();
        assert_eq!(response.status().as_u16(), 200);
    }
}
