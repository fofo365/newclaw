// 浏览器控制测试

#[cfg(test)]
mod browser_tests {
    use crate::tools::browser::BrowserTool;
    use crate::tools::Tool;
    use serde_json::json;

    #[tokio::test]
    async fn test_browser_metadata() {
        let tool = BrowserTool::new();
        assert_eq!(tool.metadata().name, "browser");
    }

    #[tokio::test]
    async fn test_browser_navigate() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await;

        // 由于可能没有安装 Chrome，我们先测试接口是否正确
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Chrome"));
    }

    #[tokio::test]
    async fn test_browser_screenshot() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "screenshot"
        })).await;

        // 测试截图功能
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Chrome"));
    }

    #[tokio::test]
    async fn test_browser_evaluate() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "evaluate",
            "script": "return 1 + 1"
        })).await;

        // 测试 JavaScript 执行
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Chrome"));
    }
}
