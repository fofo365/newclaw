// NewClaw v0.4.0 - 企业微信（WeCom）使用示例
//
// 展示如何使用 WeCom 通道进行消息发送和接收

use anyhow::Result;
use newclaw::channels::{
    WeComConfig,
    WeComClient,
    WeComMessageClient,
    WeComWebhook,
    MessageTarget,
    MediaType,
};

/// 配置示例
fn create_config() -> WeComConfig {
    WeComConfig {
        // 企业 ID
        corp_id: "your_corp_id".to_string(),
        // 应用 Secret
        corp_secret: "your_corp_secret".to_string(),
        // 应用 ID
        agent_id: "1000001".to_string(),
        // Token（用于 Webhook 签名验证）
        token: Some("your_token".to_string()),
        // EncodingAESKey（用于消息加密/解密）
        encoding_aes_key: Some("your_encoding_aes_key".to_string()),
        // 接收消息的 ID（企业 ID 或应用 ID）
        receive_id: Some("your_corp_id".to_string()),
    }
}

/// 示例 1：发送文本消息
async fn send_text_example() -> Result<()> {
    let config = create_config();
    let client = WeComMessageClient::from_config(config);
    
    // 发送文本消息给指定用户
    let result = client.send_text("user123", "Hello from NewClaw!").await?;
    println!("发送成功: {:?}", result);
    
    Ok(())
}

/// 示例 2：发送图片消息
async fn send_image_example() -> Result<()> {
    let config = create_config();
    let client = WeComMessageClient::from_config(config);
    
    // 方式 1：上传图片并发送
    let image_data = std::fs::read("image.png")?;
    let result = client.upload_and_send_image("user123", "image.png", image_data).await?;
    println!("发送成功: {:?}", result);
    
    // 方式 2：使用已有的 media_id 发送
    // let result = client.send_image("user123", "media_id_here").await?;
    
    Ok(())
}

/// 示例 3：发送文件消息
async fn send_file_example() -> Result<()> {
    let config = create_config();
    let client = WeComMessageClient::from_config(config);
    
    // 上传文件并发送
    let file_data = std::fs::read("document.pdf")?;
    let result = client.upload_and_send_file("user123", "document.pdf", file_data).await?;
    println!("发送成功: {:?}", result);
    
    Ok(())
}

/// 示例 4：使用底层 API 客户端
async fn api_client_example() -> Result<()> {
    let config = create_config();
    let client = WeComClient::new(config);
    
    // 发送到多个目标
    let target = MessageTarget {
        touser: Some("user1|user2|user3".to_string()),
        toparty: Some("1|2".to_string()),
        totag: None,
    };
    
    let result = client.send_text(&target, "Broadcast message").await?;
    println!("发送成功: {:?}", result);
    
    // 上传媒体文件
    let data = std::fs::read("video.mp4")?;
    let upload_result = client.upload_media(MediaType::Video, "video.mp4", data).await?;
    println!("上传成功，media_id: {}", upload_result.media_id);
    
    // 下载媒体文件
    let download_result = client.download_media("media_id_here").await?;
    println!("下载成功，大小: {} bytes", download_result.buffer.len());
    
    Ok(())
}

/// 示例 5：处理 Webhook 回调
fn webhook_example() -> Result<()> {
    let config = create_config();
    let webhook = WeComWebhook::from_config(&config)?;
    
    // 验证 URL（首次配置时调用）
    // GET /wecom?msg_signature=xxx&timestamp=xxx&nonce=xxx&echostr=xxx
    // let echostr = webhook.verify_url(&request)?;
    // 返回 echostr 给企业微信服务器
    
    // 处理消息回调
    // POST /wecom
    // let inbound = webhook.handle_message(
    //     &msg_signature,
    //     &timestamp,
    //     &nonce,
    //     &encrypt,
    // )?;
    // 
    // match inbound {
    //     WebhookInbound::Text(msg) => {
    //         println!("收到文本消息: {:?}", msg.text);
    //     }
    //     WebhookInbound::Event(msg) => {
    //         println!("收到事件: {:?}", msg.event);
    //     }
    //     WebhookInbound::Unknown(json) => {
    //         println!("收到未知消息: {:?}", json);
    //     }
    // }
    
    Ok(())
}

/// 示例 6：长文本自动分片
fn chunk_text_example() {
    use newclaw::channels::chunk_text;
    
    let long_text = "这是一段很长的文本...".repeat(100);
    let chunks = chunk_text(&long_text, 2048);
    
    println!("文本被分成 {} 片", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!("片段 {}: {} bytes", i + 1, chunk.len());
    }
}

fn main() {
    println!("WeCom 通道示例");
    println!("================");
    println!();
    println!("1. 配置 WeComConfig");
    println!("2. 创建 WeComMessageClient");
    println!("3. 调用 send_text/send_image/send_file 等方法");
    println!();
    println!("详细示例请查看源代码。");
}
