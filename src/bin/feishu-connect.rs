// NewClaw 飞书 WebSocket 长连接服务
//
// 独立服务，用于接收飞书消息并调用 LLM 回复
//
// v0.7.0 - 集成 ChannelProcessor，支持记忆、权限、策略机制

use anyhow::Result;
use tracing::{info, error, warn};
use std::sync::Arc;
use chrono::Utc;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::RwLock;
use std::path::PathBuf;

use newclaw::feishu_websocket::{
    EventHandler, FeishuEvent, WebSocketError, WebSocketResult,
    MessageSender,
};
use newclaw::channel::{
    ChannelProcessor, ProcessorConfig, ChannelMessage, MessageContent,
    ChannelMember, ChannelType, ChannelRole, ChannelPermission,
};
use newclaw::memory::{SQLiteMemoryStorage, StorageConfig};
use newclaw::context::StrategyEngine;
use newclaw::tools::ToolRegistry;
use newclaw::llm::QwenCodeProvider;
use newclaw::llm::OpenAIProvider;

/// 飞书连接服务默认配置文件路径
const DEFAULT_CONFIG_PATH: &str = "/etc/newclaw/feishu-connect.toml";

/// 飞书连接服务数据目录
const DEFAULT_DATA_DIR: &str = "/var/lib/newclaw/feishu-connect";

/// 飞书消息处理器（集成 ChannelProcessor）
struct FeishuProcessorHandler {
    /// 消息发送器
    sender: MessageSender,
    /// ChannelProcessor
    processor: Arc<ChannelProcessor>,
    /// 应用 ID
    app_id: String,
    /// 应用密钥
    app_secret: String,
}

impl FeishuProcessorHandler {
    /// 创建新的处理器
    fn new(
        processor: Arc<ChannelProcessor>,
        app_id: String,
        app_secret: String,
    ) -> Self {
        let sender = MessageSender::new("https://open.feishu.cn");
        Self {
            sender,
            processor,
            app_id,
            app_secret,
        }
    }
}

