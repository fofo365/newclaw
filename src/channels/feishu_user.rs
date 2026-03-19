// NewClaw v0.4.0 - 飞书用户和群组管理
//
// 核心功能：
// 1. 获取用户信息
// 2. 获取群组信息
// 3. 群组成员管理
// 4. 用户权限查询

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

/// 飞书用户管理客户端
pub struct FeishuUserClient {
    client: Client,
    base_url: String,
    app_id: String,
    app_secret: String,
    access_token: Option<String>,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// 用户 open_id
    pub open_id: String,
    /// 用户 user_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 用户 union_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub union_id: Option<String>,
    /// 用户姓名
    pub name: String,
    /// 用户英文名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en_name: Option<String>,
    /// 用户昵称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// 用户邮箱
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 用户手机号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    /// 用户性别（0: 未知, 1: 男, 2: 女）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<i32>,
    /// 用户头像 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 用户头像缩略图 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_thumb: Option<String>,
    /// 用户头像中图 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_middle: Option<String>,
    /// 用户头像大图 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_big: Option<String>,
    /// 用户职位
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// 用户部门 ID 列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department_ids: Option<Vec<String>>,
    /// 用户 Leader user_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_user_id: Option<String>,
    /// 用户城市
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// 用户国家
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// 用户工作台
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_station: Option<String>,
    /// 用户加入时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_time: Option<i64>,
    /// 用户是否激活
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    /// 用户状态（1: 激活, 2: 停用, 3: 未加入, 4: 离职）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<UserStatus>,
    /// 用户排序
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orders: Option<Vec<UserOrder>>,
    /// 自定义属性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_attrs: Option<Vec<UserCustomAttr>>,
}

/// 用户状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserStatus {
    #[serde(rename = "1")]
    Activated,
    #[serde(rename = "2")]
    Deactivated,
    #[serde(rename = "3")]
    NotJoined,
    #[serde(rename = "4")]
    Resigned,
}

/// 用户排序
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrder {
    /// 排序字段 ID
    pub order_id: String,
    /// 排序值
    pub order_value: i32,
}

/// 用户自定义属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCustomAttr {
    /// 属性 ID
    pub id: String,
    /// 属性值
    pub value: serde_json::Value,
}

/// 群组信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    /// 群组 ID
    pub chat_id: String,
    /// 群组名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 群组描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 群主 open_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    /// 群主 user_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id_type: Option<String>,
    /// 群组头像 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// 群组类型（private: 私密, public: 公开）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_mode: Option<String>,
    /// 群组类型（group: 群聊, p2p: 单聊）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_type: Option<String>,
    /// 群组是否置顶
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    /// 群组创建时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<i64>,
    /// 群组最后更新时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<i64>,
    /// 群组成员数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<i32>,
    /// 群组成员 ID 类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id_type: Option<String>,
}

/// 群组成员信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    /// 成员 ID
    pub member_id: String,
    /// 成员 ID 类型
    pub member_id_type: String,
    /// 成员名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 成员类型（user: 用户, bot: 机器人）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_type: Option<String>,
    /// 入群时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_time: Option<i64>,
    /// 成员角色（admin: 管理员, member: 成员）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_role: Option<String>,
}

/// 群组成员列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberList {
    /// 成员列表
    pub items: Vec<GroupMember>,
    /// 是否还有更多
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
    /// 分页 token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// 添加群组成员请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGroupMemberRequest {
    /// 群组 ID
    pub chat_id: String,
    /// 成员 ID 列表
    pub member_id_list: Vec<String>,
    /// 成员 ID 类型
    pub member_id_type: String,
}

/// 移除群组成员请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveGroupMemberRequest {
    /// 群组 ID
    pub chat_id: String,
    /// 成员 ID 列表
    pub member_id_list: Vec<String>,
    /// 成员 ID 类型
    pub member_id_type: String,
}

