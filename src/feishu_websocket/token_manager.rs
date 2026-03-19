// 飞书 Token 管理器 - v0.7.0
//
// 实现自动刷新机制：
// 1. Token 状态管理（记录过期时间）
// 2. 定时检查（在过期前刷新）
// 3. 自动续期机制
// 4. 支持 Tenant Access Token 和 User Access Token

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};

/// Token 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenType {
    /// 应用级别的 Tenant Access Token
    TenantAccessToken,
    /// 用户级别的 User Access Token
    UserAccessToken,
}

impl Default for TokenType {
    fn default() -> Self {
        Self::TenantAccessToken
    }
}

/// Token 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenState {
    /// Token 值
    pub token: String,
    /// Token 类型
    pub token_type: TokenType,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后刷新时间
    pub last_refreshed_at: DateTime<Utc>,
    /// 刷新次数
    pub refresh_count: u32,
    /// User Access Token 的刷新令牌（可选）
    pub refresh_token: Option<String>,
}

impl TokenState {
    /// 检查是否即将过期（默认提前5分钟）
    pub fn is_expiring_soon(&self, margin_secs: i64) -> bool {
        let now = Utc::now();
        let margin = chrono::Duration::seconds(margin_secs);
        self.expires_at <= now + margin
    }

    /// 检查是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// 获取剩余有效时间（秒）
    pub fn remaining_secs(&self) -> i64 {
        let now = Utc::now();
        (self.expires_at - now).num_seconds().max(0)
    }

    /// 获取刷新令牌（仅 UAT）
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }
}

/// Token 管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenManagerConfig {
    /// 应用 ID
    pub app_id: String,
    /// 应用密钥
    pub app_secret: String,
    /// Token 类型
    pub token_type: TokenType,
    /// 刷新提前量（秒，默认300秒=5分钟）
    pub refresh_margin_secs: i64,
    /// 检查间隔（秒，默认60秒）
    pub check_interval_secs: u64,
    /// 最大刷新失败次数
    pub max_refresh_failures: u32,
    /// 刷新失败后的重试间隔（秒）
    pub retry_interval_secs: u64,
}

impl Default for TokenManagerConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            token_type: TokenType::TenantAccessToken,
            refresh_margin_secs: 300, // 提前5分钟刷新
            check_interval_secs: 60,  // 每60秒检查一次
            max_refresh_failures: 5,
            retry_interval_secs: 30,
        }
    }
}

/// Token 管理器
pub struct TokenManager {
    /// 配置
    config: TokenManagerConfig,
    /// 当前 Token 状态
    state: Arc<RwLock<Option<TokenState>>>,
    /// HTTP 客户端
    client: reqwest::Client,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
    /// 刷新失败次数
    refresh_failures: Arc<RwLock<u32>>,
}

