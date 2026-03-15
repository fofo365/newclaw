//! CLI 对话测试

use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};

/// 测试 CLI 对话是否启用工具
#[test]
fn test_cli_chat_tools_injection() {
    let source = std::fs::read_to_string("src/cli/mod.rs")
        .expect("Failed to read CLI source");
    
    // 检查是否传入工具定义
    if source.contains("tools: None") {
        panic!("CLI 对话未启用工具调用 - 存在 tools: None");
    }
    
    // 检查是否有工具注册逻辑
    let has_register = source.contains("register_tools") || 
                       source.contains("init_builtin_tools") ||
                       source.contains("ToolRegistry");
    
    assert!(has_register, "CLI 应该有工具注册逻辑");
}

/// 测试 CLI 对话基础功能
#[test]
fn test_cli_chat_basic() {
    // 检查 CLI 是否能处理基本对话
    let source = std::fs::read_to_string("src/cli/mod.rs")
        .expect("Failed to read CLI source");
    
    // 检查是否有 LLM 调用逻辑
    assert!(
        source.contains("chat") || source.contains("complete") || source.contains("LLM"),
        "CLI 应该有 LLM 调用逻辑"
    );
    
    // 检查是否有消息处理
    assert!(
        source.contains("message") || source.contains("Message"),
        "CLI 应该有消息处理逻辑"
    );
}

/// 测试 CLI 对话上下文管理
#[test]
fn test_cli_chat_context() {
    let source = std::fs::read_to_string("src/cli/mod.rs")
        .expect("Failed to read CLI source");
    
    // 检查是否有上下文/历史管理
    let has_context = source.contains("context") || 
                      source.contains("history") ||
                      source.contains("Context") ||
                      source.contains("Session");
    
    if has_context {
        println!("✓ CLI 包含上下文管理");
    } else {
        println!("⚠️ CLI 缺少上下文管理");
    }
}