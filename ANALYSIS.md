# NewClaw v0.7.2 飞书文档问题分析

## 问题现象

1. ✅ OpenClaw 飞书工具成功创建带内容的文档
2. ❌ NewClaw 飞书工具创建文档成功但内容为空
3. ❌ NewClaw 飞书工具更新文档失败（invalid param）

## 关键发现

### OpenClaw vs NewClaw 差异

**OpenClaw 成功原因：**
- 使用飞书官方插件 `@larksuiteoapi/feishu-openclaw-plugin`
- 已正确实现文档创建和内容填充
- API 调用正确

**NewClaw 失败原因：**
- 自行实现的飞书工具
- `create_doc_with_content` 函数实现问题
- 可能分两步执行但第二步失败

## API 测试结果

✅ 使用 `batchCreate` API 成功添加内容到文档：
```json
{
  "code": 0,
  "data": {...},
  "msg": "success"
}
```

说明 API 格式和 token 都是正确的。

## 问题根因

查看 NewClaw 的 `create_doc_with_content` 实现：

```rust
pub async fn create_doc_with_content(...) -> Result<String> {
    // 1. 先创建文档
    let doc_id = self.create_doc(title, folder_token).await?;
    
    // 2. 等待文档初始化
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // 3. 添加初始块（问题可能在这里）
    let initial_block = vec![...];
    let url = format!("{}/docx/v1/documents/{}/blocks/doc", ...);
    
    // 4. 使用 PATCH 方法（错误！）
    self.http_client.patch(&url)...
}
```

**问题：**
1. 创建文档后尝试使用 `blocks/doc` PATCH 端点
2. 但我们已经修复为使用 `batchCreate` POST 端点
3. 可能代码没有完全同步或缓存问题

## 解决方案

1. **确认新版本已部署** ✅
   - 文件 MD5 一致
   - 服务已重启

2. **测试 API 调用** ✅
   - 直接调用 `batchCreate` 成功
   - Token 有效

3. **建议测试方法**
   - 在飞书中重新测试创建文档
   - 如果仍然失败，查看日志获取详细错误

## 当前状态

- ✅ OpenClaw 飞书工具正常
- ✅ API 调用格式正确
- ✅ 新代码已编译并部署
- ⚠️ 需要实际测试确认 NewClaw 是否修复

---

**请在飞书中重新测试，如果仍有问题，告诉我具体的错误信息！**