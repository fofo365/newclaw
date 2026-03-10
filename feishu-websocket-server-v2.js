#!/usr/bin/env node
/**
 * NewClaw Feishu WebSocket 服务器（接收飞书的连接）
 * 
 * 功能：
 * 1. 监听 WebSocket 端口
 * 2. 等待飞书连接
 * 3. 处理飞书推送的事件
 * 4. 转发给 Gateway
 * 5. 发送回复
 */

const WebSocket = require('ws');
const axios = require('axios');
const toml = require('@iarna/toml');

const WS_PORT = 3003;

// 从配置文件读取飞书配置
function loadFeishuConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = require('fs').readFileSync(configPath, 'utf-8');
    const config = toml.parse(content);
    
    if (config.feishu && config.feishu.accounts) {
      for (const [key, account] of Object.entries(config.feishu.accounts)) {
        if (account.enabled) {
          return {
            name: key,
            appId: account.app_id,
            appSecret: account.app_secret,
            encryptKey: account.encrypt_key || '',
            verificationToken: account.verification_token || '',
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

// 发送回复到飞书
async function sendReplyToFeishu(appId, appSecret, receiveId, content) {
  try {
    const accessToken = await getAccessToken(appId, appSecret);
    
    const response = await axios.post(
      `https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id`,
      {
        receive_id: receiveId,
        content: JSON.stringify({ text: content }),
        msg_type: 'text',
      },
      {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
      }
    );
    
    if (response.data.code === 0) {
      console.log('✅ Reply sent to Feishu');
      return true;
    } else {
      console.error('❌ Failed to send reply:', response.data.msg);
      return false;
    }
  } catch (error) {
    console.error('❌ Error sending reply:', error.message);
    return false;
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu WebSocket 服务器');
  console.log('====================================');
  console.log('');
  
  // 加载飞书配置
  const config = loadFeishuConfig();
  
  if (!config) {
    console.error('❌ 无法加载飞书配置');
    process.exit(1);
  }
  
  console.log(`📱 应用名称: ${config.name}`);
  console.log(`📱 应用 ID: ${config.appId}`);
  console.log(`🔌 监听端口: ${WS_PORT}`);
  console.log('');
  
  // 创建 WebSocket 服务器
  const wss = new WebSocket.Server({ port: WS_PORT });
  
  console.log('✅ WebSocket 服务器已启动');
  console.log('');
  console.log('📝 请在飞书开放平台配置以下信息：');
  console.log('====================================');
  console.log('');
  console.log('1. 登录飞书开放平台');
  console.log(`2. 找到应用 ${config.appId}`);
  console.log('3. 事件订阅 → 切换到长连接模式');
  console.log(`4. WebSocket URL: ws://122.51.14.70:${WS_PORT}`);
  console.log('5. 启用事件推送');
  console.log('6. 保存配置');
  console.log('');
  console.log('⏳ 等待飞书连接...');
  console.log('');
  
  // 连接映射
  const connections = new Map();
  
  wss.on('connection', (ws, req) => {
    console.log('');
    console.log('🔗 新的连接建立！');
    console.log(`   来自: ${req.socket.remoteAddress}`);
    
    // 发送欢迎消息
    ws.send(JSON.stringify({
      type: 'hello',
      app_id: config.appId,
      message: 'NewClaw Feishu Gateway is ready',
    }));
    
    ws.on('message', async (data) => {
      try {
        const message = JSON.parse(data.toString());
        console.log('📨 收到消息:', message.type || message.event_type || 'unknown');
        
        // 处理认证消息
        if (message.type === 'auth' || message.schema === '2.0') {
          console.log('✅ 认证消息');
          
          // 保存连接信息
          connections.set(ws, {
            ws,
            appId: config.appId,
            appSecret: config.appSecret,
            connectedAt: new Date(),
          });
          
          // 发送确认
          ws.send(JSON.stringify({
            code: 0,
            message: 'Authenticated',
          }));
          
          console.log('✅ 连接已认证');
          return;
        }
        
        // 处理事件推送
        if (message.header && message.header.event_type) {
          const eventType = message.header.event_type;
          console.log(`📬 事件类型: ${eventType}`);
          
          if (eventType === 'im.message.receive_v1') {
            const event = message.event;
            const content = JSON.parse(event.message.content);
            const text = content.text;
            const senderId = event.sender.sender_id.open_id;
            const chatId = event.message.chat_id;
            
            console.log(`💬 消息来自 ${senderId}: ${text}`);
            
            // 调用 Gateway 处理
            try {
              const gatewayResponse = await axios.post('http://127.0.0.1:3000/chat', {
                message: text,
                session_id: chatId,
              }, {
                timeout: 60000,
              });
              
              const reply = gatewayResponse.data.response;
              console.log(`✅ Gateway 回复: ${reply.substring(0, 100)}...`);
              
              // 发送回复到飞书
              await sendReplyToFeishu(
                config.appId,
                config.appSecret,
                senderId,
                reply
              );
            } catch (error) {
              console.error('❌ 调用 Gateway 失败:', error.message);
            }
          }
          
          return;
        }
        
        // 处理心跳
        if (message.type === 'ping' || message.type === 'heartbeat') {
          ws.send(JSON.stringify({
            type: 'pong',
            timestamp: Date.now(),
          }));
          return;
        }
        
        console.log('⚠️  未知消息类型:', Object.keys(message));
      } catch (error) {
        console.error('❌ 处理消息失败:', error.message);
      }
    });
    
    ws.on('close', () => {
      console.log('🔌 连接已关闭');
      connections.delete(ws);
    });
    
    ws.on('error', (error) => {
      console.error('❌ WebSocket 错误:', error.message);
    });
    
    ws.on('ping', () => {
      ws.pong();
    });
  });
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 正在关闭服务器...');
    
    wss.clients.forEach(client => {
      client.close();
    });
    
    wss.close(() => {
      console.log('✅ 服务器已关闭');
      process.exit(0);
    });
  });
}

main().catch((error) => {
  console.error('❌ 启动失败:', error);
  process.exit(1);
});
