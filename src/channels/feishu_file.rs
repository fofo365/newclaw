// NewClaw v0.4.0 - 飞书文件管理
//
// 核心功能：
// 1. 文件上传（支持多种文件类型）
// 2. 文件下载
// 3. 图片上传/下载
// 4. 文件信息查询
// 5. 临时文件 URL 生成

use anyhow::{Context, Result};
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};

/// 飞书文件客户端
pub struct FeishuFileClient {
    client: Client,
    base_url: String,
    app_id: String,
    app_secret: String,
    access_token: Option<String>,
}

/// 文件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    /// 流式文件（stream）
    Stream,
    /// 日志文件（log）
    Log,
    /// 其他文件（file）
    File,
    /// 图片（image）
    Image,
}

/// 文件上传结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    /// 文件 key
    pub file_key: String,
    /// 文件 token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_token: Option<String>,
    /// 文件大小（字节）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    /// 过期时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

/// 图片上传结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUploadResult {
    /// 图片 key
    pub image_key: String,
    /// 图片 token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_token: Option<String>,
    /// 过期时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件 key
    pub file_key: String,
    /// 文件名
    pub name: String,
    /// 文件大小（字节）
    pub size: i64,
    /// 文件类型（MIME type）
    #[serde(rename = "type")]
    pub file_type: String,
    /// 创建时间（Unix 时间戳）
    pub create_time: i64,
    /// 修改时间（Unix 时间戳）
    pub modify_time: i64,
    /// 文件 token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_token: Option<String>,
    /// 临时下载 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmp_url: Option<String>,
    /// URL 过期时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmp_url_expire: Option<i64>,
}

/// 图片下载结果
#[derive(Debug, Clone)]
pub struct DownloadedImage {
    /// 图片数据
    pub data: Vec<u8>,
    /// 图片类型（MIME type）
    pub content_type: String,
    /// 文件名
    pub filename: String,
}

/// 文件下载结果
#[derive(Debug, Clone)]
pub struct DownloadedFile {
    /// 文件数据
    pub data: Vec<u8>,
    /// 文件类型（MIME type）
    pub content_type: String,
    /// 文件名
    pub filename: String,
    /// 文件大小
    pub size: u64,
}

