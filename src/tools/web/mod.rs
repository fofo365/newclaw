// Web 工具模块
pub mod fetch;
pub mod search;

pub use fetch::WebFetchTool;
pub use search::WebSearchTool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_web_tools_metadata() {
        let fetch_tool = WebFetchTool;
        let search_tool = WebSearchTool::new();

        assert_eq!(fetch_tool.metadata().name, "web_fetch");
        assert_eq!(search_tool.metadata().name, "web_search");
    }
}
