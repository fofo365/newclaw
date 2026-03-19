// NewClaw v0.4.0 - 飞书 WebSocket Frame 协议
//
// 飞书 WebSocket 使用二进制协议，基于 Protobuf 格式

use anyhow::{anyhow, Result};
use prost::Message;
use crate::proto::frame;

/// Frame 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameType {
    Control = 0,
    Data = 1,
}

/// 消息类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Event,
    Card,
    Ping,
    Pong,
}

impl MessageType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "event" => Some(MessageType::Event),
            "card" => Some(MessageType::Card),
            "ping" => Some(MessageType::Ping),
            "pong" => Some(MessageType::Pong),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            MessageType::Event => "event",
            MessageType::Card => "card",
            MessageType::Ping => "ping",
            MessageType::Pong => "pong",
        }
    }
}

/// Frame 结构（包装 Protobuf）
#[derive(Debug, Clone)]
pub struct FeishuFrame {
    /// Frame 类型
    pub frame_type: FrameType,

    /// Service ID
    pub service_id: i32,

    /// Headers
    pub headers: Vec<FeishuHeader>,

    /// Payload
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FeishuHeader {
    pub key: String,
    pub value: String,
}

impl From<&frame::Header> for FeishuHeader {
    fn from(header: &frame::Header) -> Self {
        FeishuHeader {
            key: header.key.clone(),
            value: header.value.clone(),
        }
    }
}

impl FeishuFrame {
    /// 解析二进制数据为 Frame
    pub fn decode(data: &[u8]) -> Result<Self> {
        let proto_frame = frame::Frame::decode(data)
            .map_err(|e| anyhow!("Failed to decode Protobuf frame: {}", e))?;

        let frame_type = match proto_frame.method {
            0 => FrameType::Control,
            1 => FrameType::Data,
            _ => return Err(anyhow!("Unknown frame type: {}", proto_frame.method)),
        };

        let headers = proto_frame.headers
            .iter()
            .map(|h| FeishuHeader::from(h))
            .collect();

        Ok(FeishuFrame {
            frame_type,
            service_id: proto_frame.service,
            headers,
            payload: proto_frame.payload,
        })
    }

    /// 获取 Header 值
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.iter().find(|h| h.key == key).map(|h| &h.value)
    }

    /// 获取 Header 的整数值
    pub fn get_header_int(&self, key: &str) -> Option<i32> {
        self.get_header(key)
            .and_then(|v| v.parse::<i32>().ok())
    }

    /// 获取消息类型
    pub fn message_type(&self) -> Option<MessageType> {
        self.get_header("type")
            .and_then(|t| MessageType::from_str(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_from_str() {
        assert_eq!(MessageType::from_str("event"), Some(MessageType::Event));
        assert_eq!(MessageType::from_str("card"), Some(MessageType::Card));
        assert_eq!(MessageType::from_str("ping"), Some(MessageType::Ping));
        assert_eq!(MessageType::from_str("pong"), Some(MessageType::Pong));
        assert_eq!(MessageType::from_str("unknown"), None);
    }

    #[test]
    fn test_message_type_as_str() {
        assert_eq!(MessageType::Event.as_str(), "event");
        assert_eq!(MessageType::Card.as_str(), "card");
        assert_eq!(MessageType::Ping.as_str(), "ping");
        assert_eq!(MessageType::Pong.as_str(), "pong");
    }

    #[test]
    fn test_frame_empty() {
        assert!(FeishuFrame::decode(&[]).is_err());
    }
}