# NewClaw v0.7.2 - 飞书令牌修复完成

## 修复内容

### 核心问题
飞书文档创建返回 `99991663: Invalid access token for authorization` 错误。

### 根因
混淆了两种飞书访问令牌：
- `tenant_access_token` (t-开头): 应用级令牌
- `user_access_token` (u-开头): 用户级令牌 ❌ 缺失

### 修复方案

1. **区分令牌类型**
   ```rust
   pub struct FeishuConfig {
       pub tenant_access_token: Option<String>,
       pub user_access_token: Option<String>,
   }
   ```

2. **添加格式验证**
   - 验证 user_access_token 以 `u-` 开头
   - 验证 tenant_access_token 以 `t-` 开头

3. **强制使用用户令牌**
   - `create_doc` / `update_doc` 强制要求 user_access_token
   - 添加详细错误提示

### 配置更新

```toml
[feishu.accounts.default]
tenant_access_token = "t-g1043i70HP3DGACLZ5KCE42PYVYI2KXR57JZNTLY"
user_access_token = "u-xxxxxxxx"  # 需要用户授权
```

## 后续

飞书文档创建功能需要 **user_access_token**，需要在飞书中完成 OAuth 授权。

### 授权步骤

1. 在飞书中给应用发消息
2. 系统检测到缺失的 user_access_token
3. 自动发起授权流程
4. 授权完成后 token 自动保存

---

## 部署状态

✅ 代码已编译
✅ 服务已重启
✅ 修复已生效

## 注意事项

这个修复只解决了 NewClaw 的飞书令牌问题。OpenClaw 的飞书插件仍然需要单独授权。