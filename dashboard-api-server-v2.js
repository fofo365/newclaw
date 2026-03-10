#!/usr/bin/env node
/**
 * NewClaw Dashboard API Server v0.4.0
 * 带身份认证
 */

const express = require('express');
const axios = require('axios');
const toml = require('@iarna/toml');
const crypto = require('crypto');
const fs = require('fs');

const app = express();
app.use(express.json());

// 配置
const CONFIG_PATH = '/etc/newclaw/config.toml';
const PAIRING_CODES_FILE = '/tmp/newclaw_pairing_codes.json';
const SESSIONS_FILE = '/tmp/newclaw_sessions.json';

// 配对码和会话存储
let pairingCodes = {};
let sessions = {};

// 加载存储的数据
function loadStorage() {
  try {
    if (fs.existsSync(PAIRING_CODES_FILE)) {
      pairingCodes = JSON.parse(fs.readFileSync(PAIRING_CODES_FILE, 'utf-8'));
    }
    if (fs.existsSync(SESSIONS_FILE)) {
      sessions = JSON.parse(fs.readFileSync(SESSIONS_FILE, 'utf-8'));
    }
  } catch (error) {
    console.error('Failed to load storage:', error.message);
  }
}

// 保存存储的数据
function saveStorage() {
  try {
    fs.writeFileSync(PAIRING_CODES_FILE, JSON.stringify(pairingCodes, null, 2));
    fs.writeFileSync(SESSIONS_FILE, JSON.stringify(sessions, null, 2));
  } catch (error) {
    console.error('Failed to save storage:', error.message);
  }
}

// 读取配置
function loadConfig() {
  try {
    const content = fs.readFileSync(CONFIG_PATH, 'utf-8');
    return toml.parse(content);
  } catch (error) {
    console.error('Failed to load config:', error.message);
    return null;
  }
}

// 生成配对码
function generatePairingCode() {
  const code = crypto.randomBytes(4).toString('hex').toUpperCase();
  const token = crypto.randomBytes(32).toString('hex');
  
  pairingCodes[code] = {
    token,
    createdAt: Date.now(),
    expiresAt: Date.now() + 5 * 60 * 1000, // 5分钟过期
  };
  
  saveStorage();
  
  return { code, token };
}

// 验证配对码
function verifyPairingCode(code) {
  const pairing = pairingCodes[code];
  
  if (!pairing) {
    return { valid: false, error: 'Invalid pairing code' };
  }
  
  if (Date.now() > pairing.expiresAt) {
    delete pairingCodes[code];
    saveStorage();
    return { valid: false, error: 'Pairing code expired' };
  }
  
  // 创建会话
  const sessionToken = crypto.randomBytes(32).toString('hex');
  sessions[sessionToken] = {
    pairingCode: code,
    createdAt: Date.now(),
    expiresAt: Date.now() + 24 * 60 * 60 * 1000, // 24小时过期
  };
  
  // 删除已使用的配对码
  delete pairingCodes[code];
  saveStorage();
  
  return { valid: true, token: sessionToken };
}

// 验证会话令牌
function verifySessionToken(token) {
  const session = sessions[token];
  
  if (!session) {
    return { valid: false };
  }
  
  if (Date.now() > session.expiresAt) {
    delete sessions[token];
    saveStorage();
    return { valid: false };
  }
  
  return { valid: true };
}

// 认证中间件
function requireAuth(req, res, next) {
  const token = req.headers['x-session-token'];
  
  if (!token) {
    return res.status(401).json({ error: 'Unauthorized: No token provided' });
  }
  
  const verification = verifySessionToken(token);
  
  if (!verification.valid) {
    return res.status(401).json({ error: 'Unauthorized: Invalid or expired token' });
  }
  
  next();
}

// API: 生成配对码
app.post('/api/auth/pairing-code', (req, res) => {
  const { code, token } = generatePairingCode();
  
  console.log('📱 Generated pairing code:', code);
  
  res.json({
    code,
    message: 'Use this code in NewClaw CLI: newclaw dashboard pair ' + code,
    expiresAt: new Date(Date.now() + 5 * 60 * 1000).toISOString(),
  });
});

// API: 验证配对码并获取会话令牌
app.post('/api/auth/verify', (req, res) => {
  const { code } = req.body;
  
  if (!code) {
    return res.status(400).json({ error: 'Pairing code is required' });
  }
  
  const verification = verifyPairingCode(code.toUpperCase());
  
  if (!verification.valid) {
    return res.status(401).json({ error: verification.error });
  }
  
  console.log('✅ Pairing code verified:', code.toUpperCase());
  
  res.json({
    token: verification.token,
    message: 'Authentication successful',
  });
});

// API: 检查会话状态
app.get('/api/auth/status', requireAuth, (req, res) => {
  res.json({
    authenticated: true,
    message: 'Session valid',
  });
});

// API: 登出
app.post('/api/auth/logout', requireAuth, (req, res) => {
  const token = req.headers['x-session-token'];
  
  delete sessions[token];
  saveStorage();
  
  console.log('👋 User logged out');
  
  res.json({ message: 'Logged out successfully' });
});

// API: 获取 LLM 配置（需要认证）
app.get('/api/config/llm', requireAuth, (req, res) => {
  try {
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
  } catch (error) {
    console.error('Error loading config:', error);
    res.status(500).json({ error: 'Failed to load config' });
  }
});

// API: 聊天（需要认证）
app.post('/api/chat', requireAuth, async (req, res) => {
  try {
    const { message, session_id } = req.body;
    
    const response = await axios.post('http://127.0.0.1:3000/chat', {
      message,
      session_id: session_id || 'dashboard',
    }, {
      timeout: 60000,
    });
    
    res.json(response.data);
  } catch (error) {
    console.error('Error calling chat:', error.message);
    res.status(500).json({ error: 'Failed to get response' });
  }
});

// 健康检查（无需认证）
app.get('/health', (req, res) => {
  res.json({ status: 'ok', service: 'dashboard-api' });
});

// 启动服务器
loadStorage();
const PORT = 3001;
app.listen(PORT, () => {
  console.log(`🚀 Dashboard API Server listening on http://0.0.0.0:${PORT}`);
  console.log('');
  console.log('📋 API endpoints:');
  console.log('   - POST /api/auth/pairing-code (生成配对码)');
  console.log('   - POST /api/auth/verify (验证配对码)');
  console.log('   - GET  /api/auth/status (检查会话)');
  console.log('   - POST /api/auth/logout (登出)');
  console.log('');
  console.log('   - GET  /api/config/llm (需要认证)');
  console.log('   - POST /api/chat (需要认证)');
  console.log('');
  console.log('🔒 认证流程:');
  console.log('   1. 前端调用 POST /api/auth/pairing-code 获取配对码');
  console.log('   2. 用户在 CLI 执行: newclaw dashboard pair <code>');
  console.log('   3. 前端调用 POST /api/auth/verify 验证配对码');
  console.log('   4. 获得 token，后续请求携带 X-Session-Token 头');
});

// 定期清理过期数据
setInterval(() => {
  const now = Date.now();
  
  // 清理过期配对码
  for (const code in pairingCodes) {
    if (now > pairingCodes[code].expiresAt) {
      delete pairingCodes[code];
    }
  }
  
  // 清理过期会话
  for (const token in sessions) {
    if (now > sessions[token].expiresAt) {
      delete sessions[token];
    }
  }
  
  saveStorage();
}, 60 * 1000); // 每分钟清理一次
