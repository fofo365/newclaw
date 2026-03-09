// NewClaw v0.4.0 - 模型数据
// 从 zeroclaw 提取的模型信息

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型 ID
    pub id: String,
    /// 显示名称
    pub display_name: String,
    /// 提供商
    pub provider: String,
    /// 上下文长度
    pub context_length: usize,
    /// 是否支持视觉
    pub supports_vision: bool,
    /// 是否支持函数调用
    pub supports_functions: bool,
    /// 是否支持流式输出
    pub supports_streaming: bool,
    /// 输入价格 (每 1M tokens, USD)
    pub input_price: f64,
    /// 输出价格 (每 1M tokens, USD)
    pub output_price: f64,
}

/// GLM 模型列表
pub const GLM_MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "glm-4".to_string(),
        display_name: "GLM-4".to_string(),
        provider: "glm".to_string(),
        context_length: 128_000,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 14.0,
        output_price: 14.0,
    },
    ModelInfo {
        id: "glm-4-flash".to_string(),
        display_name: "GLM-4-Flash".to_string(),
        provider: "glm".to_string(),
        context_length: 128_000,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.1,
        output_price: 0.1,
    },
    ModelInfo {
        id: "glm-4-air".to_string(),
        display_name: "GLM-4-Air".to_string(),
        provider: "glm".to_string(),
        context_length: 128_000,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 1.0,
        output_price: 1.0,
    },
    ModelInfo {
        id: "glm-4-airx".to_string(),
        display_name: "GLM-4-AirX".to_string(),
        provider: "glm".to_string(),
        context_length: 8_192,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 1.0,
        output_price: 1.0,
    },
    ModelInfo {
        id: "glm-4-long".to_string(),
        display_name: "GLM-4-Long".to_string(),
        provider: "glm".to_string(),
        context_length: 1_000_000,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 1.0,
        output_price: 1.0,
    },
    ModelInfo {
        id: "glm-4-plus".to_string(),
        display_name: "GLM-4-Plus".to_string(),
        provider: "glm".to_string(),
        context_length: 128_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 50.0,
        output_price: 50.0,
    },
    ModelInfo {
        id: "glm-4v".to_string(),
        display_name: "GLM-4V".to_string(),
        provider: "glm".to_string(),
        context_length: 8_192,
        supports_vision: true,
        supports_functions: false,
        supports_streaming: true,
        input_price: 50.0,
        output_price: 50.0,
    },
    ModelInfo {
        id: "glm-4.7".to_string(),
        display_name: "GLM-4.7".to_string(),
        provider: "z.ai".to_string(),
        context_length: 131_072,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.5,
        output_price: 0.5,
    },
    ModelInfo {
        id: "glm-5".to_string(),
        display_name: "GLM-5".to_string(),
        provider: "z.ai".to_string(),
        context_length: 131_072,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 1.0,
        output_price: 2.0,
    },
    ModelInfo {
        id: "glm-z1-air".to_string(),
        display_name: "GLM-Z1-Air".to_string(),
        provider: "glm".to_string(),
        context_length: 131_072,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.35,
        output_price: 0.35,
    },
    ModelInfo {
        id: "glm-z1-airx".to_string(),
        display_name: "GLM-Z1-AirX".to_string(),
        provider: "glm".to_string(),
        context_length: 8_192,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.35,
        output_price: 0.35,
    },
    ModelInfo {
        id: "glm-z1-flash".to_string(),
        display_name: "GLM-Z1-Flash".to_string(),
        provider: "glm".to_string(),
        context_length: 131_072,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.1,
        output_price: 0.1,
    },
];

/// OpenAI 模型列表
pub const OPENAI_MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "gpt-4o".to_string(),
        display_name: "GPT-4o".to_string(),
        provider: "openai".to_string(),
        context_length: 128_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 5.0,
        output_price: 15.0,
    },
    ModelInfo {
        id: "gpt-4o-mini".to_string(),
        display_name: "GPT-4o Mini".to_string(),
        provider: "openai".to_string(),
        context_length: 128_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.15,
        output_price: 0.6,
    },
    ModelInfo {
        id: "gpt-4-turbo".to_string(),
        display_name: "GPT-4 Turbo".to_string(),
        provider: "openai".to_string(),
        context_length: 128_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 10.0,
        output_price: 30.0,
    },
    ModelInfo {
        id: "gpt-4".to_string(),
        display_name: "GPT-4".to_string(),
        provider: "openai".to_string(),
        context_length: 8_192,
        supports_vision: false,
        supports_functions: true,
        supports_streaming: true,
        input_price: 30.0,
        output_price: 60.0,
    },
    ModelInfo {
        id: "o1".to_string(),
        display_name: "o1".to_string(),
        provider: "openai".to_string(),
        context_length: 200_000,
        supports_vision: true,
        supports_functions: false,
        supports_streaming: false,
        input_price: 15.0,
        output_price: 60.0,
    },
    ModelInfo {
        id: "o1-mini".to_string(),
        display_name: "o1 Mini".to_string(),
        provider: "openai".to_string(),
        context_length: 128_000,
        supports_vision: false,
        supports_functions: false,
        supports_streaming: false,
        input_price: 3.0,
        output_price: 12.0,
    },
];

