// Token Counter Module - v0.5.0
//
// 功能：
// - 实时 token 计算
// - 预估模型输出
// - 成本估算

use std::collections::HashMap;
use tiktoken_rs::get_bpe_from_model;

/// Token 计数器
#[derive(Debug)]
pub struct TokenCounter {
    /// 缓存的编码器
    encoders: HashMap<String, tiktoken_rs::CoreBPE>,
    /// 默认编码器（用于未知模型）
    default_encoder: tiktoken_rs::CoreBPE,
}

impl TokenCounter {
    /// 创建新的 Token 计数器
    pub fn new() -> anyhow::Result<Self> {
        // 使用 cl100k_base 作为默认编码器（GPT-4/GPT-3.5）
        let default_encoder = tiktoken_rs::get_bpe_from_model("gpt-4")?;

        Ok(Self {
            encoders: HashMap::new(),
            default_encoder,
        })
    }

    /// 获取模型的编码器
    fn get_encoder(&mut self, model: &str) -> anyhow::Result<&tiktoken_rs::CoreBPE> {
        if !self.encoders.contains_key(model) {
            let encoder = self.resolve_encoder(model)?;
            self.encoders.insert(model.to_string(), encoder);
        }

        Ok(self.encoders.get(model).unwrap())
    }

    /// 解析模型对应的编码器
    fn resolve_encoder(&self, model: &str) -> anyhow::Result<tiktoken_rs::CoreBPE> {
        // GPT-4 系列
        if model.starts_with("gpt-4") {
            return tiktoken_rs::get_bpe_from_model("gpt-4");
        }

        // GPT-3.5 系列
        if model.starts_with("gpt-3.5") {
            return tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo");
        }

        // Claude 系列（近似使用 cl100k_base）
        if model.starts_with("claude") {
            return tiktoken_rs::get_bpe_from_model("gpt-4");
        }

        // GLM 系列（近似使用 cl100k_base）
        if model.starts_with("glm") {
            return tiktoken_rs::get_bpe_from_model("gpt-4");
        }

        // 默认使用 cl100k_base
        tiktoken_rs::get_bpe_from_model("gpt-4")
    }

    /// 计算文本的 token 数量
    pub fn count_tokens(&mut self, text: &str, model: &str) -> anyhow::Result<usize> {
        let encoder = self.get_encoder(model)?;
        let tokens = encoder.encode_ordinary(text);
        Ok(tokens.len())
    }

    /// 计算消息列表的 token 数量
    pub fn count_messages_tokens(
        &mut self,
        messages: &Vec<crate::llm::Message>,
        model: &str,
    ) -> anyhow::Result<usize> {
        let mut total = 0;

        for msg in messages {
            // 消息格式：<|role|>\n<|content|>\n
            let formatted = format!("{}\n{}\n", msg.role.as_str(), msg.content);
            total += self.count_tokens(&formatted, model)?;
        }

        Ok(total)
    }

    /// 估算模型的输出 token 数量
    ///
    /// 基于启发式规则：
    /// - 输入 token 越多，输出越多
    /// - 通常输出是输入的 20-50%
    pub fn estimate_output_tokens(
        &mut self,
        input_tokens: usize,
        model: &str,
    ) -> anyhow::Result<usize> {
        // 获取模型的上下文窗口限制
        let context_limit = self.get_context_limit(model);

        // 剩余可用 token
        let remaining = context_limit.saturating_sub(input_tokens);

        // 估算输出：通常使用剩余的 20-50%
        let estimated = (remaining as f32 * 0.35) as usize;

        Ok(estimated.max(100).min(remaining))
    }

