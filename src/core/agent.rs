// Agent Engine - Core agent logic

use crate::core::{ContextManager, ContextConfig};
use crate::llm::{LLMProvider, LLMMessage, LLMResponse, LLMRequest};
use anyhow::Result;

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
    llm: Option<Box<dyn LLMProvider>>,
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
            llm: None,
        })
    }

    /// Create agent with LLM provider
    pub fn with_llm(mut self, llm: Box<dyn LLMProvider>) -> Self {
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
        
        // Build messages for LLM
        let mut messages = vec![
            LLMMessage {
                role: "system".to_string(),
                content: format!("You are {}, an AI assistant.", self.name),
            }
        ];
        
        for chunk in &context {
            messages.push(LLMMessage {
                role: chunk.metadata.message_type.clone(),
                content: chunk.text.clone(),
            });
        }
        
        messages.push(LLMMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });
        
        // Call LLM
        let response = if let Some(llm) = &self.llm {
            let request = LLMRequest {
                model: self.model.clone(),
                messages,
                temperature: 0.7,
                max_tokens: Some(2000),
            };
            
            llm.chat(&request).await?
        } else {
            // Fallback to mock response
            LLMResponse {
                content: format!(
                    "Processed: {}\n\nContext chunks: {}\nModel: {}",
                    input,
                    context.len(),
                    self.model
                ),
                tokens_used: 100,
                model: self.model.clone(),
            }
        };
        
        // Add response to context
        self.memory.add_message(&response.content, "assistant")?;
        
        self.state = AgentState::Idle;
        
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
    async fn test_agent_process() {
        let mut agent = AgentEngine::new(
            "test".to_string(),
            "glm-4".to_string(),
        ).unwrap();
        
        let response = agent.process("Hello").await.unwrap();
        assert!(response.contains("Processed") || response.contains("Hello"));
    }
}
