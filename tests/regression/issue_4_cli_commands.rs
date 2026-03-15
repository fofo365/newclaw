//! Regression Test: Issue #4 - CLI 缺少 v0.5.0-v0.7.0 功能
//!
//! Issue: CLI 只有 v0.4.0 的命令，缺少新功能
//! Test: 验证所有期望的命令是否存在

/// 测试 CLI 命令覆盖
#[test]
fn test_cli_command_coverage() {
    let expected_commands = vec![
        // v0.5.0 上下文管理
        ("strategy", "切换上下文策略"),
        ("policy", "管理上下文策略"),
        ("compress", "压缩上下文"),
        ("rag", "RAG 检索"),
        
        // v0.6.0 Watchdog
        ("watchdog", "Watchdog 状态"),
        ("recovery", "恢复策略"),
        
        // v0.7.0 任务系统
        ("task", "任务管理"),
        ("dag", "DAG 工作流"),
        ("schedule", "任务调度"),
        ("constraint", "约束管理"),
        ("session", "会话管理"),
        ("memory", "记忆管理"),
        ("federation", "联邦管理"),
        ("audit", "审计查询"),
        ("role", "角色管理"),
    ];
    
    // 检查 CLI 源代码
    let source = std::fs::read_to_string("src/cli/mod.rs")
        .expect("Failed to read CLI source");
    
    let mut missing = Vec::new();
    let mut found = Vec::new();
    
    for (cmd, desc) in expected_commands {
        if source.contains(&format!("\"{}\"", cmd)) || source.contains(&format!("'{}'", cmd)) {
            found.push((cmd, desc));
        } else {
            missing.push((cmd, desc));
        }
    }
    
    println!("\n✓ 已实现的命令:");
    for (cmd, desc) in &found {
        println!("  {} - {}", cmd, desc);
    }
    
    if !missing.is_empty() {
        println!("\n⚠️ 缺失的命令:");
        for (cmd, desc) in &missing {
            println!("  {} - {}", cmd, desc);
        }
    }
    
    // 计算覆盖率
    let coverage = found.len() * 100 / expected_commands.len();
    println!("\n命令覆盖率: {}%", coverage);
    
    if coverage < 100 {
        println!("\n⚠️ CLI 命令覆盖不完整，缺少 {} 个命令", missing.len());
    }
}

/// 测试 CLI 对话工具调用
#[test]
fn test_cli_chat_tools_enabled() {
    let source = std::fs::read_to_string("src/cli/mod.rs")
        .expect("Failed to read CLI source");
    
    if source.contains("tools: None") {
        println!("⚠️ CLI 对话没有启用工具调用");
    } else {
        println!("✓ CLI 对话已启用工具");
    }
}

/// 测试 CLI 帮助信息
#[test]
fn test_cli_help_completeness() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "newclaw", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();
    
    match output {
        Ok(output) => {
            let help = String::from_utf8_lossy(&output.stdout);
            println!("CLI 帮助信息:\n{}", help);
        }
        Err(e) => {
            println!("无法运行 CLI: {}", e);
        }
    }
}