#[async_trait]
impl EventHandler for FeishuProcessorHandler {
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()> {
        match &event {
            FeishuEvent::MessageReceived { open_id, chat_id, content, .. } => {
                info!("收到消息 - 用户: {}, 群: {}", open_id, chat_id);
                
                // 解析消息内容
                let text = if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                    json.get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or(content)
                        .to_string()
                } else {
                    content.clone()
                };
                
                if text.trim().is_empty() {
                    return Ok(());
                }
                
                info!("处理消息: {}", text);
                
                // 构建 ChannelMessage
                let message = ChannelMessage {
                    message_id: format!("feishu_{}", chrono::Utc::now().timestamp_millis()),
                    channel_type: ChannelType::Feishu,
                    sender: ChannelMember {
                        channel_type: ChannelType::Feishu,
                        member_id: open_id.clone(),
                        display_name: None,
                        role: ChannelRole::User,
                    },
                    chat_id: chat_id.clone(),
                    content: MessageContent::Text(text.clone()),
                    timestamp: chrono::Utc::now().timestamp(),
                    reply_to: None,
                    metadata: serde_json::Map::new(),
                };
                
                // 使用 ChannelProcessor 处理消息
                let processor = self.processor.clone();
                let chat_id = chat_id.clone();
                let app_id = self.app_id.clone();
                let app_secret = self.app_secret.clone();
                
                tokio::spawn(async move {
                    match processor.process(&message, &[]).await {
                        Ok(result) => {
                            info!("处理完成: {} (记忆: {}条, 工具调用: {}次)", 
                                result.content.chars().take(50).collect::<String>(),
                                result.memory_count,
                                result.tool_calls
                            );
                            
                            // 发送回复
                            let token = match fetch_access_token(&app_id, &app_secret).await {
                                Ok((t, _)) => t,
                                Err(e) => {
                                    error!("获取 access_token 失败: {}", e);
                                    return;
                                }
                            };
                            
                            let sender = MessageSender::new("https://open.feishu.cn").with_token(&token);
                            match sender.send_simple_text(&chat_id, &result.content).await {
                                Ok(msg_id) => info!("消息已发送: {}", msg_id),
                                Err(e) => error!("发送消息失败: {:?}", e),
                            }
                        }
                        Err(e) => {
                            error!("处理消息失败: {}", e);
                            
                            // 发送错误提示
                            let token = match fetch_access_token(&app_id, &app_secret).await {
                                Ok((t, _)) => t,
                                Err(_) => return,
                            };
                            
                            let sender = MessageSender::new("https://open.feishu.cn").with_token(&token);
                            let _ = sender.send_simple_text(&chat_id, "抱歉，处理您的消息时遇到问题。").await;
                        }
                    }
                });
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
            std::env::var("RUST_LOG").unwrap_or_else(|_| "newclaw=info".to_string())
        )
        .init();

    info!("🚀 NewClaw Feishu WebSocket 长连接服务启动...");
    info!("📦 版本: 0.7.0 (集成 ChannelProcessor + 完整工具集)");

    // 加载配置（使用独立配置文件）
    let config = load_config()?;

    // 数据目录
    let data_dir = PathBuf::from(
        std::env::var("NEWCLAW_DATA_DIR")
            .unwrap_or_else(|_| DEFAULT_DATA_DIR.to_string())
    );
    
    // 确保数据目录存在
    std::fs::create_dir_all(&data_dir)?;
    info!("数据目录: {}", data_dir.display());

    // 检查飞书配置
    if config.feishu.accounts.is_empty() {
        warn!("未配置飞书账号，服务将退出");
        warn!("请在 {} 中配置 [feishu.accounts.*]", DEFAULT_CONFIG_PATH);
        return Ok(());
    }

    info!("找到 {} 个飞书账号配置", config.feishu.accounts.len());

    // ==================== 初始化 ChannelProcessor ====================
    
    // 1. 创建工具注册表并注册所有内置工具
    let tools = Arc::new(ToolRegistry::new());
    
    // 初始化内置工具
    if let Err(e) = newclaw::tools::init_builtin_tools_with_permissions(
        &tools,
        data_dir.clone(),
        data_dir.clone(),
        None, // 权限管理器稍后创建
    ).await {
        warn!("部分工具初始化失败: {}", e);
    }
    
    let tool_count = tools.list_tools().await.len();
    info!("✅ 工具注册表初始化完成: {} 个工具", tool_count);
    
    // 2. 创建权限管理器
    let permissions = Arc::new(ChannelPermission::new(&data_dir.join("permissions.json").display().to_string()));
    info!("✅ 权限管理器初始化完成");
    
    // 3. 创建记忆存储
    let memory_config = StorageConfig {
        db_path: data_dir.join("memory.db"),
        ..Default::default()
    };
    let memory = Arc::new(SQLiteMemoryStorage::new(memory_config)?);
    info!("✅ 记忆存储初始化完成: {}", data_dir.join("memory.db").display());
    
    // 4. 创建策略引擎
    let strategy = Arc::new(RwLock::new(StrategyEngine::new()?));
    info!("✅ 策略引擎初始化完成");
    
    // 5. 创建 LLM Provider（根据配置选择）
    let llm_provider: Arc<dyn newclaw::llm::LLMProviderV3> = match config.llm.provider.as_str() {
        "openai" => {
            let api_key = config.llm.openai.api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .unwrap_or_else(|| {
                    warn!("未配置 OpenAI API Key");
                    String::new()
                });
            let mut provider = OpenAIProvider::new(api_key);
            // 支持自定义 Base URL（兼容 OpenAI 兼容的 API）
            if let Some(base_url) = &config.llm.openai.base_url {
                provider = provider.with_base_url(base_url.clone());
                info!("✅ 使用 OpenAI Provider (自定义 URL: {})", base_url);
            } else {
                info!("✅ 使用 OpenAI Provider");
            }
            Arc::new(provider)
        }
        "qwencode" => {
            let api_key = config.llm.qwencode.api_key.clone()
                .or_else(|| std::env::var("QWENCODE_API_KEY").ok())
                .unwrap_or_else(|| {
                    warn!("未配置 QwenCode API Key");
                    String::new()
                });
            let mut provider = QwenCodeProvider::new(api_key);
            // 支持自定义 Base URL
            if let Some(base_url) = &config.llm.qwencode.base_url {
                provider = provider.with_base_url(base_url.clone());
                info!("✅ 使用 QwenCode Provider (自定义 URL: {})", base_url);
            } else {
                info!("✅ 使用 QwenCode Provider");
            }
            Arc::new(provider)
        }
        "glm" | "zhipu" => {
            let api_key = config.llm.glm.api_key.clone()
                .or_else(|| std::env::var("GLM_API_KEY").ok())
                .unwrap_or_else(|| {
                    warn!("未配置 GLM API Key");
                    String::new()
                });
            info!("✅ 使用 GLM Provider");
            Arc::new(newclaw::llm::GlmProvider::new(api_key))
        }
        _ => {
            warn!("未知的 LLM Provider: {}, 使用 GLM", config.llm.provider);
            let api_key = config.llm.glm.api_key.clone().unwrap_or_default();
            Arc::new(newclaw::llm::GlmProvider::new(api_key))
        }
    };
    
    let model = config.llm.model.clone();
    info!("✅ LLM Provider 初始化完成，模型: {}", model);
    
    // 6. 创建 ChannelProcessor 配置
    let processor_config = ProcessorConfig {
        enable_memory: true,
        enable_strategy: true,
        memory_search_limit: 5,
        max_context_tokens: 4096,
        default_strategy: newclaw::context::StrategyType::Balanced,
        ..Default::default()
    };
    
    // 7. 创建 ChannelProcessor
    let processor = Arc::new(
        ChannelProcessor::new(tools.clone(), permissions.clone(), processor_config)
            .with_memory(memory.clone())
            .with_strategy(strategy.clone())
            .with_llm(llm_provider, model)
    );
    info!("✅ ChannelProcessor 初始化完成");
    
    // ==================== 飞书连接管理 ====================
    
    // 检查并刷新过期的token
    for (account_name, account_config) in &config.feishu.accounts {
        if !account_config.enabled {
            info!("账号 {} 已禁用，跳过", account_name);
            continue;
        }

        // 检查token是否过期或即将过期（提前5分钟刷新）
        let need_refresh = account_config.access_token.is_none() 
            || account_config.token_expires_at.map_or(true, |exp| {
                let now = Utc::now().timestamp();
                exp - now < 300 // 5分钟内过期
            });

        if need_refresh {
            info!("账号 {} 的token已过期或即将过期，尝试刷新...", account_name);
            
            match fetch_access_token(&account_config.app_id, &account_config.app_secret).await {
                Ok((token, expires_in)) => {
                    info!("✅ 成功刷新账号 {} 的access_token，有效期 {} 秒", account_name, expires_in);
                }
                Err(e) => {
                    error!("❌ 刷新账号 {} 的token失败: {}", account_name, e);
                }
            }
        } else {
            info!("账号 {} 的token仍然有效", account_name);
        }
    }

    // 创建 WebSocket 管理器
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

    // 为每个账号创建独立的处理器和连接
    let mut managers = Vec::new();
    
    for (account_name, account_config) in &config.feishu.accounts {
        if !account_config.enabled {
            continue;
        }
        
        // 创建该账号的处理器
        let handler = Arc::new(FeishuProcessorHandler::new(
            processor.clone(),
            account_config.app_id.clone(),
            account_config.app_secret.clone(),
        ));
        
        let manager = Arc::new(newclaw::feishu_websocket::FeishuWebSocketManager::new(
            ws_config.clone(),
            handler,
        ));
        
        managers.push((account_name.clone(), manager, account_config.clone()));
    }

    // 启动所有管理器
    for (account_name, manager, _) in &managers {
        manager.start().await?;
        info!("启动账号 {} 的飞书连接...", account_name);
    }

    // 为每个账号建立 WebSocket 连接
    for (account_name, manager, account_config) in &managers {
        if let Err(e) = manager.connect(&account_config.app_id, &account_config.app_secret).await {
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

/// 加载配置（使用独立配置文件）
fn load_config() -> Result<newclaw::config::Config> {
    // 优先级：环境变量 > 默认路径 > 当前目录
    let config_path = std::env::var("FEISHU_CONNECT_CONFIG")
        .or_else(|_| std::env::var("NEWCLAW_CONFIG"))
        .unwrap_or_else(|_| {
            // 尝试多个路径
            let paths = [
                DEFAULT_CONFIG_PATH,
                "./feishu-connect.toml",
                "./config.toml",
            ];
            
            for path in &paths {
                if std::path::Path::new(path).exists() {
                    return path.to_string();
                }
            }
            
            // 默认返回第一个路径（即使不存在，后续会报错）
            DEFAULT_CONFIG_PATH.to_string()
        });

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