impl TokenManager {
    /// 创建新的 Token 管理器
    pub fn new(config: TokenManagerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(None)),
            client: reqwest::Client::new(),
            running: Arc::new(RwLock::new(false)),
            refresh_failures: Arc::new(RwLock::new(0)),
        }
    }

    /// 获取当前 Token
    pub async fn get_token(&self) -> Option<String> {
        let state = self.state.read().await;
        state.as_ref().map(|s| s.token.clone())
    }

    /// 获取 Token 状态
    pub async fn get_state(&self) -> Option<TokenState> {
        let state = self.state.read().await;
        state.clone()
    }

    /// 初始化 Token（首次获取）
    pub async fn initialize(&self) -> anyhow::Result<()> {
        info!("初始化飞书 Token...");
        self.refresh_token().await?;
        Ok(())
    }

    /// 刷新 Token
    pub async fn refresh_token(&self) -> anyhow::Result<()> {
        match self.config.token_type {
            TokenType::TenantAccessToken => self.refresh_tenant_token().await,
            TokenType::UserAccessToken => self.refresh_user_token().await,
        }
    }

    /// 刷新 Tenant Access Token
    async fn refresh_tenant_token(&self) -> anyhow::Result<()> {
        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";

        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret,
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            error!("获取 Tenant Access Token 失败: {} - {}", status, body);
            return Err(anyhow::anyhow!("Failed to get tenant access token: {}", status));
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        
        if json["code"].as_i64() != Some(0) {
            let msg = json["msg"].as_str().unwrap_or("Unknown error");
            error!("飞书 API 错误: {}", msg);
            return Err(anyhow::anyhow!("Feishu API error: {}", msg));
        }

        let token = json["tenant_access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No token in response"))?
            .to_string();

        let expires_in = json["expire"]
            .as_i64()
            .unwrap_or(7200); // 默认2小时

        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(expires_in);

        let state = TokenState {
            token,
            token_type: TokenType::TenantAccessToken,
            expires_at,
            created_at: now,
            last_refreshed_at: now,
            refresh_count: 0,
            refresh_token: None,
        };

        // 更新状态
        {
            let mut current = self.state.write().await;
            *current = Some(state.clone());
        }

        // 重置失败计数
        {
            let mut failures = self.refresh_failures.write().await;
            *failures = 0;
        }

        info!(
            "Tenant Access Token 已刷新，有效期至 {} (剩余 {} 秒)",
            expires_at.format("%Y-%m-%d %H:%M:%S"),
            state.remaining_secs()
        );

        Ok(())
    }

    /// 刷新 User Access Token（使用 refresh_token）
    async fn refresh_user_token(&self) -> anyhow::Result<()> {
        let current_state = self.state.read().await;
        
        // 检查是否有 refresh_token
        let refresh_token = current_state
            .as_ref()
            .and_then(|s| s.refresh_token.clone())
            .ok_or_else(|| anyhow::anyhow!("No refresh token available"))?;

        drop(current_state);

        let url = "https://open.feishu.cn/open-apis/authen/v1/refresh_access_token";

        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            error!("刷新 User Access Token 失败: {} - {}", status, body);
            return Err(anyhow::anyhow!("Failed to refresh user access token: {}", status));
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        
        let data = json.get("data")
            .ok_or_else(|| anyhow::anyhow!("No data in response"))?;

        let token = data["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No access_token in response"))?
            .to_string();

        let new_refresh_token = data["refresh_token"]
            .as_str()
            .map(|s| s.to_string());

        let expires_in = data["expires_in"]
            .as_i64()
            .unwrap_or(7200);

        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(expires_in);

        let mut refresh_count = 0;
        {
            let current = self.state.read().await;
            if let Some(ref s) = *current {
                refresh_count = s.refresh_count + 1;
            }
        }

        let state = TokenState {
            token,
            token_type: TokenType::UserAccessToken,
            expires_at,
            created_at: now,
            last_refreshed_at: now,
            refresh_count,
            refresh_token: new_refresh_token,
        };

        {
            let mut current = self.state.write().await;
            *current = Some(state.clone());
        }

        {
            let mut failures = self.refresh_failures.write().await;
            *failures = 0;
        }

        info!(
            "User Access Token 已刷新 (第 {} 次)，有效期至 {}",
            refresh_count,
            expires_at.format("%Y-%m-%d %H:%M:%S")
        );

        Ok(())
    }

    /// 启动自动刷新任务
    pub async fn start_auto_refresh(&self) {
        let running = Arc::clone(&self.running);
        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let refresh_failures = Arc::clone(&self.refresh_failures);
        let client = self.client.clone();

        // 标记为运行中
        {
            let mut r = running.write().await;
            *r = true;
        }

        tokio::spawn(async move {
            let check_interval = Duration::from_secs(config.check_interval_secs);
            
            info!("Token 自动刷新任务已启动，检查间隔: {}秒", config.check_interval_secs);

            loop {
                // 检查是否仍在运行
                {
                    let r = running.read().await;
                    if !*r {
                        info!("Token 自动刷新任务已停止");
                        break;
                    }
                }

                // 检查 Token 状态
                let should_refresh = {
                    let current = state.read().await;
                    match current.as_ref() {
                        Some(s) => {
                            // 即将过期或已过期
                            s.is_expiring_soon(config.refresh_margin_secs) || s.is_expired()
                        }
                        None => true, // 没有 Token，需要获取
                    }
                };

                if should_refresh {
                    info!("Token 即将过期或已过期，正在刷新...");
                    
                    // 尝试刷新
                    let result = match config.token_type {
                        TokenType::TenantAccessToken => {
                            Self::refresh_tenant_token_static(
                                &client,
                                &config.app_id,
                                &config.app_secret,
                            ).await
                        }
                        TokenType::UserAccessToken => {
                            // UAT 需要 refresh_token
                            let current = state.read().await;
                            if let Some(ref s) = *current {
                                if let Some(ref rt) = s.refresh_token {
                                    Self::refresh_user_token_static(&client, rt).await
                                } else {
                                    Err(anyhow::anyhow!("No refresh token"))
                                }
                            } else {
                                Err(anyhow::anyhow!("No current token state"))
                            }
                        }
                    };

                    match result {
                        Ok(new_state) => {
                            // 更新状态
                            {
                                let mut current = state.write().await;
                                *current = Some(new_state);
                            }
                            // 重置失败计数
                            {
                                let mut failures = refresh_failures.write().await;
                                *failures = 0;
                            }
                            info!("Token 自动刷新成功");
                        }
                        Err(e) => {
                            error!("Token 自动刷新失败: {}", e);
                            // 增加失败计数
                            let failures = {
                                let mut f = refresh_failures.write().await;
                                *f += 1;
                                *f
                            };

                            if failures >= config.max_refresh_failures {
                                error!("Token 刷新失败次数达到上限 ({} 次)，请检查配置", failures);
                            }
                        }
                    }
                }

                // 等待下一次检查
                tokio::time::sleep(check_interval).await;
            }
        });
    }

    /// 停止自动刷新
    pub async fn stop_auto_refresh(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Token 自动刷新任务已请求停止");
    }

    /// 静态方法：刷新 Tenant Access Token
    async fn refresh_tenant_token_static(
        client: &reqwest::Client,
        app_id: &str,
        app_secret: &str,
    ) -> anyhow::Result<TokenState> {
        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "app_id": app_id,
                "app_secret": app_secret,
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("Failed to get token: {}", status));
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        
        if json["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!("API error: {}", json["msg"].as_str().unwrap_or("unknown")));
        }

        let token = json["tenant_access_token"].as_str().unwrap().to_string();
        let expires_in = json["expire"].as_i64().unwrap_or(7200);

        let now = Utc::now();
        Ok(TokenState {
            token,
            token_type: TokenType::TenantAccessToken,
            expires_at: now + chrono::Duration::seconds(expires_in),
            created_at: now,
            last_refreshed_at: now,
            refresh_count: 0,
            refresh_token: None,
        })
    }

    /// 静态方法：刷新 User Access Token
    async fn refresh_user_token_static(
        client: &reqwest::Client,
        refresh_token: &str,
    ) -> anyhow::Result<TokenState> {
        let url = "https://open.feishu.cn/open-apis/authen/v1/refresh_access_token";

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("Failed to refresh token: {}", status));
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        let data = json.get("data").ok_or_else(|| anyhow::anyhow!("No data"))?;

        let token = data["access_token"].as_str().unwrap().to_string();
        let new_refresh_token = data["refresh_token"].as_str().map(|s| s.to_string());
        let expires_in = data["expires_in"].as_i64().unwrap_or(7200);

        let now = Utc::now();
        Ok(TokenState {
            token,
            token_type: TokenType::UserAccessToken,
            expires_at: now + chrono::Duration::seconds(expires_in),
            created_at: now,
            last_refreshed_at: now,
            refresh_count: 0,
            refresh_token: new_refresh_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_state() {
        let now = Utc::now();
        let state = TokenState {
            token: "test_token".to_string(),
            token_type: TokenType::TenantAccessToken,
            expires_at: now + chrono::Duration::hours(2),
            created_at: now,
            last_refreshed_at: now,
            refresh_count: 0,
            refresh_token: None,
        };

        assert!(!state.is_expired());
        assert!(!state.is_expiring_soon(300));
        assert!(state.remaining_secs() > 7000);
    }

    #[test]
    fn test_token_expiring_soon() {
        let now = Utc::now();
        let state = TokenState {
            token: "test_token".to_string(),
            token_type: TokenType::TenantAccessToken,
            expires_at: now + chrono::Duration::seconds(180), // 3分钟后过期
            created_at: now,
            last_refreshed_at: now,
            refresh_count: 0,
            refresh_token: None,
        };

        assert!(state.is_expiring_soon(300)); // 5分钟内过期
        assert!(!state.is_expired());
    }
}