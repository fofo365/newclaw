// Agent Loop

use crate::agent::AgentEngine;

pub struct AgentEngine {
    pub state: AgentState,
    pub tools: ToolRegistry,
    pub memory: ContextManager,
    pub llm: LLMEngine,
}

pub enum AgentState {
    Idle,
    Processing,
    AwaitingResponse,
}

pub struct AgentEngine {
    pub async fn process(&mut self, input: &str) -> Result<String> {
        self.state = AgentState::Processing;
        
        // 分析输入
        let analysis = self.analyze_input(input)?;
        
        // 选择策略（策略引擎）
        let selected_context = self
            .strategy
            .select_context(
                self.context.memory.get_recent(),
                self.max_tokens,
            )?;
        
        // 构建 prompt
        let prompt = format!(
            "{}\n\nUser: {}",
            selected_context.join("\n"),
            input
        );
        
        // 调用 LLM
        let response = self.llm.complete(&prompt).await?;
        
        // 更新上下文
        self.context.add_message(&response)?;
        
        self.state = AgentState::Idle;
        Ok(response)
    }
}
