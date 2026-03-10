#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接守护进程（完整版）
 * 
 * 功能：
 * 1. 获取访问令牌
 * 2. 主动连接到飞书 WebSocket
 * 3. 发送认证消息
 * 4. 处理事件
 * 5. 转发给 Gateway
 * 6. 发送回复到飞书
 */

const WebSocket = require('ws');
const axios = axios;
const toml = require('@iarna/toml');

// 从配置文件读取飞书配置
function loadFeishuConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = require('fs').readFileSync(configPath, 'utf-8');
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

// 建立长连接
async function connectToFeishu(config) {
  console.log('🔌 Connecting to Feishu WebSocket...');
  console.log(`   App ID: ${config.appId}`);
  
  try {
    // 步骤 1: 获取访问令牌
    const accessToken = await getAccessToken(config.appId, config.appSecret);
    console.log('✅ Got access token');
    
    // 步骤 2: 建立 WebSocket 连接
    const wsUrl = 'wss://open.feishu.cn/open-apis/ws/v2';
    const ws = new WebSocket(wsUrl, {
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'X-Feishu-App-Id': config.appId,
      },
    });
    
    let heartbeatInterval;
    let reconnectAttempts = 0;
    const maxReconnectAttempts = 10;
    
    ws.on('open', () => {
      console.log('✅ Connected to Feishu WebSocket');
      reconnectAttempts = 0; // 重置重连计数
      
      // 发送认证消息
      ws.send(JSON.stringify({
        app_id: config.appId,
        secret: config.app_secret,
      }));
      
      // 启动心跳
      heartbeatInterval = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            type: 'heartbeat',
            timestamp: Date.now(),
          }));
          // console.log('💓 Heartbeat sent');
        }
      }, 30000); // 每30秒
      
      ws.on('close', () => {
        clearInterval(heartbeatInterval);
        
        if (reconnectAttempts < maxReconnectAttempts) {
          reconnectAttempts++;
          const delay = Math.min(reconnectAttempts * 5, 60); // 指数退避：5s, 10s, 15s...
          console.log(`🔌 Connection closed, reconnecting in ${delay}s... (attempt ${reconnectAttempts}/${maxReconnectAttempts})`);
          
          setTimeout(() => {
            connectToFeishu(config);
          }, delay * 1000);
        } else {
          console.error('❌ Max reconnection attempts reached, giving up');
          process.exit(1);
        }
      });
    });
    
    ws.on('message', async (data) => {
      try {
        const message = JSON.parse(data.toString());
        console.log('📨 Received message from Feishu');
        
        // 处理认证响应
        if (message.code === 0) {
          console.log('✅ Authentication successful');
          return;
        }
        
        // 处理事件
        if (message.event) {
          await handleEvent(message.event, config);
        }
      } catch (error) {
        console.error('❌ Error parsing message:', error.message);
      }
    });
    
    ws.on('error', (error) => {
      console.error('❌ WebSocket error:', error.message);
    });
    
    ws.on('close', () => {
      console.log('🔌 Connection closed');
    });
    
    return ws;
  } catch (error) {
    console.error('❌ Error connecting to Feishu:', error.message);
      console.error('⚠️  Will retry in 30 seconds...');
      setTimeout(() => {
        connectToFeishu(config);
      }, 30000);
    }
  });
}

// 处理事件
async function handleEvent(event, config) {
  console.log('📬 Event:', event.type);
  
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
      await sendReplyToFeishu(
        config.appId,
        config.appSecret,
        senderId,
        reply
      );
    } catch (error) {
      console.error('❌ Error calling Gateway:', error.message);
    }
  } else if (event.type === 'im.chat.access_event.bot_p2p_chat_entered_v1') {
    console.log('👋 User opened chat with bot');
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw 飞书长连接守护进程启动...');
  console.log('');
  
  // 加载飞书配置
  const config = loadFeishuConfig();
  
  if (!config) {
    console.error('❌ 无法加载飞书配置');
    process.exit(1);
  }
  
  if (!config.enabled) {
    console.log('⚠️  飞书账号在配置中已禁用');
    process.exit(0);
  }
  
  console.log(`📱 App ID: ${config.appId}`);
  console.log(`🔗 连接模式: ${config.connectionMode}`);
  console.log('');
  
  // 根据连接模式选择策略
  if (config.connectionMode === 'websocket') {
    console.log('🔌 使用 WebSocket 长连接模式');
    await connectToFeishu(config);
  } else {
    console.log('⚠️  HTTP 回调模式暂不支持');
    console.log('   请在飞书开放平台切换到长连接模式');
    process.exit(0);
  }
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 Shutting down...');
    process.exit(0);
  });
}

main().catch((error) => {
  console.error('❌ 启动失败:', error);
  process.exit(1);
});
