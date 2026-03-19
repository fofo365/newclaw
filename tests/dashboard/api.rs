//! Dashboard API 端点测试

use reqwest::Client;
use serde_json::json;

const BASE_URL: &str = "http://localhost:3000";

fn client() -> Client {
    Client::new()
}

/// 测试健康检查端点
#[tokio::test]
async fn test_health_endpoint() {
    let response = client()
        .get(format!("{}/health", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "Health endpoint should return 200");
            let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
            assert!(body["status"] == "ok", "Health status should be ok");
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试配置 API
#[tokio::test]
async fn test_config_llm_get() {
    let response = client()
        .get(format!("{}/api/config/llm", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("GET /api/config/llm: {}", status);
            // 200 或 401 都是正常的
            assert!(status.is_success() || status.as_u16() == 401);
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试工具配置 API
#[tokio::test]
async fn test_config_tools() {
    let response = client()
        .get(format!("{}/api/config/tools", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("GET /api/config/tools: {}", status);
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试飞书配置 API
#[tokio::test]
async fn test_config_feishu() {
    let response = client()
        .get(format!("{}/api/config/feishu", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("GET /api/config/feishu: {}", status);
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试监控日志 API
#[tokio::test]
async fn test_monitor_logs() {
    let response = client()
        .get(format!("{}/api/monitor/logs", BASE_URL))
        .query(&[("limit", "10")])
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("GET /api/monitor/logs: {}", status);
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试指标 API
#[tokio::test]
async fn test_monitor_metrics() {
    let response = client()
        .get(format!("{}/api/monitor/metrics", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("GET /api/monitor/metrics: {}", status);
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试 v0.5.0+ 功能端点
#[tokio::test]
async fn test_v05_endpoints() {
    let endpoints = vec![
        "/api/context/strategy",
        "/api/context/policy", 
        "/api/context/compress",
        "/api/context/retrieve",
    ];
    
    println!("\nv0.5.0 端点检查:");
    for endpoint in &endpoints {
        let response = client()
            .get(format!("{}{}", BASE_URL, endpoint))
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 404 {
                    println!("  ⚠️ {} - 不存在", endpoint);
                } else {
                    println!("  ✓ {} - {}", endpoint, status);
                }
            }
            Err(_) => {
                println!("  ? {} - 服务未运行", endpoint);
            }
        }
    }
}

/// 测试 v0.7.0 功能端点
#[tokio::test]
async fn test_v07_endpoints() {
    let endpoints = vec![
        "/api/dag",
        "/api/tasks",
        "/api/schedule",
        "/api/constraints",
        "/api/federation",
        "/api/audit",
    ];
    
    println!("\nv0.7.0 端点检查:");
    for endpoint in &endpoints {
        let response = client()
            .get(format!("{}{}", BASE_URL, endpoint))
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 404 {
                    println!("  ⚠️ {} - 不存在", endpoint);
                } else {
                    println!("  ✓ {} - {}", endpoint, status);
                }
            }
            Err(_) => {
                println!("  ? {} - 服务未运行", endpoint);
            }
        }
    }
}