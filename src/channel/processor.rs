// Channel Processor - v0.7.0
//
// 统一消息处理器，集成：
// - 记忆管理 (MemoryStorage) - 支持多层隔离：用户/通道/Agent/命名空间
// - 策略管理 (StrategyEngine) - 支持动态调整
// - 权限控制 (ChannelPermission)
// - 工具调用 (ToolRegistry)
// - LLM 调用 (LLMProvider)

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, debug};

use super::{ChannelType, ChannelMember, ChannelMessage, ChannelResponse, MessageContent};
use crate::tools::ToolRegistry;
use crate::channel::ChannelPermission;
use crate::memory::{MemoryStorage, MemoryEntry, MemoryType, MemoryScope, HybridSearchConfig};
use crate::context::{StrategyEngine, StrategyType};
use crate::llm::{LLMProviderV3, ChatRequest, Message, MessageRole, ToolDefinition};

/// 处理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// 是否启用记忆
    pub enable_memory: bool,
    /// 是否启用策略
    pub enable_strategy: bool,
    /// 默认策略
    pub default_strategy: StrategyType,
    /// 最大上下文 tokens
    pub max_context_tokens: usize,
    /// 记忆搜索限制
    pub memory_search_limit: usize,
    /// 默认 Agent ID
    pub default_agent_id: String,
    /// 默认命名空间
    pub default_namespace: String,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            enable_memory: true,
            enable_strategy: true,
            default_strategy: StrategyType::Balanced,
            max_context_tokens: 8000,
            memory_search_limit: 5,
            default_agent_id: "default".to_string(),
            default_namespace: "default".to_string(),
        }
    }
}

/// 处理结果
#[derive(Debug, Clone)]
pub struct ProcessResult {
    /// 响应内容
    pub content: String,
    /// 使用的 tokens
    pub tokens: Option<TokenUsage>,
    /// 使用的策略
    pub strategy: Option<StrategyType>,
    /// 记忆条目数
    pub memory_count: usize,
    /// 工具调用次数
    pub tool_calls: usize,
    /// 处理时间 (ms)
    pub latency_ms: u64,
    /// 隔离维度
    pub scope: MemoryScope,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub total: u32,
}

/// 通道处理器
pub struct ChannelProcessor {
    /// 配置
    config: ProcessorConfig,
    /// 工具注册表
    tools: Arc<ToolRegistry>,
    /// 权限管理
    permissions: Arc<ChannelPermission>,
    /// 记忆存储 (可选)
    memory: Option<Arc<dyn MemoryStorage>>,
    /// 策略引擎 (可选)
    strategy: Option<Arc<RwLock<StrategyEngine>>>,
    /// LLM Provider (可选)
    llm: Option<Arc<dyn LLMProviderV3>>,
    /// 模型名称
    model: String,
    /// 温度参数
    temperature: f32,
    /// 最大 tokens
    max_tokens: usize,
    /// Agent ID
    agent_id: String,
    /// 命名空间
    namespace: String,
}

impl ChannelProcessor {
    /// 创建新的处理器
    pub fn new(
        tools: Arc<ToolRegistry>,
        permissions: Arc<ChannelPermission>,
        config: ProcessorConfig,
    ) -> Self {
        let agent_id = config.default_agent_id.clone();
        let namespace = config.default_namespace.clone();
        Self {
            config,
            tools,
            permissions,
            memory: None,
            strategy: None,
            llm: None,
            model: "glm-4".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            agent_id,
            namespace,
        }
    }

    /// 设置记忆存储
    pub fn with_memory(mut self, memory: Arc<dyn MemoryStorage>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// 设置策略引擎
    pub fn with_strategy(mut self, strategy: Arc<RwLock<StrategyEngine>>) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// 设置 LLM Provider
    pub fn with_llm(mut self, llm: Arc<dyn LLMProviderV3>, model: String) -> Self {
        self.llm = Some(llm);
        self.model = model;
        self
    }

    /// 设置 LLM 参数
    pub fn with_llm_params(mut self, temperature: f32, max_tokens: usize) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }

    /// 设置 Agent ID
    pub fn with_agent_id(mut self, agent_id: &str) -> Self {
        self.agent_id = agent_id.to_string();
        self
    }

    /// 设置命名空间
    pub fn with_namespace(mut self, namespace: &str) -> Self {
        self.namespace = namespace.to_string();
        self
    }

    /// 构建隔离维度
    pub fn build_scope(&self, user_id: &str, channel: &str) -> MemoryScope {
        MemoryScope::for_channel(user_id, channel)
            .with_agent(&self.agent_id)
            .with_namespace(&self.namespace)
    }

