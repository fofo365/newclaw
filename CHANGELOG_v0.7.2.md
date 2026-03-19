# NewClaw v0.7.2 - 飞书令牌修复

## 问题

飞书文档创建功能返回错误：
```
99991663: Invalid access token for authorization
```

## 根因

混淆了两种飞书访问令牌：
- `tenant_access_token` (t-开头): 应用级令牌
- `user_access_token` (u-开头): 用户级令牌

配置文件中的 `access_token` 实际是 tenant_access_token，但代码把它当成 user_access_token 使用。

创建/更新文档等需要用户身份的操作必须用 **user_access_token**。

## 修复

### 1. 区分令牌类型

```rust
pub struct FeishuConfig {
    pub tenant_access_token: Option<String>,  // 应用级（t-）
    pub user_access_token: Option<String>,    // 用户级（u-）
}
```

### 2. 添加令牌验证

```rust
pub async fn get_access_token_with_type(&self, require_user: bool) -> Result<String> {
    if require_user {
        if let Some(user_token) = &self.config.user_access_token {
            if user_token.starts_with("u-") {
                return Ok(user_token.clone());
            }
            tracing::warn!("user_access_token 格式不正确");
        }
        return Err(anyhow!("需要用户级令牌"));
    }
    // ... 处理 tenant_access_token
}
```

### 3. 强制使用用户令牌

```rust
pub async fn create_doc(&self, title: &str, folder_token: Option<&str>) -> Result<String> {
    let token = self.get_access_token_with_type(true).await?;  // require_user=true
    // ...
}
```

## 配置更新

```toml
[feishu.accounts.default]
tenant_access_token = "t-g1043i70HP3DGACLZ5KCE42PYVYI2KXR57JZNTLY"
user_access_token = "u-xxxxxxxx"  # 需要用户授权后添加
```

## 其他修复

- 修复 response 借用移动错误
- 更新 dashboard 配置初始化

## 后续

用户需要在飞书中完成 OAuth 授权以获取 user_access_token。