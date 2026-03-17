// NewClaw 飞书 WebSocket 长连接服务
//
// 独立服务，用于接收飞书消息并调用 LLM 回复
//
// v0.7.0 - 集成 ChannelProcessor，支持记忆、权限、策略机制
//        - 集成 TokenManager，支持 Token 自动刷新
//        - 会话历史持久化，重启不丢失

use anyhow::Result;
use tracing::{info, error, warn, debug};
use std::sync::Arc;
use chrono::Utc;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::RwLock;
use tokio::sync::Mutex;
use std::path::PathBuf;
use std::collections::HashMap;
use rusqlite::{Connection, params};

use newclaw::feishu_websocket::{
    EventHandler, FeishuEvent, WebSocketError, WebSocketResult,
    MessageSender,
    TokenManager, TokenManagerConfig, TokenType,
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
use newclaw::llm::{Message, MessageRole};

/// 飞书连接服务默认配置文件路径
const DEFAULT_CONFIG_PATH: &str = "/etc/newclaw/feishu-connect.toml";

/// 飞书连接服务数据目录
const DEFAULT_DATA_DIR: &str = "/var/lib/newclaw/feishu-connect";

/// 会话历史最大消息数（用户+助手消息总数）
const MAX_HISTORY_LENGTH: usize = 20;

/// 会话历史存储（持久化）
pub struct SessionHistoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionHistoryStore {
    /// 创建新的会话历史存储
    pub fn new(db_path: &PathBuf) -> Result<Self> {
        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let conn = Connection::open(db_path)?;
        
        // 创建表
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS session_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_session_chat_id ON session_history(chat_id);
            CREATE INDEX IF NOT EXISTS idx_session_created ON session_history(created_at);
            "#,
        )?;
        
        info!("会话历史存储初始化完成: {}", db_path.display());
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    
    /// 加载所有会话历史
    pub async fn load_all(&self) -> Result<HashMap<String, Vec<Message>>> {
        let conn = self.conn.lock().await;
        let mut map: HashMap<String, Vec<Message>> = HashMap::new();
        
        let mut stmt = conn.prepare(
            "SELECT chat_id, role, content FROM session_history ORDER BY created_at ASC"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        
        for row in rows {
            let (chat_id, role, content) = row?;
            let message = Message {
                role: match role.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::User,
                },
                content,
                tool_calls: None,
                tool_call_id: None,
            };
            
            map.entry(chat_id).or_insert_with(Vec::new).push(message);
        }
        
        // 限制每个会话的历史长度
        for (_, messages) in map.iter_mut() {
            if messages.len() > MAX_HISTORY_LENGTH {
                let start = messages.len() - MAX_HISTORY_LENGTH;
                *messages = messages.split_off(start);
            }
        }
        
        let total_sessions = map.len();
        let total_messages: usize = map.values().map(|v| v.len()).sum();
        info!("加载会话历史: {} 个会话, {} 条消息", total_sessions, total_messages);
        
        Ok(map)
    }
    
    /// 添加消息到会话历史
    pub async fn add_message(&self, chat_id: &str, message: &Message) -> Result<()> {
        let conn = self.conn.lock().await;
        let role = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        };
        
        conn.execute(
            "INSERT INTO session_history (chat_id, role, content, created_at) VALUES (?, ?, ?, ?)",
            params![
                chat_id,
                role,
                message.content,
                Utc::now().to_rfc3339()
            ],
        )?;
        
        Ok(())
    }
    
    /// 清理旧的会话历史（保留最近 N 条）
    pub async fn cleanup(&self, chat_id: &str, keep_count: usize) -> Result<()> {
        let conn = self.conn.lock().await;
        
        // 获取该会话的总消息数
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM session_history WHERE chat_id = ?",
            params![chat_id],
            |row| row.get(0),
        )?;
        
        if count > keep_count {
            // 删除最旧的消息
            conn.execute(
                r#"
                DELETE FROM session_history 
                WHERE chat_id = ? AND id IN (
                    SELECT id FROM session_history 
                    WHERE chat_id = ? 
                    ORDER BY created_at ASC 
                    LIMIT ?
                )
                "#,
                params![chat_id, chat_id, count - keep_count],
            )?;
            
            debug!("清理会话历史: chat_id={}, 删除 {} 条旧消息", chat_id, count - keep_count);
        }
        
        Ok(())
    }
}

/// 飞书消息处理器（集成 ChannelProcessor + TokenManager）
struct FeishuProcessorHandler {
    /// 消息发送器
    sender: MessageSender,
    /// ChannelProcessor
    processor: Arc<ChannelProcessor>,
    /// Token 管理器
    token_manager: Arc<TokenManager>,
    /// 会话历史（按 chat_id 隔离，内存缓存）
    session_history: Arc<RwLock<HashMap<String, Vec<Message>>>>,
    /// 会话历史存储（持久化）
    session_store: Arc<SessionHistoryStore>,
}

