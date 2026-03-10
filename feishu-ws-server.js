#!/usr/bin/env node
/**
 * NewClaw Feishu WebSocket 接收服务器
 * 
 * 功能：
 * 1. 监听端口 3003，接收飞书的 WebSocket 连接
 * 2. 处理连接、认证、心跳
 * 3. 接收消息事件
 * 4. 转发给 Gateway 处理
 * 5. 发送回复到飞书
 */

const WebSocket = require('ws');
const axios = require('axios');
const toml = require('@iarna/toml');
const fs = require('fs');

const PORT = 3003;

// 从配置文件读取飞书配置
function loadFeishuConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    const config = toml.parse(content);
    
    if (config.feishu && config.feishu.accounts) {
      // 遍历所有账号
      for (const [key, account] of Object.entries(config.feishu.accounts)) {
        if (account.enabled) {
          console.log(`✅ Loaded Feishu account: ${key}`);
          return {
            appId: account.app_id,
            appSecret: account.app_secret,
            encryptKey: account.encrypt_key || '',
            verificationToken: account.verification_token || '',
            enabled: account.enabled !== false,
            connectionMode: account.connection_mode || 'websocket',
          };
        }
      }
    }
    
    console.error('❌ No enabled Feishu account found');
    return null;
  } catch (error) {
    console.error('❌ Error loading config:', error.message);
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
        app_app_secret: appSecret,
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

// 发送消息到飞书
async function sendReplyToFeishu(appId, appSecret, receiveId, content) {
  try {
    console.log(`📤 Sending reply to ${receiveId}: ${content.substring(0, 50)}...`);
    
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
      console.log('✅ Reply sent successfully');
      return true;
    } else {
      console.error('❌ Failed to send reply:', response.data.msg);
      return false;
    }
  } catch (error) {
    console.error('❌ Error sending reply to Feishu:', error.message);
    return false;
  }
}

// WebSocket 服务器
const wss = new WebSocket.Server({ port: PORT });

console.log(`🚀 Feishu WebSocket 接收服务器启动...`);
console.log(`   监听端口: ${PORT}`);
console.log(`   WebSocket URL: ws://122.51.14.70:${PORT}`);

// 连接映射
const connections = new Map(); // connection_id -> { ws, appId, appSecret }

wss.on('connection', (ws, req) => {
  console.log('🔗 New connection received');
  
  // 发送连接确认
  ws.send(JSON.stringify({
    code: 0,
    message: 'Connected to NewClaw Feishu Gateway',
  }));
  
  ws.on('message', async (messageData) => {
    try {
      const message = JSON.parse(messageData.toString());
      console.log('📨 Received message:', message.type);
      
      switch (message.type) {
        case 'hello':
          // 握手协议
          console.log('🤝 Handshake from Feishu');
          
          // 获取账号配置
          const config = loadFeishuConfig();
          if (!config) {
            console.error('❌ No Feishu config found');
            ws.close();
            return;
          }
          
          // 认证并保存连接
          connections.set(ws, {
            ws,
            appId: config.appId,
            appSecret: config.appSecret,
          });
          
          // 发送认证确认
          ws.send(JSON.stringify({
            code: 0,
            message: 'Authenticated',
          }));
          
          console.log(`✅ Connection authenticated for app: ${config.appId}`);
          break;
          
        case 'ping':
          // 心跳
          ws.send(JSON.stringify({
            code: 0,
            message: 'pong',
          }));
          break;
          
        case 'event':
          // 事件推送
          const event = message.event;
          console.log(`📬 Event received:`, event.type);
          
          if (event.type === 'im.message.receive_v1') {
            const content = JSON.parse(event.message.content);
            const text = content.text;
            const senderId = event.sender.sender_id.open_id;
            const chatId = event.message.chat_id;
            
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
              const connection = connections.get(ws);
              if (connection) {
                await sendReplyToFeishu(
                  connection.appId,
                  connection.appSecret,
                  senderId,
                  reply
                );
              }
            } catch (error) {
              console.error('❌ Error calling Gateway:', error.message);
            }
          }
          break;
          
        default:
          console.log('⚠️  Unknown message type:', message.type);
      }
    } catch (error) {
      console.error('❌ Error processing message:', error);
    }
  });
  
  ws.on('close', () => {
    console.log('🔌 Connection closed');
    connections.delete(ws);
  });
  
  ws.on('error', (error) => {
    console.error('❌ WebSocket error:', error);
  });
});

// 保持进程运行
process.on('SIGINT', () => {
  console.log('\n👋 Shutting down...');
  
  // 关闭所有连接
  wss.clients.forEach(client => {
    client.close();
  });
  
  wss.close(() => {
    process.exit(0);
  });
});

console.log('');
console.log('📝 飞书开放平台配置信息：');
console.log('   WebSocket URL: ws://122.51.14.70:3003');
console.log('   或者: ws://122.51.14.70:3003/feishu/ws (备用路径)');
console.log('');
console.log('⏳ 等待飞书连接...');
