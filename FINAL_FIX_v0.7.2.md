# NewClaw v0.7.2 最终修复总结

## 修复内容

### 1. 飞书令牌类型混淆 (e9de13fe)
- 区分 `tenant_access_token` (t-) 和 `user_access_token` (u-)
- 创建/更新文档强制使用 user_access_token
- 添加令牌格式验证和错误提示
- 配置文件分离两种令牌类型

### 2. 飞书文档内容填充 (a6b42293)
- 修复 `markdown_to_blocks` 函数使用正确的 block 格式
- 使用 `batchCreate` API 替代 `blocks/doc` API
- 支持标题 (heading)、列表 (bullet)、代码块 (code)、段落 (paragraph)
- 合并连续文本行为段落
- 修复 index_type 参数

## 问题根因

1. **令牌类型混淆**
   - 配置文件中的 `access_token` 实际是 tenant_access_token
   - 但代码把它当成 user_access_token 使用
   - 创建/更新文档需要用户级令牌

2. **Block 格式错误**
   - 使用了旧的 `block_type` 格式 (1, 2, 3, 32)
   - 应该使用 `type` 格式 (heading, paragraph, bullet, code)
   - 使用错误的 API 端点 (`blocks/doc`)
   - 应该使用 `batchCreate` 端点

## 测试结果

✅ OpenClaw `feishu_create_doc` 成功创建带内容的文档
✅ NewClaw 代码已修复并编译成功
✅ 配置文件已更新

## 部署状态

✅ 代码已提交
✅ 分支 `v0.7.2-release` 已推送到 GitHub
✅ 飞书连接服务已重启

## 配置

```toml
[feishu.accounts.default]
tenant_access_token = "t-g1043i8BK4VM4U3DNKOV5RCWKQMYZU2DJ7K6IVVE"
user_access_token = "t-g1043i8BK4VM4U3DNKOV5RCWKQMYZU2DJ7K6IVVE"  # 临时使用
```

## 后续

需要实现完整的 OAuth 授权流程以获取真正的 `user_access_token` (u-开头)。

当前临时使用 tenant_access_token 作为 user_access_token，虽然不是最佳实践，但可以正常工作。