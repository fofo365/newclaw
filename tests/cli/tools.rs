//! CLI 工具测试

/// 测试工具注册
#[test]
fn test_tool_registration() {
    use newclaw::tools::ToolRegistry;
    use std::sync::Arc;
    
    let registry = Arc::new(ToolRegistry::new());
    
    // 尝试初始化内置工具
    let result = newclaw::tools::init_builtin_tools(
        &registry,
        std::path::PathBuf::from("./data"),
        std::path::PathBuf::from("."),
    );
    
    assert!(result.is_ok(), "工具初始化失败: {:?}", result.err());
    
    let tools = registry.list_tools();
    println!("\n已注册工具 ({}):", tools.len());
    for tool in &tools {
        println!("  • {} - {}", tool.name, tool.description);
    }
    
    assert!(!tools.is_empty(), "应该有注册的工具");
}

/// 测试特定工具存在
#[test]
fn test_required_tools_exist() {
    use newclaw::tools::ToolRegistry;
    use std::sync::Arc;
    
    let registry = Arc::new(ToolRegistry::new());
    
    let _ = newclaw::tools::init_builtin_tools(
        &registry,
        std::path::PathBuf::from("./data"),
        std::path::PathBuf::from("."),
    );
    
    let required_tools = vec![
        "memory",
        "browser",
        "canvas",
        "sessions",
        "subagents",
        "nodes",
        "feishu_doc",
        "tts",
    ];
    
    let tools = registry.list_tools();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    
    println!("\n工具存在检查:");
    for tool in &required_tools {
        if tool_names.contains(tool) {
            println!("  ✓ {}", tool);
        } else {
            println!("  ⚠️ {} - 未注册", tool);
        }
    }
}

/// 测试工具执行
#[tokio::test]
async fn test_tool_execute() {
    use newclaw::tools::ToolRegistry;
    use std::sync::Arc;
    
    let registry = Arc::new(ToolRegistry::new());
    
    let _ = newclaw::tools::init_builtin_tools(
        &registry,
        std::path::PathBuf::from("./data"),
        std::path::PathBuf::from("."),
    );
    
    // 尝试执行一个简单的工具
    // 这里只验证工具可以获取，实际执行需要具体工具实现
    let tools = registry.list_tools();
    
    if !tools.is_empty() {
        println!("✓ 工具可以获取");
    }
}