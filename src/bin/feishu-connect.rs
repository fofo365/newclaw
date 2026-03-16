// NewClaw 飞书 WebSocket 长连接服务
//
// 独立服务，用于接收飞书消息并调用 LLM 回复，支持工具调用

use anyhow::Result;
use tracing::{info, error, warn};
use std::sync::Arc;
use chrono::Utc;
use serde_json::json;

use async_trait::async_trait;
use newclaw::feishu_websocket::{
    EventHandler, FeishuEvent, WebSocketError, WebSocketResult,
    MessageSender, ToolManager, ToolCallRequest, build_tools_system_prompt,
};

/// 对话历史消息
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

/// 智能 LLM 事件处理器
struct LLMEventHandler {
    /// 消息发送器
    sender: MessageSender,
    /// GLM API Key
    api_key: String,
    /// GLM 模型
    model: String,
    /// 工具管理器
    tool_manager: Arc<ToolManager>,
}

impl LLMEventHandler {
    fn new(api_key: String, model: String) -> Self {
        let sender = MessageSender::new("https://open.feishu.cn");
        let tool_manager = Arc::new(ToolManager::new());
        Self {
            sender,
            api_key,
            model,
            tool_manager,
        }
    }

    /// 调用 GLM 生成回复（支持工具调用）
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let tools = self.tool_manager.get_all_tools();
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

                messages.push(json!({
                    "role": "user",
                    "content": format!("工具执行结果:\n{}\n\n请基于以上结果回答用户的问题。",
                        serde_json::to_string_pretty(&tool_results).unwrap_or_default())
                }));

                continue; // 继续下一轮
            }

            // 没有工具调用，这就是最终答案
            final_answer = content;
            break;
        }

        if final_answer.is_empty() {
            final_answer = "抱歉，我无法生成有效的回复。".to_string();
        }

        Ok(final_answer)
    }

    /// 调用 GLM API
    async fn call_glm_api(&self, messages: &[serde_json::Value]) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let url = "https://api.z.ai/api/paas/v4/chat/completions";

        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        info!("GLM API 请求: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let json: serde_json::Value = response.json().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "GLM API 错误: {} - {:?}",
                status,
                json
            ));
        }

        Ok(json)
    }

    /// 从 LLM 回复中提取工具调用
    fn extract_tool_calls(&self, content: &str) -> Option<Vec<ToolCallRequest>> {
        // 尝试从 Markdown 代码块中提取 JSON
        let json_str = if let Some(start) = content.find("```json") {
            let start = start + 7;
            if let Some(end) = content[start..].find("```") {
                &content[start..start + end]
            } else {
                return None;
            }
        } else if let Some(start) = content.find("```") {
            let start = start + 3;
            if let Some(end) = content[start..].find("```") {
                &content[start..start + end]
            } else {
                return None;
            }
        } else {
            // 尝试直接解析整个内容
            content
        };

        // 解析 JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(tool_calls) = json.get("tool_calls").and_then(|v| v.as_array()) {
                let mut calls = Vec::new();
                for tc in tool_calls {
                    if let (Some(name), Some(args)) = (
                        tc.get("name").and_then(|n| n.as_str()),
                        tc.get("arguments").and_then(|a| a.as_object()),
                    ) {
                        let mut arguments = std::collections::HashMap::new();
                        for (k, v) in args {
                            if let Some(s) = v.as_str() {
                                arguments.insert(k.clone(), s.to_string());
                            }
                        }
                        calls.push(ToolCallRequest {
                            name: name.to_string(),
                            arguments,
                        });
                    }
                }
                if !calls.is_empty() {
                    return Some(calls);
                }
            }
        }

        None
    }

    /// 获取 access_token 并发送回复
    async fn reply_message(&self, chat_id: &str, text: &str, app_id: &str, app_secret: &str) {
        // 获取 access_token
        let token = match fetch_access_token(app_id, app_secret).await {
            Ok((t, _)) => t,
            Err(e) => {
                error!("获取 access_token 失败: {}", e);
                return;
            }
        };

        // 创建带 token 的发送器
        let sender = MessageSender::new("https://open.feishu.cn").with_token(&token);

        // 发送消息
        match sender.send_simple_text(chat_id, text).await {
            Ok(msg_id) => info!("消息已发送: {}", msg_id),
            Err(e) => error!("发送消息失败: {:?}", e),
        }
    }
}

