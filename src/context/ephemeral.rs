//! Layer 0: Ephemeral Context (瞬时层)
//!
//! 当前 LLM 调用上下文，严格 Token 预算控制，零持久化

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::llm::provider::{Message, MessageRole};

/// Token 预算配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// 最大 token 数
    pub max_tokens: usize,
    /// System Prompt 预算
    pub system_budget: usize,
    /// 最近对话轮数预算
    pub turns_budget: usize,
    /// 安全边界（保留给响应）
    pub safety_margin: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            system_budget: 512,
            turns_budget: 2048,
            safety_margin: 512,
        }
    }
}

impl TokenBudget {
    /// 创建新的 Token 预算
    pub fn new(max_tokens: usize) -> Self {
        let system_budget = (max_tokens as f64 * 0.125) as usize; // 12.5%
        let turns_budget = (max_tokens as f64 * 0.5) as usize;    // 50%
        let safety_margin = (max_tokens as f64 * 0.125) as usize; // 12.5%
        
        Self {
            max_tokens,
            system_budget,
            turns_budget,
            safety_margin,
        }
    }
    
    /// 获取可用预算
    pub fn available(&self) -> usize {
        self.max_tokens.saturating_sub(self.safety_margin)
    }
}

/// 自适应分配器
#[derive(Debug, Clone)]
pub struct AdaptiveAllocator {
    /// System Prompt 实际使用
    system_used: usize,
    /// 最近对话实际使用
    turns_used: usize,
    /// 动态调整因子
    adjustment_factor: f32,
}

impl Default for AdaptiveAllocator {
    fn default() -> Self {
        Self {
            system_used: 0,
            turns_used: 0,
            adjustment_factor: 1.0,
        }
    }
}

impl AdaptiveAllocator {
    /// 创建新的分配器
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 计算分配
    pub fn allocate(&mut self, budget: &TokenBudget) -> Allocation {
        let available = budget.available();
        
        // 动态调整：如果 system 用得少，给 turns 更多
        let system_alloc = if self.system_used < budget.system_budget / 2 {
            (budget.system_budget as f32 * 0.7) as usize
        } else {
            budget.system_budget
        };
        
        let turns_alloc = available.saturating_sub(system_alloc);
        
        Allocation {
            system_budget: system_alloc,
            turns_budget: turns_alloc,
        }
    }
    
    /// 更新使用情况
    pub fn update_usage(&mut self, system_used: usize, turns_used: usize) {
        self.system_used = system_used;
        self.turns_used = turns_used;
        
        // 动态调整因子
        let usage_ratio = turns_used as f32 / self.turns_used.max(1) as f32;
        self.adjustment_factor = if usage_ratio > 0.9 {
            1.1 // 需要更多空间
        } else if usage_ratio < 0.5 {
            0.9 // 可以给更多
        } else {
            1.0
        };
    }
}

/// 分配结果
#[derive(Debug, Clone)]
pub struct Allocation {
    pub system_budget: usize,
    pub turns_budget: usize,
}

/// 瞬时上下文
#[derive(Debug, Clone)]
pub struct EphemeralContext {
    /// System Prompt
    system_prompt: Option<String>,
    /// 最近 N 轮对话
    recent_turns: VecDeque<Message>,
    /// Token 预算
    token_budget: TokenBudget,
    /// 自适应分配器
    allocator: AdaptiveAllocator,
    /// 最大保留轮数
    max_turns: usize,
    /// 当前 token 计数
    current_tokens: usize,
}

impl Default for EphemeralContext {
    fn default() -> Self {
        Self::new(TokenBudget::default())
    }
}

