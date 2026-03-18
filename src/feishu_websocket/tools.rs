// 飞书通道工具适配层
//
// 此模块不定义独立的工具，而是适配统一的 ToolRegistry
// 所有工具定义在 src/tools/ 下，通过 init_builtin_tools 注册
// v0.7.0: 集成权限控制

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// 引入统一的工具系统
use crate::tools::{ToolRegistry, ToolMetadata, init_builtin_tools_with_permissions};
use crate::channel::ChannelPermission;
use std::path::PathBuf;

/// LLM 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// 工具名称
    pub name: String,
    /// 工具参数
    pub arguments: HashMap<String, String>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 工具名称
    pub tool_name: String,
    /// 是否成功
    pub success: bool,
    /// 执行结果
    pub output: String,
    /// 错误信息（如果失败）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 工具管理器
///
/// 包装统一的 ToolRegistry，为飞书通道提供工具调用能力
/// 所有工具通过 init_builtin_tools 注册，确保与 CLI 等其他通道一致
/// v0.7.0: 集成权限控制
pub struct ToolManager {
    /// 统一的工具注册表
    registry: Arc<ToolRegistry>,
    /// 权限管理器 (v0.7.0)
    permissions: Arc<ChannelPermission>,
    /// 工具元数据缓存
    tools_cache: Arc<RwLock<Vec<ToolMetadata>>>,
}

impl ToolManager {
    /// 创建新的工具管理器
    ///
    /// 初始化统一的 ToolRegistry 并注册所有内置工具
    pub async fn new() -> Self {
        let registry = Arc::new(ToolRegistry::new());
        let permissions = Arc::new(ChannelPermission::new("/var/lib/newclaw/permissions.json"));
        let tools_cache = Arc::new(RwLock::new(Vec::new()));
        
        let manager = Self {
            registry,
            permissions,
            tools_cache,
        };
        
        // 使用统一的 init_builtin_tools 初始化所有工具 (带权限管理)
        if let Err(e) = manager.init_tools().await {
            warn!("初始化工具失败: {}", e);
        }
        
        manager
    }

    /// 从现有的 ToolRegistry 和权限管理器创建
    ///
    /// 允许共享同一个 ToolRegistry 和权限管理器实例
    pub fn from_registry_and_permissions(
        registry: Arc<ToolRegistry>,
        permissions: Arc<ChannelPermission>,
    ) -> Self {
        let tools_cache = Arc::new(RwLock::new(Vec::new()));
        Self {
            registry,
            permissions,
            tools_cache,
        }
    }

    /// 初始化所有内置工具
    async fn init_tools(&self) -> Result<()> {
        let data_dir = PathBuf::from("/var/lib/newclaw");
        let workspace = PathBuf::from("/var/lib/newclaw/workspace");

        // 确保目录存在
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&workspace)?;
        
        // 调用统一的 init_builtin_tools_with_permissions
        init_builtin_tools_with_permissions(
            &self.registry,
            data_dir,
            workspace,
            Some(Arc::clone(&self.permissions)),
        ).await?;
        
        // 刷新缓存
        self.refresh_cache().await?;
        
