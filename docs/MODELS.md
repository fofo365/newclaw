# NewClaw v0.4.0 - 支持的模型列表

本文档列出了 NewClaw 支持的所有 LLM 模型。

## 模型统计

- **总模型数**: 150+ 模型
- **支持 Provider**: 40+ 个

## 国产模型

### GLM / 智谱 AI

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 (输入/输出, $/1M tokens) |
|--------------|---------|---------|-----------|------|------------------------------|
| glm, zhipu | glm-4 | GLM-4 | 128K | ❌ | $14 / $14 |
| glm, zhipu | glm-4-flash | GLM-4-Flash | 128K | ❌ | $0.1 / $0.1 |
| glm, zhipu | glm-4-air | GLM-4-Air | 128K | ❌ | $1 / $1 |
| glm, zhipu | glm-4-airx | GLM-4-AirX | 8K | ❌ | $1 / $1 |
| glm, zhipu | glm-4-long | GLM-4-Long | 1M | ❌ | $1 / $1 |
| glm, zhipu | glm-4-plus | GLM-4-Plus | 128K | ✅ | $50 / $50 |
| glm, zhipu | glm-4v | GLM-4V | 8K | ✅ | $50 / $50 |
| glm, zhipu | glm-z1-air | GLM-Z1-Air | 131K | ❌ | $0.35 / $0.35 |
| glm, zhipu | glm-z1-airx | GLM-Z1-AirX | 8K | ❌ | $0.35 / $0.35 |
| glm, zhipu | glm-z1-flash | GLM-Z1-Flash | 131K | ❌ | $0.1 / $0.1 |

### GLMCode / z.ai (Coding API)

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| z.ai, zai, glmcode | glm-4.7 | GLM-4.7 | 131K | ❌ | $0.5 / $0.5 |
| z.ai, zai, glmcode | glm-5 | GLM-5 | 131K | ✅ | $1 / $2 |

### Qwen / 通义千问

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| qwen, dashscope | qwen-max | Qwen Max | 32K | ❌ | $2 / $6 |
| qwen, dashscope | qwen-plus | Qwen Plus | 128K | ✅ | $0.8 / $2 |
| qwen, dashscope | qwen-turbo | Qwen Turbo | 128K | ❌ | $0.3 / $0.6 |
| qwen-code | qwen3-coder-plus | Qwen3 Coder Plus | 128K | ❌ | $0.5 / $1 |

### DeepSeek

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| deepseek | deepseek-chat | DeepSeek Chat | 64K | ❌ | $0.27 / $1.1 |
| deepseek | deepseek-reasoner | DeepSeek Reasoner | 64K | ❌ | $0.55 / $2.19 |
| deepseek | deepseek-coder | DeepSeek Coder | 64K | ❌ | $0.14 / $0.28 |

### Moonshot / Kimi

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| moonshot, kimi | kimi-k2.5 | Kimi K2.5 | 128K | ❌ | $2 / $10 |
| moonshot, kimi | kimi-k2-thinking | Kimi K2 Thinking | 128K | ❌ | $4 / $20 |
| moonshot, kimi | moonshot-v1-8k | Moonshot V1 8K | 8K | ❌ | $0.5 / $0.5 |
| moonshot, kimi | moonshot-v1-32k | Moonshot V1 32K | 32K | ❌ | $1 / $1 |
| moonshot, kimi | moonshot-v1-128k | Moonshot V1 128K | 131K | ❌ | $2 / $2 |
| kimi-code | kimi-for-coding | Kimi for Coding | 128K | ❌ | $2 / $10 |

### MiniMax

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| minimax | MiniMax-M2.5 | MiniMax M2.5 | 128K | ❌ | $0.5 / $1 |
| minimax | MiniMax-M2.5-highspeed | MiniMax M2.5 High-Speed | 64K | ❌ | $0.3 / $0.6 |
| minimax | MiniMax-M2.1 | MiniMax M2.1 | 128K | ❌ | $0.4 / $0.8 |

