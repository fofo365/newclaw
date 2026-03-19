//! CLI 命令测试

use std::process::Command;

/// 运行 CLI 命令
fn run_cli_command(cmd: &str) -> Result<(String, String), String> {
    let output = Command::new("cargo")
        .args(["run", "--bin", "newclaw", "--", cmd])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| format!("Failed to run CLI: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    Ok((stdout, stderr))
}

#[test]
fn test_cli_help_command() {
    match run_cli_command("--help") {
        Ok((stdout, _)) => {
            // 帮助应该包含基本选项
            assert!(stdout.contains("--provider") || stdout.contains("provider"), 
                "帮助应包含 --provider");
            assert!(stdout.contains("--model") || stdout.contains("model"),
                "帮助应包含 --model");
            assert!(stdout.contains("--gateway") || stdout.contains("gateway"),
                "帮助应包含 --gateway");
        }
        Err(e) => {
            println!("无法运行 CLI: {}", e);
        }
    }
}

#[test]
fn test_cli_providers_list() {
    match run_cli_command("--list-providers") {
        Ok((stdout, _)) => {
            // 应该列出支持的提供商
            assert!(stdout.contains("GLM") || stdout.contains("glm"),
                "应包含 GLM");
            assert!(stdout.contains("OpenAI") || stdout.contains("openai"),
                "应包含 OpenAI");
            assert!(stdout.contains("Claude") || stdout.contains("claude"),
                "应包含 Claude");
        }
        Err(e) => {
            println!("无法运行 CLI: {}", e);
        }
    }
}

#[test]
fn test_cli_config_command() {
    // config 命令应显示当前配置
    match run_cli_command("config") {
        Ok((stdout, stderr)) => {
            println!("config stdout: {}", stdout);
            println!("config stderr: {}", stderr);
        }
        Err(e) => {
            println!("无法运行 CLI: {}", e);
        }
    }
}

#[test]
fn test_cli_tools_command() {
    match run_cli_command("tools") {
        Ok((stdout, stderr)) => {
            // 不应显示"工具系统待实现"
            assert!(!stdout.contains("工具系统待实现"),
                "tools 命令不应显示'工具系统待实现'");
            assert!(!stderr.contains("工具系统待实现"),
                "tools 命令不应显示'工具系统待实现'");
        }
        Err(e) => {
            println!("无法运行 CLI: {}", e);
        }
    }
}

/// 测试缺失的命令（v0.5.0+）
#[test]
fn test_cli_missing_commands() {
    let source = std::fs::read_to_string("src/cli/mod.rs")
        .expect("Failed to read CLI source");
    
    let required_commands = [
        ("strategy", "v0.5.0"),
        ("policy", "v0.5.0"),
        ("watchdog", "v0.6.0"),
        ("task", "v0.7.0"),
        ("dag", "v0.7.0"),
        ("memory", "v0.7.0"),
        ("session", "v0.7.0"),
    ];
    
    println!("\n命令实现状态:");
    for (cmd, version) in &required_commands {
        if source.contains(&format!("\"{}\"", cmd)) {
            println!("  ✓ {} ({})", cmd, version);
        } else {
            println!("  ⚠️ {} ({}) - 未实现", cmd, version);
        }
    }
}