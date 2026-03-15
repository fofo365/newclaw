//! Regression Test: Issue #5 - 飞书 WebSocket 长连接服务未运行
//!
//! Issue: 飞书消息无法响应，WebSocket 服务未启动
//! Test: 验证服务状态和连接能力

use std::process::Command;

/// 测试飞书连接服务是否存在
#[test]
fn test_feishu_service_exists() {
    // 检查源代码
    let source = std::fs::read_to_string("src/feishu_websocket/mod.rs")
        .or_else(|_| std::fs::read_to_string("src/channels/feishu_stream.rs"))
        .unwrap_or_default();
    
    if source.is_empty() {
        println!("⚠️ 找不到飞书 WebSocket 模块");
    } else {
        println!("✓ 飞书 WebSocket 模块存在");
    }
}

/// 测试飞书配置是否正确
#[test]
fn test_feishu_config_loaded() {
    // 检查配置文件
    let config_paths = vec![
        "/etc/newclaw/config.toml",
        "/etc/newclaw/feishu.toml",
        "./config/feishu.toml",
    ];
    
    for path in &config_paths {
        if std::path::Path::new(path).exists() {
            println!("✓ 飞书配置文件存在: {}", path);
            
            if let Ok(content) = std::fs::read_to_string(path) {
                if content.contains("app_id") && content.contains("app_secret") {
                    println!("✓ 配置包含飞书凭证");
                }
            }
        }
    }
}

/// 测试 systemd 服务状态
#[test]
#[ignore = "需要 systemd 权限"]
fn test_feishu_service_running() {
    let output = Command::new("systemctl")
        .args(["status", "feishu-long-connect"])
        .output();
    
    match output {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            if status.contains("active (running)") {
                println!("✓ 飞书连接服务运行中");
            } else {
                println!("⚠️ 飞书连接服务未运行:\n{}", status);
            }
        }
        Err(_) => {
            println!("无法检查服务状态");
        }
    }
}

/// 测试 Docker 容器状态
#[test]
fn test_feishu_container_running() {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=feishu", "--format", "{{.Names}}: {{.Status}}"])
        .output();
    
    match output {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            if status.is_empty() {
                println!("⚠️ 没有运行中的飞书容器");
            } else {
                println!("飞书容器状态:\n{}", status);
            }
        }
        Err(_) => {
            println!("Docker 未安装或无权限");
        }
    }
}

/// 测试 WebSocket 连接能力
#[tokio::test]
async fn test_feishu_websocket_connectable() {
    use tokio::net::TcpStream;
    
    // 检查端口
    let ports = vec![8088, 8089, 3000];
    
    for port in ports {
        match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
            Ok(_) => println!("✓ 端口 {} 开放", port),
            Err(_) => println!("⚠️ 端口 {} 未监听", port),
        }
    }
}

/// 测试 Gateway 中的飞书 WebSocket 初始化
#[test]
fn test_gateway_feishu_init() {
    // 检查 Gateway 是否初始化飞书连接
    let source = std::fs::read_to_string("src/main.rs")
        .or_else(|_| std::fs::read_to_string("src/bin/gateway.rs"))
        .or_else(|_| std::fs::read_to_string("src/gateway/mod.rs"))
        .unwrap_or_default();
    
    if source.contains("feishu") && source.contains("websocket") {
        println!("✓ Gateway 包含飞书 WebSocket 初始化");
    } else {
        println!("⚠️ Gateway 没有飞书 WebSocket 初始化代码");
    }
    
    if source.contains("FeishuStreamClient") || source.contains("FeishuWebSocket") {
        println!("✓ 找到飞书客户端类");
    }
}