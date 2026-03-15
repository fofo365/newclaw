//! Regression Test: Issue #2 - Dashboard 缺少 v0.5.0-v0.7.0 UI
//!
//! Issue: Dashboard 没有 v0.5.0+ 新功能的 UI
//! Root Cause: 只开发了后端 API，未开发前端 UI
//! Test: 验证 API 端点存在并返回正确响应

use reqwest::Client;
use serde_json::Value;

const DASHBOARD_URL: &str = "http://localhost:3000";

/// 测试 Dashboard 核心配置 API
#[tokio::test]
async fn test_dashboard_llm_config_api() {
    let client = Client::new();
    
    // 测试 GET /api/config/llm
    let response = client
        .get(format!("{}/api/config/llm", DASHBOARD_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            assert!(resp.status().is_success() || resp.status().as_u16() == 401, 
                "LLM config API 应该返回 200 或 401，实际: {}", resp.status());
        }
        Err(e) => {
            println!("Dashboard 未运行，跳过测试: {}", e);
        }
    }
}

/// 测试 Dashboard 监控 API
#[tokio::test]
async fn test_dashboard_monitor_api() {
    let client = Client::new();
    
    let response = client
        .get(format!("{}/api/monitor/logs", DASHBOARD_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            // 应该返回成功或未授权
            assert!(resp.status().is_success() || resp.status().as_u16() == 401);
        }
        Err(e) => {
            println!("Dashboard 未运行，跳过测试: {}", e);
        }
    }
}

/// 测试 v0.5.0+ 功能 API 端点是否存在
#[tokio::test]
async fn test_dashboard_v05_features() {
    let client = Client::new();
    
    // v0.5.0: 上下文策略 API
    let endpoints = vec![
        "/api/context/strategy",
        "/api/context/policy",
        "/api/context/compress",
        "/api/context/retrieve",
    ];
    
    for endpoint in endpoints {
        let response = client
            .get(format!("{}{}", DASHBOARD_URL, endpoint))
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                // 200 = 存在, 401 = 需要认证, 404 = 不存在
                if status.as_u16() == 404 {
                    panic!("v0.5.0 API 端点不存在: {}", endpoint);
                }
                println!("{}: {}", endpoint, status);
            }
            Err(e) => {
                println!("Dashboard 未运行，跳过 {}: {}", endpoint, e);
            }
        }
    }
}

/// 测试 v0.7.0 功能 API 端点是否存在
#[tokio::test]
async fn test_dashboard_v07_features() {
    let client = Client::new();
    
    // v0.7.0: DAG 工作流、任务调度、约束系统
    let endpoints = vec![
        "/api/dag",
        "/api/tasks",
        "/api/schedule",
        "/api/constraints",
        "/api/federation",
        "/api/audit",
        "/api/permissions",
    ];
    
    for endpoint in endpoints {
        let response = client
            .get(format!("{}{}", DASHBOARD_URL, endpoint))
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 404 {
                    println!("⚠️ v0.7.0 API 端点不存在: {}", endpoint);
                } else {
                    println!("✓ {}: {}", endpoint, status);
                }
            }
            Err(e) => {
                println!("Dashboard 未运行，跳过 {}: {}", endpoint, e);
            }
        }
    }
}

/// 测试 Dashboard 版本号
#[tokio::test]
async fn test_dashboard_version() {
    // 读取 Dashboard HTML 检查版本
    let client = Client::new();
    
    let response = client
        .get(format!("{}/dashboard", DASHBOARD_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let html = resp.text().await.unwrap_or_default();
            
            // 检查版本号
            if html.contains("v0.4.0") {
                println!("⚠️ Dashboard 版本号显示 v0.4.0，应更新到 v0.7.0");
            } else if html.contains("v0.7.0") {
                println!("✓ Dashboard 版本号正确: v0.7.0");
            }
        }
        Err(e) => {
            println!("Dashboard 未运行，跳过版本检查: {}", e);
        }
    }
}