// Message Format Module
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message ID type
pub type MessageId = String;

/// Agent ID type
pub type AgentId = String;

/// Standard inter-agent message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterAgentMessage {
    /// Unique message ID
    pub id: MessageId,
    
    /// Sender agent ID
    pub from: AgentId,
    
    /// Recipient agent ID (or "broadcast" for broadcast messages)
    pub to: AgentId,
    
    /// Message timestamp (Unix timestamp in seconds)
    pub timestamp: i64,
    
    /// Message payload
    pub payload: MessagePayload,
    
    /// Message priority
    #[serde(default)]
    pub priority: MessagePriority,
    
    /// Correlation ID for request-response tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<MessageId>,
    
    /// Message metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl InterAgentMessage {
    /// Create a new message
    pub fn new(from: AgentId, to: AgentId, payload: MessagePayload) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            timestamp: chrono::Utc::now().timestamp(),
            payload,
            priority: MessagePriority::Normal,
            correlation_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a request message
    pub fn request(from: AgentId, to: AgentId, request: Request) -> Self {
        Self::new(from, to, MessagePayload::Request(request))
    }

    /// Create a response message
    pub fn response(from: AgentId, to: AgentId, response: Response, correlation_id: MessageId) -> Self {
        let mut msg = Self::new(from, to, MessagePayload::Response(response));
        msg.correlation_id = Some(correlation_id);
        msg
    }

    /// Create an event message
    pub fn event(from: AgentId, to: AgentId, event: Event) -> Self {
        Self::new(from, to, MessagePayload::Event(event))
    }

    /// Create a command message
    pub fn command(from: AgentId, to: AgentId, command: Command) -> Self {
        Self::new(from, to, MessagePayload::Command(command))
    }

    /// Set priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, correlation_id: MessageId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if message is a broadcast
    pub fn is_broadcast(&self) -> bool {
        self.to == "broadcast" || self.to == "*"
    }
}

/// Message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    Request(Request),
    Response(Response),
    Event(Event),
    Command(Command),
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Query request
    Query {
        query: String,
        context: Option<String>,
    },
    
    /// Task request
    Task {
        task_type: String,
        parameters: serde_json::Value,
    },
    
    /// Data request
    Data {
        resource: String,
        action: String,
        filters: Option<serde_json::Value>,
    },
    
    /// Custom request
    Custom {
        request_type: String,
        data: serde_json::Value,
    },
}

/// Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Success response
    Success {
        data: serde_json::Value,
        message: Option<String>,
    },
    
    /// Error response
    Error {
        code: String,
        message: String,
        details: Option<serde_json::Value>,
    },
    
    /// Partial response (for streaming)
    Partial {
        data: serde_json::Value,
        sequence: u32,
        total: Option<u32>,
    },
}

impl Response {
    pub fn success(data: serde_json::Value) -> Self {
        Self::Success {
            data,
            message: None,
        }
    }

    pub fn success_with_message(data: serde_json::Value, message: String) -> Self {
        Self::Success {
            data,
            message: Some(message),
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn error_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }
}

/// Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Agent status change
    AgentStatus {
        agent_id: AgentId,
        status: String,
    },
    
    /// Message received
    MessageReceived {
        from: AgentId,
        message_id: MessageId,
    },
    
    /// Task completed
    TaskCompleted {
        task_id: String,
        result: serde_json::Value,
    },
    
    /// System event
    System {
        event_type: String,
        data: serde_json::Value,
    },
    
    /// Custom event
    Custom {
        event_type: String,
        data: serde_json::Value,
    },
}

/// Command types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Shutdown command
    Shutdown {
        reason: Option<String>,
    },
    
    /// Configuration update
    UpdateConfig {
        config: serde_json::Value,
    },
    
    /// Reload request
    Reload {
        component: String,
    },
    
    /// Health check
    HealthCheck,
    
    /// Custom command
    Custom {
        command_type: String,
        parameters: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = InterAgentMessage::request(
            "agent-1".to_string(),
            "agent-2".to_string(),
            Request::Query {
                query: "hello".to_string(),
                context: None,
            },
        );

        assert_eq!(msg.from, "agent-1");
        assert_eq!(msg.to, "agent-2");
        assert!(matches!(msg.payload, MessagePayload::Request(_)));
    }

    #[test]
    fn test_message_serialization() {
        let msg = InterAgentMessage::request(
            "agent-1".to_string(),
            "agent-2".to_string(),
            Request::Query {
                query: "test".to_string(),
                context: None,
            },
        );

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: InterAgentMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.from, deserialized.from);
    }

    #[test]
    fn test_broadcast_detection() {
        let msg = InterAgentMessage::new(
            "agent-1".to_string(),
            "broadcast".to_string(),
            MessagePayload::Event(Event::System {
                event_type: "test".to_string(),
                data: serde_json::json!({}),
            }),
        );

        assert!(msg.is_broadcast());
    }

    #[test]
    fn test_response_helpers() {
        let success = Response::success(serde_json::json!({"result": "ok"}));
        assert!(matches!(success, Response::Success { .. }));

        let error = Response::error("ERR001", "Something went wrong");
        assert!(matches!(error, Response::Error { .. }));
    }
}
