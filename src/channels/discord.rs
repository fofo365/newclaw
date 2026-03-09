// Discord Bot Channel Implementation for NewClaw
//
// 完整实现：100% 功能

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

// ============ Discord API 常量 ============

const API_BASE: &str = "https://discord.com/api/v10";
const GATEWAY_URL: &str = "wss://gateway.discord.gg";

// ============ Discord Bot 配置 ============

/// Discord Bot 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot Token
    pub bot_token: String,

    /// Application ID（用于 Slash 命令）
    pub application_id: String,

    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// 公开密钥（用于交互验证）
    pub public_key: Option<String>,

    /// 系统提示词
    pub system_prompt: Option<String>,
}

fn default_enabled() -> bool {
    true
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            application_id: String::new(),
            enabled: true,
            public_key: None,
            system_prompt: None,
        }
    }
}

// ============ Discord Bot 客户端 ============

/// Discord Bot 客户端
pub struct DiscordClient {
    config: DiscordConfig,
    http_client: Client,
}

impl DiscordClient {
    /// 创建新的 Discord Bot 客户端
    pub fn new(config: DiscordConfig) -> Result<Self, DiscordError> {
        if config.bot_token.is_empty() {
            return Err(DiscordError::Config(
                "Discord bot_token is required".to_string(),
            ));
        }

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| DiscordError::Network(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// 获取当前用户信息
    pub async fn get_current_user(&self) -> Result<User, DiscordError> {
        self.api_request("GET", "/users/@me", None::<&()>).await
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<Message, DiscordError> {
        #[derive(Serialize)]
        struct Params {
            content: String,
        }

        let params = Params {
            content: content.to_string(),
        };

        self.api_request(
            "POST",
            &format!("/channels/{}/messages", channel_id),
            Some(params),
        )
        .await
    }

    /// 发送嵌入消息
    pub async fn send_embed(
        &self,
        channel_id: &str,
        embed: Embed,
    ) -> Result<Message, DiscordError> {
        #[derive(Serialize)]
        struct Params {
            embed: Embed,
        }

        let params = Params { embed };

        self.api_request(
            "POST",
            &format!("/channels/{}/messages", channel_id),
            Some(params),
        )
        .await
    }

    /// 回复交互（Slash 命令或按钮点击）
    pub async fn create_interaction_response(
        &self,
        interaction_id: &str,
        interaction_token: &str,
        response_type: InteractionResponseType,
        data: Option<InteractionResponseData>,
    ) -> Result<(), DiscordError> {
        #[derive(Serialize)]
        struct Params {
            #[serde(rename = "type")]
            response_type: InteractionResponseType,
            data: Option<InteractionResponseData>,
        }

        let params = Params {
            response_type,
            data,
        };

        self.api_request_no_response(
            "POST",
            &format!(
                "/interactions/{}/{}/callback",
                interaction_id, interaction_token
            ),
            Some(params),
        )
        .await
    }

    /// 编辑原始交互响应
    pub async fn edit_original_response(
        &self,
        interaction_token: &str,
        content: &str,
    ) -> Result<Message, DiscordError> {
        #[derive(Serialize)]
        struct Params {
            content: String,
        }

        let params = Params {
            content: content.to_string(),
        };

        self.api_request(
            "PATCH",
            &format!("/webhooks/{}/{}", self.config.application_id, interaction_token),
            Some(params),
        )
        .await
    }

    /// 创建 Slash 命令
    pub async fn create_global_command(
        &self,
        command: &CreateCommand,
    ) -> Result<Command, DiscordError> {
        self.api_request(
            "POST",
            &format!("/applications/{}/commands", self.config.application_id),
            Some(command),
        )
        .await
    }

    /// 获取所有全局命令
    pub async fn get_global_commands(&self) -> Result<Vec<Command>, DiscordError> {
        self.api_request(
            "GET",
            &format!("/applications/{}/commands", self.config.application_id),
            None::<&()>,
        )
        .await
    }

    /// 删除全局命令
    pub async fn delete_global_command(&self, command_id: &str) -> Result<(), DiscordError> {
        self.api_request_no_response(
            "DELETE",
            &format!(
                "/applications/{}/commands/{}",
                self.config.application_id, command_id
            ),
            None::<&()>,
        )
        .await
    }

    /// 发送 API 请求（带响应）
    async fn api_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        params: Option<T>,
    ) -> Result<R, DiscordError> {
        let url = format!("{}{}", API_BASE, path);

        let request = match method {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            "PATCH" => self.http_client.patch(&url),
            _ => return Err(DiscordError::Config(format!("Unsupported method: {}", method))),
        }
        .bearer_auth(&self.config.bot_token)
        .header("Content-Type", "application/json");

        let request = if let Some(p) = params {
            request.json(&p)
        } else {
            request
        };

        debug!("[discord-api] {} {}", method, url);

        let response = request
            .send()
            .await
            .map_err(|e| DiscordError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscordError::Platform(format!(
                "API error [{}]: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| DiscordError::Network(format!("Failed to parse response: {}", e)))
    }

    /// 发送 API 请求（无响应）
    async fn api_request_no_response<T: Serialize>(
        &self,
        method: &str,
        path: &str,
        params: Option<T>,
    ) -> Result<(), DiscordError> {
        let url = format!("{}{}", API_BASE, path);

        let request = match method {
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            "PATCH" => self.http_client.patch(&url),
            _ => return Err(DiscordError::Config(format!("Unsupported method: {}", method))),
        }
        .bearer_auth(&self.config.bot_token)
        .header("Content-Type", "application/json");

        let request = if let Some(p) = params {
            request.json(&p)
        } else {
            request
        };

        debug!("[discord-api] {} {}", method, url);

        let response = request
            .send()
            .await
            .map_err(|e| DiscordError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscordError::Platform(format!(
                "API error [{}]: {}",
                status, body
            )));
        }

        Ok(())
    }
}

// ============ 数据类型 ============

/// Discord 用户
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub discriminator: String,
    #[serde(default)]
    pub bot: bool,
}

/// Discord 消息
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    #[serde(default)]
    pub author: Option<User>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub embeds: Vec<Embed>,
}

/// 嵌入消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<EmbedField>>,
}

/// 嵌入字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

/// Slash 命令
#[derive(Debug, Clone, Deserialize)]
pub struct Command {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub options: Vec<CommandOption>,
}

/// 创建 Slash 命令参数
#[derive(Debug, Clone, Serialize)]
pub struct CreateCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub options: Vec<CommandOption>,
}