#[async_trait]
impl EventHandler for LLMEventHandler {
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()> {
        match &event {
            FeishuEvent::MessageReceived {
                app_id,
                open_id,
                chat_id,
                content,
                ..
            } => {
                info!("收到消息 - 用户: {}, 群: {}, 内容: {}", open_id, chat_id, content);

                // 解析消息内容
                let text = if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                    json.get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or(content)
                        .to_string()
                } else {
                    content.clone()
                };

                // 跳过空消息
                if text.trim().is_empty() {
                    return Ok(());
                }

                info!("处理消息: {}", text);

                // 调用 LLM 生成回复（支持工具调用）
                match self.call_llm(&text).await {
                    Ok(reply) => {
                        info!("LLM 最终回复: {}", reply);
                        // 发送回复
                        self.reply_message(
                            chat_id,
                            &reply,
                            app_id,
                            "zYNommBXUXSDzUaULXUfMhBJKjK6LyAZ",
                        )
                        .await;
                    }
                    Err(e) => {
                        error!("LLM 调用失败: {}", e);
                        // 发送错误提示
                        self.reply_message(
                            chat_id,
                            "抱歉，我遇到了一些问题，请稍后再试。",
                            app_id,
                            "zYNommBXUXSDzUaULXUfMhBJKjK6LyAZ",
                        )
                        .await;
                    }
                }
            }
            _ => {
                info!("忽略事件: {:?}", event);
            }
        }
        Ok(())
    }

    async fn on_connect(&self, app_id: &str) -> WebSocketResult<()> {
        info!("✅ 连接成功: {}", app_id);
        Ok(())
    }

    async fn on_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        warn!("⚠️ 连接断开: {}", app_id);
        Ok(())
    }

    async fn on_error(&self, app_id: &str, error: &WebSocketError) -> WebSocketResult<()> {
        error!("❌ 错误 [{}]: {:?}", app_id, error);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "newclaw=info".to_string()),
        )
        .init();

    info!("🚀 NewClaw Feishu WebSocket 长连接服务启动（支持工具调用）...");

    // 加载配置
    let config = load_config()?;

    // 检查飞书配置
    if config.feishu.accounts.is_empty() {
        warn!("未配置飞书账号，服务将退出");
        warn!("请在 /etc/newclaw/config.toml 中配置 [feishu.accounts.*]");
        return Ok(());
    }

    info!("找到 {} 个飞书账号配置", config.feishu.accounts.len());

    // 获取 GLM API Key
    let api_key = config
        .llm
        .glm
        .api_key
        .clone()
        .or_else(|| std::env::var("GLM_API_KEY").ok())
        .unwrap_or_else(|| {
            warn!("未配置 GLM API Key，将无法调用 LLM");
            String::new()
        });

    let model = config.llm.model.clone();
    info!("使用模型: {}", model);

    // 检查并刷新过期的token
    for (account_name, account_config) in &config.feishu.accounts {
        if !account_config.enabled {
            info!("账号 {} 已禁用，跳过", account_name);
            continue;
        }

        // 检查token是否过期或即将过期（提前5分钟刷新）
        let need_refresh = account_config.access_token.is_none()
            || account_config
                .token_expires_at
                .map_or(true, |exp| {
                    let now = Utc::now().timestamp();
                    exp - now < 300 // 5分钟内过期
                });

        if need_refresh {
            info!(
                "账号 {} 的token已过期或即将过期，尝试刷新...",
                account_name
            );

            // 获取新token
            match fetch_access_token(&account_config.app_id, &account_config.app_secret).await {
                Ok((token, expires_in)) => {
                    info!(
                        "✅ 成功刷新账号 {} 的access_token，有效期 {} 秒",
                        account_name, expires_in
                    );
                    // TODO: 更新配置文件
                }
                Err(e) => {
                    error!("❌ 刷新账号 {} 的token失败: {}", account_name, e);
                }
            }
        } else {
            info!("账号 {} 的token仍然有效", account_name);
        }
    }

    // 创建 WebSocket 管理器（使用 LLM 事件处理器）
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

    let event_handler = Arc::new(LLMEventHandler::new(api_key, model));
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

/// 获取飞书 access_token
async fn fetch_access_token(app_id: &str, app_secret: &str) -> Result<(String, u32)> {
    let client = reqwest::Client::new();
    let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "app_id": app_id,
            "app_secret": app_secret,
        }))
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;

    if json["code"].as_i64() != Some(0) {
        return Err(anyhow::anyhow!("Feishu API error: {:?}", json["msg"]));
    }

    let token = json["tenant_access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No token in response"))?
        .to_string();

    let expires_in = json["expire"].as_u64().unwrap_or(7200) as u32;

    Ok((token, expires_in))
}