    /// 处理消息
    pub async fn process(
        &self,
        message: &ChannelMessage,
        session_messages: &[Message],
    ) -> Result<ProcessResult> {
        let start = std::time::Instant::now();
        let mut memory_count = 0;
        let mut tool_calls = 0;
        let mut tokens = None;
        let mut strategy_used = None;

        // 1. 构建隔离维度
        let user_id = &message.sender.member_id;
        let channel = message.channel_type.to_string();
        let scope = self.build_scope(user_id, &channel);

        debug!("处理消息: user={}, channel={}, scope={:?}", user_id, channel, scope);

        // 2. 检查权限
        if !self.check_permission(&message.sender, "chat").await {
            return Ok(ProcessResult {
                content: "权限不足，无法进行对话".to_string(),
                tokens: None,
                strategy: None,
                memory_count: 0,
                tool_calls: 0,
                latency_ms: start.elapsed().as_millis() as u64,
                scope,
            });
        }

        // 3. 检索相关记忆（使用隔离维度）
        let memory_context = if self.config.enable_memory {
            if let Some(ref memory) = self.memory {
                let text = message.content.as_text().unwrap_or("");
                
                // 使用隔离维度搜索
                let search_config = HybridSearchConfig {
                    top_k: self.config.memory_search_limit,
                    ..Default::default()
                };
                
                match memory.search_hybrid_with_scope(text, &search_config, &scope).await {
                    Ok(results) => {
                        memory_count = results.len();
                        if !results.is_empty() {
                            let context: Vec<String> = results
                                .iter()
                                .map(|r| format!("[{}] {}", r.id, r.content))
                                .collect();
                            Some(format!("相关记忆:\n{}\n", context.join("\n")))
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        warn!("记忆搜索失败: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        // 4. 构建消息列表
        let mut messages = Vec::new();

        // 添加记忆上下文
        if let Some(ref mem_ctx) = memory_context {
            messages.push(Message {
                role: MessageRole::System,
                content: mem_ctx.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // 添加历史消息
        messages.extend(session_messages.to_vec());

        // 添加当前消息
        let user_content = message.content.to_string_content();
        messages.push(Message {
            role: MessageRole::User,
            content: user_content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });

        // 5. 应用策略截断
        let messages = if self.config.enable_strategy {
            if let Some(ref strategy) = self.strategy {
                let mut engine = strategy.write().await;
                match engine.apply(
                    &messages,
                    self.config.default_strategy.clone(),
                    self.config.max_context_tokens,
                    &self.model,
                ).await {
                    Ok(truncated) => {
                        strategy_used = Some(self.config.default_strategy.clone());
                        debug!("策略截断: {} -> {} 消息", messages.len(), truncated.len());
                        truncated
                    }
                    Err(e) => {
                        warn!("策略应用失败: {}", e);
                        messages
                    }
                }
            } else {
                messages
            }
        } else {
            messages
        };

        // 6. 调用 LLM
        let response = if let Some(ref llm) = self.llm {
            self.call_llm_with_tools(llm.as_ref(), messages.clone(), &mut tool_calls).await?
        } else {
            // Mock 响应
            format!("[Mock] 收到消息: {}", user_content)
        };

        // 7. 存储到记忆（使用隔离维度）
        if self.config.enable_memory {
            if let Some(ref memory) = self.memory {
                let now = chrono::Utc::now();
                
                // 存储用户消息
                let user_entry = MemoryEntry {
                    id: format!("msg_{}_user", message.message_id),
                    created_at: now,
                    last_accessed: now,
                    memory_type: MemoryType::Conversation,
                    importance: 0.5,
                    content: user_content.clone(),
                    metadata: std::collections::HashMap::from([
                        ("channel".to_string(), serde_json::json!(channel.clone())),
                        ("sender".to_string(), serde_json::json!(user_id.clone())),
                        ("timestamp".to_string(), serde_json::json!(message.timestamp)),
                    ]),
                    source_agent: Some(self.agent_id.clone()),
                    tags: vec!["user_message".to_string(), channel.clone()],
                };
                
                if let Err(e) = memory.store_with_scope(&user_entry, &scope).await {
                    warn!("存储用户消息到记忆失败: {}", e);
                }
                
                // 存储助手响应
                let assistant_entry = MemoryEntry {
                    id: format!("msg_{}_assistant", message.message_id),
                    created_at: now,
                    last_accessed: now,
                    memory_type: MemoryType::Conversation,
                    importance: 0.5,
                    content: response.clone(),
                    metadata: std::collections::HashMap::from([
                        ("channel".to_string(), serde_json::json!(channel.clone())),
                        ("strategy".to_string(), serde_json::json!(strategy_used.as_ref().map(|s| format!("{:?}", s)))),
                    ]),
                    source_agent: Some(self.agent_id.clone()),
                    tags: vec!["assistant_message".to_string(), channel.clone()],
                };
                
                if let Err(e) = memory.store_with_scope(&assistant_entry, &scope).await {
                    warn!("存储助手响应到记忆失败: {}", e);
                }
                
                info!("存储记忆: user={}, channel={}, agent={}, ns={}", 
                    scope.user_id, scope.channel, 
                    scope.agent_id.as_deref().unwrap_or("-"),
                    scope.namespace.as_deref().unwrap_or("-"));
            }
        }

        Ok(ProcessResult {
            content: response,
            tokens,
            strategy: strategy_used,
            memory_count,
            tool_calls,
            latency_ms: start.elapsed().as_millis() as u64,
            scope,
        })
    }

    /// 检查权限
    async fn check_permission(&self, member: &ChannelMember, action: &str) -> bool {
        self.permissions.check(member, action).await
    }

    /// 调用 LLM (带工具支持)
    async fn call_llm_with_tools(
        &self,
        llm: &dyn LLMProviderV3,
        mut messages: Vec<Message>,
        tool_calls_count: &mut usize,
    ) -> Result<String> {
        // 获取工具定义
        let tool_definitions = self.get_tool_definitions().await;

        // 工具调用循环
        let max_rounds = 15;

        for _round in 0..max_rounds {
            let request = ChatRequest {
                messages: messages.clone(),
                model: self.model.clone(),
                temperature: self.temperature,
                max_tokens: Some(self.max_tokens),
                top_p: None,
                stop: None,
                tools: if tool_definitions.is_empty() { None } else { Some(tool_definitions.clone()) },
            };

            let response = llm.chat(request).await?;

            // 检查工具调用
            if let Some(ref tool_calls) = response.message.tool_calls {
                if tool_calls.is_empty() {
                    return Ok(response.message.content);
                }

                // 添加助手消息
                messages.push(Message {
                    role: MessageRole::Assistant,
                    content: response.message.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                });

                // 执行工具
                for tool_call in tool_calls {
                    *tool_calls_count += 1;
                    info!("执行工具: {} ({})", tool_call.name, tool_call.id);

                    let args: serde_json::Value = serde_json::from_str(&tool_call.arguments)
                        .unwrap_or_else(|_| serde_json::json!({}));

                    let result = match self.tools.call(&tool_call.name, args).await {
                        Ok(r) => r.to_string(),
                        Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
                    };

                    debug!("工具结果: {}", if result.len() > 200 { &result[..200] } else { &result });

                    messages.push(Message {
                        role: MessageRole::Tool,
                        content: result,
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }

                continue;
            }

            return Ok(response.message.content);
        }

        Err(anyhow::anyhow!("超过最大工具调用轮数"))
    }

    /// 获取工具定义
    async fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.list_tools().await;
        tools
            .into_iter()
            .map(|t| ToolDefinition {
                name: t.name,
                description: t.description,
                parameters: t.parameters,
            })
            .collect()
    }

    /// 获取处理器统计信息
    pub fn stats(&self) -> ProcessorStats {
        ProcessorStats {
            memory_enabled: self.memory.is_some(),
            strategy_enabled: self.strategy.is_some(),
            llm_enabled: self.llm.is_some(),
            model: self.model.clone(),
            agent_id: self.agent_id.clone(),
            namespace: self.namespace.clone(),
        }
    }

    /// 动态调整策略
    pub async fn set_strategy(&mut self, strategy_type: StrategyType) -> Result<()> {
        self.config.default_strategy = strategy_type.clone();
        info!("策略已调整为: {:?}", strategy_type);
        Ok(())
    }

    /// 动态调整 Agent ID
    pub fn set_agent_id(&mut self, agent_id: &str) {
        self.agent_id = agent_id.to_string();
        info!("Agent ID 已设置为: {}", agent_id);
    }

    /// 动态调整命名空间
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = namespace.to_string();
        info!("命名空间已设置为: {}", namespace);
    }
}

/// 处理器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorStats {
    pub memory_enabled: bool,
    pub strategy_enabled: bool,
    pub llm_enabled: bool,
    pub model: String,
    pub agent_id: String,
    pub namespace: String,
}

/// 创建默认处理器
pub async fn create_default_processor(
    tools: Arc<ToolRegistry>,
    permissions: Arc<ChannelPermission>,
) -> Result<ChannelProcessor> {
    use crate::memory::{SQLiteMemoryStorage, StorageConfig};
    
    let config = ProcessorConfig::default();

    // 创建记忆存储
    let storage_config = StorageConfig::default();
    let memory = Arc::new(SQLiteMemoryStorage::new(storage_config)?);

    // 创建策略引擎
    let strategy = Arc::new(RwLock::new(StrategyEngine::new()?));

    Ok(ChannelProcessor::new(tools, permissions, config)
        .with_memory(memory)
        .with_strategy(strategy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_config_default() {
        let config = ProcessorConfig::default();
        assert!(config.enable_memory);
        assert!(config.enable_strategy);
    }

    #[test]
    fn test_build_scope() {
        let tools = Arc::new(ToolRegistry::new());
        let permissions = Arc::new(ChannelPermission::new("./data/test_permissions.json"));
        
        let config = ProcessorConfig {
            default_agent_id: "agent1".to_string(),
            default_namespace: "ns1".to_string(),
            ..Default::default()
        };
        
        let processor = ChannelProcessor::new(tools, permissions, config);
        let scope = processor.build_scope("user1", "feishu");
        
        assert_eq!(scope.user_id, "user1");
        assert_eq!(scope.channel, "feishu");
        assert_eq!(scope.agent_id, Some("agent1".to_string()));
        assert_eq!(scope.namespace, Some("ns1".to_string()));
    }
}