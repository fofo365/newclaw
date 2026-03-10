// 工具权限管理

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::ToolError;

/// 权限级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// 文件读取
    FileRead,
    /// 文件写入
    FileWrite,
    /// 文件编辑
    FileEdit,
    /// Shell 执行
    ShellExec,
    /// 网络请求
    NetworkAccess,
}

/// 权限管理器
pub struct PermissionManager {
    /// 允许的权限
    allowed: Arc<RwLock<HashSet<Permission>>>,
    /// 工具权限映射
    tool_permissions: Arc<RwLock<HashMap<String, HashSet<Permission>>>>,
}

impl PermissionManager {
    /// 创建新的权限管理器
    pub fn new() -> Self {
        Self {
            allowed: Arc::new(RwLock::new(HashSet::new())),
            tool_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 授予权限
    pub async fn grant(&self, permission: Permission) {
        let mut allowed = self.allowed.write().await;
        allowed.insert(permission);
    }
    
    /// 撤销权限
    pub async fn revoke(&self, permission: Permission) {
        let mut allowed = self.allowed.write().await;
        allowed.remove(&permission);
    }
    
    /// 检查权限
    pub async fn check(&self, permission: Permission) -> Result<(), ToolError> {
        let allowed = self.allowed.read().await;
        if allowed.contains(&permission) {
            Ok(())
        } else {
            Err(ToolError::PermissionDenied(format!("{:?}", permission)))
        }
    }
    
    /// 设置工具所需权限
    pub async fn set_tool_permissions(&self, tool: &str, permissions: HashSet<Permission>) {
        let mut tool_perms = self.tool_permissions.write().await;
        tool_perms.insert(tool.to_string(), permissions);
    }
    
    /// 检查工具权限
    pub async fn check_tool(&self, tool: &str) -> Result<(), ToolError> {
        let tool_perms = self.tool_permissions.read().await;
        if let Some(required) = tool_perms.get(tool) {
            let allowed = self.allowed.read().await;
            for perm in required {
                if !allowed.contains(perm) {
                    return Err(ToolError::PermissionDenied(format!(
                        "工具 {} 需要权限 {:?}",
                        tool, perm
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_grant_permission() {
        let manager = PermissionManager::new();
        manager.grant(Permission::FileRead).await;
        
        assert!(manager.check(Permission::FileRead).await.is_ok());
        assert!(manager.check(Permission::FileWrite).await.is_err());
    }
    
    #[tokio::test]
    async fn test_revoke_permission() {
        let manager = PermissionManager::new();
        manager.grant(Permission::FileRead).await;
        manager.revoke(Permission::FileRead).await;
        
        assert!(manager.check(Permission::FileRead).await.is_err());
    }
    
    #[tokio::test]
    async fn test_tool_permissions() {
        let manager = PermissionManager::new();
        
        // 设置工具权限
        let mut perms = HashSet::new();
        perms.insert(Permission::FileRead);
        manager.set_tool_permissions("read", perms).await;
        
        // 检查权限
        assert!(manager.check_tool("read").await.is_err());
        
        // 授予权限
        manager.grant(Permission::FileRead).await;
        assert!(manager.check_tool("read").await.is_ok());
    }
}
