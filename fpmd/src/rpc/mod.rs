pub mod dispatch;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method:  String,
    pub params:  Option<Value>,
    pub id:      Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<RpcError>,
    pub id:      Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code:    i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:    Option<Value>,
}

impl Response {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), result: Some(result), error: None, id }
    }

    pub fn err(id: Option<Value>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result:  None,
            error:   Some(RpcError { code, message: message.into(), data: None }),
            id,
        }
    }

    pub fn parse_error() -> Self {
        Self::err(None, -32700, "Parse error")
    }

    pub fn method_not_found(id: Option<Value>, method: &str) -> Self {
        Self::err(id, -32601, format!("Method not found: {method}"))
    }

    pub fn invalid_params(id: Option<Value>, msg: impl Into<String>) -> Self {
        Self::err(id, -32602, msg)
    }
}