    /// 获取模型的上下文窗口限制
    fn get_context_limit(&self, model: &str) -> usize {
        // GPT-4 系列
        if model.contains("gpt-4") {
            if model.contains("32k") {
                return 32768;
            } else if model.contains("16k") {
                return 16384;
            }
            return 8192;
        }

        // GPT-3.5 系列
        if model.contains("gpt-3.5") {
            if model.contains("16k") {
                return 16384;
            }
            return 4096;
        }

        // Claude 系列
        if model.starts_with("claude") {
            if model.contains("100k") {
                return 100000;
            }
            return 9000;
        }

        // GLM 系列
        if model.starts_with("glm") {
            if model.contains("long") {
                return 32768;
            }
            return 8192;
        }

        // 默认
        4096
    }

    /// 计算成本（美元）
    ///
    /// 基于各提供商的定价（2025 年）
    pub fn calculate_cost(
        &mut self,
        input_tokens: usize,
        output_tokens: usize,
        model: &str,
    ) -> anyhow::Result<f64> {
        let (input_price, output_price) = self.get_pricing(model)?;

        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

        Ok(input_cost + output_cost)
    }

    /// 获取模型的定价（每百万 token 的美元价格）
    fn get_pricing(&self, model: &str) -> anyhow::Result<(f64, f64)> {
        // GPT-4
        if model.starts_with("gpt-4") && !model.starts_with("gpt-4-turbo") {
            return Ok((30.0, 60.0)); // $30/60 per million
        }

        // GPT-4 Turbo
        if model.starts_with("gpt-4-turbo") {
            return Ok((10.0, 30.0));
        }

        // GPT-3.5 Turbo
        if model.starts_with("gpt-3.5") {
            return Ok((0.5, 1.5));
        }

        // Claude 3 Opus
        if model.contains("claude-3-opus") {
            return Ok((15.0, 75.0));
        }

        // Claude 3 Sonnet
        if model.contains("claude-3-sonnet") {
            return Ok((3.0, 15.0));
        }

        // Claude 3 Haiku
        if model.contains("claude-3-haiku") {
            return Ok((0.25, 1.25));
        }

        // GLM-4
        if model.starts_with("glm-4") {
            return Ok((0.5, 0.5)); // 估算价格
        }

        // 默认（GPT-3.5 Turbo 价格）
        Ok((0.5, 1.5))
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new().expect("Failed to create TokenCounter")
    }
}

/// Token 使用统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsageStats {
    /// 输入 token 数量
    pub input_tokens: usize,
    /// 输出 token 数量
    pub output_tokens: usize,
    /// 总 token 数量
    pub total_tokens: usize,
    /// 估算成本（美元）
    pub estimated_cost: f64,
    /// 模型名称
    pub model: String,
}

impl TokenUsageStats {
    /// 创建新的统计
    pub fn new(
        input_tokens: usize,
        output_tokens: usize,
        model: &str,
        counter: &mut TokenCounter,
    ) -> anyhow::Result<Self> {
        let total_tokens = input_tokens + output_tokens;
        let estimated_cost = counter.calculate_cost(input_tokens, output_tokens, model)?;

        Ok(Self {
            input_tokens,
            output_tokens,
            total_tokens,
            estimated_cost,
            model: model.to_string(),
        })
    }

    /// 格式化为可读字符串
    pub fn format(&self) -> String {
        format!(
            "Token Usage: {} in + {} out = {} total (${:.4})",
            self.input_tokens,
            self.output_tokens,
            self.total_tokens,
            self.estimated_cost
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens() {
        let mut counter = TokenCounter::new().unwrap();

        let text = "Hello, world!";
        let count = counter.count_tokens(text, "gpt-4").unwrap();

        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_estimate_output() {
        let mut counter = TokenCounter::new().unwrap();

        let input = 1000;
        let estimated = counter.estimate_output_tokens(input, "gpt-4").unwrap();

        assert!(estimated > 0);
        assert!(estimated < 10000);
    }

    #[test]
    fn test_calculate_cost() {
        let mut counter = TokenCounter::new().unwrap();

        let cost = counter.calculate_cost(1000, 500, "gpt-3.5-turbo").unwrap();

        assert!(cost > 0.0);
        assert!(cost < 1.0);
    }
}
