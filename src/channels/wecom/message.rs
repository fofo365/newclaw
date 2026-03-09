// NewClaw v0.4.0 - 企业微信（WeCom）消息客户端
//
// 高级消息发送接口：
// 1. 文本消息（支持长文本自动分片）
// 2. 图片消息
// 3. 文件消息
// 4. Markdown 消息

use anyhow::Result;

use super::client::WeComClient;
use super::types::*;

/// WeCom 消息客户端
pub struct WeComMessageClient {
    client: WeComClient,
}

impl WeComMessageClient {
    /// 创建新的消息客户端
    pub fn new(client: WeComClient) -> Self {
        Self { client }
    }
    
    /// 从配置创建
    pub fn from_config(config: WeComConfig) -> Self {
        Self::new(WeComClient::new(config))
    }
    
    /// 发送文本消息
    pub async fn send_text(&self, user_id: &str, text: &str) -> Result<SendMessageResponse> {
        let target = MessageTarget {
            touser: Some(user_id.to_string()),
            ..Default::default()
        };
        self.client.send_text(&target, text).await
    }
    
    /// 发送文本消息到部门
    pub async fn send_text_to_party(&self, party_id: &str, text: &str) -> Result<SendMessageResponse> {
        let target = MessageTarget {
            toparty: Some(party_id.to_string()),
            ..Default::default()
        };
        self.client.send_text(&target, text).await
    }
    
    /// 发送文本消息到标签
    pub async fn send_text_to_tag(&self, tag_id: &str, text: &str) -> Result<SendMessageResponse> {
        let target = MessageTarget {
            totag: Some(tag_id.to_string()),
            ..Default::default()
        };
        self.client.send_text(&target, text).await
    }
    
    /// 发送图片
    pub async fn send_image(&self, user_id: &str, media_id: &str) -> Result<SendMessageResponse> {
        let target = MessageTarget {
            touser: Some(user_id.to_string()),
            ..Default::default()
        };
        self.client.send_image(&target, media_id).await
    }
    
    /// 发送文件
    pub async fn send_file(&self, user_id: &str, media_id: &str) -> Result<SendMessageResponse> {
        let target = MessageTarget {
            touser: Some(user_id.to_string()),
            ..Default::default()
        };
        self.client.send_file(&target, media_id).await
    }
    
    /// 发送视频
    pub async fn send_video(
        &self,
        user_id: &str,
        media_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> Result<SendMessageResponse> {
        let target = MessageTarget {
            touser: Some(user_id.to_string()),
            ..Default::default()
        };
        self.client.send_video(&target, media_id, title, description).await
    }
    
    /// 上传并发送图片
    pub async fn upload_and_send_image(&self, user_id: &str, filename: &str, data: Vec<u8>) -> Result<SendMessageResponse> {
        // 上传图片
        let upload = self.client.upload_media(MediaType::Image, filename, data).await?;
        
        // 发送图片
        self.send_image(user_id, &upload.media_id).await
    }
    
    /// 上传并发送文件
    pub async fn upload_and_send_file(&self, user_id: &str, filename: &str, data: Vec<u8>) -> Result<SendMessageResponse> {
        // 上传文件
        let upload = self.client.upload_media(MediaType::File, filename, data).await?;
        
        // 发送文件
        self.send_file(user_id, &upload.media_id).await
    }
    
    /// 获取底层客户端
    pub fn client(&self) -> &WeComClient {
        &self.client
    }
    
    /// 获取可变客户端
    pub fn client_mut(&mut self) -> &mut WeComClient {
        &mut self.client
    }
}

/// 长文本分片器
pub fn chunk_text(text: &str, max_bytes: usize) -> Vec<String> {
    let bytes = text.as_bytes();
    if bytes.len() <= max_bytes {
        return vec![text.to_string()];
    }
    
    let mut chunks = Vec::new();
    let mut start = 0;
    
    while start < bytes.len() {
        let mut end = std::cmp::min(start + max_bytes, bytes.len());
        
        // 确保不在 UTF-8 字符中间切分
        while end < bytes.len() && !text.is_char_boundary(end) {
            end -= 1;
        }
        
        // 尝试在句子/段落边界切分
        if end < bytes.len() {
            let slice = &text[start..end];
            if let Some(last_newline) = slice.rfind('\n') {
                end = start + last_newline + 1;
            } else if let Some(last_space) = slice.rfind(' ') {
                end = start + last_space + 1;
            }
        }
        
        chunks.push(text[start..end].to_string());
        start = end;
    }
    
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_text_small() {
        let text = "Hello, world!";
        let chunks = chunk_text(text, 100);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }
    
    #[test]
    fn test_chunk_text_large() {
        let text = "This is a test. ".repeat(100);
        let chunks = chunk_text(&text, 100);
        assert!(chunks.len() > 1);
        
        // 验证所有块组合后等于原文
        let combined: String = chunks.join("");
        assert_eq!(combined, text);
    }
    
    #[test]
    fn test_chunk_text_utf8() {
        let text = "你好世界！这是一段中文测试文本。".repeat(20);
        let chunks = chunk_text(&text, 100);
        assert!(chunks.len() > 1);
        
        // 验证每个块都是有效的 UTF-8
        for chunk in &chunks {
            assert!(chunk.is_char_boundary(chunk.len()));
        }
        
        // 验证组合后等于原文
        let combined: String = chunks.join("");
        assert_eq!(combined, text);
    }
}
