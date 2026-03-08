// Agent Engine - Core agent logic

use crate::core::{ContextManager, ContextConfig};
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
        })
    }

    /// Process user input and generate response
    pub async fn process(&mut self, input: &str) -> Result<String> {
        self.state = AgentState::Processing;
        
        // Add input to context
        self.memory.add_message(input, "user")?;
        
        // Retrieve relevant context
        let context = self.memory.retrieve_relevant(input, 10)?;
        
        // TODO: Implement LLM call
        // For now, return a simple response
        let response = format!(
            "Processed: {}\nContext chunks: {}\nModel: {}",
            input,
            context.len(),
            self.model
        );
        
        // Add response to context
        self.memory.add_message(&response, "assistant")?;
        
        self.state = AgentState::Idle;
        
        Ok(response)
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
        assert!(response.contains("Processed"));
    }
}
