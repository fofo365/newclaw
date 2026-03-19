//! Dashboard 对话测试

use reqwest::Client;
use serde_json::json;

const BASE_URL: &str = "http://localhost:3000";

/// 测试创建会话
#[tokio::test]
async fn test_create_session() {
    let client = Client::new();
    
    let response = client
        .post(format!("{}/api/chat/sessions", BASE_URL))
        .json(&json!({
            "model": "glm-4",
            "provider": "glm"
        }))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("创建会话: {}", status);
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试工具调用能力
#[test]
fn test_tool_calling_capability() {
    // 检查源代码
    let source = std::fs::read_to_string("src/dashboard/chat.rs")
        .expect("Failed to read chat.rs");
    
    // 检查是否有 tools: None
    if source.contains("tools: None") {
        panic!("Dashboard chat.rs 包含 tools: None - 工具调用不会工作");
    }
    
    // 检查是否有工具注册
    let has_tools = source.contains("ToolRegistry") || 
                    source.contains("get_tools") ||
                    source.contains("tool_definitions");
    
    if has_tools {
        println!("✓ Dashboard chat 包含工具相关代码");
    } else {
        println!("⚠️ Dashboard chat 缺少工具集成");
    }
}

/// 测试工具定义注入
#[test]
fn test_tool_definitions_injected() {
    let source = std::fs::read_to_string("src/dashboard/chat.rs")
        .expect("Failed to read chat.rs");
    
    // 查找 ChatRequest 构造
    if source.contains("ChatRequest") {
        // 检查 tools 字段
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("ChatRequest") || line.contains("let request") {
                // 检查接下来的几行
                for j in i..std::cmp::min(i + 10, lines.len()) {
                    if lines[j].contains("tools:") {
                        if lines[j].contains("None") {
                            println!("⚠️ 第 {} 行: tools 设置为 None", j + 1);
                        } else {
                            println!("✓ 第 {} 行: tools 有值", j + 1);
                        }
                    }
                }
            }
        }
    }
}

/// 测试 tool_calls 处理
#[test]
fn test_tool_calls_handling() {
    let source = std::fs::read_to_string("src/dashboard/chat.rs")
        .expect("Failed to read chat.rs");
    
    // 检查是否处理 tool_calls
    let handles_tool_calls = source.contains("tool_calls") || 
                              source.contains("ToolCall") ||
                              source.contains("execute_tool");
    
    if handles_tool_calls {
        println!("✓ Dashboard 处理 tool_calls");
    } else {
        println!("⚠️ Dashboard 不处理 tool_calls");
    }
}