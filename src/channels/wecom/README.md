# WeCom 通道集成

企业微信（WeCom/WeChat Work）通道集成模块。

## 功能特性

- ✅ **AccessToken 管理**：自动缓存和过期刷新
- ✅ **消息发送**：文本、图片、文件、视频
- ✅ **媒体上传/下载**：支持各种媒体类型
- ✅ **Webhook 处理**：消息接收和事件处理
- ✅ **消息加密/解密**：AES-256-CBC 加密
- ✅ **签名验证**：SHA1 签名校验
- ✅ **长文本分片**：自动分割超长消息

## 快速开始

### 1. 配置

```rust
use newclaw::channels::WeComConfig;

let config = WeComConfig {
    corp_id: "your_corp_id".to_string(),
    corp_secret: "your_corp_secret".to_string(),
    agent_id: "1000001".to_string(),
    token: Some("your_token".to_string()),
    encoding_aes_key: Some("your_encoding_aes_key".to_string()),
    receive_id: Some("your_corp_id".to_string()),
};
```

### 2. 发送消息

```rust
use newclaw::channels::WeComMessageClient;

let client = WeComMessageClient::from_config(config);

// 发送文本
client.send_text("user123", "Hello!").await?;

// 发送图片
let image_data = std::fs::read("image.png")?;
client.upload_and_send_image("user123", "image.png", image_data).await?;
```

### 3. 接收 Webhook

```rust
use newclaw::channels::WeComWebhook;

let webhook = WeComWebhook::from_config(&config)?;

// 验证 URL
let echostr = webhook.verify_url(&request)?;

// 处理消息
let inbound = webhook.handle_message(&msg_signature, &timestamp, &nonce, &encrypt)?;
```

## API 参考

### WeComClient

底层 API 客户端，提供完整的 API 访问。

```rust
let client = WeComClient::new(config);

// 获取 AccessToken
let token = client.get_access_token().await?;

// 发送文本消息
let target = MessageTarget {
    touser: Some("user123".to_string()),
    ..Default::default()
};
client.send_text(&target, "Hello!").await?;

// 上传媒体
let media = client.upload_media(MediaType::Image, "image.png", data).await?;

// 下载媒体
let download = client.download_media("media_id").await?;
```

### WeComMessageClient

高级消息客户端，提供便捷的消息发送方法。

```rust
let client = WeComMessageClient::from_config(config);

// 发送到用户
client.send_text("user123", "Hello!").await?;

// 发送到部门
client.send_text_to_party("1", "Hello team!").await?;

// 发送到标签
client.send_text_to_tag("tag1", "Hello tagged users!").await?;

// 上传并发送图片
client.upload_and_send_image("user123", "image.png", data).await?;
```

### WeComWebhook

Webhook 处理器，用于接收企业微信回调。

```rust
let webhook = WeComWebhook::from_config(&config)?;

// URL 验证（首次配置）
let echostr = webhook.verify_url(&request)?;

// 处理消息回调
let inbound = webhook.handle_message(
    &msg_signature,
    &timestamp,
    &nonce,
    &encrypt
)?;

match inbound {
    WebhookInbound::Text(msg) => {
        println!("收到文本: {:?}", msg.text);
    }
    WebhookInbound::Event(msg) => {
        println!("收到事件: {:?}", msg.event);
    }
    WebhookInbound::Unknown(json) => {
        println!("未知消息: {:?}", json);
    }
}
```

### WeComCrypto

加密工具类，用于消息加解密。

```rust
let crypto = WeComCrypto::new(
    encoding_aes_key,
    Some(token),
    Some(receive_id),
)?;

// 解密消息
let plaintext = crypto.decrypt(&encrypt)?;

// 加密消息
let encrypted = crypto.encrypt(&plaintext)?;

// 验证签名
let valid = crypto.verify(&timestamp, &nonce, &encrypt, &signature);

// 计算签名
let signature = crypto.compute_signature(&timestamp, &nonce, &encrypt);
```

## 类型定义

### WeComConfig

```rust
pub struct WeComConfig {
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: String,
    pub token: Option<String>,
    pub encoding_aes_key: Option<String>,
    pub receive_id: Option<String>,
}
```

### MessageTarget

```rust
pub struct MessageTarget {
    pub touser: Option<String>,
    pub toparty: Option<String>,
    pub totag: Option<String>,
}
```

### MediaType

```rust
pub enum MediaType {
    Image,
    Voice,
    Video,
    File,
}
```

### WebhookInbound

```rust
pub enum WebhookInbound {
    Text(WebhookTextMessage),
    Event(WebhookEventMessage),
    Unknown(serde_json::Value),
}
```

## 常量

```rust
pub mod limits {
    pub const TEXT_MAX_BYTES: usize = 2048;
    pub const TOKEN_REFRESH_BUFFER_MS: i64 = 60_000;
    pub const REQUEST_TIMEOUT_MS: u64 = 15_000;
    pub const MAX_REQUEST_BODY_SIZE: usize = 1024 * 1024;
}
```

## 错误处理

所有 API 都返回 `anyhow::Result<T>`，可以使用 `?` 操作符进行错误传播。

```rust
async fn send_message() -> Result<()> {
    let client = WeComMessageClient::from_config(config);
    client.send_text("user123", "Hello!").await?;
    Ok(())
}
```

## 测试

```bash
cargo test --lib wecom
```

## 迁移自 OpenClaw TypeScript

这个 Rust 实现是从 OpenClaw 的 TypeScript 扩展迁移而来：

- `/root/.openclaw/extensions/wecom/` → `/root/newclaw/src/channels/wecom/`

### 迁移对照表

| TypeScript | Rust |
|------------|------|
| `WeComConfig` | `WeComConfig` |
| `getAccessToken()` | `get_access_token()` |
| `sendText()` | `send_text()` |
| `uploadMedia()` | `upload_media()` |
| `decryptMessage()` | `decrypt_message()` |
| `encryptMessage()` | `encrypt_message()` |
| `computeMsgSignature()` | `compute_msg_signature()` |
| `verifyWecomSignature()` | `verify_signature()` |

## 注意事项

1. **AccessToken 缓存**：客户端会自动缓存 AccessToken 并在过期前自动刷新
2. **消息加密**：企业微信要求消息使用 AES-256-CBC 加密
3. **签名验证**：所有 Webhook 回调都需要验证签名
4. **媒体有效期**：上传的临时素材有效期为 3 天
5. **文本限制**：单条文本消息最大 2048 字节

## 企业微信 API 文档

- [企业微信开发文档](https://developer.work.weixin.qq.com/document/)
- [消息推送](https://developer.work.weixin.qq.com/document/path/90236)
- [接收消息](https://developer.work.weixin.qq.com/document/path/90238)
- [加解密库](https://developer.work.weixin.qq.com/document/path/90306)