### StepFun / 阶跃星辰

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| stepfun, step | step-3.5-flash | Step 3.5 Flash | 64K | ❌ | $0.4 / $0.8 |
| stepfun, step | step-3 | Step 3 | 128K | ❌ | $2 / $6 |
| stepfun, step | step-1o-turbo-vision | Step 1o Turbo Vision | 64K | ✅ | $0.5 / $1 |

### Hunyuan / 腾讯混元

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| hunyuan, tencent | hunyuan-t1-latest | Hunyuan T1 | 128K | ❌ | $1 / $3 |
| hunyuan, tencent | hunyuan-turbo-latest | Hunyuan Turbo | 64K | ❌ | $0.3 / $0.6 |

### Doubao / 火山引擎

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| doubao, volcengine, ark | doubao-1-5-pro-32k-250115 | Doubao 1.5 Pro 32K | 32K | ❌ | $0.5 / $1 |

### Qianfan / 百度千帆

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| qianfan, baidu | ernie-4.0-8k | ERNIE 4.0 8K | 8K | ❌ | $3 / $6 |
| qianfan, baidu | ernie-3.5-8k | ERNIE 3.5 8K | 8K | ❌ | $1.2 / $2.4 |

### SiliconFlow

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| siliconflow, silicon-cloud | Pro/zai-org/GLM-4.7 | GLM-4.7 Pro | 131K | ❌ | $0.5 / $0.5 |
| siliconflow | Pro/deepseek-ai/DeepSeek-V3.2 | DeepSeek V3.2 Pro | 128K | ❌ | $0.27 / $1.1 |
| siliconflow | Qwen/Qwen3-32B | Qwen3 32B | 32K | ❌ | $0.3 / $0.6 |

## 国际模型

### OpenAI

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| openai | gpt-4o | GPT-4o | 128K | ✅ | $5 / $15 |
| openai | gpt-4o-mini | GPT-4o Mini | 128K | ✅ | $0.15 / $0.6 |
| openai | gpt-4-turbo | GPT-4 Turbo | 128K | ✅ | $10 / $30 |
| openai | gpt-4 | GPT-4 | 8K | ❌ | $30 / $60 |
| openai | o1 | o1 | 200K | ✅ | $15 / $60 |
| openai | o1-mini | o1 Mini | 128K | ❌ | $3 / $12 |
| openai | gpt-5.2 | GPT-5.2 | 200K | ✅ | $10 / $30 |
| openai | gpt-5-mini | GPT-5 Mini | 128K | ✅ | $0.5 / $1.5 |

### Claude / Anthropic

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| claude, anthropic | claude-3-5-sonnet-20241022 | Claude 3.5 Sonnet | 200K | ✅ | $3 / $15 |
| claude, anthropic | claude-3-5-haiku-20241022 | Claude 3.5 Haiku | 200K | ✅ | $0.8 / $4 |
| claude, anthropic | claude-3-opus-20240229 | Claude 3 Opus | 200K | ✅ | $15 / $75 |
| claude, anthropic | claude-3-sonnet-20240229 | Claude 3 Sonnet | 200K | ✅ | $3 / $15 |
| claude, anthropic | claude-3-haiku-20240307 | Claude 3 Haiku | 200K | ✅ | $0.25 / $1.25 |
| claude, anthropic | claude-sonnet-4-6 | Claude Sonnet 4.6 | 200K | ✅ | $3 / $15 |

### Gemini / Google

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| gemini, google | gemini-2.5-pro | Gemini 2.5 Pro | 1M | ✅ | $1.25 / $5 |
| gemini, google | gemini-2.5-flash | Gemini 2.5 Flash | 1M | ✅ | $0.075 / $0.3 |

### Groq

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| groq | llama-3.3-70b-versatile | Llama 3.3 70B Versatile | 128K | ❌ | $0.59 / $0.79 |

