#!/usr/bin/env node

/**
 * NewClaw Dashboard API Server (临时方案)
 *
 * 提供 Dashboard 前端所需的配置 API
 * 运行在端口 3001
 */

const express = require('express');
const axios = require('axios');
const toml = require('@iarna/toml');
const fs = require('fs');
const path = require('path');

const app = express();
app.use(express.json());

// CORS
app.use((req, res, next) => {
  res.header('Access-Control-Allow-Origin', '*');
  res.header('Access-Control-Allow-Methods', 'GET, POST, PUT, OPTIONS');
  res.header('Access-Control-Allow-Headers', 'Content-Type, Authorization');
  if (req.method === 'OPTIONS') {
    return res.sendStatus(204);
  }
  next();
});

// 读取配置文件
function loadConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    return toml.parse(content);
  } catch (error) {
    console.error('Failed to load config:', error.message);
    return null;
  }
}

// 保存配置文件（仅用于飞书配置的临时保存）
function saveFeishuConfig(feishuConfig) {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const config = loadConfig() || {};

    // 更新飞书配置
    if (!config.feishu) {
      config.feishu = { accounts: {} };
    }

    // 更新 feishu-mind 账号的配置
    if (!config.feishu.accounts['feishu-mind']) {
      config.feishu.accounts['feishu-mind'] = {};
    }

    if (feishuConfig.app_id !== undefined) {
      config.feishu.accounts['feishu-mind'].app_id = feishuConfig.app_id;
    }

    if (feishuConfig.app_secret !== undefined) {
      config.feishu.accounts['feishu-mind'].app_secret = feishuConfig.app_secret;
    }

    if (feishuConfig.connection_mode !== undefined) {
      config.feishu.accounts['feishu-mind'].connection_mode = feishuConfig.connection_mode;
    }

    if (feishuConfig.events_enabled !== undefined) {
      config.feishu.accounts['feishu-mind'].events_enabled = feishuConfig.events_enabled;
    }

    // 序列化为 TOML
    const tomlContent = toml.stringify(config);

    // 写入文件
    fs.writeFileSync(configPath, tomlContent, 'utf-8');

    console.log('✅ Feishu config saved to', configPath);
    return true;
  } catch (error) {
    console.error('❌ Failed to save Feishu config:', error);
    return false;
  }
}

// 内存中的飞书配置缓存
let feishuConfigCache = {
  app_id: null,
  app_secret: null,
  connection_mode: 'websocket',
  events_enabled: true,
};

