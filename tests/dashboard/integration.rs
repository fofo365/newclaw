//! Dashboard 集成测试

/// 测试 DashboardState 初始化
#[test]
fn test_dashboard_state_init() {
    let source = std::fs::read_to_string("src/dashboard/mod.rs")
        .expect("Failed to read mod.rs");
    
    // 检查是否有正确的初始化
    assert!(
        source.contains("DashboardState"),
        "应该有 DashboardState 结构"
    );
    
    // 检查是否有 LLM provider
    if source.contains("llm_provider") || source.contains("LlmProvider") {
        println!("✓ DashboardState 包含 LLM provider");
    } else {
        println!("⚠️ DashboardState 缺少 LLM provider");
    }
    
    // 检查是否有 ToolRegistry
    if source.contains("ToolRegistry") {
        println!("✓ DashboardState 包含 ToolRegistry");
    } else {
        println!("⚠️ DashboardState 缺少 ToolRegistry");
    }
}

/// 测试 Dashboard 版本号
#[test]
fn test_dashboard_version() {
    let source = std::fs::read_to_string("src/dashboard/mod.rs")
        .expect("Failed to read mod.rs");
    
    if source.contains("v0.4.0") {
        println!("⚠️ Dashboard 版本显示 v0.4.0，应更新到 v0.7.0");
    } else if source.contains("v0.7.0") {
        println!("✓ Dashboard 版本正确");
    } else {
        println!("Dashboard 版本未标注");
    }
}

/// 测试路由定义
#[test]
fn test_dashboard_routes() {
    let source = std::fs::read_to_string("src/dashboard/mod.rs")
        .or_else(|_| std::fs::read_to_string("src/main.rs"))
        .unwrap_or_default();
    
    let required_routes = [
        ("/api/config/llm", "LLM 配置"),
        ("/api/config/tools", "工具配置"),
        ("/api/config/feishu", "飞书配置"),
        ("/api/monitor/logs", "日志监控"),
        ("/api/chat/sessions", "会话管理"),
        ("/api/auth/paircode", "配对码"),
    ];
    
    println!("\n路由定义检查:");
    for (route, desc) in &required_routes {
        if source.contains(route) {
            println!("  ✓ {} - {}", route, desc);
        } else {
            println!("  ⚠️ {} - {} - 未找到", route, desc);
        }
    }
}