impl FeishuFileClient {
    /// 创建新的文件客户端
    pub fn new(app_id: String, app_secret: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://open.feishu.cn/open-apis".to_string(),
            app_id,
            app_secret,
            access_token: None,
        }
    }
    
    /// 设置基础 URL（用于测试）
    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
    
    /// 确保访问令牌有效
    pub async fn ensure_token(&mut self) -> Result<()> {
        if self.access_token.is_some() {
            return Ok(());
        }
        
        self.refresh_token().await
    }
    
    /// 刷新访问令牌
    async fn refresh_token(&mut self) -> Result<()> {
        let url = format!("{}/auth/v3/tenant_access_token/internal", self.base_url);
        
        let request_body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret,
        });
        
        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to request access token")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse token response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get access token: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get access token: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let token = json["tenant_access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No token in response"))?
            .to_string();
        
        self.access_token = Some(token);
        info!("Successfully obtained access token");
        
        Ok(())
    }
    
    /// 上传文件（从文件路径）
    ///
    /// # 参数
    /// - `file_path`: 文件路径
    /// - `file_type`: 文件类型（stream/log/file）
    /// - `file_name`: 文件名（可选，不提供则使用文件路径中的文件名）
    ///
    /// # 返回
    /// - `UploadResult`: 包含 file_key 等信息
    pub async fn upload_file(
        &mut self,
        file_path: &str,
        file_type: FileType,
        file_name: Option<&str>,
    ) -> Result<UploadResult> {
        self.ensure_token().await?;
        
        let path = Path::new(file_path);
        let filename = file_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });
        
        // 读取文件
        let mut file = File::open(file_path)
            .await
            .context(format!("Failed to open file: {}", file_path))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .context("Failed to read file")?;
        
        debug!("Uploading file: {} ({} bytes)", filename, buffer.len());
        
        self.upload_file_bytes(&buffer, &filename, file_type).await
    }
    
    /// 上传文件（从字节数据）
    ///
    /// # 参数
    /// - `data`: 文件数据
    /// - `filename`: 文件名
    /// - `file_type`: 文件类型
    ///
    /// # 返回
    /// - `UploadResult`: 包含 file_key 等信息
    pub async fn upload_file_bytes(
        &mut self,
        data: &[u8],
        filename: &str,
        file_type: FileType,
    ) -> Result<UploadResult> {
        self.ensure_token().await?;
        
        let url = format!("{}/drive/v1/medias/upload_all", self.base_url);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let file_type_str = match file_type {
            FileType::Stream => "stream",
            FileType::Log => "log",
            FileType::File => "file",
            FileType::Image => "file", // 图片也用 file 类型上传
        };
        
        // 构建 multipart 表单
        let form = multipart::Form::new()
            .text("file_name", filename.to_string())
            .text("file_type", file_type_str.to_string())
            .text("parent_type", "ccm_import_file".to_string())
            .part("file", multipart::Part::bytes(data.to_vec())
                .file_name(filename.to_string()));
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await
            .context("Failed to upload file")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse upload response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to upload file: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to upload file: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let result: UploadResult = serde_json::from_value(json["data"].clone())
            .context("Failed to parse upload result")?;
        
        info!("Successfully uploaded file: {} -> {}", filename, result.file_key);
        
        Ok(result)
    }
    
    /// 上传图片（从文件路径）
    ///
    /// # 参数
    /// - `image_path`: 图片文件路径
    ///
    /// # 返回
    /// - `ImageUploadResult`: 包含 image_key 等信息
    pub async fn upload_image(&mut self, image_path: &str) -> Result<ImageUploadResult> {
        self.ensure_token().await?;
        
        let path = Path::new(image_path);
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image")
            .to_string();
        
        // 读取图片文件
        let mut file = File::open(image_path)
            .await
            .context(format!("Failed to open image file: {}", image_path))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .context("Failed to read image file")?;
        
        debug!("Uploading image: {} ({} bytes)", filename, buffer.len());
        
        self.upload_image_bytes(&buffer, &filename).await
    }
    
    /// 上传图片（从字节数据）
    ///
    /// # 参数
    /// - `data`: 图片数据
    /// - `filename`: 文件名
    ///
    /// # 返回
    /// - `ImageUploadResult`: 包含 image_key 等信息
    pub async fn upload_image_bytes(
        &mut self,
        data: &[u8],
        filename: &str,
    ) -> Result<ImageUploadResult> {
        self.ensure_token().await?;
        
        let url = format!("{}/im/v1/images", self.base_url);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        // 构建 multipart 表单
        let form = multipart::Form::new()
            .text("image_type", "message".to_string())
            .part("image", multipart::Part::bytes(data.to_vec())
                .file_name(filename.to_string()));
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await
            .context("Failed to upload image")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse image upload response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to upload image: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to upload image: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let result: ImageUploadResult = serde_json::from_value(json["data"].clone())
            .context("Failed to parse image upload result")?;
        
        info!("Successfully uploaded image: {} -> {}", filename, result.image_key);
        
        Ok(result)
    }
    
    /// 下载文件
    ///
    /// # 参数
    /// - `file_key`: 文件 key
    ///
    /// # 返回
    /// - `DownloadedFile`: 包含文件数据和信息
    pub async fn download_file(&mut self, file_key: &str) -> Result<DownloadedFile> {
        self.ensure_token().await?;
        
        let url = format!("{}/drive/v1/medias/{}/download", self.base_url, file_key);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Downloading file: {}", file_key);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to download file")?;
        
        if !response.status().is_success() {
            error!("Failed to download file: HTTP {}", response.status());
            return Err(anyhow::anyhow!(
                "Failed to download file: HTTP {}",
                response.status()
            ));
        }
        
        // 提取文件名和内容类型
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        
        let filename = response
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                // 解析 Content-Disposition: attachment; filename="xxx"
                v.split("filename=")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string())
            })
            .unwrap_or_else(|| format!("file_{}", file_key));
        
        let data = response
            .bytes()
            .await
            .context("Failed to read file data")?
            .to_vec();
        
        let size = data.len() as u64;
        
        info!("Successfully downloaded file: {} ({} bytes)", filename, size);
        
        Ok(DownloadedFile {
            data,
            content_type,
            filename,
            size,
        })
    }
    
    /// 下载图片
    ///
    /// # 参数
    /// - `image_key`: 图片 key
    ///
    /// # 返回
    /// - `DownloadedImage`: 包含图片数据和信息
    pub async fn download_image(&mut self, image_key: &str) -> Result<DownloadedImage> {
        self.ensure_token().await?;
        
        let url = format!("{}/im/v1/images/{}", self.base_url, image_key);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Downloading image: {}", image_key);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to download image")?;
        
        if !response.status().is_success() {
            error!("Failed to download image: HTTP {}", response.status());
            return Err(anyhow::anyhow!(
                "Failed to download image: HTTP {}",
                response.status()
            ));
        }
        
        // 提取文件名和内容类型
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        
        let filename = response
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.split("filename=")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string())
            })
            .unwrap_or_else(|| format!("image_{}.png", image_key));
        
        let data = response
            .bytes()
            .await
            .context("Failed to read image data")?
            .to_vec();
        
        info!("Successfully downloaded image: {} ({} bytes)", filename, data.len());
        
        Ok(DownloadedImage {
            data,
            content_type,
            filename,
        })
    }
    
    /// 获取文件信息
    ///
    /// # 参数
    /// - `file_key`: 文件 key
    ///
    /// # 返回
    /// - `FileInfo`: 文件详细信息
    pub async fn get_file_info(&mut self, file_key: &str) -> Result<FileInfo> {
        self.ensure_token().await?;
        
        let url = format!("{}/drive/v1/medias/{}", self.base_url, file_key);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Getting file info: {}", file_key);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get file info")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse file info response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get file info: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get file info: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let info: FileInfo = serde_json::from_value(json["data"].clone())
            .context("Failed to parse file info")?;
        
        debug!("Successfully got file info: {}", file_key);
        
        Ok(info)
    }
    
    /// 获取临时下载 URL
    ///
    /// # 参数
    /// - `file_key`: 文件 key
    /// - `extra`: 额外参数（如 {"deadline": 1234567890}）
    ///
    /// # 返回
    /// - 临时下载 URL
    pub async fn get_temporary_url(
        &mut self,
        file_key: &str,
        extra: Option<serde_json::Value>,
    ) -> Result<String> {
        self.ensure_token().await?;
        
        let url = format!("{}/drive/v1/medias/{}/temporary_url", self.base_url, file_key);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Getting temporary URL for file: {}", file_key);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&extra.unwrap_or(serde_json::json!({})))
            .send()
            .await
            .context("Failed to get temporary URL")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse temporary URL response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get temporary URL: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get temporary URL: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let tmp_url = json["data"]["tmp_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No temporary URL in response"))?
            .to_string();
        
        info!("Successfully got temporary URL for file: {}", file_key);
        
        Ok(tmp_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_type_serialization() {
        let file_type = FileType::Stream;
        let json = serde_json::to_string(&file_type).unwrap();
        assert_eq!(json, "\"stream\"");
        
        let file_type = FileType::File;
        let json = serde_json::to_string(&file_type).unwrap();
        assert_eq!(json, "\"file\"");
    }
    
    #[test]
    fn test_upload_result_deserialization() {
        let json = serde_json::json!({
            "file_key": "test_file_key",
            "file_token": "test_token",
            "size": 1024,
            "expires_in": 3600
        });
        
        let result: UploadResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.file_key, "test_file_key");
        assert_eq!(result.file_token, Some("test_token".to_string()));
        assert_eq!(result.size, Some(1024));
    }
    
    #[test]
    fn test_image_upload_result_deserialization() {
        let json = serde_json::json!({
            "image_key": "test_image_key",
            "image_token": "test_token",
            "expires_in": 3600
        });
        
        let result: ImageUploadResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.image_key, "test_image_key");
        assert_eq!(result.image_token, Some("test_token".to_string()));
    }
    
    #[test]
    fn test_file_info_deserialization() {
        let json = serde_json::json!({
            "file_key": "test_file_key",
            "name": "test.txt",
            "size": 1024,
            "type": "text/plain",
            "create_time": 1234567890,
            "modify_time": 1234567890,
            "file_token": "test_token",
            "tmp_url": "https://example.com/file",
            "tmp_url_expire": 1234567890
        });
        
        let info: FileInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.file_key, "test_file_key");
        assert_eq!(info.name, "test.txt");
        assert_eq!(info.size, 1024);
        assert_eq!(info.file_type, "text/plain");
    }
}