### Mistral

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| mistral | mistral-large-latest | Mistral Large | 128K | ❌ | $2 / $6 |
| mistral | codestral-latest | Codestral | 64K | ❌ | $0.3 / $0.9 |

### xAI / Grok

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| xai, grok | grok-4-1-fast-reasoning | Grok 4.1 Fast Reasoning | 128K | ❌ | $3 / $15 |
| xai, grok | grok-4 | Grok 4 | 128K | ✅ | $5 / $25 |

### Perplexity

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| perplexity | sonar-pro | Sonar Pro | 128K | ❌ | $3 / $15 |
| perplexity | sonar | Sonar | 64K | ❌ | $1 / $5 |

### Cohere

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| cohere | command-a-03-2025 | Command A | 128K | ❌ | $2.5 / $10 |
| cohere | command-r-08-2024 | Command R | 128K | ❌ | $0.5 / $1.5 |

### Together AI

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| together-ai | meta-llama/Llama-3.3-70B-Instruct-Turbo | Llama 3.3 70B Turbo | 128K | ❌ | $0.88 / $0.88 |
| together-ai | moonshotai/Kimi-K2.5 | Kimi K2.5 | 128K | ❌ | $2 / $10 |

### Fireworks AI

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| fireworks | accounts/fireworks/models/llama-v3p3-70b-instruct | Llama 3.3 70B | 128K | ❌ | $0.9 / $0.9 |

### NVIDIA NIM

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| nvidia | meta/llama-3.3-70b-instruct | Llama 3.3 70B | 128K | ❌ | $0.6 / $0.6 |
| nvidia | deepseek-ai/deepseek-v3.2 | DeepSeek V3.2 | 128K | ❌ | $0.27 / $1.1 |

### Cerebras

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| cerebras | llama3.1-70b | Llama 3.1 70B | 128K | ❌ | $0.6 / $0.6 |
| cerebras | llama3.1-8b | Llama 3.1 8B | 128K | ❌ | $0.1 / $0.1 |

### AI21

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| ai21 | jamba-1.5-large | Jamba 1.5 Large | 256K | ❌ | $2 / $8 |
| ai21 | jamba-1.5-mini | Jamba 1.5 Mini | 256K | ❌ | $0.2 / $0.8 |

### SambaNova

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| sambanova | Meta-Llama-3.3-70B-Instruct | Llama 3.3 70B | 128K | ❌ | $0.6 / $0.6 |
| sambanova | DeepSeek-R1 | DeepSeek R1 | 128K | ❌ | $0.55 / $2.19 |

### Venice

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| venice | zai-org-glm-5 | GLM-5 | 131K | ✅ | $1 / $2 |
| venice | claude-sonnet-4-6 | Claude Sonnet 4.6 | 200K | ✅ | $3 / $15 |
| venice | deepseek-v3.2 | DeepSeek V3.2 | 128K | ❌ | $0.27 / $1.1 |

### Hugging Face

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| huggingface, hf | meta-llama/Llama-3.3-70B-Instruct | Llama 3.3 70B | 128K | ❌ | $0.9 / $0.9 |
| huggingface | Qwen/Qwen2.5-Coder-32B-Instruct | Qwen 2.5 Coder 32B | 32K | ❌ | $0.3 / $0.6 |

### Replicate

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| replicate | meta/meta-llama-3-70b-instruct | Llama 3 70B | 128K | ❌ | $0.9 / $0.9 |
| replicate | deepseek-ai/deepseek-v3 | DeepSeek V3 | 128K | ❌ | $0.27 / $1.1 |

### AWS Bedrock

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| bedrock, aws-bedrock | anthropic.claude-sonnet-4-6 | Claude Sonnet 4.6 | 200K | ✅ | $3 / $15 |
| bedrock | anthropic.claude-opus-4-6-v1 | Claude Opus 4.6 | 200K | ✅ | $15 / $75 |
| bedrock | anthropic.claude-haiku-4-5-20251001-v1:0 | Claude Haiku 4.5 | 200K | ✅ | $0.8 / $4 |

