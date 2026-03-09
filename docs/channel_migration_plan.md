# 通道迁移计划

## 📅 日期：2026-03-09
## 🎯 目标：将 TypeScript 扩展迁移到 Rust 通道

---

## 📋 源代码分析

### 1. WeCom（企业微信）

**源代码位置**: `/root/.openclaw/extensions/wecom/`

**核心文件**:
- `src/agent/api-client.ts` - API 客户端（Token 管理、消息发送）
- `src/crypto/` - 加密相关（AES、签名、XML）
- `src/monitor.ts` - Webhook 处理
- `src/channel.ts` - 通道插件
- `src/types/` - 类型定义

**主要功能**:
- ✅ AccessToken 获取和缓存
- ✅ 文本消息发送（单聊/群聊）
- ✅ 图片消息发送
- ✅ 文件上传
- ✅ Webhook 消息接收
- ✅ 消息加密/解密
- ✅ 签名验证

**API 端点**:
- Token: `https://qyapi.weixin.qq.com/cgi-bin/gettoken`
- 消息: `https://qyapi.weixin.qq.com/cgi-bin/message/send`
- 群聊: `https://qyapi.weixin.qq.com/cgi-bin/appchat/send`
- 媒体: `https://qyapi.weixin.qq.com/cgi-bin/media/upload`

---

### 2. QQ Bot

**源代码位置**: `/root/.openclaw/extensions/qqbot/`

**核心文件**:
- `src/api.ts` - API 客户端（Token 管理、消息发送）
- `src/channel.ts` - 通道插件
- `src/gateway.ts` - Gateway 连接
- `src/outbound.ts` - 出站消息处理
- `src/types.ts` - 类型定义

**主要功能**:
- ✅ AccessToken 获取和缓存
- ✅ 文本消息发送
- ✅ 图片消息发送
- ✅ Markdown 消息发送
- ✅ Gateway WebSocket 连接
- ✅ 事件处理

**API 端点**:
- Token: `https://bots.qq.com/app/getAppAccessToken`
- API: `https://api.sgroup.qq.com`
- Gateway: WebSocket 连接

---

## 🏗️ 迁移策略

### 阶段 1: WeCom 迁移（3-5 天）

#### 文件结构：
```
src/channels/wecom/
├── mod.rs           # 模块导出
├── client.rs        # WeCom 客户端（Token 管理）
├── message.rs       # 消息发送（文本、图片、文件）
├── crypto.rs        # 加密/解密（AES、签名）
├── webhook.rs       # Webhook 处理
└── types.rs         # 类型定义
```

#### 实现步骤：
1. **创建基础结构**（1 天）
   - 定义配置结构（`WeComConfig`）
   - 实现 Token 管理和缓存
   - 创建 HTTP 客户端

2. **实现消息发送**（1-2 天）
   - 文本消息发送（单聊/群聊）
   - 图片消息发送
   - 文件上传和发送

3. **实现 Webhook 处理**（1 天）
   - 消息接收和解析
   - 加密/解密
   - 签名验证

4. **集成和测试**（1 天）
   - 单元测试
   - 集成测试
   - 文档编写

---

### 阶段 2: QQ Bot 迁移（3-5 天）

#### 文件结构：
```
src/channels/qq/
├── mod.rs           # 模块导出
├── client.rs        # QQ Bot 客户端（Token 管理）
├── message.rs       # 消息发送（文本、图片、Markdown）
├── gateway.rs       # Gateway WebSocket 连接
├── event.rs         # 事件处理
└── types.rs         # 类型定义
```

#### 实现步骤：
1. **创建基础结构**（1 天）
   - 定义配置结构（`QQConfig`）
   - 实现 Token 管理和缓存
   - 创建 HTTP 客户端

2. **实现消息发送**（1-2 天）
   - 文本消息发送
   - 图片消息发送
   - Markdown 消息发送

3. **实现 Gateway 连接**（1-2 天）
   - WebSocket 连接管理
   - 心跳机制
   - 事件接收和处理

4. **集成和测试**（1 天）
   - 单元测试
   - 集成测试
   - 文档编写

---

### 阶段 3: 统一 Channel Trait（1 天）

定义统一的 `Channel` trait，让所有通道实现相同的接口：

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    /// 发送消息
    async fn send_message(&self, msg: Message) -> Result<MessageId>;
    
    /// 发送文件
    async fn send_file(&self, file: FileMessage) -> Result<MessageId>;
    
    /// 发送图片
    async fn send_image(&self, image: ImageMessage) -> Result<MessageId>;
    
    /// 获取通道信息
    fn channel_info(&self) -> ChannelInfo;
}
```

---

## 📊 预估工作量

| 任务 | 预估时间 | 优先级 |
|------|----------|--------|
| WeCom 迁移 | 3-5 天 | P0 |
| QQ Bot 迁移 | 3-5 天 | P0 |
| 统一 Channel Trait | 1 天 | P0 |
| Telegram 迁移 | 2-3 天 | P1 |
| Discord 迁移 | 2-3 天 | P1 |

**总计**: 11-17 天（约 2-3 周）

---

## ✅ 成功标准

1. **功能完整性**: 所有核心功能都已迁移
2. **测试覆盖**: 每个模块都有单元测试
3. **性能**: 性能不低于 TypeScript 版本
4. **文档**: 完整的 API 文档和使用示例
5. **兼容性**: 与现有系统集成无问题

---

## 🚀 开始实施

下一步：**开始 WeCom 通道迁移**

1. 创建 `src/channels/wecom/` 目录结构
2. 实现基础配置和客户端
3. 实现消息发送功能
4. 实现 Webhook 处理
5. 编写测试和文档

---

**计划创建时间**: 2026-03-09 19:00 UTC
