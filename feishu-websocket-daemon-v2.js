#!/usr/bin/env node
/**
 * NewClaw Feishu WebSocket 长连接守护进程（简化版）
 *
 * 功能：
 * 1. 连接飞书 WebSocket
 * 2. 接收消息事件
 * 3. 转发给 Gateway 处理
 * 4. 发送回复到飞书
 */

const WebSocket = require('ws');
const axios = require('axios');
const toml = require('@iarna/toml');
const fs = require('fs');

// 从配置文件读取
function loadConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    return toml.parse(content);
  } catch (error) {
    console.error('❌ Failed to load config:', error.message);
    return null;
  }
}

function getFeishuConfig(config) {
  try {
    const accounts = config.feishu?.accounts || {};
    const account = accounts['feishu-mind'] || Object.values(accounts)[0];
    
    if (!account) {
      console.error('❌ No Feishu account found in config');
      return null;
    }
    
    console.log(`✅ Loaded Feishu config for app: ${account.app_id}`);
    return account;
  } catch (error) {
    console.error('❌ Error parsing Feishu config:', error.message);
    return null;
  }
}

// 获取访问令牌
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
async function startWebSocket(appId, appSecret) {
  console.log('🔌 Starting Feishu WebSocket connection...');
  console.log(`   App ID: ${appId}`);
  
  // 步骤 1: 获取访问令牌
  const accessToken = await getAccessToken(appId, appSecret);
  console.log('✅ Got access token');
  
  // 步骤 2: 获取 WebSocket 连接信息
  // 注意：飞书的 WebSocket 需要先通过 HTTP API 建立握手
  const wsUrl = 'wss://open.feishu.cn/open-apis/ws/v2';
  
  const ws = new WebSocket(wsUrl, {
    headers: {
      'Authorization': `Bearer ${accessToken}`,
      'X-Feishu-App-Id': appId,
    },
  });
  
  let heartbeatInterval;
  
  ws.on('open', () => {
    console.log('✅ Connected to Feishu WebSocket');
    
    // 发送认证消息
    ws.send(JSON.stringify({
      type: 'auth',
      app_id: appId,
      access_token: accessToken,
    }));
    
    // 启动心跳（每30秒）
    heartbeatInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          type: 'heartbeat',
          timestamp: Date.now(),
        }));
        // console.log('💓 Heartbeat sent');
      }
    }, 30000);
    
    ws.on('close', () => {
      clearInterval(heartbeatInterval);
      console.log('🔌 Connection closed, reconnecting in 5 seconds...');
      setTimeout(() => {
        startWebSocket(appId, appSecret);
      }, 5000);
    });
  });
  
  ws.on('message', async (data) => {
    try {
      const message = JSON.parse(data.toString());
      console.log('📨 Received message from Feishu');
      
      // 处理不同类型的消息
      switch (message.type) {
        case 'event':
          await handleEvent(message, appId, appSecret);
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
      startWebSocket(appId, appSecret);
    }, 5000);
  });
  
  return ws;
}

// 处理事件
async function handleEvent(message, appId, appSecret) {
  console.log('📬 Event received:', message);
  
  if (message.event_type === 'im.message.receive_v1') {
    const content = JSON.parse(message.event.message.content);
    const text = content.text;
    const senderId = message.event.sender.sender_id.open_id;
    const chatId = message.event.message.chat_id;
    
    console.log(`💬 Message from ${senderId}: ${text}`);
    
    // 调用 Gateway 处理
    try {
      const response = await axios.post('http://127.0.0.1:3000/chat', {
        message: text,
        session_id: chatId,
      }, {
        timeout: 60000,
      });
      
      const reply = response.data.response;
      console.log(`✅ Gateway reply: ${reply.substring(0, 100)}...`);
      
      // 发送回复到飞书
      await sendReplyToFeishu(appId, appSecret, senderId, reply);
    } catch (error) {
      console.error('❌ Error calling Gateway:', error.message);
    }
  }
}

// 发送消息到飞书
async function sendReplyToFeishu(appId, appSecret, receiveId, content) {
  try {
    // 获取访问令牌
    const tokenResponse = await axios.post(
      'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal',
      {
        app_id: appId,
        app_secret: appSecret,
      }
    );
    
    const accessToken = tokenResponse.data.tenant_access_token;
    
    // 发送消息
    const response = await axios.post(
      `https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id`,
      {
        receive_id: receiveId,
        content: content,
        msg_type: 'text',
      },
      {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
      }
    );
    
    if (response.data.code === 0) {
      console.log('✅ Reply sent to Feishu successfully');
    } else {
      console.error('❌ Failed to send reply:', response.data.msg);
    }
  } catch (error) {
    console.error('❌ Error sending reply to Feishu:', error.message);
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu WebSocket Daemon starting...');
  
  const config = loadConfig();
  const feishuConfig = getFeishuConfig(config);
  
  if (!feishuConfig) {
    console.error('❌ Failed to load Feishu config');
    process.exit(1);
  }
  
  if (!feishuConfig.enabled) {
    console.log('⚠️  Feishu is disabled in config');
    process.exit(0);
  }
  
  // 启动 WebSocket 连接
  await startWebSocket(feishuConfig.app_id, feishuConfig.app_secret);
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 Shutting down...');
    process.exit(0);
  });
}

main().catch((error) => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});