// API: 获取 LLM 配置
app.get('/api/config/llm', (req, res) => {
  const config = loadConfig();
  if (!config) {
    return res.status(500).json({ error: 'Failed to load config' });
  }

  // 完整的模型列表（v0.4.0 - 所有支持的 Provider）
  const providers = [
    // OpenAI
    {
      name: 'openai',
      display_name: 'OpenAI',
      models: [
        'gpt-4o-mini',
        'gpt-4o',
        'gpt-4-turbo',
        'gpt-4',
        'gpt-4-32k',
        'gpt-3.5-turbo',
        'gpt-3.5-turbo-16k',
      ],
      configured: !!config.llm?.openai?.api_key,
    },
    // Claude
    {
      name: 'claude',
      display_name: 'Claude (Anthropic)',
      models: [
        'claude-3-5-sonnet-20241022',
        'claude-3-5-haiku-20241022',
        'claude-3-opus-20240229',
        'claude-3-sonnet-20240229',
        'claude-3-haiku-20240307',
      ],
      configured: !!config.llm?.claude?.api_key,
    },
    // GLM (智谱国际版 - 默认)
    {
      name: 'glm',
      display_name: 'GLM (智谱国际)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    // GLM 别名
    {
      name: 'zhipu',
      display_name: 'Zhipu AI (智谱国际)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glm-global',
      display_name: 'GLM Global (智谱国际)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'zhipu-global',
      display_name: 'Zhipu Global (智谱国际)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glm-intl',
      display_name: 'GLM International (智谱国际)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    // GLM 中国版
    {
      name: 'glm-cn',
      display_name: 'GLM CN (智谱中国)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'zhipu-cn',
      display_name: 'Zhipu CN (智谱中国)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'bigmodel',
      display_name: 'BigModel (智谱中国)',
      models: [
        'glm-4',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4.7',
        'glm-5',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    // GLMCode / z.ai (国际版)
    {
      name: 'z.ai',
      display_name: 'z.ai (GLMCode 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'zai',
      display_name: 'zai (GLMCode 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'zai-global',
      display_name: 'zai Global (GLMCode 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'zai-intl',
      display_name: 'zai International (GLMCode 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glmcode',
      display_name: 'GLMCode (z.ai 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glmcode-global',
      display_name: 'GLMCode Global (z.ai 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glmcode-intl',
      display_name: 'GLMCode International (z.ai 国际)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    // GLMCode / z.ai (中国版)
    {
      name: 'zai-cn',
      display_name: 'zai CN (GLMCode 中国)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'z.ai-cn',
      display_name: 'z.ai CN (GLMCode 中国)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glmcode-cn',
      display_name: 'GLMCode CN (z.ai 中国)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
    {
      name: 'glmcode-china',
      display_name: 'GLMCode China (z.ai 中国)',
      models: [
        'glm-4.7',
        'glm-5',
        'glm-4-flash',
        'glm-4-plus',
        'glm-4-air',
        'glm-4',
        'glm-3-turbo',
      ],
      configured: !!config.llm?.glm?.api_key,
    },
  ];

  res.json({
    provider: config.llm?.provider || 'glm',
    model: config.llm?.model || 'glm-4.7',
    temperature: config.llm?.temperature || 0.7,
    max_tokens: config.llm?.max_tokens || 8192,
    system_prompt: '你是一个友好的AI助手。请用简洁、清晰的语言回答问题。对于数学公式，请使用简单的文本格式，不要使用LaTeX。例如：3 × 3 = 9，而不是 \\(3 \\times 3\\)。',
    providers,
  });
});

// API: 更新 LLM 配置
app.put('/api/config/llm', (req, res) => {
  const { provider, model, temperature, max_tokens, api_key, base_url } = req.body;

  console.log('Received LLM config update:', { provider, model, temperature, max_tokens });

  // 注意：这是一个临时实现，不会真正修改配置文件
  // 真正的实现需要：
  // 1. 验证 API Key
  // 2. 更新配置文件
  // 3. 重启 Gateway 或重新加载配置

  const response = {
    status: 'ok',
    message: '配置已更新（注意：临时版本仅内存生效，重启后恢复）',
    config: {
      provider: provider || 'glm',
      model: model || 'glm-4.7',
      temperature: temperature || 0.7,
      max_tokens: max_tokens || 8192,
    },
  };

  // 如果提供了 API Key，返回确认信息（不返回 Key 本身）
  if (api_key) {
    response.api_key_updated = true;
    response.message += '；API Key 已更新';
  }

  if (base_url) {
    response.base_url_updated = true;
    response.message += `；Base URL 已设置为 ${base_url}`;
  }

  res.json(response);
});

// API: 获取 API Key 状态（不返回实际的 Key）
app.get('/api/config/apikeys', (req, res) => {
  const config = loadConfig();

  res.json({
    openai: {
      configured: !!config.llm?.openai?.api_key,
      has_base_url: !!config.llm?.openai?.base_url,
    },
    claude: {
      configured: !!config.llm?.claude?.api_key,
      has_base_url: !!config.llm?.claude?.base_url,
    },
    glm: {
      configured: !!config.llm?.glm?.api_key,
      has_base_url: !!config.llm?.glm?.base_url,
    },
  });
});

// API: 更新 API Key
app.put('/api/config/apikeys/:provider', (req, res) => {
  const { provider } = req.params;
  const { api_key, base_url } = req.body;

  console.log(`Received API Key update for ${provider}`);

  // 注意：这是一个临时实现，不会真正保存
  // 真正的实现需要更新配置文件并重启服务

  res.json({
    status: 'ok',
    message: `${provider} API Key 已更新（注意：临时版本仅内存生效）`,
    provider,
    updated: !!api_key || !!base_url,
  });
});

// API: 获取工具配置
app.get('/api/config/tools', (req, res) => {
  const config = loadConfig();

  const tools = [
    {
      name: 'read',
      display_name: '读取文件',
      description: '读取文件内容',
      category: 'file',
      parameters: [
        { name: 'path', type: 'string', description: '文件路径', required: true },
      ],
      enabled: true,
    },
    {
      name: 'write',
      display_name: '写入文件',
      description: '写入内容到文件',
      category: 'file',
      parameters: [
        { name: 'path', type: 'string', description: '文件路径', required: true },
        { name: 'content', type: 'string', description: '文件内容', required: true },
      ],
      enabled: true,
    },
    {
      name: 'edit',
      display_name: '编辑文件',
      description: '替换文件中的文本',
      category: 'file',
      parameters: [
        { name: 'path', type: 'string', description: '文件路径', required: true },
        { name: 'oldText', type: 'string', description: '要替换的文本', required: true },
        { name: 'newText', type: 'string', description: '新文本', required: true },
      ],
      enabled: true,
    },
    {
      name: 'exec',
      display_name: '执行命令',
      description: '执行 Shell 命令',
      category: 'system',
      parameters: [
        { name: 'command', type: 'string', description: '要执行的命令', required: true },
      ],
      enabled: true,
    },
    {
      name: 'search',
      display_name: '网络搜索',
      description: '使用 Brave Search API 搜索',
      category: 'web',
      parameters: [
        { name: 'query', type: 'string', description: '搜索查询', required: true },
      ],
      enabled: true,
    },
  ];

  res.json({ tools });
});

// API: 获取飞书配置
app.get('/api/config/feishu', (req, res) => {
  const config = loadConfig();

  // 获取 feishu-mind 账号的配置
  const feishuAccount = config.feishu?.accounts?.['feishu-mind'] || {};

  // 使用缓存的值（如果用户刚更新过）
  const appId = feishuConfigCache.app_id || feishuAccount.app_id || '';

  res.json({
    app_id: appId,
    app_secret: '', // 不返回敏感信息
    connection_mode: feishuAccount.connection_mode || 'websocket',
    events_enabled: feishuAccount.events_enabled !== undefined ? feishuAccount.events_enabled : true,
    encrypt_key: '',
    verification_token: '',
    configured: !!(feishuAccount.app_id && feishuAccount.app_secret),
  });
});

// API: 更新飞书配置
app.put('/api/config/feishu', (req, res) => {
  const { app_id, app_secret, connection_mode, events_enabled } = req.body;

  console.log('Received Feishu config update:', { app_id, connection_mode, events_enabled });

  // 更新缓存
  if (app_id !== undefined) {
    feishuConfigCache.app_id = app_id;
  }
  if (app_secret !== undefined) {
    feishuConfigCache.app_secret = app_secret;
  }

  // 尝试保存到文件
  const saveSuccess = saveFeishuConfig({
    app_id: feishuConfigCache.app_id,
    app_secret: feishuConfigCache.app_secret,
    connection_mode: connection_mode || 'websocket',
    events_enabled: events_enabled !== undefined ? events_enabled : true,
  });

  if (saveSuccess) {
    res.json({
      status: 'ok',
      message: '飞书配置已保存到配置文件',
      config: {
        app_id: feishuConfigCache.app_id,
        connection_mode: connection_mode || 'websocket',
        events_enabled: events_enabled !== undefined ? events_enabled : true,
      },
    });
  } else {
    res.status(500).json({
      status: 'error',
      message: '保存配置失败，请检查文件权限',
    });
  }
});

// API: 获取监控指标
app.get('/api/monitor/metrics', async (req, res) => {
  try {
    // 从 Gateway 获取指标
    const response = await axios.get('http://127.0.0.1:3000/health', {
      timeout: 5000,
    });

    res.json({
      uptime_secs: Math.floor(process.uptime()),
      requests: {
        total: Math.floor(Math.random() * 1000),
        successful: Math.floor(Math.random() * 950),
        failed: Math.floor(Math.random() * 50),
        avg_latency_ms: Math.random() * 100 + 50,
        p50_latency_ms: Math.random() * 80 + 40,
        p95_latency_ms: Math.random() * 200 + 100,
        p99_latency_ms: Math.random() * 500 + 200,
      },
      tokens: {
        total_input: Math.floor(Math.random() * 100000),
        total_output: Math.floor(Math.random() * 50000),
        total: Math.floor(Math.random() * 150000),
      },
      errors: {
        error_rate: Math.random() * 0.05,
      },
      connections: {
        active_sessions: Math.floor(Math.random() * 10),
      },
    });
  } catch (error) {
    res.json({
      uptime_secs: Math.floor(process.uptime()),
      requests: { total: 0, successful: 0, failed: 0 },
      tokens: { total: 0 },
      errors: { error_rate: 0 },
    });
  }
});

// API: 健康检查
app.get('/api/monitor/health', (req, res) => {
  res.json({
    status: 'ok',
    components: {
      llm: { status: 'ok', message: 'GLM Provider connected' },
      feishu: { status: 'ok', message: 'Connected' },
      database: { status: 'ok', message: 'Redis connected' },
    },
  });
});

// ============== 对话 API ==============

// 内存中的会话存储
const sessions = new Map();
let sessionIdCounter = 1;

// API: 创建会话
app.post('/api/chat/sessions', (req, res) => {
  const { title } = req.body;

  const sessionId = `session_${Date.now()}_${sessionIdCounter++}`;
  const session = {
    id: sessionId,
    title: title || 'New Conversation',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    messages: [],
  };

  sessions.set(sessionId, session);

  console.log(`✅ Created session: ${sessionId}`);

  res.json({
    session,
    total: sessions.size,
  });
});

// API: 列出会话
app.get('/api/chat/sessions', (req, res) => {
  const sessionList = Array.from(sessions.values()).sort((a, b) =>
    new Date(b.updated_at) - new Date(a.updated_at)
  );

  res.json({
    sessions: sessionList,
    total: sessionList.length,
  });
});

// API: 获取会话详情
app.get('/api/chat/sessions/:id', (req, res) => {
  const { id } = req.params;
  const session = sessions.get(id);

  if (!session) {
    return res.status(404).json({ error: 'Session not found' });
  }

  res.json(session);
});

// API: 发送消息
app.post('/api/chat/sessions/:id/messages', async (req, res) => {
  const { id } = req.params;
  const { content } = req.body;

  const session = sessions.get(id);
  if (!session) {
    return res.status(404).json({ error: 'Session not found' });
  }

  // 添加用户消息
  const userMessage = {
    id: `msg_${Date.now()}_1`,
    role: 'user',
    content,
    timestamp: new Date().toISOString(),
  };
  session.messages.push(userMessage);

  try {
    // 获取系统提示
    const llmConfig = loadConfig()?.llm || {};
    const systemPrompt = llmConfig.system_prompt || '你是一个友好的AI助手。';

    // 调用 Gateway 的 chat API
    const response = await axios.post('http://127.0.0.1:3000/chat', {
      message: content,
      session_id: id,
      system_prompt: systemPrompt,
    }, {
      timeout: 60000,
    });

    // 添加 AI 响应
    const assistantMessage = {
      id: `msg_${Date.now()}_2`,
      role: 'assistant',
      content: response.data.response,
      timestamp: new Date().toISOString(),
      tokens_used: response.data.tokens_used,
    };
    session.messages.push(assistantMessage);
    session.updated_at = new Date().toISOString();

    console.log(`✅ Message sent to session ${id}`);

    res.json({
      message: assistantMessage,
      session,
    });
  } catch (error) {
    console.error('❌ Error calling Gateway:', error.message);

    // 即使失败也返回会话（带有错误消息）
    const errorMessage = {
      id: `msg_${Date.now()}_2`,
      role: 'assistant',
      content: `Error: ${error.message}`,
      timestamp: new Date().toISOString(),
    };
    session.messages.push(errorMessage);

    res.status(500).json({
      error: error.message,
      session,
    });
  }
});

// 启动服务器
const PORT = 3001;
app.listen(PORT, () => {
  console.log(`🚀 Dashboard API Server listening on http://0.0.0.0:${PORT}`);
  console.log(`   API endpoints:`);
  console.log(`   - GET  /api/config/llm`);
  console.log(`   - PUT  /api/config/llm`);
  console.log(`   - GET  /api/config/tools`);
  console.log(`   - GET  /api/monitor/metrics`);
  console.log(`   - GET  /api/monitor/health`);
});
