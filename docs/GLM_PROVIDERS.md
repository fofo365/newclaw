# NewClaw Provider 配置指南

NewClaw v0.4.0 支持 40+ 个 LLM Provider，包括国产和国际主流模型。

## 国产模型 Provider

### GLM / 智谱 AI

| Provider 名称 | 区域 | API 端点 | 说明 |
|--------------|------|---------|------|
| `glm` | 国际 | `https://api.z.ai/api/paas/v4` | GLM 国际区域（推荐） |
| `glm-global` | 国际 | `https://api.z.ai/api/paas/v4` | 同 `glm` |
| `zhipu` | 国际 | `https://api.z.ai/api/paas/v4` | 智谱国际别名 |
| `glm-cn` | 中国 | `https://open.bigmodel.cn/api/paas/v4` | GLM 中国区域 |
| `bigmodel` | 中国 | `https://open.bigmodel.cn/api/paas/v4` | 大模型别名 |
| `zhipu-cn` | 中国 | `https://open.bigmodel.cn/api/paas/v4` | 智谱中国别名 |

**支持模型**: glm-4, glm-4-flash, glm-4-air, glm-4-airx, glm-4-long, glm-4-plus, glm-4v, glm-z1-air, glm-z1-airx, glm-z1-flash

### GLMCode / z.ai (Coding API)

| Provider 名称 | 区域 | API 端点 | 说明 |
|--------------|------|---------|------|
| `z.ai` | 国际 | `https://api.z.ai/api/coding/paas/v4` | z.ai 国际区域（推荐） |
| `zai` | 国际 | `https://api.z.ai/api/coding/paas/v4` | 同 `z.ai` |
| `glmcode` | 国际 | `https://api.z.ai/api/coding/paas/v4` | GLMCode 国际别名 |
| `zai-cn` | 中国 | `https://open.bigmodel.cn/api/coding/paas/v4` | z.ai 中国区域 |
| `z.ai-cn` | 中国 | `https://open.bigmodel.cn/api/coding/paas/v4` | 同 `zai-cn` |
| `glmcode-cn` | 中国 | `https://open.bigmodel.cn/api/coding/paas/v4` | GLMCode 中国别名 |

**支持模型**: glm-4.7, glm-5

### Qwen / 通义千问

| Provider 名称 | 区域 | API 端点 |
|--------------|------|---------|
| `qwen` | 中国 | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| `qwen-cn` | 中国 | 同上 |
| `qwen-intl` | 国际 | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` |
| `qwen-code` | OAuth | Qwen Code OAuth |
| `dashscope` | 中国 | 同 `qwen` |

**支持模型**: qwen-max, qwen-plus, qwen-turbo, qwen3-coder-plus

### DeepSeek

| Provider 名称 | API 端点 |
|--------------|---------|
| `deepseek` | `https://api.deepseek.com` |

**支持模型**: deepseek-chat, deepseek-reasoner, deepseek-coder

### Moonshot / Kimi

| Provider 名称 | 区域 | API 端点 |
|--------------|------|---------|
| `moonshot` | 国际 | `https://api.moonshot.ai/v1` |
| `moonshot-cn` | 中国 | `https://api.moonshot.cn/v1` |
| `kimi` | 国际 | 同 `moonshot` |
| `kimi-code` | Coding | `https://api.kimi.com/coding/v1` |

**支持模型**: kimi-k2.5, kimi-k2-thinking, moonshot-v1-8k/32k/128k, kimi-for-coding

### MiniMax

| Provider 名称 | 区域 | API 端点 |
|--------------|------|---------|
| `minimax` | 国际 | `https://api.minimax.io/v1` |
| `minimax-cn` | 中国 | `https://api.minimaxi.com/v1` |

**支持模型**: MiniMax-M2.5, MiniMax-M2.5-highspeed, MiniMax-M2.1

### StepFun / 阶跃星辰

| Provider 名称 | API 端点 |
|--------------|---------|
| `stepfun`, `step` | `https://api.stepfun.com/v1` |

**支持模型**: step-3.5-flash, step-3, step-1o-turbo-vision

### Hunyuan / 腾讯混元

| Provider 名称 | API 端点 |
|--------------|---------|
| `hunyuan`, `tencent` | `https://api.hunyuan.cloud.tencent.com/v1` |

**支持模型**: hunyuan-t1-latest, hunyuan-turbo-latest

### Doubao / 火山引擎

| Provider 名称 | API 端点 |
|--------------|---------|
| `doubao`, `volcengine`, `ark` | `https://ark.cn-beijing.volces.com/api/v3` |

**支持模型**: doubao-1-5-pro-32k-250115

### Qianfan / 百度千帆

| Provider 名称 | API 端点 |
|--------------|---------|
| `qianfan`, `baidu` | `https://aip.baidubce.com` |

**支持模型**: ernie-4.0-8k, ernie-3.5-8k

### SiliconFlow

| Provider 名称 | API 端点 |
|--------------|---------|
| `siliconflow`, `silicon-cloud` | `https://api.siliconflow.cn/v1` |

**支持模型**: Pro/zai-org/GLM-4.7, Pro/deepseek-ai/DeepSeek-V3.2, Qwen/Qwen3-32B

## 国际模型 Provider

### OpenAI

