// NewClaw 飞书 WebSocket 长连接服务 - v0.7.0
//
// 独立服务，用于接收飞书消息并调用 LLM 回复，支持：
// - 工具调用
// - 记忆管理（多层隔离：用户/通道/Agent/命名空间）
// - 策略管理（动态调整上下文策略）
// - 上下文管理

use anyhow::Result;
use tracing::{info, error, warn};
use std::sync::Arc;
use serde_json::json;
use tokio::sync::RwLock;
use std::collections::HashMap;

use async_trait::async_trait;
use newclaw::feishu_websocket::{
    EventHandler, FeishuEvent, WebSocketError, WebSocketResult,
    MessageSender, ToolManager, ToolCallRequest, build_tools_system_prompt,
};
use newclaw::channel::{ChannelProcessor, ProcessorConfig};
use newclaw::tools::ToolRegistry;
use newclaw::channel::ChannelPermission;
use newclaw::memory::{SQLiteMemoryStorage, StorageConfig};
use newclaw::context::StrategyEngine;
use newclaw::llm::{GlmProvider, GlmConfig, GlmRegion, GlmProviderType};

/// 对话历史消息
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

/// 智能 LLM 事件处理器 - v0.7.0
/// 
/// 集成：
/// - 记忆管理（多层隔离）
/// - 策略管理（动态调整）
/// - 工具调用
struct LLMEventHandler {
    /// 消息发送器
    sender: MessageSender,
    /// GLM API Key
    api_key: String,
    /// GLM 模型
    model: String,
    /// 工具管理器
    tool_manager: Arc<ToolManager>,
    /// 通道处理器 - v0.7.0
    processor: Arc<RwLock<ChannelProcessor>>,
    /// 对话历史
    conversation_history: Arc<RwLock<Vec<newclaw::llm::Message>>>,
}