        let count = self.tools_cache.read().await.len();
        info!("✅ 飞书通道工具初始化完成，共 {} 个工具", count);
        Ok(())
    }

    /// 刷新工具缓存
    pub async fn refresh_cache(&self) -> Result<()> {
        let tools = self.registry.list_tools().await;
        *self.tools_cache.write().await = tools;
        Ok(())
    }

    /// 获取所有工具定义
    ///
    /// 返回已注册的所有工具元数据，用于构建 LLM 系统提示词
    pub async fn get_all_tools(&self) -> Vec<ToolMetadata> {
        self.tools_cache.read().await.clone()
    }

    /// 获取工具定义
    pub async fn get_tool(&self, name: &str) -> Option<ToolMetadata> {
        self.registry.get_tool(name).await
    }

    /// 执行工具调用
    ///
    /// 将参数转换为 JSON 并调用统一的 ToolRegistry
    pub async fn execute_tool(&self, call: &ToolCallRequest) -> ToolResult {
        let tool_name = &call.name;
        
        info!("执行工具: {}，参数: {:?}", tool_name, call.arguments);

        // 转换参数格式（String -> Value）
        let args: serde_json::Value = call.arguments.iter()
            .map(|(k, v)| {
                // 尝试解析为 JSON，失败则作为字符串
                let value = serde_json::from_str(v).unwrap_or(serde_json::Value::String(v.clone()));
                (k.clone(), value)
            })
            .collect::<serde_json::Map<String, serde_json::Value>>()
            .into();

        // 调用统一的 ToolRegistry
        match self.registry.call(tool_name, args).await {
            Ok(result) => {
                let output = serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|_| result.to_string());
                ToolResult {
                    tool_name: tool_name.clone(),
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                tool_name: tool_name.clone(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        // 同步版本的 default，实际使用时应调用 new()
        let registry = Arc::new(ToolRegistry::new());
        let permissions = Arc::new(ChannelPermission::default());
        let tools_cache = Arc::new(RwLock::new(Vec::new()));
        Self {
            registry,
            permissions,
            tools_cache,
        }
    }
}

/// 构建工具的系统提示词
///
/// 根据已注册的工具生成 LLM 系统提示词
pub fn build_tools_system_prompt(tools: &[ToolMetadata]) -> String {
    let mut prompt = r#"你是一个智能助手，拥有完整的工具调用能力。

你有以下可用工具：

"#.to_string();

    // 按类别分组工具
    let mut categories: HashMap<&str, Vec<&ToolMetadata>> = HashMap::new();
    
    for tool in tools {
        let category = if tool.name.starts_with("feishu_") {
            "飞书工具"
        } else if tool.name.contains("memory") {
            "记忆工具"
        } else if tool.name == "read" || tool.name == "write" || tool.name == "edit" {
            "文件工具"
        } else if tool.name.starts_with("web_") || tool.name.contains("search") || tool.name.contains("fetch") {
            "Web工具"
        } else if tool.name == "exec" {
            "执行工具"
        } else if tool.name.starts_with("session") || tool.name.starts_with("subagent") {
            "会话/代理工具"
        } else if tool.name.starts_with("node") {
            "节点工具"
        } else if tool.name == "browser" {
            "浏览器工具"
        } else if tool.name == "canvas" {
            "Canvas工具"
        } else if tool.name == "tts" {
            "TTS工具"
        } else {
            "系统工具"
        };
        
        categories.entry(category).or_default().push(tool);
    }

    for (category, category_tools) in &categories {
        prompt.push_str(&format!("\n### {}\n", category));
        for tool in category_tools {
            prompt.push_str(&format!("- `{}`: {}\n", tool.name, tool.description));
        }
    }

    prompt.push_str(r#"

## 使用规则

1. **选择合适的工具**：根据用户需求选择最合适的工具
2. **工具调用格式**：使用 JSON 格式调用工具
3. **结果处理**：基于工具执行结果回答，不要编造信息
4. **错误处理**：如果工具执行失败，告诉用户原因

## 工具调用格式

```json
{
  "tool_calls": [
    {"name": "工具名称", "arguments": {"参数名": "参数值"}}
  ]
}
```

## 示例

用户：帮我查看服务器状态
```json
{
  "tool_calls": [
    {"name": "exec", "arguments": {"command": "systemctl status newclaw-*"}},
    {"name": "exec", "arguments": {"command": "df -h"}}
  ]
}
```

用户：读取记忆中的内容
```json
{
  "tool_calls": [
    {"name": "memory_search", "arguments": {"query": "关键词"}}
  ]
}
```

用户：帮我搜索飞书文档
```json
{
  "tool_calls": [
    {"name": "feishu_doc", "arguments": {"action": "search", "query": "搜索词"}}
  ]
}
```

用户：读取一个文件
```json
{
  "tool_calls": [
    {"name": "read", "arguments": {"path": "/path/to/file"}}
  ]
}
```

用户：搜索网络
```json
{
  "tool_calls": [
    {"name": "web_search", "arguments": {"query": "搜索关键词"}}
  ]
}
```
"#);

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_manager_creation() {
        let manager = ToolManager::new().await;
        let tools = manager.get_all_tools().await;
        // 应该有 15+ 个工具
        assert!(tools.len() >= 15, "Expected at least 15 tools, got {}", tools.len());
    }

    #[test]
    fn test_build_system_prompt() {
        let tools = vec![
            ToolMetadata {
                name: "read".to_string(),
                description: "读取文件".to_string(),
                parameters: serde_json::json!({}),
            },
            ToolMetadata {
                name: "feishu_doc".to_string(),
                description: "飞书文档".to_string(),
                parameters: serde_json::json!({}),
            },
        ];
        let prompt = build_tools_system_prompt(&tools);
        assert!(prompt.contains("read"));
        assert!(prompt.contains("feishu_doc"));
    }
}