# NewClaw Dashboard

NewClaw v0.4.0 Dashboard Web UI

## 功能

### 模块 1: 配置界面 ✅
- **LLM 配置**
  - Provider 选择 (OpenAI, Claude, GLM, GLMCode)
  - 模型参数配置 (temperature, max_tokens)
  - API Key 状态显示

- **工具配置**
  - 启用/禁用工具
  - 工具参数详情

- **飞书配置**
  - App ID/Secret 配置
  - WebSocket/轮询模式选择

### 模块 2: 监控面板 ✅
- **日志查看**
  - 实时日志流
  - 日志级别过滤
  - 搜索和导出

- **性能指标**
  - 请求/响应时间
  - Token 使用统计
  - 错误率监控
  - 连接状态

- **健康检查**
  - 组件状态
  - 运行时间

### 模块 3: 对话界面 ✅
- **聊天窗口**
  - 多轮对话
  - 消息历史
  - 流式输出 (SSE)

- **调试工具**
  - 查看原始请求/响应
  - Token 计数
  - 性能分析

### 模块 4: 管理功能 ✅
- **用户管理**
  - 添加/删除用户
  - 用户角色分配
  - 使用配额

- **API Key 管理**
  - 生成/撤销 API Key
  - Key 权限配置
  - 使用统计

## 技术栈

### 后端
- Rust (Actix-web/Axum)
- Tower-http (静态文件服务)
- Tokio (异步运行时)

### 前端
- React 18
- TypeScript
- Vite 5
- Ant Design 5
- ECharts (图表)
- Axios (HTTP 客户端)

## 开发

### 后端

```bash
# 构建后端
cargo build --release

# 运行 Dashboard 示例
cargo run --example dashboard_example
```

### 前端

```bash
# 进入前端目录
cd dashboard-ui

# 安装依赖
npm install

# 开发模式
npm run dev

# 构建生产版本
npm run build
```

### 静态文件

构建后的前端文件在 `static/` 目录：

```bash
# 构建前端并复制到 static/
cd dashboard-ui && npm run build
cp -r dist/* ../static/
```

## API 端点

### 配置 API
- `GET /api/config/llm` - 获取 LLM 配置
- `PUT /api/config/llm` - 更新 LLM 配置
- `GET /api/config/tools` - 获取工具配置
- `PUT /api/config/tools` - 更新工具配置
- `GET /api/config/feishu` - 获取飞书配置
- `PUT /api/config/feishu` - 更新飞书配置

### 监控 API
- `GET /api/monitor/logs` - 获取日志列表
- `GET /api/monitor/logs/stream` - WebSocket 日志流
- `GET /api/monitor/metrics` - 获取性能指标
- `GET /api/monitor/health` - 健康检查

### 对话 API
- `GET /api/chat/sessions` - 列出会话
- `POST /api/chat/sessions` - 创建会话
- `GET /api/chat/sessions/:id` - 获取会话详情
- `POST /api/chat/sessions/:id/messages` - 发送消息
- `GET /api/chat/sessions/:id/stream` - 流式响应 (SSE)

### 管理 API
- `GET /api/admin/users` - 列出用户
- `POST /api/admin/users` - 创建用户
- `DELETE /api/admin/users/:id` - 删除用户
- `GET /api/admin/apikeys` - 列出 API Keys
- `POST /api/admin/apikeys` - 创建 API Key
- `DELETE /api/admin/apikeys/:id` - 撤销 API Key

## 配置

### 环境变量

```bash
# LLM 配置
LLM_PROVIDER=glm
LLM_MODEL=glm-4
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GLM_API_KEY=id.secret

# 飞书配置
FEISHU_APP_ID=cli_...
FEISHU_APP_SECRET=...
FEISHU_ENCRYPT_KEY=...
FEISHU_VERIFICATION_TOKEN=...

# Dashboard 配置
DASHBOARD_PORT=8080
DASHBOARD_AUTH_ENABLED=false
```

### config.toml

```toml
[dashboard]
enabled = true
port = 8080
auth_enabled = false
jwt_secret = "your-secret-key"
session_timeout_secs = 3600
```

## 文件结构

```
/root/newclaw/
├── src/dashboard/          # Dashboard 后端模块
│   ├── mod.rs             # 模块入口
│   ├── config_api.rs      # 配置 API
│   ├── monitor.rs         # 监控 API
│   ├── chat.rs            # 对话 API
│   ├── admin.rs           # 管理 API
│   ├── metrics.rs         # 指标收集
│   ├── session.rs         # 会话管理
│   └── dashboard.html     # 降级 HTML
├── dashboard-ui/           # 前端代码
│   ├── src/
│   │   ├── components/    # 组件
│   │   ├── pages/         # 页面
│   │   ├── services/      # API 服务
│   │   └── App.tsx        # 应用入口
│   └── package.json
├── static/                 # 构建后的静态文件
│   ├── index.html
│   └── assets/
└── examples/
    └── dashboard_example.rs
```

## 下一步

### 待实现功能
- [ ] 实际 LLM 集成（目前是模拟响应）
- [ ] WebSocket 日志流（需要启用 axum ws feature）
- [ ] 数据持久化（SQLite/文件）
- [ ] JWT 认证
- [ ] 告警通知
- [ ] 国际化

### 优化建议
- [ ] 代码分割（减少 JS bundle 大小）
- [ ] 更多的图表类型
- [ ] 深色主题
- [ ] 移动端适配