impl LLMEventHandler {
    async fn new(api_key: String, model: String) -> Self {
        let sender = MessageSender::new("https://open.feishu.cn");
        let tool_manager = Arc::new(ToolManager::new().await);
        
        // 创建工具注册表和权限管理
        let tools = Arc::new(ToolRegistry::new());
        let permissions = Arc::new(ChannelPermission::new("./data/feishu_permissions.json"));
        
        // 创建处理器配置
        let config = ProcessorConfig {
            enable_memory: true,
            enable_strategy: true,
            default_strategy: newclaw::context::StrategyType::Balanced,
            max_context_tokens: 8000,
            memory_search_limit: 5,
            default_agent_id: "feishu-bot".to_string(),
            default_namespace: "feishu".to_string(),
        };
        
        // 创建记忆存储
        let storage_config = StorageConfig {
            db_path: std::path::PathBuf::from("data/feishu_memory.db"),
            ..Default::default()
        };
        let memory = Arc::new(SQLiteMemoryStorage::new(storage_config).expect("Failed to create memory storage"));
        
        // 创建策略引擎
        let strategy = Arc::new(RwLock::new(StrategyEngine::new().expect("Failed to create strategy engine")));
        
        // 创建 LLM Provider
        let glm_config = GlmConfig {
            region: GlmRegion::International,
            provider_type: GlmProviderType::Glm,
            model: model.clone(),
            temperature: 0.7,
            max_tokens: 4096,
        };
        let llm = Arc::new(GlmProvider::with_config(api_key.clone(), glm_config));
        
        // 创建通道处理器
        let processor = ChannelProcessor::new(tools, permissions, config)
            .with_memory(memory)
            .with_strategy(strategy)
            .with_llm(llm, model.clone())
            .with_agent_id("feishu-bot")
            .with_namespace("feishu");
        
        Self {
            sender,
            api_key,
            model,
            tool_manager,
            processor: Arc::new(RwLock::new(processor)),
            conversation_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 处理消息（使用 ChannelProcessor）
    async fn process_message(
        &self,
        user_id: &str,
        chat_id: &str,
        content: &str,
    ) -> Result<String> {
        use newclaw::channel::{ChannelMessage, ChannelMember, ChannelType, ChannelRole, MessageContent};
        use newclaw::llm::{Message, MessageRole};
        
        // 构建通道消息
        let message = ChannelMessage {
            message_id: format!("msg_{}", chrono::Utc::now().timestamp_millis()),
            channel_type: ChannelType::Feishu,
            sender: ChannelMember {
                channel_type: ChannelType::Feishu,
                member_id: user_id.to_string(),
                display_name: None,
                role: ChannelRole::User,
            },
            chat_id: chat_id.to_string(),
            content: MessageContent::Text(content.to_string()),
            timestamp: chrono::Utc::now().timestamp(),
            reply_to: None,
            metadata: serde_json::Map::new(),
        };
        
        // 获取对话历史
        let history = self.conversation_history.read().await.clone();
        
        // 使用处理器处理消息
        let processor = self.processor.read().await;
        let result = processor.process(&message, &history).await?;
        
        // 更新对话历史
        {
            let mut history = self.conversation_history.write().await;
            history.push(Message {
                role: MessageRole::User,
                content: content.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
            history.push(Message {
                role: MessageRole::Assistant,
                content: result.content.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
            
            // 限制历史长度
            if history.len() > 20 {
                let start = history.len() - 20;
                *history = history.split_off(start);
            }
        }
        
        info!(
            "消息处理完成: 用户={}, 通道={}, 策略={:?}, 记忆={}, 工具={}, 延迟={}ms",
            user_id,
            result.scope.channel,
            result.strategy,
            result.memory_count,
            result.tool_calls,
            result.latency_ms
        );
        
        Ok(result.content)
    }

    /// 调用 GLM 生成回复（支持工具调用）- 兼容旧逻辑
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let tools = self.tool_manager.get_all_tools().await;
        let system_prompt = build_tools_system_prompt(&tools);

        // 构建请求体
        let mut messages = vec![
            json!({"role": "system", "content": system_prompt}),
            json!({"role": "user", "content": prompt}),
        ];

        let mut max_iterations = 5; // 最多5轮工具调用
        let mut final_answer = String::new();

        for iteration in 0..max_iterations {
            info!("LLM 调用第 {} 轮", iteration + 1);

            // 调用 LLM
            let response = self.call_glm_api(&messages).await?;
            let content = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            info!("LLM 原始回复: {}", content);

            // 检查是否包含工具调用
            if let Some(tool_calls) = self.extract_tool_calls(&content) {
                info!("检测到工具调用: {} 个", tool_calls.len());

                // 执行所有工具调用
                let mut tool_results = Vec::new();
                for tool_call in &tool_calls {
                    let result = self.tool_manager.execute_tool(tool_call).await;
                    info!(
                        "工具 {} 执行: {}",
                        tool_call.name,
                        if result.success { "成功" } else { "失败" }
                    );

                    tool_results.push(json!({
                        "tool": tool_call.name,
                        "success": result.success,
                        "output": result.output,
                        "error": result.error,
                    }));
                }

                // 将工具调用和结果添加到对话历史
                messages.push(json!({
                    "role": "assistant",
                    "content": content,
                }));

                // 添加工具结果作为用户消息
                messages.push(json!({
                    "role": "user",
                    "content": format!("工具执行结果: {}", serde_json::to_string(&tool_results)?),
                }));
            } else {
                // 没有工具调用，提取最终答案
                final_answer = self.extract_final_answer(&content);
                break;
            }
        }

        if final_answer.is_empty() {
            final_answer = "抱歉，我无法生成有效的回复。".to_string();
        }

        Ok(final_answer)
    }

    /// 提取工具调用
    fn extract_tool_calls(&self, content: &str) -> Option<Vec<ToolCallRequest>> {
        // 简单的 JSON 提取逻辑
        if !content.contains("tool_calls") {
            return None;
        }

        // 尝试解析 JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(calls) = json.get("tool_calls").and_then(|v| v.as_array()) {
                let tool_calls: Vec<ToolCallRequest> = calls
                    .iter()
                    .filter_map(|call| {
                        let args: HashMap<String, String> = call
                            .get("arguments")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default();
                        
                        Some(ToolCallRequest {
                            name: call.get("name")?.as_str()?.to_string(),
                            arguments: args,
                        })
                    })
                    .collect();
                if !tool_calls.is_empty() {
                    return Some(tool_calls);
                }
            }
        }

        // 尝试从文本中提取
        let mut tool_calls = Vec::new();
        for line in content.lines() {
            if line.starts_with("tool:") || line.starts_with("工具:") {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    tool_calls.push(ToolCallRequest {
                        name: parts[1].trim().to_string(),
                        arguments: HashMap::new(),
                    });
                }
            }
        }

        if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        }
    }

    /// 提取最终答案
    fn extract_final_answer(&self, content: &str) -> String {
        // 尝试解析 JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(answer) = json.get("answer").and_then(|v| v.as_str()) {
                return answer.to_string();
            }
            if let Some(response) = json.get("response").and_then(|v| v.as_str()) {
                return response.to_string();
            }
        }

        // 尝试提取 answer: 或 答案: 后面的内容
        for line in content.lines() {
            if line.starts_with("answer:") || line.starts_with("答案:") {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    return parts[1].trim().to_string();
                }
            }
        }

        // 如果没有特殊标记，返回整个内容
        content.to_string()
    }

    /// 调用 GLM API
    async fn call_glm_api(&self, messages: &[serde_json::Value]) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();

        let url = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "temperature": 0.7,
                "max_tokens": 4096,
            }))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            error!("GLM API 错误: {} - {}", status, text);
            return Err(anyhow::anyhow!("GLM API error: {} - {}", status, text));
        }

        let json: serde_json::Value = serde_json::from_str(&text)?;
        Ok(json)
    }
}