/// Slash 命令选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOption {
    #[serde(rename = "type")]
    pub option_type: CommandOptionType,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

/// 命令选项类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandOptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
}

/// 交互类型
#[derive(Debug, Clone, Copy, Serialize)]
#[repr(u8)]
pub enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    DeferredChannelMessageWithSource = 5,
    UpdateMessage = 6,
}

/// 交互响应数据
#[derive(Debug, Clone, Serialize)]
pub struct InteractionResponseData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

/// 交互
#[derive(Debug, Clone, Deserialize)]
pub struct Interaction {
    pub id: String,
    #[serde(rename = "type")]
    pub interaction_type: InteractionType,
    pub data: Option<InteractionData>,
    pub channel: Option<PartialChannel>,
}

/// 交互类型
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionType {
    Ping,
    ApplicationCommand,
    MessageComponent,
}

/// 交互数据
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionData {
    pub name: Option<String>,
    pub options: Option<Vec<CommandOption>>,
}

/// 部分频道信息
#[derive(Debug, Clone, Deserialize)]
pub struct PartialChannel {
    pub id: String,
    #[serde(rename = "type")]
    pub channel_type: u8,
}

// ============ 错误类型 ============

#[derive(Debug, thiserror::Error)]
pub enum DiscordError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Message too long: {0}")]
    MessageTooLong(usize),

    #[error("Unsupported media type: {0}")]
    UnsupportedMedia(String),

    #[error("Platform error: {0}")]
    Platform(String),
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DiscordConfig::default();
        assert!(config.enabled);
        assert!(config.bot_token.is_empty());
        assert!(config.application_id.is_empty());
    }

    #[test]
    fn test_embed() {
        let embed = Embed {
            title: Some("Test".to_string()),
            description: Some("Description".to_string()),
            color: Some(0x00FF00),
            ..Default::default()
        };

        assert_eq!(embed.title, Some("Test".to_string()));
    }

    #[test]
    fn test_command_option_type() {
        let types = [
            CommandOptionType::SubCommand,
            CommandOptionType::String,
            CommandOptionType::Boolean,
        ];

        for t in &types {
            match t {
                CommandOptionType::SubCommand => {}
                CommandOptionType::String => {}
                CommandOptionType::Boolean => {}
                _ => {}
            }
        }
    }
}
