// 测试工具调用功能
use newclaw::feishu_websocket::tools::{ToolManager, ToolCallRequest};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试工具调用功能\n");

    // 创建工具管理器
    let manager = ToolManager::new().await;

    // 显示所有可用工具
    println!("📋 可用工具列表:");
    for tool in manager.get_all_tools().await {
        println!("  - {}: {}", tool.name, tool.description);
    }
    println!();

    // 测试 systemctl_status
    println!("🔧 测试 systemctl_status:");
    let call = ToolCallRequest {
        name: "systemctl_status".to_string(),
        arguments: {
            let mut args = HashMap::new();
            args.insert("service".to_string(), "newclaw-*".to_string());
            args
        },
    };
    let result = manager.execute_tool(&call).await;
    println!("  成功: {}", result.success);
    if result.success {
        println!("  输出:\n{}", result.output);
    } else if let Some(err) = result.error {
        println!("  错误: {}", err);
    }
    println!();

    // 测试 ps_list
    println!("🔧 测试 ps_list:");
    let call = ToolCallRequest {
        name: "ps_list".to_string(),
        arguments: {
            let mut args = HashMap::new();
            args.insert("filter".to_string(), "newclaw".to_string());
            args
        },
    };
    let result = manager.execute_tool(&call).await;
    println!("  成功: {}", result.success);
    if result.success {
        println!("  输出:\n{}", result.output);
    } else if let Some(err) = result.error {
        println!("  错误: {}", err);
    }
    println!();

    // 测试 disk_usage
    println!("🔧 测试 disk_usage:");
    let call = ToolCallRequest {
        name: "disk_usage".to_string(),
        arguments: HashMap::new(),
    };
    let result = manager.execute_tool(&call).await;
    println!("  成功: {}", result.success);
    if result.success {
        println!("  输出:\n{}", result.output);
    } else if let Some(err) = result.error {
        println!("  错误: {}", err);
    }
    println!();

    // 测试 memory_usage
    println!("🔧 测试 memory_usage:");
    let call = ToolCallRequest {
        name: "memory_usage".to_string(),
        arguments: HashMap::new(),
    };
    let result = manager.execute_tool(&call).await;
    println!("  成功: {}", result.success);
    if result.success {
        println!("  输出:\n{}", result.output);
    } else if let Some(err) = result.error {
        println!("  错误: {}", err);
    }
    println!();

    // 测试 tail_log
    println!("🔧 测试 tail_log:");
    let call = ToolCallRequest {
        name: "tail_log".to_string(),
        arguments: {
            let mut args = HashMap::new();
            args.insert("file".to_string(), "/var/log/newclaw/*.log".to_string());
            args.insert("lines".to_string(), "10".to_string());
            args
        },
    };
    let result = manager.execute_tool(&call).await;
    println!("  成功: {}", result.success);
    if result.success {
        println!("  输出:\n{}", result.output);
    } else if let Some(err) = result.error {
        println!("  错误: {}", err);
    }
    println!();

    println!("✅ 所有测试完成！");
    Ok(())
}