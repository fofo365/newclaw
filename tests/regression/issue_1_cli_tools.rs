//! Regression Test: Issue #1 - CLI tools 命令显示"工具系统待实现"
//!
//! Issue: 在 CLI 中输入 tools 命令，显示"工具系统待实现"
//! Root Cause: register_tools 和 list_tools 未正确调用 ToolRegistry
//! Fix: 调用 init_builtin_tools 并正确显示工具列表

use std::process::Command;

/// 测试 CLI tools 命令应显示工具列表
#[test]
fn test_cli_tools_command_shows_tool_list() {
    // 启动 CLI 并执行 tools 命令
    let output = Command::new("cargo")
        .args(["run", "--bin", "newclaw", "--", "--test-command", "tools"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run CLI");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // 不应该显示"工具系统待实现"
    assert!(
        !stdout.contains("工具系统待实现"),
        "CLI tools 命令不应显示'工具系统待实现'\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    
    // 应该显示"Available Tools"
    assert!(
        stdout.contains("Available Tools") || stdout.contains("工具"),
        "CLI tools 命令应显示工具列表\nstdout: {}",
        stdout
    );
}

/// 测试 ToolRegistry 与 CLI 的集成
#[test]
fn test_tool_registry_cli_integration() {
    use newclaw::tools::ToolRegistry;
    use std::sync::Arc;
    
    let registry = Arc::new(ToolRegistry::new());
    
    // 检查 init_builtin_tools 是否可用
    let result = newclaw::tools::init_builtin_tools(
        &registry,
        std::path::PathBuf::from("./data"),
        std::path::PathBuf::from("."),
    );
    
    // 初始化应该成功
    assert!(result.is_ok(), "init_builtin_tools 失败: {:?}", result.err());
    
    // 应该有注册的工具
    let tools = registry.list_tools();
    assert!(!tools.is_empty(), "ToolRegistry 应该有工具");
    
    // 打印已注册的工具
    println!("已注册工具: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());
}

/// 测试 CLI 交互模式工具列表
#[test]
#[ignore = "需要交互式输入，手动运行"]
fn test_cli_interactive_tools() {
    // 这个测试需要手动运行或使用 expect/pty
    // cargo run --bin newclaw
    // > tools
    // 应该显示工具列表
}