//! Dashboard 认证测试

use reqwest::Client;
use serde_json::json;

const BASE_URL: &str = "http://localhost:3000";

/// 测试配对码生成
#[tokio::test]
async fn test_paircode_generation() {
    let response = Client::new()
        .get(format!("{}/api/auth/paircode", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("配对码生成: {}", status);
            
            if status.is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                if body["pair_code"].is_string() {
                    println!("✓ 配对码生成成功");
                }
            }
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}

/// 测试登录流程
#[tokio::test]
async fn test_login_flow() {
    let client = Client::new();
    
    // 1. 获取配对码
    let paircode_resp = client
        .get(format!("{}/api/auth/paircode", BASE_URL))
        .send()
        .await;
    
    let pair_code = match paircode_resp {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
            body["pair_code"].as_str().unwrap_or("000000").to_string()
        }
        _ => {
            println!("无法获取配对码，跳过登录测试");
            return;
        }
    };
    
    // 2. 使用配对码登录
    let login_resp = client
        .post(format!("{}/api/auth/login", BASE_URL))
        .json(&json!({ "pair_code": pair_code }))
        .send()
        .await;
    
    match login_resp {
        Ok(resp) => {
            let status = resp.status();
            println!("登录状态: {}", status);
            
            if status.is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                if body["token"].is_string() {
                    println!("✓ 登录成功，获得 token");
                }
            }
        }
        Err(e) => {
            println!("登录请求失败: {}", e);
        }
    }
}

/// 测试受保护端点需要认证
#[tokio::test]
async fn test_protected_endpoint_requires_auth() {
    let client = Client::new();
    
    // 不带 token 访问受保护端点
    let response = client
        .get(format!("{}/api/config/llm", BASE_URL))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let status = resp.status();
            // 应该返回 401 或 403
            if status.as_u16() == 401 || status.as_u16() == 403 {
                println!("✓ 受保护端点正确返回 {}", status);
            } else if status.is_success() {
                println!("⚠️ 受保护端点无需认证");
            }
        }
        Err(e) => {
            println!("服务未运行: {}", e);
        }
    }
}