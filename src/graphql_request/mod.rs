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
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
        }
    }

    pub async fn make_request(
        &self,
        endpoint: config::Endpoint,
        parameters: Option<HashMap<String, Value>>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut request = self.client.request(reqwest::Method::POST, &self.url);
        let mut default_headers = HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        request = request.headers(default_headers);
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
        let json = serde_json::to_string(&body).unwrap();
        request = request.body(json.clone());
        debug!("JSON: {:?}", json);
        let response = request.send().await;
        response
    }
}