impl EphemeralContext {
    /// 创建新的瞬时上下文
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            system_prompt: None,
            recent_turns: VecDeque::with_capacity(10),
            token_budget: budget,
            allocator: AdaptiveAllocator::new(),
            max_turns: 3, // 默认保留最近 3 轮
            current_tokens: 0,
        }
    }
    
    /// 设置 System Prompt
    pub fn set_system_prompt(&mut self, prompt: String) -> Result<()> {
        let tokens = Self::estimate_tokens(&prompt);
        
        if tokens > self.token_budget.system_budget {
            anyhow::bail!(
                "System prompt too long: {} > {}",
                tokens,
                self.token_budget.system_budget
            );
        }
        
        // 更新 token 计数
        if let Some(ref old) = self.system_prompt {
            self.current_tokens -= Self::estimate_tokens(old);
        }
        self.current_tokens += tokens;
        
        self.system_prompt = Some(prompt);
        Ok(())
    }
    
    /// 添加消息
    pub fn push(&mut self, msg: Message) -> Result<()> {
        let tokens = Self::estimate_tokens(&msg.content);
        
        // 检查预算
        let allocation = self.allocator.allocate(&self.token_budget);
        if self.current_tokens + tokens > allocation.turns_budget {
            // 需要淘汰旧消息
            self.evict_oldest(tokens)?;
        }
        
        // 添加消息
        self.current_tokens += tokens;
        self.recent_turns.push_back(msg);
        
        // 限制轮数
        while self.recent_turns.len() > self.max_turns {
            if let Some(old) = self.recent_turns.pop_front() {
                self.current_tokens -= Self::estimate_tokens(&old.content);
            }
        }
        
        Ok(())
    }
    
    /// 淘汰最旧的消息
    fn evict_oldest(&mut self, needed: usize) -> Result<()> {
        while self.current_tokens + needed > self.token_budget.turns_budget 
            && !self.recent_turns.is_empty() 
        {
            if let Some(old) = self.recent_turns.pop_front() {
                self.current_tokens -= Self::estimate_tokens(&old.content);
            }
        }
        
        if self.current_tokens + needed > self.token_budget.turns_budget {
            anyhow::bail!("Cannot fit message even after eviction");
        }
        
        Ok(())
    }
    
    /// 获取当前上下文（序列化为 LLM 输入）
    pub fn to_llm_context(&self) -> Vec<Message> {
        let mut messages = Vec::new();
        
        // System Prompt
        if let Some(ref prompt) = self.system_prompt {
            messages.push(Message {
                role: MessageRole::System,
                content: prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        
        // 最近对话
        messages.extend(self.recent_turns.iter().cloned());
        
        messages
    }
    
    /// 清空（调用结束时）
    pub fn clear(&mut self) {
        self.recent_turns.clear();
        self.current_tokens = 0;
        // System Prompt 通常保留
    }
    
    /// 完全重置（包括 System Prompt）
    pub fn reset(&mut self) {
        self.system_prompt = None;
        self.recent_turns.clear();
        self.current_tokens = 0;
    }
    
    /// 获取当前 token 计数
    pub fn token_count(&self) -> usize {
        self.current_tokens
    }
    
    /// 获取剩余预算
    pub fn remaining_budget(&self) -> usize {
        self.token_budget.available().saturating_sub(self.current_tokens)
    }
    
    /// 设置最大保留轮数
    pub fn set_max_turns(&mut self, max: usize) {
        self.max_turns = max;
        
        // 如果当前轮数超过新限制，淘汰多余的
        while self.recent_turns.len() > self.max_turns {
            if let Some(old) = self.recent_turns.pop_front() {
                self.current_tokens -= Self::estimate_tokens(&old.content);
            }
        }
    }
    
    /// 估算 token 数量（简化实现：每 4 字符 ≈ 1 token）
    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4 + 1
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> EphemeralStats {
        EphemeralStats {
            system_prompt_tokens: self.system_prompt.as_ref()
                .map(|s| Self::estimate_tokens(s))
                .unwrap_or(0),
            turns_count: self.recent_turns.len(),
            turns_tokens: self.current_tokens,
            budget_used_percent: (self.current_tokens as f32 / self.token_budget.max_tokens as f32 * 100.0) as u8,
        }
    }
}

/// 瞬时上下文统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralStats {
    pub system_prompt_tokens: usize,
    pub turns_count: usize,
    pub turns_tokens: usize,
    pub budget_used_percent: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_budget_default() {
        let budget = TokenBudget::default();
        assert_eq!(budget.max_tokens, 4096);
        assert!(budget.system_budget > 0);
        assert!(budget.turns_budget > 0);
    }
    
    #[test]
    fn test_token_budget_new() {
        let budget = TokenBudget::new(8192);
        assert_eq!(budget.max_tokens, 8192);
        assert!(budget.available() < 8192);
    }
    
    #[test]
    fn test_ephemeral_context_new() {
        let ctx = EphemeralContext::default();
        assert!(ctx.system_prompt.is_none());
        assert!(ctx.recent_turns.is_empty());
        assert_eq!(ctx.token_count(), 0);
    }
    
    #[test]
    fn test_set_system_prompt() {
        let mut ctx = EphemeralContext::default();
        ctx.set_system_prompt("You are a helpful assistant.".to_string()).unwrap();
        
        assert!(ctx.system_prompt.is_some());
        assert!(ctx.token_count() > 0);
    }
    
    #[test]
    fn test_push_message() {
        let mut ctx = EphemeralContext::default();
        
        ctx.push(Message {
            role: MessageRole::User,
            content: "Hello!".to_string(),
            
            tool_calls: None,
            tool_call_id: None,
        }).unwrap();
        
        assert_eq!(ctx.recent_turns.len(), 1);
        assert!(ctx.token_count() > 0);
    }
    
    #[test]
    fn test_max_turns_limit() {
        let mut ctx = EphemeralContext::default();
        ctx.set_max_turns(2);
        
        for i in 0..5 {
            ctx.push(Message {
                role: MessageRole::User,
                content: format!("Message {}", i),
                
                tool_calls: None,
                tool_call_id: None,
            }).unwrap();
        }
        
        assert_eq!(ctx.recent_turns.len(), 2);
    }
    
    #[test]
    fn test_to_llm_context() {
        let mut ctx = EphemeralContext::default();
        ctx.set_system_prompt("System prompt".to_string()).unwrap();
        ctx.push(Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            
            tool_calls: None,
            tool_call_id: None,
        }).unwrap();
        
        let messages = ctx.to_llm_context();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::User);
    }
    
    #[test]
    fn test_clear() {
        let mut ctx = EphemeralContext::default();
        ctx.set_system_prompt("System".to_string()).unwrap();
        ctx.push(Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            
            tool_calls: None,
            tool_call_id: None,
        }).unwrap();
        
        ctx.clear();
        
        assert!(ctx.system_prompt.is_some()); // 保留
        assert!(ctx.recent_turns.is_empty());
        assert_eq!(ctx.token_count(), 0);
    }
    
    #[test]
    fn test_reset() {
        let mut ctx = EphemeralContext::default();
        ctx.set_system_prompt("System".to_string()).unwrap();
        ctx.push(Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            
            tool_calls: None,
            tool_call_id: None,
        }).unwrap();
        
        ctx.reset();
        
        assert!(ctx.system_prompt.is_none()); // 清空
        assert!(ctx.recent_turns.is_empty());
        assert_eq!(ctx.token_count(), 0);
    }
    
    #[test]
    fn test_stats() {
        let mut ctx = EphemeralContext::default();
        ctx.set_system_prompt("System prompt here".to_string()).unwrap();
        ctx.push(Message {
            role: MessageRole::User,
            content: "Test message content".to_string(),
            
            tool_calls: None,
            tool_call_id: None,
        }).unwrap();
        
        let stats = ctx.stats();
        assert!(stats.system_prompt_tokens > 0);
        assert_eq!(stats.turns_count, 1);
        assert!(stats.turns_tokens > 0);
    }
}