/// Claude 模型列表
pub const CLAUDE_MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "claude-3-5-sonnet-20241022".to_string(),
        display_name: "Claude 3.5 Sonnet".to_string(),
        provider: "claude".to_string(),
        context_length: 200_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 3.0,
        output_price: 15.0,
    },
    ModelInfo {
        id: "claude-3-5-haiku-20241022".to_string(),
        display_name: "Claude 3.5 Haiku".to_string(),
        provider: "claude".to_string(),
        context_length: 200_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.8,
        output_price: 4.0,
    },
    ModelInfo {
        id: "claude-3-opus-20240229".to_string(),
        display_name: "Claude 3 Opus".to_string(),
        provider: "claude".to_string(),
        context_length: 200_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 15.0,
        output_price: 75.0,
    },
    ModelInfo {
        id: "claude-3-sonnet-20240229".to_string(),
        display_name: "Claude 3 Sonnet".to_string(),
        provider: "claude".to_string(),
        context_length: 200_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 3.0,
        output_price: 15.0,
    },
    ModelInfo {
        id: "claude-3-haiku-20240307".to_string(),
        display_name: "Claude 3 Haiku".to_string(),
        provider: "claude".to_string(),
        context_length: 200_000,
        supports_vision: true,
        supports_functions: true,
        supports_streaming: true,
        input_price: 0.25,
        output_price: 1.25,
    },
];

/// 获取所有模型
pub fn get_all_models() -> Vec<&'static ModelInfo> {
    GLM_MODELS.iter()
        .chain(OPENAI_MODELS.iter())
        .chain(CLAUDE_MODELS.iter())
        .collect()
}

/// 根据提供商获取模型
pub fn get_models_by_provider(provider: &str) -> Vec<&'static ModelInfo> {
    let provider_lower = provider.to_lowercase();
    
    if provider_lower.starts_with("glm") || provider_lower == "zhipu" || provider_lower.starts_with("zai") || provider_lower.starts_with("z.ai") {
        GLM_MODELS.to_vec()
    } else if provider_lower == "openai" {
        OPENAI_MODELS.to_vec()
    } else if provider_lower == "claude" || provider_lower == "anthropic" {
        CLAUDE_MODELS.to_vec()
    } else {
        vec![]
    }
}

/// 查找模型信息
pub fn find_model(model_id: &str) -> Option<&'static ModelInfo> {
    get_all_models().into_iter().find(|m| m.id == model_id)
}

/// 获取默认模型
pub fn get_default_model(provider: &str) -> &'static str {
    let provider_lower = provider.to_lowercase();
    
    if provider_lower.starts_with("glm") || provider_lower == "zhipu" {
        "glm-4"
    } else if provider_lower.starts_with("zai") || provider_lower.starts_with("z.ai") || provider_lower.starts_with("glmcode") {
        "glm-4.7"
    } else if provider_lower == "openai" {
        "gpt-4o-mini"
    } else if provider_lower == "claude" || provider_lower == "anthropic" {
        "claude-3-5-sonnet-20241022"
    } else {
        "glm-4"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_glm_models() {
        let models = get_models_by_provider("glm");
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "glm-4"));
    }
    
    #[test]
    fn test_get_zai_models() {
        let models = get_models_by_provider("z.ai");
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "glm-4.7"));
    }
    
    #[test]
    fn test_find_model() {
        let model = find_model("glm-4");
        assert!(model.is_some());
        assert_eq!(model.unwrap().display_name, "GLM-4");
    }
    
    #[test]
    fn test_default_model() {
        assert_eq!(get_default_model("glm"), "glm-4");
        assert_eq!(get_default_model("z.ai"), "glm-4.7");
        assert_eq!(get_default_model("openai"), "gpt-4o-mini");
        assert_eq!(get_default_model("claude"), "claude-3-5-sonnet-20241022");
    }
}
