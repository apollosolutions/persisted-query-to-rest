use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

// TODO: Review the use of pub(crate) and pub
// TODO: Review the serde attributes

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, JsonSchema)]
/// The HTTP method for the endpoint to accept
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "lowercase")]
/// The log level that the server should use
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "lowercase")]
/// The kind of parameter that is expected if it is not a string
pub enum ParamKind {
    INT,
    STRING,
    FLOAT,
    OBJECT,
    ARRAY,
    BOOLEAN,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Config {
    /// The common configuration for the server
    pub common: ServerConfig,
    /// The list of endpoints that the server should expose
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ServerConfig {
    #[serde(default = "default_server_listen")]
    /// The address that the server should listen on
    pub listen: String,
    #[serde(default = "default_server_path_prefix")]
    /// The prefix for the endpoints the server should use; defaults to `/api/v1`
    pub path_prefix: String,
    /// The GraphQL endpoint the server will forward requests to
    pub graphql_endpoint: String,
    /// Basic logging configuration
    pub logging: Option<Logging>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Endpoint {
    /// The path that the endpoint should be exposed on
    pub path: String,
    #[serde(default = "default_endpoint_method")]
    /// The method that the endpoint should accept
    pub method: HttpMethod,
    /// The persisted query ID that the endpoint should use
    pub pq_id: String,
    /// The query parameters that the endpoint should accept
    pub query_params: Option<Vec<Parameter>>,
    /// The path arguments that the endpoint should accept
    pub path_arguments: Option<Vec<Parameter>>,
    /// The body parameters that the endpoint should accept
    pub body_params: Option<Vec<Parameter>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Parameter {
    /// The parameter name that the user will use; e.g. `id` in `/user/:id` or /user/?id=1234
    pub from: String,
    /// If the operation uses a different name, this is the name the variable should be renamed to
    pub to: Option<String>,
    #[serde(default = "default_parameter_required")]
    /// Whether the parameter is required or not; by default it is false
    pub required: bool,
    #[serde(default = "default_parameter_kind")]
    /// The kind of parameter that is expected if it is not a string
    pub kind: ParamKind,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Logging {
    #[serde(default = "default_logging_format")]
    /// The format that the logs should be output in
    pub format: String,
    #[serde(default = "default_logging_level")]
    /// The log level that the server should use
    pub level: LogLevel,
}

fn default_endpoint_method() -> HttpMethod {
    HttpMethod::GET
}
fn default_parameter_required() -> bool {
    false
}
fn default_parameter_kind() -> ParamKind {
    ParamKind::STRING
}
fn default_server_path_prefix() -> String {
    "/api/v1".to_string()
}
fn default_server_listen() -> String {
    "127.0.0.1:4000".to_string()
}
fn default_logging_format() -> String {
    "pretty".to_string()
}
fn default_logging_level() -> LogLevel {
    LogLevel::INFO
}

// TODO: improve error handling
pub fn parse_config(path: &str) -> Config {
    let yaml_contents = std::fs::read_to_string(path).expect("Failed to read config file");
    let config: Config = serde_yaml::from_str(&yaml_contents).expect("Failed to parse config file");

    config
}

impl ParamKind {
    pub fn from_str(&self, param: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match self {
            ParamKind::INT => match param.parse::<i64>() {
                Ok(i) => Ok(Value::Number(i.into())),
                Err(e) => Err(Box::from(e.to_string().as_str())),
            },
            ParamKind::STRING => Ok(Value::String(param.to_string())),
            ParamKind::FLOAT => match param.parse::<f64>() {
                Ok(f) => Ok(Value::Number(Number::from_f64(f).unwrap())),
                Err(e) => Err(Box::from(e.to_string().as_str())),
            },
            ParamKind::OBJECT | ParamKind::ARRAY => match serde_json::from_str(param) {
                Ok(j) => Ok(j),
                Err(e) => Err(Box::from(e.to_string().as_str())),
            },
            ParamKind::BOOLEAN => match param.parse::<bool>() {
                Ok(b) => Ok(Value::Bool(b)),
                Err(e) => Err(Box::from(e.to_string().as_str())),
            },
        }
    }
}

pub fn generate_schema() {
    let schema = schema_for!(Config);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
