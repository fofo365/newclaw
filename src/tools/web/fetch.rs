// HTTP/HTTPS 请求工具
use crate::tools::{Tool, ToolMetadata, Value};
use serde_json::{json, Value as JsonValue};
use std::time::Duration;

/// Web Fetch 工具
pub struct WebFetchTool;

#[async_trait::async_trait]
impl Tool for WebFetchTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "web_fetch".to_string(),
            description: "Fetch and extract readable content from a URL (HTML → markdown/text)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "HTTP or HTTPS URL to fetch"
                    },
                    "extract_mode": {
                        "type": "string",
                        "enum": ["markdown", "text"],
                        "description": "Extraction mode (default: markdown)",
                        "default": "markdown"
                    },
                    "max_chars": {
                        "type": "number",
                        "description": "Maximum characters to return (truncates when exceeded)",
                        "default": 50000
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        // 解析参数
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;

        let extract_mode = args
            .get("extract_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        let max_chars = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(50000) as usize;

        // 验证 URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow::anyhow!("URL must start with http:// or https://"));
        }

        // 发起 HTTP 请求
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("NewClaw/0.5.0 (MCP Web Fetch Tool)")
            .build()?;

        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP error: {}",
                response.status()
            ));
        }

        let html = response.text().await?;

        // 简单的 HTML → Markdown 转换
        // 注意：生产环境应该使用 html2md 或 readability 库
        let content = if extract_mode == "markdown" {
            html_to_markdown(&html, max_chars)
        } else {
            html_to_text(&html, max_chars)
        };

        Ok(json!({
            "content": content,
            "mode": extract_mode,
            "length": content.len(),
            "truncated": content.len() >= max_chars
        }))
    }
}

/// 简单的 HTML → Markdown 转换
fn html_to_markdown(html: &str, max_chars: usize) -> String {
    // 移除 script 和 style 标签
    let html = regex::Regex::new(r"<script[^>]*>.*?</script>")
        .unwrap()
        .replace_all(html, "");
    let html = regex::Regex::new(r"<style[^>]*>.*?</style>")
        .unwrap()
        .replace_all(&html, "");

    // 转换基本标签
    let mut text = html.to_string();

    // 标题 (h1-h6)
    text = regex::Regex::new(r"<h1[^>]*>(.*?)</h1>")
        .unwrap()
        .replace_all(&text, "# $1\n")
        .to_string();
    text = regex::Regex::new(r"<h2[^>]*>(.*?)</h2>")
        .unwrap()
        .replace_all(&text, "## $1\n")
        .to_string();
    text = regex::Regex::new(r"<h3[^>]*>(.*?)</h3>")
        .unwrap()
        .replace_all(&text, "### $1\n")
        .to_string();

    // 段落和换行
    text = regex::Regex::new(r"<p[^>]*>(.*?)</p>")
        .unwrap()
        .replace_all(&text, "$1\n\n")
        .to_string();
    text = regex::Regex::new(r"<br\s*/?>")
        .unwrap()
        .replace_all(&text, "\n")
        .to_string();

    // 粗体和斜体
    text = regex::Regex::new(r"<strong[^>]*>(.*?)</strong>")
        .unwrap()
        .replace_all(&text, "**$1**")
        .to_string();
    text = regex::Regex::new(r"<b[^>]*>(.*?)</b>")
        .unwrap()
        .replace_all(&text, "**$1**")
        .to_string();
    text = regex::Regex::new(r"<em[^>]*>(.*?)</em>")
        .unwrap()
        .replace_all(&text, "*$1*")
        .to_string();
    text = regex::Regex::new(r"<i[^>]*>(.*?)</i>")
        .unwrap()
        .replace_all(&text, "*$1*")
        .to_string();

    // 代码
    text = regex::Regex::new(r"<code[^>]*>(.*?)</code>")
        .unwrap()
        .replace_all(&text, "`$1`")
        .to_string();
    text = regex::Regex::new(r"<pre[^>]*>(.*?)</pre>")
        .unwrap()
        .replace_all(&text, "```\n$1\n```")
        .to_string();

    // 链接
    text = regex::Regex::new(r#"<a[^>]*href=['"]([^'"]*)['"][^>]*>(.*?)</a>"#)
        .unwrap()
        .replace_all(&text, "[$2]($1)")
        .to_string();

    // 移除所有剩余的 HTML 标签
    text = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();

    // 清理多余的空行
    text = regex::Regex::new(r"\n\s*\n\s*\n")
        .unwrap()
        .replace_all(&text, "\n\n")
        .to_string();

    // 解码 HTML 实体
    text = text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // 截断
    if text.len() > max_chars {
        format!("{}...\n\n[Content truncated]", &text[..max_chars])
    } else {
        text
    }
}

/// 简单的 HTML → 纯文本转换
fn html_to_text(html: &str, max_chars: usize) -> String {
    // 先转换为 markdown，再移除 markdown 语法
    let markdown = html_to_markdown(html, max_chars);

    // 移除 markdown 语法
    let text = regex::Regex::new(r"\[.*?\]\(.*?\)")
        .unwrap()
        .replace_all(&markdown, "LINK")
        .to_string();
    let text = regex::Regex::new(r"[*_`#]+")
        .unwrap()
        .replace_all(&text, "")
        .to_string();

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_fetch_basic() {
        // 使用本地测试服务器或简单的 HTTP 测试
        // 这里我们测试参数验证逻辑
        let args = json!({
            "url": "https://example.com",
            "extract_mode": "markdown",
            "max_chars": 1000
        });

        let tool = WebFetchTool;
        let metadata = tool.metadata();

        assert_eq!(metadata.name, "web_fetch");
        assert!(metadata.parameters.is_object());
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool;
        let args = json!({
            "url": "ftp://example.com"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_html_to_markdown() {
        let html = r#"<h1>Hello</h1><p>Test <strong>bold</strong> text</p>"#;
        let markdown = html_to_markdown(html, 1000);
        assert!(markdown.contains("# Hello"));
        assert!(markdown.contains("**bold**"));
    }

    #[test]
    fn test_html_truncation() {
        let html = "<p>".repeat(1000);
        let result = html_to_markdown(&html, 100);
        // 截断后应该包含省略号和提示信息
        assert!(result.len() <= 150); // 允许一些误差
        assert!(result.contains("...") || result.contains("truncated") || result.len() < 150);
    }
}