#[async_trait]
impl EventHandler for LLMEventHandler {
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()> {
        match event {
            FeishuEvent::MessageReceived {
                app_id,
                open_id,
                chat_id,
                user_id,
                content,
                message_id,
                create_time,
            } => {
                info!(
                    "收到消息: app_id={}, open_id={}, chat_id={}, content={}",
                    app_id, open_id, chat_id, content
                );

                // 使用新的 ChannelProcessor 处理消息
                match self.process_message(&open_id, &chat_id, &content).await {
                    Ok(response) => {
                        // 发送回复
                        if let Err(e) = self
                            .sender
                            .send_simple_text(&chat_id, &response)
                            .await
                        {
                            error!("发送回复失败: {}", e);
                        } else {
                            info!("回复已发送: {}", response);
                        }
                    }
                    Err(e) => {
                        error!("处理消息失败: {}", e);
                        // 发送错误提示
                        let error_msg = format!("处理消息时发生错误: {}", e);
                        if let Err(e) = self
                            .sender
                            .send_simple_text(&chat_id, &error_msg)
                            .await
                        {
                            error!("发送错误提示失败: {}", e);
                        }
                    }
                }
            }
            FeishuEvent::MessageRead { chat_id, read_time } => {
                info!("消息已读: chat_id={}, read_time={}", chat_id, read_time);
            }
            FeishuEvent::UserTyping { chat_id, open_id } => {
                info!("用户正在输入: chat_id={}, open_id={}", chat_id, open_id);
            }
            FeishuEvent::BotAdded { app_id, chat_id } => {
                info!("机器人被添加: app_id={}, chat_id={}", app_id, chat_id);
            }
            FeishuEvent::BotRemoved { app_id, chat_id } => {
                info!("机器人被移除: app_id={}, chat_id={}", app_id, chat_id);
            }
            FeishuEvent::Error { code, message } => {
                error!("错误事件: code={}, message={}", code, message);
            }
        }

        Ok(())
    }

    async fn on_connect(&self, app_id: &str) -> WebSocketResult<()> {
        info!("✅ 飞书 WebSocket 连接成功: {}", app_id);
        Ok(())
    }

    async fn on_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        warn!("⚠️ 飞书 WebSocket 断开连接: {}", app_id);
        Ok(())
    }

    async fn on_error(&self, app_id: &str, error: &WebSocketError) -> WebSocketResult<()> {
        error!("❌ 飞书 WebSocket 错误: {} - {:?}", app_id, error);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("🚀 NewClaw Feishu WebSocket 长连接服务 v0.7.0 启动中...");

    // 加载配置
    let config = load_config()?;

    // 获取 API Key
    let api_key = std::env::var("GLM_API_KEY")
        .or_else(|_| {
            config
                .llm
                .glm
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("GLM_API_KEY not set"))
        })
        .expect("GLM API Key is required");

    let model = config.llm.model.clone();

    info!("使用模型: {}", model);

    // 创建 WebSocket 配置
    let ws_config = newclaw::feishu_websocket::WebSocketConfig {
        base_url: "https://open.feishu.cn/open-apis".to_string(),
        app_id: String::new(),
        app_secret: String::new(),
        heartbeat_interval: std::time::Duration::from_secs(30),
        heartbeat_timeout: std::time::Duration::from_secs(10),
        max_heartbeat_failures: 3,
        enable_auto_reconnect: true,
        max_reconnect_attempts: 10,
        initial_reconnect_delay: std::time::Duration::from_secs(1),
        max_reconnect_delay: std::time::Duration::from_secs(60),
        max_connections: 10,
        log_level: newclaw::feishu_websocket::LogLevel::Info,
    };

    let event_handler = Arc::new(LLMEventHandler::new(api_key, model).await);
    let manager = Arc::new(newclaw::feishu_websocket::FeishuWebSocketManager::new(
        ws_config,
        event_handler,
    ));

    // 启动管理器
    manager.start().await?;

    // 为每个账号启动连接
    for (account_name, account_config) in &config.feishu.accounts {
        if !account_config.enabled {
            info!("账号 {} 已禁用，跳过", account_name);
            continue;
        }

        info!("启动账号 {} 的飞书连接...", account_name);

        if let Err(e) = manager
            .connect(&account_config.app_id, &account_config.app_secret)
            .await
        {
            error!("启动账号 {} 连接失败: {}", account_name, e);
        } else {
            info!("账号 {} 连接成功", account_name);
        }
    }

    info!("✅ 所有飞书连接已启动，等待消息...");

    // 等待终止信号
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {
                info!("收到 SIGTERM 信号，正在关闭服务...");
            }
            _ = sigint.recv() => {
                info!("收到 SIGINT 信号，正在关闭服务...");
            }
        }
    }

    info!("👋 NewClaw Feishu WebSocket 长连接服务已停止");
    Ok(())
}

/// 加载配置
fn load_config() -> Result<newclaw::config::Config> {
    let config_path = std::env::var("NEWCLAW_CONFIG")
        .unwrap_or_else(|_| "/etc/newclaw/config.toml".to_string());

    let config = newclaw::config::Config::from_file(&config_path)?;
    info!("已加载配置: {}", config_path);
    Ok(config)
}