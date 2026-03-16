// NewClaw v0.7.0 - 统一通道层抽象
//
// 通道层是所有消息入口的统一抽象：
// - CLI Channel: 命令行交互
// - Feishu Channel: 飞书消息
// - Dashboard Channel: Web UI
// - Telegram/Discord/WhatsApp 等
//
// 所有通道共享：
// - 统一的工具系统 (ToolRegistry)
// - 统一的权限控制 (ChannelPermission)
// - 统一的记忆系统 (MemoryStore)

pub mod permission;
pub mod base;
pub mod manager;

pub use permission::{ChannelPermission, PermissionRule, PermissionConfig, PermissionRuleBuilder, PermissionRuleType};
pub use base::{Channel, ChannelMessage, ChannelResponse, ChannelContext, ChannelStatus, MessageContent, BaseChannel};
pub use manager::{ChannelManager, ChannelRegistry};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::tools::ToolRegistry;

/// 通道类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// 命令行
    Cli,
    /// 飞书
    Feishu,
    /// Dashboard Web UI
    Dashboard,
    /// Telegram
    Telegram,
    /// Discord
    Discord,
    /// WhatsApp
    WhatsApp,
    /// 企业微信
    WeCom,
    /// 钉钉
    DingTalk,
    /// QQ
    QQ,
    /// 自定义
    Custom(String),
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Cli => write!(f, "cli"),
            ChannelType::Feishu => write!(f, "feishu"),
            ChannelType::Dashboard => write!(f, "dashboard"),
            ChannelType::Telegram => write!(f, "telegram"),
            ChannelType::Discord => write!(f, "discord"),
            ChannelType::WhatsApp => write!(f, "whatsapp"),
            ChannelType::WeCom => write!(f, "wecom"),
            ChannelType::DingTalk => write!(f, "dingtalk"),
            ChannelType::QQ => write!(f, "qq"),
            ChannelType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl From<&str> for ChannelType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cli" => ChannelType::Cli,
            "feishu" => ChannelType::Feishu,
            "dashboard" => ChannelType::Dashboard,
            "telegram" => ChannelType::Telegram,
            "discord" => ChannelType::Discord,
            "whatsapp" => ChannelType::WhatsApp,
            "wecom" => ChannelType::WeCom,
            "dingtalk" => ChannelType::DingTalk,
            "qq" => ChannelType::QQ,
            other => ChannelType::Custom(other.to_string()),
        }
    }
}

/// 通道成员标识
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChannelMember {
    /// 通道类型
    pub channel_type: ChannelType,
    /// 成员 ID (如 open_id, user_id, session_id)
    pub member_id: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 角色
    pub role: ChannelRole,
}

/// 通道角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChannelRole {
    /// 普通用户
    User,
    /// 管理员
    Admin,
    /// 超级管理员
    SuperAdmin,
    /// 机器人
    Bot,
    /// 自定义角色
    Custom(String),
}

impl Default for ChannelRole {
    fn default() -> Self {
        ChannelRole::User
    }
}

/// 通道能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    /// 支持富文本
    pub rich_text: bool,
    /// 支持图片
    pub images: bool,
    /// 支持文件
    pub files: bool,
    /// 支持卡片
    pub cards: bool,
    /// 支持语音
    pub voice: bool,
    /// 支持视频
    pub video: bool,
    /// 支持位置
    pub location: bool,
    /// 支持回复
    pub reply: bool,
    /// 支持编辑
    pub edit: bool,
    /// 支持删除
    pub delete: bool,
    /// 支持反应
    pub reactions: bool,
}

impl Default for ChannelCapabilities {
    fn default() -> Self {
        Self {
            rich_text: true,
            images: true,
            files: true,
            cards: false,
            voice: false,
            video: false,
            location: false,
            reply: true,
            edit: false,
            delete: false,
            reactions: false,
        }
    }
}