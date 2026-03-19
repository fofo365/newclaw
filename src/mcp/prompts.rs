// MCP 提示词系统

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{McpError, McpResult};

/// 提示词元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// 提示词名称
    pub name: String,
    /// 提示词描述
    pub description: String,
    /// 参数定义
    pub arguments: Vec<PromptArgument>,
}

/// 提示词参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// 参数名称
    pub name: String,
    /// 参数描述
    pub description: String,
    /// 是否必需
    pub required: bool,
}

/// 提示词消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    /// 角色
    pub role: PromptRole,
    /// 内容
    pub content: PromptContent,
}

/// 提示词角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    /// 用户
    User,
    /// 助手
    Assistant,
    /// 系统
    System,
}

/// 提示词内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PromptContent {
    /// 文本内容
    Text(String),
    /// 多媒体内容
    Media {
        /// 类型
        r#type: String,
        /// 文本
        text: String,
        /// 数据（可选）
        data: Option<String>,
    },
}

/// 提示词模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 提示词名称
    pub name: String,
    /// 提示词消息列表
    pub messages: Vec<PromptMessage>,
}

/// 提示词注册表
pub struct PromptRegistry {
    /// 提示词列表
    prompts: Arc<RwLock<HashMap<String, PromptMetadata>>>,
}