### OpenRouter

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| openrouter | anthropic/claude-sonnet-4.6 | Claude Sonnet 4.6 | 200K | ✅ | $3 / $15 |
| openrouter | openai/gpt-5.2 | GPT-5.2 | 200K | ✅ | $10 / $30 |
| openrouter | deepseek/deepseek-v3.2 | DeepSeek V3.2 | 128K | ❌ | $0.27 / $1.1 |
| openrouter | x-ai/grok-4.1-fast | Grok 4.1 Fast | 128K | ❌ | $3 / $15 |
| openrouter | meta-llama/llama-4-maverick | Llama 4 Maverick | 128K | ❌ | $0.6 / $0.6 |
| openrouter | google/gemini-3-pro-preview | Gemini 3 Pro Preview | 1M | ✅ | $1.25 / $5 |

### Novita

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| novita | minimax/minimax-m2.5 | MiniMax M2.5 | 128K | ❌ | $0.5 / $1 |

## 本地模型

### Ollama

| Provider 别名 | 模型 ID | 显示名称 | 上下文长度 | 视觉 | 价格 |
|--------------|---------|---------|-----------|------|------|
| ollama | llama3.2 | Llama 3.2 | 128K | ✅ | 免费 |
| ollama | mistral | Mistral 7B | 32K | ❌ | 免费 |
| ollama | codellama | Code Llama | 16K | ❌ | 免费 |
| ollama | phi3 | Phi-3 | 128K | ❌ | 免费 |

## 默认模型

| Provider | 默认模型 |
|----------|---------|
| glm, zhipu | glm-4 |
| z.ai, zai, glmcode | glm-4.7 |
| openai | gpt-4o-mini |
| claude, anthropic | claude-3-5-sonnet-20241022 |
| deepseek | deepseek-chat |
| qwen, dashscope | qwen-plus |
| moonshot, kimi | kimi-k2.5 |
| minimax | MiniMax-M2.5 |
| stepfun, step | step-3.5-flash |
| hunyuan, tencent | hunyuan-t1-latest |
| doubao, volcengine | doubao-1-5-pro-32k-250115 |
| qianfan, baidu | ernie-4.0-8k |
| gemini, google | gemini-2.5-pro |
| groq | llama-3.3-70b-versatile |
| mistral | mistral-large-latest |
| xai, grok | grok-4-1-fast-reasoning |
| perplexity | sonar-pro |
| cohere | command-a-03-2025 |
| together-ai | meta-llama/Llama-3.3-70B-Instruct-Turbo |
| fireworks | accounts/fireworks/models/llama-v3p3-70b-instruct |
| nvidia | meta/llama-3.3-70b-instruct |
| ollama | llama3.2 |
| cerebras | llama3.1-70b |
| ai21 | jamba-1.5-large |
| sambanova | Meta-Llama-3.3-70B-Instruct |
| venice | zai-org-glm-5 |
| huggingface, hf | meta-llama/Llama-3.3-70B-Instruct |
| replicate | meta/meta-llama-3-70b-instruct |
| bedrock | anthropic.claude-sonnet-4-6 |
| openrouter | anthropic/claude-sonnet-4.6 |
| novita | minimax/minimax-m2.5 |
| siliconflow | Pro/zai-org/GLM-4.7 |

## 使用示例

### 命令行

```bash
# 使用 GLM
newclaw --provider glm --model glm-4

# 使用 DeepSeek
newclaw --provider deepseek --model deepseek-chat

# 使用 Qwen
newclaw --provider qwen --model qwen-plus

# 使用 Ollama 本地模型
newclaw --provider ollama --model llama3.2
```

### 环境变量

```bash
export LLM_PROVIDER=deepseek
export LLM_MODEL=deepseek-chat
```

### 配置文件

```toml
[llm]
provider = "qwen"
model = "qwen-plus"
```

## 更多信息

- [GLM Provider 配置](./GLM_PROVIDERS.md)
- [快速参考](./v0.4.0-quick-reference.md)
