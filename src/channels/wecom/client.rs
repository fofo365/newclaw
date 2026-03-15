// NewClaw v0.4.0 - 企业微信（WeCom）API 客户端
//
// 核心功能：
// 1. AccessToken 管理（缓存、自动刷新）
// 2. 消息发送（文本、图片、文件）
// 3. 媒体上传/下载

use anyhow::{anyhow, Result};
use reqwest::{multipart, Client};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::types::*;

/// Token 缓存
struct TokenCache {
    token: String,
    expires_at: Instant,
}

/// WeCom API 客户端
pub struct WeComClient {
    config: WeComConfig,
    http: Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl WeComClient {
    /// 创建新的 WeCom 客户端
    pub fn new(config: WeComConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_millis(limits::REQUEST_TIMEOUT_MS))
            .user_agent("NewClaw/0.4.0")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            config,
            http,
            token_cache: Arc::new(RwLock::new(None)),
        }
    }
    
    /// 获取 AccessToken
    ///
    /// 自动管理缓存和过期刷新
    pub async fn get_access_token(&self) -> Result<String> {
        // 检查缓存
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                let buffer = Duration::from_millis(limits::TOKEN_REFRESH_BUFFER_MS as u64);
                if cached.expires_at > Instant::now() + buffer {
                    return Ok(cached.token.clone());
                }
            }
        }
        
        // 刷新 token
        self.refresh_access_token().await
    }
    
    /// 强制刷新 AccessToken
    async fn refresh_access_token(&self) -> Result<String> {
        let url = format!(
            "{}?corpid={}&corpsecret={}",
            api::GET_TOKEN,
            urlencoding::encode(&self.config.corp_id),
            urlencoding::encode(&self.config.corp_secret)
        );
        
        let response = self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        // 检查错误
        if let Some(errcode) = json.get("errcode").and_then(|v| v.as_i64()) {
            if errcode != 0 {
                let errmsg = json.get("errmsg").and_then(|v| v.as_str()).unwrap_or("unknown");
                return Err(anyhow!("Get token failed: {} - {}", errcode, errmsg));
            }
        }
        
        let access_token = json.get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing access_token in response"))?;
        
        let expires_in = json.get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(7200);
        
        // 更新缓存
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: access_token.to_string(),
                expires_at: Instant::now() + Duration::from_secs(expires_in as u64),
            });
        }
        
        Ok(access_token.to_string())
    }
    
    /// 发送文本消息
    pub async fn send_text(&self, to: &MessageTarget, text: &str) -> Result<SendMessageResponse> {
        let token = self.get_access_token().await?;
        let url = format!("{}?access_token={}", api::SEND_MESSAGE, urlencoding::encode(&token));
        
        let body = json!({
            "touser": to.touser,
            "toparty": to.toparty,
            "totag": to.totag,
            "msgtype": "text",
            "agentid": self.config.agent_id,
            "text": {
                "content": text
            }
        });
        
        let response = self.http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let result: SendMessageResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        if result.errcode != 0 {
            return Err(anyhow!("Send message failed: {} - {}", result.errcode, result.errmsg));
        }
        
        Ok(result)
    }
    
    /// 发送图片消息
    pub async fn send_image(&self, to: &MessageTarget, media_id: &str) -> Result<SendMessageResponse> {
        let token = self.get_access_token().await?;
        let url = format!("{}?access_token={}", api::SEND_MESSAGE, urlencoding::encode(&token));
        
        let body = json!({
            "touser": to.touser,
            "toparty": to.toparty,
            "totag": to.totag,
            "msgtype": "image",
            "agentid": self.config.agent_id,
            "image": {
                "media_id": media_id
            }
        });
        
        let response = self.http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let result: SendMessageResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        if result.errcode != 0 {
            return Err(anyhow!("Send image failed: {} - {}", result.errcode, result.errmsg));
        }
        
        Ok(result)
    }
    
    /// 发送文件消息
    pub async fn send_file(&self, to: &MessageTarget, media_id: &str) -> Result<SendMessageResponse> {
        let token = self.get_access_token().await?;
        let url = format!("{}?access_token={}", api::SEND_MESSAGE, urlencoding::encode(&token));
        
        let body = json!({
            "touser": to.touser,
            "toparty": to.toparty,
            "totag": to.totag,
            "msgtype": "file",
            "agentid": self.config.agent_id,
            "file": {
                "media_id": media_id
            }
        });
        
        let response = self.http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let result: SendMessageResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        if result.errcode != 0 {
            return Err(anyhow!("Send file failed: {} - {}", result.errcode, result.errmsg));
        }
        
        Ok(result)
    }
    
    /// 发送视频消息
    pub async fn send_video(
        &self,
        to: &MessageTarget,
        media_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> Result<SendMessageResponse> {
        let token = self.get_access_token().await?;
        let url = format!("{}?access_token={}", api::SEND_MESSAGE, urlencoding::encode(&token));
        
        let body = json!({
            "touser": to.touser,
            "toparty": to.toparty,
            "totag": to.totag,
            "msgtype": "video",
            "agentid": self.config.agent_id,
            "video": {
                "media_id": media_id,
                "title": title,
                "description": description
            }
        });
        
        let response = self.http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let result: SendMessageResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        if result.errcode != 0 {
            return Err(anyhow!("Send video failed: {} - {}", result.errcode, result.errmsg));
        }
        
        Ok(result)
    }
    
    /// 上传媒体文件
    pub async fn upload_media(
        &self,
        media_type: MediaType,
        filename: &str,
        data: Vec<u8>,
    ) -> Result<UploadMediaResponse> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}?access_token={}&type={}",
            api::UPLOAD_MEDIA,
            urlencoding::encode(&token),
            media_type.as_str()
        );
        
        let part = multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(guess_mime_type(filename))
            .unwrap_or_else(|_| multipart::Part::bytes(vec![]).file_name(filename.to_string()));
        
        let form = multipart::Form::new()
            .part("media", part);
        
        let response = self.http
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let result: UploadMediaResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        if result.errcode != 0 {
            return Err(anyhow!("Upload media failed: {} - {}", result.errcode, result.errmsg));
        }
        
        Ok(result)
    }
    
    /// 下载媒体文件
    pub async fn download_media(&self, media_id: &str) -> Result<DownloadMediaResponse> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}?access_token={}&media_id={}",
            api::DOWNLOAD_MEDIA,
            urlencoding::encode(&token),
            urlencoding::encode(media_id)
        );
        
        let response = self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        
        // 检查是否是错误 JSON
        if content_type.contains("application/json") {
            let json: ErrorResponse = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse error response: {}", e))?;
            return Err(anyhow!("Download failed: {} - {}", json.errcode, json.errmsg));
        }
        
        // 提取文件名
        let filename = response
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                // 解析 filename="xxx" 或 filename=xxx
                let re = regex::Regex::new(r#"filename[*]?=["']?([^"';\s]+)["']?"#).ok()?;
                re.captures(s)
                    .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            });
        
        let buffer = response
            .bytes()
            .await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))?
            .to_vec();
        
        Ok(DownloadMediaResponse {
            buffer,
            content_type,
            filename,
        })
    }
    
    /// 获取配置（用于测试）
    pub fn config(&self) -> &WeComConfig {
        &self.config
    }
}

/// 根据文件名猜测 MIME 类型
fn guess_mime_type(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "amr" => "audio/amr",
        "mp4" => "video/mp4",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type("test.jpg"), "image/jpeg");
        assert_eq!(guess_mime_type("test.png"), "image/png");
        assert_eq!(guess_mime_type("test.mp4"), "video/mp4");
        assert_eq!(guess_mime_type("test.unknown"), "application/octet-stream");
    }
}
