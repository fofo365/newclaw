// MCP 传输层抽象

use async_trait::async_trait;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use super::{McpError, McpResult};
use super::protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};

/// 传输层 trait
#[async_trait]
pub trait Transport: Send + Sync {
    /// 发送请求并接收响应
    async fn send_request(&self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse>;

    /// 发送通知（无响应）
    async fn send_notification(&self, notification: JsonRpcNotification) -> McpResult<()>;

    /// 接收消息（用于服务器端）
    async fn receive_message(&self) -> McpResult<ServerMessage>;

    /// 关闭连接
    async fn close(&self) -> McpResult<()>;

    /// 检查是否已连接
    async fn is_connected(&self) -> bool;
}

/// 服务器消息（请求或通知）
#[derive(Debug, Clone)]
pub enum ServerMessage {
    /// JSON-RPC 请求
    Request(JsonRpcRequest),
    /// JSON-RPC 通知
    Notification(JsonRpcNotification),
}

/// Stdio 传输（用于本地进程通信）
pub struct StdioTransport {
    _phantom: std::marker::PhantomData<()>,
}

impl StdioTransport {
    /// 创建新的 stdio 传输
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let json = serde_json::to_string(&request)?;
        println!("{}", json);
        io::stdout().flush()?;

        // 读取响应
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let response: JsonRpcResponse = serde_json::from_str(&input)?;
        Ok(response)
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> McpResult<()> {
        let json = serde_json::to_string(&notification)?;
        println!("{}", json);
        io::stdout().flush()?;
        Ok(())
    }

    async fn receive_message(&self) -> McpResult<ServerMessage> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        // 尝试解析为请求
        if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&input) {
            return Ok(ServerMessage::Request(request));
        }

        // 尝试解析为通知
        if let Ok(notification) = serde_json::from_str::<JsonRpcNotification>(&input) {
            return Ok(ServerMessage::Notification(notification));
        }

        Err(McpError::TransportError("Invalid message format".to_string()))
    }

    async fn close(&self) -> McpResult<()> {
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        true // Stdio 总是连接的
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP 传输（用于网络通信）
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    /// 连接到远程服务器
    pub fn connect(addr: &str) -> McpResult<Self> {
        let stream = TcpStream::connect(addr)
            .map_err(|e| McpError::TransportError(format!("Failed to connect: {}", e)))?;
        Ok(Self { stream })
    }

    /// 从监听器接受连接
    pub fn accept(listener: &TcpListener) -> McpResult<Self> {
        let (stream, _) = listener.accept()
            .map_err(|e| McpError::TransportError(format!("Failed to accept: {}", e)))?;
        Ok(Self { stream })
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let json = serde_json::to_string(&request)?;

        // 发送请求
        let mut stream = &self.stream;
        writeln!(stream, "{}", json)?;
        stream.flush()?;

        // 读取响应
        let mut reader = BufReader::new(&self.stream);
        let mut input = String::new();
        reader.read_line(&mut input)?;
        let response: JsonRpcResponse = serde_json::from_str(&input)?;
        Ok(response)
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> McpResult<()> {
        let json = serde_json::to_string(&notification)?;
        let mut stream = &self.stream;
        writeln!(stream, "{}", json)?;
        stream.flush()?;
        Ok(())
    }

    async fn receive_message(&self) -> McpResult<ServerMessage> {
        let mut reader = BufReader::new(&self.stream);
        let mut input = String::new();
        reader.read_line(&mut input)?;

        // 尝试解析为请求
        if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&input) {
            return Ok(ServerMessage::Request(request));
        }

        // 尝试解析为通知
        if let Ok(notification) = serde_json::from_str::<JsonRpcNotification>(&input) {
            return Ok(ServerMessage::Notification(notification));
        }

        Err(McpError::TransportError("Invalid message format".to_string()))
    }

    async fn close(&self) -> McpResult<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
            .map_err(|e| McpError::TransportError(format!("Failed to close: {}", e)))?;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.stream.peer_addr().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::protocol::JsonRpcId;

    #[tokio::test]
    async fn test_stdio_transport_creation() {
        let transport = StdioTransport::new();
        assert!(transport.is_connected().await);
    }

    #[test]
    fn test_server_message_classification() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(1),
            method: "test".to_string(),
            params: None,
        };

        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
        };

        // 测试请求分类
        let request_json = serde_json::to_string(&request).unwrap();
        if let Ok(parsed) = serde_json::from_str::<JsonRpcRequest>(&request_json) {
            let _ = ServerMessage::Request(parsed);
        }

        // 测试通知分类
        let notification_json = serde_json::to_string(&notification).unwrap();
        if let Ok(parsed) = serde_json::from_str::<JsonRpcNotification>(&notification_json) {
            let _ = ServerMessage::Notification(parsed);
        }
    }
}
