// NewClaw v0.3.0 - 流式响应支持
//
// 核心功能：
// 1. SSE (Server-Sent Events) 流式响应
// 2. WebSocket 流式响应
// 3. Feishu 流式（分块发送）

use crate::llm::provider::{ChatRequest, ChatResponse, LLMProviderV3, LLMError};
use futures::{Stream, StreamExt};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// 流式响应类型
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 数据块
    Data(String),
    /// 完成标记
    Done,
    /// 错误
    Error(String),
}

/// 流式响应包装器
pub struct StreamingResponse {
    pub chunks: Vec<StreamChunk>,
    pub complete: bool,
}

impl Default for StreamingResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingResponse {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            complete: false,
        }
    }
    
    pub fn add_chunk(&mut self, chunk: String) {
        self.chunks.push(StreamChunk::Data(chunk));
    }
    
    pub fn finish(&mut self) {
        self.complete = true;
        self.chunks.push(StreamChunk::Done);
    }
}

/// SSE 事件格式
#[derive(Debug, Clone)]
pub struct SSEEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
    pub retry: Option<u64>,
}

impl SSEEvent {
    pub fn new(data: String) -> Self {
        Self {
            id: None,
            event: None,
            data,
            retry: None,
        }
    }
    
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }
    
    pub fn with_event(mut self, event: String) -> Self {
        self.event = Some(event);
        self
    }
    
    /// 格式化为 SSE 协议
    pub fn format(&self) -> String {
        let mut lines = Vec::new();
        
        if let Some(id) = &self.id {
            lines.push(format!("id: {}", id));
        }
        
        if let Some(event) = &self.event {
            lines.push(format!("event: {}", event));
        }
        
        if let Some(retry) = &self.retry {
            lines.push(format!("retry: {}", retry));
        }
        
        // 数据可能包含多行
        for line in self.data.lines() {
            lines.push(format!("data: {}", line));
        }
        
        // SSE 协议要求每个消息以两个换行符结束
        lines.push(String::new()); // 空行表示消息结束
        lines.push(String::new()); // 额外的空行确保双换行
        
        lines.join("\n")
    }
}

/// 流式 LLM 调用
pub async fn stream_llm_response<P: LLMProviderV3>(
    provider: &P,
    request: ChatRequest,
    mut callback: impl FnMut(StreamChunk),
) -> Result<ChatResponse, LLMError> {
    // 先尝试流式调用
    match provider.chat_stream(request.clone()).await {
        Ok(mut stream) => {
            let mut full_content = String::new();
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        full_content.push_str(&chunk);
                        callback(StreamChunk::Data(chunk));
                    }
                    Err(e) => {
                        callback(StreamChunk::Error(e.to_string()));
                        return Err(e);
                    }
                }
            }
            
            callback(StreamChunk::Done);
            
            // 返回完整的响应
            Ok(ChatResponse {
                message: crate::llm::provider::Message {
                    role: crate::llm::provider::MessageRole::Assistant,
                    content: full_content,
                    tool_calls: None,
                    tool_call_id: None,
                },
                usage: crate::llm::provider::TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                finish_reason: Some("stop".to_string()),
                model: request.model,
            })
        }
        Err(LLMError::ApiError(msg)) if msg.contains("not implemented") => {
            // 流式不支持，降级到普通调用
            let response = provider.chat(request).await?;
            
            // 模拟流式：分批发送
            let content = response.message.content.clone();
            let chunk_size = 20; // 每 20 字符一块
            
            for (i, chunk) in content.as_bytes().chunks(chunk_size).enumerate() {
                let chunk_str = String::from_utf8_lossy(chunk).to_string();
                callback(StreamChunk::Data(chunk_str));
                
                // 模拟网络延迟
                if i < content.len() / chunk_size {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
            }
            
            callback(StreamChunk::Done);
            
            Ok(response)
        }
        Err(e) => Err(e),
    }
}

/// WebSocket 流式响应
#[pin_project]
pub struct WebSocketStream {
    #[pin]
    inner: Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>,
}

impl WebSocketStream {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<String, LLMError>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

impl Stream for WebSocketStream {
    type Item = Result<String, LLMError>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}

/// Feishu 流式响应（分块发送）
pub struct FeishuStreamAdapter {
    pub chunks: Vec<String>,
    pub message_id: Option<String>,
}

impl Default for FeishuStreamAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl FeishuStreamAdapter {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            message_id: None,
        }
    }
    
    pub fn add_chunk(&mut self, chunk: String) {
        self.chunks.push(chunk);
    }
    
    /// 获取下一个要发送的块
    pub fn next_chunk(&self, index: usize) -> Option<String> {
        self.chunks.get(index).cloned()
    }
    
    /// 是否需要编辑（而非创建新消息）
    pub fn should_edit(&self, index: usize) -> bool {
        index > 0 && self.message_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sse_event_format() {
        let event = SSEEvent::new("Hello, World!".to_string())
            .with_id("123".to_string())
            .with_event("message".to_string());
        
        let formatted = event.format();
        
        assert!(formatted.contains("id: 123"));
        assert!(formatted.contains("event: message"));
        assert!(formatted.contains("data: Hello, World!"));
        assert!(formatted.ends_with("\n\n"));
    }
    
    #[test]
    fn test_sse_event_multiline() {
        let event = SSEEvent::new("Line 1\nLine 2\nLine 3".to_string());
        let formatted = event.format();
        
        assert!(formatted.contains("data: Line 1"));
        assert!(formatted.contains("data: Line 2"));
        assert!(formatted.contains("data: Line 3"));
    }
    
    #[test]
    fn test_streaming_response() {
        let mut response = StreamingResponse::new();
        
        response.add_chunk("Hello".to_string());
        response.add_chunk(" World".to_string());
        response.finish();
        
        assert_eq!(response.chunks.len(), 3);
        assert!(response.complete);
        
        match &response.chunks[0] {
            StreamChunk::Data(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Data chunk"),
        }
    }
    
    #[tokio::test]
    async fn test_feishu_stream_adapter() {
        let mut adapter = FeishuStreamAdapter::new();
        
        adapter.add_chunk("Chunk 1".to_string());
        adapter.add_chunk("Chunk 2".to_string());
        adapter.add_chunk("Chunk 3".to_string());
        
        assert_eq!(adapter.next_chunk(0), Some("Chunk 1".to_string()));
        assert_eq!(adapter.next_chunk(1), Some("Chunk 2".to_string()));
        assert_eq!(adapter.next_chunk(2), Some("Chunk 3".to_string()));
        assert_eq!(adapter.next_chunk(3), None);
    }
}
