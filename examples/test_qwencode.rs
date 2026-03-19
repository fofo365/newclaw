use std::env;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: usize,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量获取 API Key
    let api_key = env::var("QWENCODE_API_KEY")
        .expect("Please set QWENCODE_API_KEY environment variable");

    println!("🔑 API Key found (length: {})", api_key.len());

    // 创建客户端
    let client = Client::new();

    // 构建请求
    let request = ChatRequest {
        model: "glm-4.7".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "请回复: 测试成功".to_string(),
            },
        ],
        temperature: 0.7,
        max_tokens: 100,
        stream: false,
    };

    println!("📤 Sending test request to qwencode/glm-4.7...");

    // 发送请求
    let response = client
        .post("https://api.qwencode.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    println!("📥 Response status: {}", response.status());

    if !response.status().is_success() {
        let error_text = response.text().await?;
        println!("❌ Error response: {}", error_text);
        return Err(format!("API request failed: {}", error_text).into());
    }

    let chat_response: ChatResponse = response.json().await?;

    if let Some(choice) = chat_response.choices.first() {
        println!("✅ Test successful!");
        println!("📝 Response: {}", choice.message.content);
    } else {
        println!("⚠️  No choices in response");
    }

    Ok(())
}