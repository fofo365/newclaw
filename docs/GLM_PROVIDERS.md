# GLM 多区域 Provider 支持

NewClaw v0.4.0 支持 GLM 的 4 个 Provider 区域：

## Provider 列表

### GLM (标准 API)

| Provider 名称 | 区域 | API 端点 | 说明 |
|--------------|------|---------|------|
| `glm` | 国际 | `https://api.z.ai/api/paas/v4` | GLM 国际区域（推荐） |
| `glm-global` | 国际 | `https://api.z.ai/api/paas/v4` | 同 `glm` |
| `zhipu` | 国际 | `https://api.z.ai/api/paas/v4` | 智谱国际别名 |
| `glm-cn` | 中国 | `https://open.bigmodel.cn/api/paas/v4` | GLM 中国区域 |
| `bigmodel` | 中国 | `https://open.bigmodel.cn/api/paas/v4` | 大模型别名 |
| `zhipu-cn` | 中国 | `https://open.bigmodel.cn/api/paas/v4` | 智谱中国别名 |

### GLMCode / z.ai (Coding API)

| Provider 名称 | 区域 | API 端点 | 说明 |
|--------------|------|---------|------|
| `z.ai` | 国际 | `https://api.z.ai/api/coding/paas/v4` | z.ai 国际区域（推荐） |
| `zai` | 国际 | `https://api.z.ai/api/coding/paas/v4` | 同 `z.ai` |
| `glmcode` | 国际 | `https://api.z.ai/api/coding/paas/v4` | GLMCode 国际别名 |
| `zai-cn` | 中国 | `https://open.bigmodel.cn/api/coding/paas/v4` | z.ai 中国区域 |
| `z.ai-cn` | 中国 | `https://open.bigmodel.cn/api/coding/paas/v4` | 同 `zai-cn` |
| `glmcode-cn` | 中国 | `https://open.bigmodel.cn/api/coding/paas/v4` | GLMCode 中国别名 |

## 使用方式

### 1. 命令行参数

```bash
# 使用 GLM 国际区域
newclaw --provider glm

# 使用 GLM 中国区域
newclaw --provider glm-cn

# 使用 z.ai 国际区域（Coding）
newclaw --provider z.ai

# 使用 z.ai 中国区域（Coding）
newclaw --provider zai-cn

# 指定 GLM 区域
newclaw --provider glm --glm-region china
```

### 2. 环境变量

```bash
# 设置 Provider
export LLM_PROVIDER=glm

# 设置 GLM 区域
export GLM_REGION=international  # 或 china

# 设置 GLM 类型
export GLM_TYPE=glm  # 或 glmcode

# 设置 API Key（格式: id.secret）
export GLM_API_KEY=your_id.your_secret
```

### 3. 配置文件 (config.toml)

```toml
[llm]
provider = "glm"
model = "glm-4"
temperature = 0.7
max_tokens = 4096

[llm.glm]
api_key = "your_id.your_secret"
region = "international"  # 或 "china"
provider_type = "glm"     # 或 "glmcode"
# base_url = "https://custom.endpoint"  # 可选自定义端点
```

### 4. Gateway API

启动 Gateway：

```bash
newclaw --gateway --provider glm-cn --port 3000
```

查看支持的 Provider：

```bash
curl http://localhost:3000/providers
```

## 区域区别

| 特性 | 国际区域 (api.z.ai) | 中国区域 (open.bigmodel.cn) |
|-----|-------------------|--------------------------|
| 访问速度 | 海外用户更快 | 中国用户更快 |
| 模型可用性 | 全部模型 | 全部模型 |
| 稳定性 | 高 | 高 |
| 账号体系 | GLM 国际账号 | GLM 中国账号 |

## API Key 格式

GLM API Key 使用 `id.secret` 格式：

```
1234567890.abcdefghijklmnop
```

- 第一部分是 API Key ID
- 第二部分是 API Key Secret
- 用于生成 JWT Token 进行认证

## 默认模型

| Provider | 默认模型 |
|----------|---------|
| `glm`, `glm-*` | `glm-4` |
| `z.ai`, `zai-*`, `glmcode*` | `glm-4.7` |

可以通过 `--model` 或 `LLM_MODEL` 环境变量覆盖。

## 故障排除

### 1. API Key 无效

```
GLM API key not set or invalid format. Expected 'id.secret'.
```

确保 API Key 格式正确：`id.secret`

### 2. 区域选择

- 如果在中国使用，推荐 `glm-cn` 或 `zai-cn`
- 如果在海外使用，推荐 `glm` 或 `z.ai`

### 3. 网络问题

- 中国区域：`open.bigmodel.cn`
- 国际区域：`api.z.ai`

如果无法连接，尝试切换区域。
