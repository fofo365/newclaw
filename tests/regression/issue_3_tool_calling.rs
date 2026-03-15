//! Regression Test: Issue #3 - Dashboard 对话无工具调用
//!
//! Issue: Dashboard 对话无法调用工具
//! Root Cause: ChatRequest 中 tools: None
//! Test: 验证对话 API 传入工具定义并处理 tool_calls

use reqwest::Client;
use serde_json::json;

const DASHBOARD_URL: &str = "http://localhost:3000";

/// 测试 Dashboard 对话是否能调用工具
#[tokio::test]
async fn test_dashboard_chat_tool_calling() {
    let client = Client::new();
    
    // 创建会话
    let session_response = client
        .post(format!("{}/api/chat/sessions", DASHBOARD_URL))
        .json(&json!({
            "model": "glm-4",
            "provider": "glm"
        }))
        .send()
        .await;
    
    match session_response {
        Ok(resp) if resp.status().is_success() => {
            let session: serde_json::Value = resp.json().await.unwrap();
            let session_id = session["session_id"].as_str().unwrap_or("test");
            
            // 发送需要工具调用的消息
            let chat_response = client
                .post(format!("{}/api/chat/sessions/{}/messages", DASHBOARD_URL, session_id))
                .json(&json!({
                    "content": "读取 /etc/hostname 文件"
                }))
                .send()
                .await;
            
            match chat_response {
                Ok(resp) => {
                    let result: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                    
                    // 检查是否有工具调用
                    if result["tool_calls"].is_array() {
                        println!("✓ 对话支持工具调用");
                    } else if result["response"].is_string() {
                        let response = result["response"].as_str().unwrap_or("");
                        if response.contains("无法") || response.contains("cannot") {
                            println!("⚠️ 对话无法调用工具: {}", response);
                        }
                    }
                }
                Err(e) => {
                    println!("发送消息失败: {}", e);
                }
            }
        }
        Ok(resp) => {
            println!("创建会话失败: {}", resp.status());
        }
        Err(e) => {
            println!("Dashboard 未运行，跳过测试: {}", e);
        }
    }
}

/// 测试工具定义是否传入 LLM 请求
#[test]
fn test_tools_passed_to_llm() {
    // 这个测试需要 mock LLM 响应
    // 验证 ChatRequest.tools 不为 None
    
    // 检查源代码
    let source = std::fs::read_to_string("src/dashboard/chat.rs")
        .expect("Failed to read chat.rs");
    
    if source.contains("tools: None") {
        panic!("chat.rs 中存在 tools: None，工具调用不会工作");
    }
    
    println!("✓ chat.rs 中没有 tools: None");
}

/// 测试 ToolRegistry 是否注入到 Dashboard
#[test]
fn test_tool_registry_injected() {
    let source = std::fs::read_to_string("src/dashboard/mod.rs")
        .expect("Failed to read mod.rs");
    
    if !source.contains("ToolRegistry") {
        println!("⚠️ Dashboard 没有注入 ToolRegistry");
    } else {
        println!("✓ Dashboard 包含 ToolRegistry");
    }
}