/// 权限信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionInfo {
    /// 权限 ID
    pub permission_id: String,
    /// 权限名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 权限描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否启用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl FeishuUserClient {
    /// 创建新的用户管理客户端
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
    
    /// 获取用户信息
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `user_id_type`: 用户 ID 类型（open_id/user_id/union_id）
    ///
    /// # 返回
    /// - `UserInfo`: 用户详细信息
    pub async fn get_user_info(
        &mut self,
        user_id: &str,
        user_id_type: &str,
    ) -> Result<UserInfo> {
        self.ensure_token().await?;
        
        let url = format!(
            "{}/contact/v3/users/{}?user_id_type={}",
            self.base_url, user_id, user_id_type
        );
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Getting user info: {} ({})", user_id, user_id_type);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get user info")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse user info response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get user info: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get user info: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let user_info: UserInfo = serde_json::from_value(json["data"]["user"].clone())
            .context("Failed to parse user info")?;
        
        info!("Successfully got user info: {}", user_info.name);
        
        Ok(user_info)
    }
    
    /// 批量获取用户信息
    ///
    /// # 参数
    /// - `user_ids`: 用户 ID 列表
    /// - `user_id_type`: 用户 ID 类型
    ///
    /// # 返回
    /// - 用户信息列表
    pub async fn get_users_info(
        &mut self,
        user_ids: &[&str],
        user_id_type: &str,
    ) -> Result<Vec<UserInfo>> {
        self.ensure_token().await?;
        
        let url = format!(
            "{}/contact/v3/users/batch?user_id_type={}",
            self.base_url, user_id_type
        );
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let request_body = serde_json::json!({
            "user_ids": user_ids
        });
        
        debug!("Batch getting user info: {:?}", user_ids);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_body)
            .send()
            .await
            .context("Failed to batch get user info")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse batch user info response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to batch get user info: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to batch get user info: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let users: Vec<UserInfo> = serde_json::from_value(json["data"]["users"].clone())
            .context("Failed to parse users info")?;
        
        info!("Successfully got {} users info", users.len());
        
        Ok(users)
    }
    
    /// 获取群组信息
    ///
    /// # 参数
    /// - `chat_id`: 群组 ID
    ///
    /// # 返回
    /// - `GroupInfo`: 群组详细信息
    pub async fn get_group_info(&mut self, chat_id: &str) -> Result<GroupInfo> {
        self.ensure_token().await?;
        
        let url = format!("{}/im/v1/chats/{}", self.base_url, chat_id);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Getting group info: {}", chat_id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get group info")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse group info response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get group info: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get group info: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let group_info: GroupInfo = serde_json::from_value(json["data"].clone())
            .context("Failed to parse group info")?;
        
        info!("Successfully got group info: {:?}", group_info.name);
        
        Ok(group_info)
    }
    
    /// 获取群组成员列表
    ///
    /// # 参数
    /// - `chat_id`: 群组 ID
    /// - `member_id_type`: 成员 ID 类型
    /// - `page_size`: 分页大小（可选）
    /// - `page_token`: 分页 token（可选）
    ///
    /// # 返回
    /// - `GroupMemberList`: 群组成员列表
    pub async fn get_group_members(
        &mut self,
        chat_id: &str,
        member_id_type: &str,
        page_size: Option<i32>,
        page_token: Option<&str>,
    ) -> Result<GroupMemberList> {
        self.ensure_token().await?;
        
        let mut url = format!(
            "{}/im/v1/chats/{}/members?member_id_type={}",
            self.base_url, chat_id, member_id_type
        );
        
        if let Some(size) = page_size {
            url = format!("{}&page_size={}", url, size);
        }
        
        if let Some(token) = page_token {
            url = format!("{}&page_token={}", url, token);
        }
        
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        debug!("Getting group members: {}", chat_id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .context("Failed to get group members")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse group members response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get group members: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get group members: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let member_list: GroupMemberList = serde_json::from_value(json["data"].clone())
            .context("Failed to parse group member list")?;
        
        info!("Successfully got {} group members", member_list.items.len());
        
        Ok(member_list)
    }
    
    /// 添加群组成员
    ///
    /// # 参数
    /// - `chat_id`: 群组 ID
    /// - `member_ids`: 成员 ID 列表
    /// - `member_id_type`: 成员 ID 类型
    ///
    /// # 返回
    /// - 是否成功
    pub async fn add_to_group(
        &mut self,
        chat_id: &str,
        member_ids: &[&str],
        member_id_type: &str,
    ) -> Result<()> {
        self.ensure_token().await?;
        
        let url = format!("{}/im/v1/chats/{}/members", self.base_url, chat_id);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let request_body = AddGroupMemberRequest {
            chat_id: chat_id.to_string(),
            member_id_list: member_ids.iter().map(|s| s.to_string()).collect(),
            member_id_type: member_id_type.to_string(),
        };
        
        debug!("Adding members to group: {} -> {:?}", chat_id, member_ids);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_body)
            .send()
            .await
            .context("Failed to add group members")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse add group members response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to add group members: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to add group members: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        info!("Successfully added {} members to group", member_ids.len());
        
        Ok(())
    }
    
    /// 移除群组成员
    ///
    /// # 参数
    /// - `chat_id`: 群组 ID
    /// - `member_ids`: 成员 ID 列表
    /// - `member_id_type`: 成员 ID 类型
    ///
    /// # 返回
    /// - 是否成功
    pub async fn remove_from_group(
        &mut self,
        chat_id: &str,
        member_ids: &[&str],
        member_id_type: &str,
    ) -> Result<()> {
        self.ensure_token().await?;
        
        let url = format!("{}/im/v1/chats/{}/members", self.base_url, chat_id);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let request_body = RemoveGroupMemberRequest {
            chat_id: chat_id.to_string(),
            member_id_list: member_ids.iter().map(|s| s.to_string()).collect(),
            member_id_type: member_id_type.to_string(),
        };
        
        debug!("Removing members from group: {} -> {:?}", chat_id, member_ids);
        
        let response = self.client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_body)
            .send()
            .await
            .context("Failed to remove group members")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse remove group members response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to remove group members: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to remove group members: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        info!("Successfully removed {} members from group", member_ids.len());
        
        Ok(())
    }
    
    /// 检查用户权限
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `permission_id`: 权限 ID
    /// - `user_id_type`: 用户 ID 类型
    ///
    /// # 返回
    /// - 是否有权限
    pub async fn check_user_permission(
        &mut self,
        user_id: &str,
        permission_id: &str,
        user_id_type: &str,
    ) -> Result<bool> {
        // 这里需要根据实际的飞书权限 API 实现
        // 暂时返回 true
        debug!("Checking permission: {} -> {} ({})", user_id, permission_id, user_id_type);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_info_deserialization() {
        let json = serde_json::json!({
            "open_id": "ou_xxx",
            "user_id": "xxx",
            "union_id": "on_xxx",
            "name": "Test User",
            "en_name": "Test",
            "nickname": "Tester",
            "email": "test@example.com",
            "mobile": "+8613800138000",
            "gender": 1,
            "avatar_url": "https://example.com/avatar.png",
            "position": "Engineer",
            "department_ids": ["dept_1", "dept_2"],
            "is_active": true,
            "status": "1"
        });
        
        let user_info: UserInfo = serde_json::from_value(json).unwrap();
        assert_eq!(user_info.open_id, "ou_xxx");
        assert_eq!(user_info.name, "Test User");
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
    }
    
    #[test]
    fn test_group_info_deserialization() {
        let json = serde_json::json!({
            "chat_id": "oc_xxx",
            "name": "Test Group",
            "description": "Test group description",
            "owner_id": "ou_xxx",
            "avatar": "https://example.com/group.png",
            "chat_mode": "group",
            "chat_type": "group",
            "create_time": 1234567890,
            "member_count": 10
        });
        
        let group_info: GroupInfo = serde_json::from_value(json).unwrap();
        assert_eq!(group_info.chat_id, "oc_xxx");
        assert_eq!(group_info.name, Some("Test Group".to_string()));
        assert_eq!(group_info.member_count, Some(10));
    }
    
    #[test]
    fn test_group_member_deserialization() {
        let json = serde_json::json!({
            "member_id": "ou_xxx",
            "member_id_type": "open_id",
            "name": "Test User",
            "member_type": "user",
            "join_time": 1234567890,
            "member_role": "member"
        });
        
        let member: GroupMember = serde_json::from_value(json).unwrap();
        assert_eq!(member.member_id, "ou_xxx");
        assert_eq!(member.member_id_type, "open_id");
        assert_eq!(member.member_role, Some("member".to_string()));
    }
    
    #[test]
    fn test_add_group_member_request() {
        let request = AddGroupMemberRequest {
            chat_id: "oc_xxx".to_string(),
            member_id_list: vec!["ou_1".to_string(), "ou_2".to_string()],
            member_id_type: "open_id".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("oc_xxx"));
        assert!(json.contains("ou_1"));
        assert!(json.contains("open_id"));
    }
    
    #[test]
    fn test_user_status() {
        let status = UserStatus::Activated;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"1\"");
        
        let status = UserStatus::Resigned;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"4\"");
    }
}
