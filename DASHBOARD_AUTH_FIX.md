# Dashboard 认证问题诊断

## 问题
Dashboard API 返回：`Missing request extension: Extension of type 'alloc::sync::Arc<newclaw::dashboard::DashboardState>'`

## 根因
1. API handler 使用 `Extension(state)` 提取 DashboardState
2. Router 使用 `.with_state(state)` 设置 state
3. Axum 0.8 中，Extension 和 State 是两种不同的机制
4. 当使用 `.with_state()` 时，API 应该使用 `State(state)` 而不是 `Extension(state)`

## 解决方案
修改所有 API handler，将 `Extension(state)` 改为 `State(state)`

## 受影响的文件
- src/dashboard/config_api.rs
- src/dashboard/monitor.rs
- src/dashboard/chat.rs
- src/dashboard/admin.rs
- src/dashboard/auth.rs

## 需要修改的模式
```rust
// 旧代码（错误）
pub async fn get_llm_config(
    Extension(state): Extension<Arc<DashboardState>>,
) -> Result<Json<LLMConfigResponse>, AppError>

// 新代码（正确）
pub async fn get_llm_config(
    State(state): State<Arc<DashboardState>>,
) -> Result<Json<LLMConfigResponse>, AppError>
```

## Auth 中间件
- 创建一个简单的认证检查中间件
- 验证 Bearer token
- 为所有需要认证的路由添加此中间件
- 公开路由（/api/auth/*, /metrics, /health）不需要认证