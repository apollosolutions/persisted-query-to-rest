
use serde::{Deserialize, Serialize};

// TODO: Review the use of pub(crate) and pub
// TODO: Review the serde attributes
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub(crate) common: ServerConfig,    
    pub(crate) endpoints: Vec<Endpoint>,
    pub(crate) logging: Logging,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub(crate) listen_on: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) path_prefix: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Endpoint {
    pub(crate) path: String,
    pub(crate) method: Option<HttpMethod>,
    pub(crate) pq_id: String,
    pub(crate) query_params: Option<Vec<QueryParam>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct QueryParam {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) required: bool
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Logging {
    pub(crate) format: String,
}

// TODO: improve error handling
pub fn parse_config(path: &str) -> Config {
    let yaml_contents = std::fs::read_to_string(path).expect("Failed to read config file");
    let config: Config = serde_yaml::from_str(&yaml_contents).expect("Failed to parse config file");

    config
}