impl FeishuProcessorHandler {
    /// 创建新的处理器
    fn new(
        processor: Arc<ChannelProcessor>,
        token_manager: Arc<TokenManager>,
        session_history: HashMap<String, Vec<Message>>,
        session_store: Arc<SessionHistoryStore>,
    ) -> Self {
        let sender = MessageSender::new("https://open.feishu.cn");
        Self {
            sender,
            processor,
            token_manager,
            session_history: Arc::new(RwLock::new(session_history)),
            session_store,
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
                
                // 克隆需要的变量
                let processor = self.processor.clone();
                let chat_id = chat_id.clone();
                let token_manager = self.token_manager.clone();
                let session_history = self.session_history.clone();
                let session_store = self.session_store.clone();
                let user_text = text.clone();
                
                tokio::spawn(async move {
                    // 获取会话历史
                    let history = {
                        let histories = session_history.read().await;
                        histories.get(&chat_id).cloned().unwrap_or_default()
                    };
                    
                    info!("会话历史: {} 条消息", history.len());
                    
                    match processor.process(&message, &history).await {
                        Ok(result) => {
                            info!("处理完成: {} (记忆: {}条, 工具调用: {}次)", 
                                result.content.chars().take(50).collect::<String>(),
                                result.memory_count,
                                result.tool_calls
                            );
                            
                            // 创建用户消息和助手消息
                            let user_msg = Message {
                                role: MessageRole::User,
                                content: user_text.clone(),
                                tool_calls: None,
                                tool_call_id: None,
                            };
                            let assistant_msg = Message {
                                role: MessageRole::Assistant,
                                content: result.content.clone(),
                                tool_calls: None,
                                tool_call_id: None,
                            };
                            
                            // 持久化到数据库
                            if let Err(e) = session_store.add_message(&chat_id, &user_msg).await {
                                warn!("持久化用户消息失败: {}", e);
                            }
                            if let Err(e) = session_store.add_message(&chat_id, &assistant_msg).await {
                                warn!("持久化助手消息失败: {}", e);
                            }
                            
                            // 更新内存中的会话历史
                            {
                                let mut histories = session_history.write().await;
                                let session = histories.entry(chat_id.clone()).or_insert_with(Vec::new);
                                
                                session.push(user_msg);
                                session.push(assistant_msg);
                                
                                // 限制历史长度
                                if session.len() > MAX_HISTORY_LENGTH {
                                    let start = session.len() - MAX_HISTORY_LENGTH;
                                    *session = session.split_off(start);
                                }
                                
                                debug!("会话历史更新: chat_id={}, 长度={}", chat_id, session.len());
                            }
                            
                            // 清理数据库中的旧消息
                            if let Err(e) = session_store.cleanup(&chat_id, MAX_HISTORY_LENGTH).await {
                                warn!("清理旧会话历史失败: {}", e);
                            }
                            
                            // 从 TokenManager 获取缓存的 token
                            let token = match token_manager.get_token().await {
                                Some(t) => t,
                                None => {
                                    error!("Token 不可用，请检查 TokenManager 配置");
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
                            
                            // 从 TokenManager 获取 token
                            let token = match token_manager.get_token().await {
                                Some(t) => t,
                                None => {
                                    error!("Token 不可用");
                                    return;
                                }
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
    info!("📦 版本: 0.7.0 (集成 ChannelProcessor + TokenManager + 会话持久化)");

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
        None,
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
    
    // ==================== 初始化会话历史存储 ====================
    
    let session_store = Arc::new(SessionHistoryStore::new(&data_dir.join("session_history.db"))?);
    let session_history = session_store.load_all().await?;
    info!("✅ 会话历史存储初始化完成");
    
    // ==================== 初始化 TokenManager ====================
    
    // 获取第一个启用的账号配置
    let (first_account_name, first_account_config) = config.feishu.accounts
        .iter()
        .find(|(_, cfg)| cfg.enabled)
        .map(|(name, cfg)| (name.clone(), cfg.clone()))
        .ok_or_else(|| anyhow::anyhow!("没有启用的飞书账号"))?;
    
    // 创建 TokenManager 配置
    let token_manager_config = TokenManagerConfig {
        app_id: first_account_config.app_id.clone(),
        app_secret: first_account_config.app_secret.clone(),
        token_type: TokenType::TenantAccessToken,
        refresh_margin_secs: 300,     // 提前 5 分钟刷新
        check_interval_secs: 60,      // 每 60 秒检查一次
        max_refresh_failures: 5,
        retry_interval_secs: 30,
    };
    
    // 创建 TokenManager
    let token_manager = Arc::new(TokenManager::new(token_manager_config));
    
    // 初始化 Token（首次获取）
    token_manager.initialize().await?;
    info!("✅ TokenManager 初始化完成");
    
    // 启动自动刷新任务
    token_manager.start_auto_refresh().await;
    info!("✅ Token 自动刷新任务已启动");
    
    // ==================== 飞书连接管理 ====================
    
    // 创建 WebSocket 管理器配置
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
        
        // 创建该账号的处理器（使用共享的 TokenManager 和会话历史）
        let handler = Arc::new(FeishuProcessorHandler::new(
            processor.clone(),
            token_manager.clone(),
            session_history.clone(),
            session_store.clone(),
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
        
        // 停止 Token 自动刷新
        token_manager.stop_auto_refresh().await;
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
            
            DEFAULT_CONFIG_PATH.to_string()
        });

    let config = newclaw::config::Config::from_file(&config_path)?;
    info!("已加载配置: {}", config_path);
    Ok(config)
}