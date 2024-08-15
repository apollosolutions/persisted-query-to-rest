mod config;
use serde::{Serialize, Deserialize};

#[derive(Clone, Derivative, Serialize)]
struct Config {
    pub(crate) server: Server,    
    pub(crate) endpoints: Vec<Endpoint>,
    pub(crate) logging: Logging,
}

#[derive(Clone, Derivative, Serialize)]
struct ServerConfig {
    pub(crate) listen_on: String,
    pub(crate) port: Option<u16>,
    pub(crate) path_prefix: Option<String>,
}

#[derive(Clone, Derivative, Serialize)]
struct Endpoint {
    path: String,
    method: String,
    persisted_query_id: String,
    query_params: Vec<QueryParam>,
}

#[derive(Clone, Derivative, Serialize)]
struct QueryParam {
    from: String,
    to: String,
    required: Boolean
}

#[derive(Clone, Derivative, Serialize)]
struct Logging {
    format: String,
}

