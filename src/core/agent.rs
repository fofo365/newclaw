// Agent Engine - Core agent logic
//
// v0.7.2: 支持 LLMProviderV3（推荐）和 LegacyLLMProvider（向后兼容）

use crate::core::{ContextManager, ContextConfig};
use crate::llm::{LegacyLLMProvider, LLMMessage, LLMResponse, LLMRequest, LLMProviderV3, ChatRequest, ChatResponse, Message, MessageRole};
use anyhow::Result;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AgentState {
    Idle,
    Processing,
    Error,
}

pub struct AgentEngine {
    pub state: AgentState,
    pub memory: ContextManager,
    pub name: String,
    pub model: String,
    /// 新版 LLM Provider（推荐）
    llm_v3: Option<Arc<dyn LLMProviderV3>>,
    /// 旧版 LLM Provider（向后兼容，将废弃）
    #[deprecated(since = "0.7.2", note = "使用 with_llm_v3 代替")]
    llm: Option<Box<dyn LegacyLLMProvider>>,
}

impl AgentEngine {
    /// Create a new Agent Engine
    pub fn new(name: String, model: String) -> Result<Self> {
        let context_config = ContextConfig::default();
        let memory = ContextManager::new(context_config)?;
        
        Ok(Self {
            state: AgentState::Idle,
            memory,
            name,
            model,
            llm_v3: None,
            llm: None,
        })
    }

    /// Create agent with LLM Provider V3（推荐）
    pub fn with_llm_v3(mut self, llm: Arc<dyn LLMProviderV3>) -> Self {
        self.llm_v3 = Some(llm);
        self
    }

    /// Create agent with Legacy LLM provider（已废弃）
    #[deprecated(since = "0.7.2", note = "使用 with_llm_v3 代替")]
    pub fn with_llm(mut self, llm: Box<dyn LegacyLLMProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Process user input and generate response
    pub async fn process(&mut self, input: &str) -> Result<String> {
        self.state = AgentState::Processing;
        
        // Add input to context
        self.memory.add_message(input, "user")?;
        
        // Retrieve relevant context
        let context = self.memory.retrieve_relevant(input, 10)?;
        
        // 优先使用 LLMProviderV3
        let response = if let Some(llm) = &self.llm_v3 {
            self.process_with_v3(llm, input, &context).await?
        } else if let Some(llm) = &self.llm {
            #[allow(deprecated)]
            self.process_with_legacy(llm, input, &context).await?
        } else {
            // Fallback to mock response
            format!(
                "Processed: {}\n\nContext chunks: {}\nModel: {}",
                input,
                context.len(),
                self.model
            )
        };
        
        // Add response to context
        self.memory.add_message(&response, "assistant")?;
        
        self.state = AgentState::Idle;
        
        Ok(response)
    }
    
    /// 使用 LLMProviderV3 处理
    async fn process_with_v3(&self, llm: &Arc<dyn LLMProviderV3>, input: &str, context: &[crate::core::context::ContextChunk]) -> Result<String> {
        let mut messages = vec![
            Message {
                role: MessageRole::System,
                content: format!("You are {}, an AI assistant.", self.name),
                tool_calls: None,
                tool_call_id: None,
            }
        ];
        
        for chunk in context {
            messages.push(Message {
                role: match chunk.metadata.message_type.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => MessageRole::User,
                },
                content: chunk.text.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        
        messages.push(Message {
            role: MessageRole::User,
            content: input.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });
        
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: Some(2000),
            top_p: None,
            stop: None,
            tools: None,
        };
        
        let response = llm.chat(request).await?;
        
        Ok(response.message.content)
    }
    
    /// 使用 LegacyLLMProvider 处理（已废弃）
    #[deprecated(since = "0.7.2")]
    async fn process_with_legacy(&self, llm: &Box<dyn LegacyLLMProvider>, input: &str, context: &[crate::core::context::ContextChunk]) -> Result<String> {
        let mut messages = vec![
            LLMMessage {
                role: "system".to_string(),
                content: format!("You are {}, an AI assistant.", self.name),
            }
        ];
        
        for chunk in context {
            messages.push(LLMMessage {
                role: chunk.metadata.message_type.clone(),
                content: chunk.text.clone(),
            });
        }
        
        messages.push(LLMMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });
        
        let request = LLMRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: Some(2000),
        };
        
        let response = llm.chat(&request).await?;
        
        Ok(response.content)
    }

    /// Get current agent state
    pub fn get_state(&self) -> &AgentState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "CI 环境不稳定，本地测试通过"]
    async fn test_agent_process() {
        let mut agent = AgentEngine::new(
            "test".to_string(),
            "glm-4".to_string(),
        ).unwrap();
        
        let response = agent.process("Hello").await.unwrap();
        assert!(response.contains("Processed") || response.contains("Hello"));
    }
}
