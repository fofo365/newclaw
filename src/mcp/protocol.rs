// MCP 协议 - JSON-RPC 2.0 消息格式

use serde::{Deserialize, Serialize};
use std::fmt;

/// JSON-RPC 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC 版本（必须是 "2.0"）
    pub jsonrpc: String,
    /// 请求 ID（用于关联响应）
    pub id: JsonRpcId,
    /// 方法名
    pub method: String,
    /// 参数（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// 创建新的请求
    pub fn new(id: JsonRpcId, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params: None,
        }
    }

    /// 设置参数
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// JSON-RPC 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC 版本（必须是 "2.0"）
    pub jsonrpc: String,
    /// 请求 ID
    pub id: JsonRpcId,
    /// 结果（成功时存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 错误（失败时存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<super::JsonRpcError>,
}

impl JsonRpcResponse {
    /// 创建成功响应
    pub fn success(id: JsonRpcId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(id: JsonRpcId, error: super::JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// 检查是否成功
    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }

    /// 检查是否失败
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// JSON-RPC ID
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
#[derive(Default)]
pub enum JsonRpcId {
    /// 字符串 ID
    String(String),
    /// 数字 ID
    Number(i64),
    /// 空 ID（用于通知）
    #[default]
    Null,
}

impl fmt::Display for JsonRpcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
            Self::Null => write!(f, "null"),
        }
    }
}


/// JSON-RPC 通知（无响应的请求）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC 版本（必须是 "2.0"）
    pub jsonrpc: String,
    /// 方法名
    pub method: String,
    /// 参数（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    /// 创建新的通知
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
        }
    }

    /// 设置参数
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let request = JsonRpcRequest::new(JsonRpcId::Number(1), "test_method");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test_method\""));
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(
            JsonRpcId::Number(1),
            serde_json::json!({"result": "ok"})
        );
        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let error = super::super::JsonRpcError::method_not_found("test");
        let response = JsonRpcResponse::error(JsonRpcId::Number(1), error);
        assert!(!response.is_success());
        assert!(response.is_error());
    }
}
