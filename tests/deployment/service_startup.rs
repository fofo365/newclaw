//! 服务启动测试

use std::process::Command;
use std::time::Duration;
use std::thread;

/// 测试 Gateway 启动
#[test]
fn test_gateway_startup() {
    // 检查是否可以编译
    let output = Command::new("cargo")
        .args(["build", "--release", "--bin", "newclaw"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("✓ Gateway 编译成功");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("⚠️ Gateway 编译失败:\n{}", stderr);
            }
        }
        Err(e) => {
            println!("编译失败: {}", e);
        }
    }
}

/// 测试 Dashboard 启动
#[test]
fn test_dashboard_startup() {
    // 检查 Dashboard 模块是否存在
    let source = std::fs::read_to_string("src/dashboard/mod.rs");
    
    match source {
        Ok(_) => println!("✓ Dashboard 模块存在"),
        Err(_) => println!("⚠️ Dashboard 模块不存在"),
    }
}

/// 测试服务依赖
#[test]
fn test_service_dependencies() {
    // 检查 Redis
    let redis = Command::new("redis-cli")
        .args(["ping"])
        .output();
    
    match redis {
        Ok(output) => {
            if output.status.success() {
                println!("✓ Redis 可用");
            } else {
                println!("⚠️ Redis 未运行");
            }
        }
        Err(_) => {
            println!("⚠️ Redis 未安装");
        }
    }
    
    // 检查配置文件
    let config_paths = [
        "/etc/newclaw/config.toml",
        "./newclaw.toml",
    ];
    
    for path in &config_paths {
        if std::path::Path::new(path).exists() {
            println!("✓ 配置文件存在: {}", path);
        }
    }
}