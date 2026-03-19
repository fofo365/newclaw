// MCP 资源系统

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{McpError, McpResult};

/// 检测文件的 MIME 类型
fn detect_mime_type(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_lowercase().as_str() {
        "txt" => "text/plain".to_string(),
        "html" | "htm" => "text/html".to_string(),
        "css" => "text/css".to_string(),
        "js" => "application/javascript".to_string(),
        "json" => "application/json".to_string(),
        "xml" => "application/xml".to_string(),
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "gif" => "image/gif".to_string(),
        "svg" => "image/svg+xml".to_string(),
        "pdf" => "application/pdf".to_string(),
        "zip" => "application/zip".to_string(),
        "md" | "markdown" => "text/markdown".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

/// 资源元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    /// 资源 URI
    pub uri: String,
    /// 资源名称
    pub name: String,
    /// 资源描述
    pub description: String,
    /// MIME 类型
    pub mime_type: Option<String>,
}

/// 资源内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// 资源 URI
    pub uri: String,
    /// MIME 类型
    pub mime_type: Option<String>,
    /// 文本内容
    pub text: Option<String>,
    /// 二进制内容（Base64 编码）
    pub blob: Option<String>,
}

/// 资源注册表
pub struct ResourceRegistry {
    /// 资源列表
    resources: Arc<RwLock<HashMap<String, ResourceMetadata>>>,
}

impl ResourceRegistry {
    /// 创建新的资源注册表
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册资源
    pub async fn register(&self, metadata: ResourceMetadata) -> McpResult<()> {
        let mut resources = self.resources.write().await;
        resources.insert(metadata.uri.clone(), metadata);
        Ok(())
    }

    /// 注销资源
    pub async fn unregister(&self, uri: &str) -> McpResult<()> {
        let mut resources = self.resources.write().await;
        resources.remove(uri)
            .ok_or_else(|| McpError::ResourceNotFound(uri.to_string()))?;
        Ok(())
    }

    /// 列出所有资源
    pub async fn list_resources(&self) -> Vec<ResourceMetadata> {
        let resources = self.resources.read().await;
        resources.values().cloned().collect()
    }

    /// 获取资源元数据
    pub async fn get_resource(&self, uri: &str) -> McpResult<ResourceMetadata> {
        let resources = self.resources.read().await;
        resources.get(uri)
            .cloned()
            .ok_or_else(|| McpError::ResourceNotFound(uri.to_string()))
    }

    /// 读取资源内容
    pub async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        // 验证资源是否存在
        let metadata = self.get_resource(uri).await?;

        // 根据 URI scheme 选择读取策略
        let content = if uri.starts_with("file://") {
            self.read_file_resource(uri).await?
        } else if uri.starts_with("http://") || uri.starts_with("https://") {
            self.read_http_resource(uri).await?
        } else if uri.starts_with("data://") {
            self.read_data_resource(uri).await?
        } else {
            return Err(McpError::ResourceNotFound(format!(
                "Unsupported URI scheme: {}",
                uri
            )));
        };

        Ok(content)
    }

    /// 读取文件资源
    async fn read_file_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        // 提取文件路径
        let path = uri.trim_start_matches("file://");

        // 读取文件内容
        let content = tokio::fs::read_to_string(path).await?;

        // 检测 MIME 类型
        let mime_type = detect_mime_type(path);

        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some(mime_type),
            text: Some(content),
            blob: None,
        })
    }

    /// 读取 HTTP 资源
    async fn read_http_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        let client = reqwest::Client::new();

        let response = client.get(uri).send().await
            .map_err(|e| McpError::TransportError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(McpError::TransportError(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // 获取 Content-Type
        let mime_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // 尝试读取为文本
        let text = response.text().await
            .map_err(|e| McpError::TransportError(e.to_string()))?;

        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type,
            text: Some(text),
            blob: None,
        })
    }

    /// 读取数据资源（内存中的模拟数据）
    async fn read_data_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        // 解析 URI: data://resource_name
        let resource_name = uri.trim_start_matches("data://");

        match resource_name {
            "time" => {
                let now = chrono::Local::now();
                Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(now.format("%Y-%m-%d %H:%M:%S").to_string()),
                    blob: None,
                })
            }
            "uptime" => {
                // 模拟系统运行时间
                Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some("System uptime: 2 hours 30 minutes".to_string()),
                    blob: None,
                })
            }
            _ => Err(McpError::ResourceNotFound(format!(
                "Unknown data resource: {}",
                resource_name
            ))),
        }
    }

    /// 订阅资源更新（占位符）
    pub async fn subscribe_resource(&self, _uri: &str) -> McpResult<()> {
        // TODO: 实现资源订阅功能
        Ok(())
    }

    /// 取消订阅资源
    pub async fn unsubscribe_resource(&self, _uri: &str) -> McpResult<()> {
        // TODO: 实现取消订阅功能
        Ok(())
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_registry() {
        let registry = ResourceRegistry::new();

        // 注册资源
        let metadata = ResourceMetadata {
            uri: "file:///example.txt".to_string(),
            name: "Example File".to_string(),
            description: "An example resource".to_string(),
            mime_type: Some("text/plain".to_string()),
        };

        registry.register(metadata).await.unwrap();

        // 列出资源
        let resources = registry.list_resources().await;
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "file:///example.txt");

        // 获取资源
        let resource = registry.get_resource("file:///example.txt").await.unwrap();
        assert_eq!(resource.name, "Example File");

        // 注销资源
        registry.unregister("file:///example.txt").await.unwrap();
        let resources = registry.list_resources().await;
        assert_eq!(resources.len(), 0);
    }

    #[tokio::test]
    async fn test_resource_not_found() {
        let registry = ResourceRegistry::new();
        let result = registry.get_resource("file:///non_existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_data_resource() {
        let registry = ResourceRegistry::new();

        // 注册数据资源
        let metadata = ResourceMetadata {
            uri: "data://time".to_string(),
            name: "Current Time".to_string(),
            description: "Current system time".to_string(),
            mime_type: Some("text/plain".to_string()),
        };
        registry.register(metadata).await.unwrap();

        // 读取资源
        let content = registry.read_resource("data://time").await.unwrap();
        assert!(content.text.is_some());
        assert!(content.text.unwrap().contains("-"));
        assert_eq!(content.mime_type.unwrap(), "text/plain");
    }

    #[tokio::test]
    async fn test_read_file_resource() {
        let registry = ResourceRegistry::new();

        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_mcp_resource.txt");
        tokio::fs::write(&file_path, "Hello, MCP Resources!").await.unwrap();

        // 注册文件资源
        let uri = format!("file://{}", file_path.display());
        let metadata = ResourceMetadata {
            uri: uri.clone(),
            name: "Test File".to_string(),
            description: "A test file".to_string(),
            mime_type: None,  // 自动检测
        };
        registry.register(metadata).await.unwrap();

        // 读取资源
        let content = registry.read_resource(&uri).await.unwrap();
        assert_eq!(content.text.unwrap(), "Hello, MCP Resources!");
        assert_eq!(content.mime_type.unwrap(), "text/plain");

        // 清理
        tokio::fs::remove_file(&file_path).await.unwrap();
    }
}
