// Dashboard 会话管理
//
// 管理用户会话和认证

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 用户会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// 会话管理器
pub struct SessionManager {
    sessions: std::collections::HashMap<String, Session>,
    session_timeout_secs: u64,
}

impl SessionManager {
    pub fn new(session_timeout_secs: u64) -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
            session_timeout_secs,
        }
    }
    
    /// 创建新会话
    pub fn create_session(&mut self, user_id: String, ip: Option<String>, user_agent: Option<String>) -> Session {
        let now = Utc::now();
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(self.session_timeout_secs as i64),
            last_activity: now,
            ip_address: ip,
            user_agent,
        };
        
        self.sessions.insert(session.id.clone(), session.clone());
        session
    }
    
    /// 验证会话
    pub fn validate_session(&mut self, session_id: &str) -> Option<Session> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            let now = Utc::now();
            
            // 检查是否过期
            if session.expires_at < now {
                self.sessions.remove(session_id);
                return None;
            }
            
            // 更新最后活动时间
            session.last_activity = now;
            session.expires_at = now + chrono::Duration::seconds(self.session_timeout_secs as i64);
            
            return Some(session.clone());
        }
        
        None
    }
    
    /// 销毁会话
    pub fn destroy_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
    
    /// 清理过期会话
    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.sessions.retain(|_, session| session.expires_at > now);
    }
    
    /// 获取活跃会话数
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub session_id: String,
    pub user: UserInfo,
    pub expires_at: DateTime<Utc>,
}

/// 用户信息（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_manager() {
        let mut manager = SessionManager::new(3600);
        
        // 创建会话
        let session = manager.create_session("user1".to_string(), None, None);
        assert_eq!(session.user_id, "user1");
        
        // 验证会话
        let validated = manager.validate_session(&session.id);
        assert!(validated.is_some());
        
        // 销毁会话
        manager.destroy_session(&session.id);
        let validated = manager.validate_session(&session.id);
        assert!(validated.is_none());
    }
}