| Provider 名称 | API 端点 |
|--------------|---------|
| `openai` | `https://api.openai.com/v1` |

**支持模型**: gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4, o1, o1-mini, gpt-5.2, gpt-5-mini

### Claude / Anthropic

| Provider 名称 | API 端点 |
|--------------|---------|
| `claude`, `anthropic` | Anthropic API |

**支持模型**: claude-3-5-sonnet, claude-3-5-haiku, claude-3-opus, claude-3-sonnet, claude-3-haiku, claude-sonnet-4-6

### Gemini / Google

| Provider 名称 | API 端点 |
|--------------|---------|
| `gemini`, `google` | Google AI API |

**支持模型**: gemini-2.5-pro, gemini-2.5-flash

### 其他国际 Provider

| Provider | 模型示例 |
|----------|---------|
| `groq` | llama-3.3-70b-versatile |
| `mistral` | mistral-large-latest, codestral-latest |
| `xai`, `grok` | grok-4-1-fast-reasoning, grok-4 |
| `perplexity` | sonar-pro, sonar |
| `cohere` | command-a-03-2025, command-r-08-2024 |
| `together-ai` | meta-llama/Llama-3.3-70B-Instruct-Turbo |
| `fireworks` | accounts/fireworks/models/llama-v3p3-70b-instruct |
| `nvidia` | meta/llama-3.3-70b-instruct |
| `cerebras` | llama3.1-70b |
| `ai21` | jamba-1.5-large |
| `sambanova` | Meta-Llama-3.3-70B-Instruct |
| `venice` | zai-org-glm-5 |
| `huggingface`, `hf` | meta-llama/Llama-3.3-70B-Instruct |
| `replicate` | meta/meta-llama-3-70b-instruct |
| `bedrock` | anthropic.claude-sonnet-4-6 |
| `openrouter` | anthropic/claude-sonnet-4.6 |
| `novita` | minimax/minimax-m2.5 |

## 本地模型

### Ollama

| Provider 名称 | API 端点 |
|--------------|---------|
| `ollama` | `http://localhost:11434/v1` (默认) |

**支持模型**: llama3.2, mistral, codellama, phi3

## 使用方式

### 1. 命令行参数

```bash
# 使用 GLM 国际区域
newclaw --provider glm

# 使用 GLM 中国区域
newclaw --provider glm-cn

# 使用 z.ai 国际区域（Coding）
newclaw --provider z.ai

# 使用 DeepSeek
newclaw --provider deepseek --model deepseek-chat

# 使用 Qwen
newclaw --provider qwen --model qwen-plus

# 使用 Ollama 本地模型
newclaw --provider ollama --model llama3.2
```

### 2. 环境变量

```bash
# 设置 Provider
export LLM_PROVIDER=glm

# 设置模型
export LLM_MODEL=glm-4

# 设置 API Key
export GLM_API_KEY=your_id.your_secret
export DEEPSEEK_API_KEY=your_api_key
export DASHSCOPE_API_KEY=your_api_key  # Qwen
export MOONSHOT_API_KEY=your_api_key
```

### 3. 配置文件 (config.toml)

```toml
[llm]
provider = "deepseek"
model = "deepseek-chat"
temperature = 0.7
max_tokens = 4096

[llm.deepseek]
api_key = "your_api_key"
```

### 4. Gateway API

启动 Gateway：

```bash
# 使用 GLM
newclaw --gateway --provider glm --port 3000

# 使用 DeepSeek
newclaw --gateway --provider deepseek --port 3000
```

查看支持的 Provider：

```bash
curl http://localhost:3000/providers
```

## 默认模型

| Provider | 默认模型 |
|----------|---------|
| `glm`, `glm-*` | `glm-4` |
| `z.ai`, `zai-*`, `glmcode*` | `glm-4.7` |
| `deepseek` | `deepseek-chat` |
| `qwen` | `qwen-plus` |
| `moonshot`, `kimi` | `kimi-k2.5` |
| `minimax` | `MiniMax-M2.5` |
| `stepfun` | `step-3.5-flash` |
| `hunyuan` | `hunyuan-t1-latest` |
| `doubao` | `doubao-1-5-pro-32k-250115` |
| `qianfan` | `ernie-4.0-8k` |
| `openai` | `gpt-4o-mini` |
| `claude` | `claude-3-5-sonnet-20241022` |
| `gemini` | `gemini-2.5-pro` |
| `ollama` | `llama3.2` |

可以通过 `--model` 或 `LLM_MODEL` 环境变量覆盖。

## API Key 格式

### GLM / 智谱
GLM API Key 使用 `id.secret` 格式：
```
1234567890.abcdefghijklmnop
```

### 其他 Provider
大多数使用标准的 Bearer Token 格式。

## 故障排除

### 1. API Key 无效
确保 API Key 格式正确，检查是否过期。

### 2. 区域选择
- 中国用户：推荐 `glm-cn`, `qwen-cn`, `moonshot-cn`
- 海外用户：推荐 `glm`, `qwen-intl`, `moonshot`

### 3. 网络问题
如果无法连接，尝试切换区域或检查网络代理设置。

## 更多信息

- [完整模型列表](./MODELS.md)
- [快速参考](./v0.4.0-quick-reference.md)