impl PromptRegistry {
    /// 创建新的提示词注册表
    pub fn new() -> Self {
        Self {
            prompts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册提示词
    pub async fn register(&self, metadata: PromptMetadata) -> McpResult<()> {
        let mut prompts = self.prompts.write().await;
        prompts.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// 注销提示词
    pub async fn unregister(&self, name: &str) -> McpResult<()> {
        let mut prompts = self.prompts.write().await;
        prompts.remove(name)
            .ok_or_else(|| McpError::PromptNotFound(name.to_string()))?;
        Ok(())
    }

    /// 列出所有提示词
    pub async fn list_prompts(&self) -> Vec<PromptMetadata> {
        let prompts = self.prompts.read().await;
        prompts.values().cloned().collect()
    }

    /// 获取提示词元数据
    pub async fn get_prompt(&self, name: &str) -> McpResult<PromptMetadata> {
        let prompts = self.prompts.read().await;
        prompts.get(name)
            .cloned()
            .ok_or_else(|| McpError::PromptNotFound(name.to_string()))
    }

    /// 获取提示词模板
    pub async fn get_prompt_template(&self, name: &str, args: HashMap<String, String>) -> McpResult<PromptTemplate> {
        // 获取提示词元数据
        let metadata = self.get_prompt(name).await?;

        // 验证参数
        let args_json = serde_json::to_value(args.clone())?;
        Self::validate_prompt_arguments(&args_json, &metadata.arguments)?;

        // 生成提示词消息
        let messages = self.generate_prompt_messages(&metadata, &args)?;

        Ok(PromptTemplate {
            name: name.to_string(),
            messages,
        })
    }

    /// 生成提示词消息
    fn generate_prompt_messages(&self, metadata: &PromptMetadata, args: &HashMap<String, String>) -> McpResult<Vec<PromptMessage>> {
        match metadata.name.as_str() {
            "summarize" => {
                let text = args.get("text")
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'text' argument".to_string()))?;

                Ok(vec![
                    PromptMessage {
                        role: PromptRole::System,
                        content: PromptContent::Text(
                            "You are a helpful assistant that summarizes text concisely.".to_string()
                        ),
                    },
                    PromptMessage {
                        role: PromptRole::User,
                        content: PromptContent::Text(format!(
                            "Please summarize the following text:\n\n{}",
                            text
                        )),
                    },
                ])
            }

            "translate" => {
                let text = args.get("text")
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'text' argument".to_string()))?;
                let target_lang = args.get("target_language")
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'target_language' argument".to_string()))?;

                Ok(vec![
                    PromptMessage {
                        role: PromptRole::System,
                        content: PromptContent::Text(
                            format!("You are a professional translator. Translate text to {}.", target_lang)
                        ),
                    },
                    PromptMessage {
                        role: PromptRole::User,
                        content: PromptContent::Text(format!("Translate: {}", text)),
                    },
                ])
            }

            "code_review" => {
                let code = args.get("code")
                    .ok_or_else(|| McpError::InvalidArguments("Missing 'code' argument".to_string()))?;
                let language = args.get("language").cloned().unwrap_or_else(|| "unknown".to_string());

                Ok(vec![
                    PromptMessage {
                        role: PromptRole::System,
                        content: PromptContent::Text(
                            "You are an expert code reviewer. Analyze the code for bugs, performance issues, and best practices.".to_string()
                        ),
                    },
                    PromptMessage {
                        role: PromptRole::User,
                        content: PromptContent::Text(format!(
                            "Please review this {} code:\n\n```{}\n{}\n```",
                            language, language, code
                        )),
                    },
                ])
            }

            _ => {
                // 默认模板
                Ok(vec![
                    PromptMessage {
                        role: PromptRole::System,
                        content: PromptContent::Text(metadata.description.clone()),
                    },
                    PromptMessage {
                        role: PromptRole::User,
                        content: PromptContent::Text(
                            args.iter()
                                .map(|(k, v)| format!("{}: {}", k, v))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ),
                    },
                ])
            }
        }
    }

    /// 验证提示词参数
    fn validate_prompt_arguments(arguments: &serde_json::Value, arg_definitions: &[PromptArgument]) -> McpResult<()> {
        if !arguments.is_object() {
            return Err(McpError::InvalidArguments(
                "Arguments must be a JSON object".to_string()
            ));
        }

        let args_map = arguments.as_object().unwrap();

        // 检查必需参数
        for arg in arg_definitions {
            if arg.required && !args_map.contains_key(&arg.name) {
                return Err(McpError::InvalidArguments(
                    format!("Missing required argument: {}", arg.name)
                ));
            }
        }

        Ok(())
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_registry() {
        let registry = PromptRegistry::new();

        // 注册提示词
        let metadata = PromptMetadata {
            name: "summarize".to_string(),
            description: "Summarize text".to_string(),
            arguments: vec![PromptArgument {
                name: "text".to_string(),
                description: "Text to summarize".to_string(),
                required: true,
            }],
        };

        registry.register(metadata).await.unwrap();

        // 列出提示词
        let prompts = registry.list_prompts().await;
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "summarize");

        // 获取提示词
        let prompt = registry.get_prompt("summarize").await.unwrap();
        assert_eq!(prompt.name, "summarize");

        // 注销提示词
        registry.unregister("summarize").await.unwrap();
        let prompts = registry.list_prompts().await;
        assert_eq!(prompts.len(), 0);
    }

    #[tokio::test]
    async fn test_prompt_not_found() {
        let registry = PromptRegistry::new();
        let result = registry.get_prompt("non_existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prompt_template_summarize() {
        let registry = PromptRegistry::new();

        // 注册 summarize 提示词
        let metadata = PromptMetadata {
            name: "summarize".to_string(),
            description: "Summarize text".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "text".to_string(),
                    description: "Text to summarize".to_string(),
                    required: true,
                }
            ],
        };
        registry.register(metadata).await.unwrap();

        // 获取提示词模板
        let mut args = HashMap::new();
        args.insert("text".to_string(), "This is a long text that needs to be summarized.".to_string());

        let template = registry.get_prompt_template("summarize", args).await.unwrap();
        assert_eq!(template.name, "summarize");
        assert_eq!(template.messages.len(), 2);
        assert_eq!(template.messages[0].role, PromptRole::System);
        match &template.messages[1].content {
            PromptContent::Text(text) => assert!(text.contains("summarize")),
            _ => panic!("Expected Text content"),
        }
    }

    #[tokio::test]
    async fn test_prompt_template_translate() {
        let registry = PromptRegistry::new();

        // 注册 translate 提示词
        let metadata = PromptMetadata {
            name: "translate".to_string(),
            description: "Translate text".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "text".to_string(),
                    description: "Text to translate".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target_language".to_string(),
                    description: "Target language".to_string(),
                    required: true,
                },
            ],
        };
        registry.register(metadata).await.unwrap();

        // 获取提示词模板
        let mut args = HashMap::new();
        args.insert("text".to_string(), "Hello, world!".to_string());
        args.insert("target_language".to_string(), "Chinese".to_string());

        let template = registry.get_prompt_template("translate", args).await.unwrap();
        assert_eq!(template.messages.len(), 2);
        match &template.messages[1].content {
            PromptContent::Text(text) => assert!(text.contains("Translate")),
            _ => panic!("Expected Text content"),
        }
    }

    #[tokio::test]
    async fn test_prompt_template_missing_required_argument() {
        let registry = PromptRegistry::new();

        // 注册需要参数的提示词
        let metadata = PromptMetadata {
            name: "summarize".to_string(),
            description: "Summarize text".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "text".to_string(),
                    description: "Text to summarize".to_string(),
                    required: true,
                }
            ],
        };
        registry.register(metadata).await.unwrap();

        // 缺少必需参数
        let args = HashMap::new();
        let result = registry.get_prompt_template("summarize", args).await;
        assert!(result.is_err());
    }
}
