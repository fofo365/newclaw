#!/usr/bin/env node

/**
 * NewClaw Feishu WebSocket 长连接守护进程
 *
 * 维持与飞书服务器的 WebSocket 长连接
 */

const WebSocket = require('ws');
const axios = require('axios');
const toml = require('@iarna/toml');
const fs = require('fs');

// 读取配置
function loadConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    return toml.parse(content);
  } catch (error) {
    console.error('❌ Failed to load config:', error.message);
    process.exit(1);
  }
}

function getFeishuConfig(config) {
  const accounts = config.feishu?.accounts || {};
  const account = accounts['feishu-mind'] || Object.values(accounts)[0];
  
  if (!account) {
    console.error('❌ No Feishu account found in config');
    process.exit(1);
  }
  
  if (!account.enabled) {
    console.error('❌ Feishu is disabled in config');
    process.exit(1);
  }
  
  return {
    appId: account.app_id,
    appSecret: account.app_secret,
    encryptKey: account.encrypt_key || '',
    verificationToken: account.verification_token || '',
  };
}

// 获取飞书访问令牌
async function getAccessToken(appId, appSecret) {
  try {
    const response = await axios.post(
      'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal',
      {
        app_id: appId,
        app_secret: appSecret,
      }
    );
    
    if (response.data.code === 0) {
      return response.data.tenant_access_token;
    } else {
      throw new Error(`Failed to get access token: ${response.data.msg}`);
    }
  } catch (error) {
    console.error('❌ Error getting access token:', error.message);
    throw error;
  }
}

// 建立长连接
async function connectWebSocket(appId, appSecret) {
  console.log('🔌 Connecting to Feishu WebSocket...');
  console.log(`   App ID: ${appId}`);
  
  // 获取访问令牌
  const accessToken = await getAccessToken(appId, appSecret);
  console.log('✅ Got access token');
  
  // 飞书 WebSocket 端点
  // 注意：实际使用时需要先调用 HTTP API 获取 WebSocket 连接信息
  const wsUrl = 'wss://open.feishu.cn/open-apis/ws/v2';
  
  const ws = new WebSocket(wsUrl, {
    headers: {
      'Authorization': `Bearer ${accessToken}`,
      'X-Feishu-App-Id': appId,
    },
  });
  
  ws.on('open', () => {
    console.log('✅ Connected to Feishu WebSocket');
    
    // 发送认证消息
    ws.send(JSON.stringify({
      type: 'auth',
      app_id: appId,
      access_token: accessToken,
    }));
    
    // 启动心跳
    const heartbeatInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          type: 'heartbeat',
          timestamp: Date.now(),
        }));
        console.log('💓 Heartbeat sent');
      }
    }, 30000); // 每30秒
    
    ws.on('close', () => {
      clearInterval(heartbeatInterval);
    });
  });
  
  ws.on('message', (data) => {
    try {
      const message = JSON.parse(data.toString());
      console.log('📨 Received message:', message);
      
      // 处理不同类型的消息
      switch (message.type) {
        case 'event':
          console.log('📬 Event received:', message.event);
          // 这里可以转发给 Gateway 处理
          break;
          
        case 'heartbeat_ack':
          console.log('💓 Heartbeat acknowledged');
          break;
          
        case 'auth_ack':
          console.log('✅ Authentication acknowledged');
          break;
          
        default:
          console.log('⚠️  Unknown message type:', message.type);
      }
    } catch (error) {
      console.error('❌ Error parsing message:', error.message);
    }
  });
  
  ws.on('error', (error) => {
    console.error('❌ WebSocket error:', error.message);
  });
  
  ws.on('close', () => {
    console.log('🔌 Connection closed, reconnecting in 5 seconds...');
    setTimeout(() => {
      connectWebSocket(appId, appSecret);
    }, 5000);
  });
  
  return ws;
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu WebSocket Daemon starting...');
  
  const config = loadConfig();
  const feishuConfig = getFeishuConfig(config);
  
  console.log(`✅ Loaded Feishu config for app: ${feishuConfig.appId}`);
  
  // 连接到飞书
  const ws = await connectWebSocket(feishuConfig.appId, feishuConfig.appSecret);
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 Shutting down...');
    ws.close();
    process.exit(0);
  });
}

main().catch((error) => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});
