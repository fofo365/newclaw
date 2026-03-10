#!/usr/bin/env node
/**
 * NewClaw Feishu WebSocket 长连接服务器（使用 SDK）
 * 
 * 功能：
 * 1. 监听 WebSocket 端口，等待飞书连接
 * 2. 处理飞书推送的事件
 * 3. 使用 SDK 转发给 Gateway
 * 4. 使用 SDK 发送回复
 */

const WebSocket = require('ws');
const { Client } = require('@larksuiteoapi/node-sdk');
const toml = require('@iarna/toml');
const axios = require('axios');

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
          };
        }
      }
    }
    
    return null;
  } catch (error) {
    console.error('❌ Error loading config:', error.message);
    return null;
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu WebSocket 长连接服务器（使用 SDK）');
  console.log('=======================================================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 应用: ${config.name} (${config.appId})`);
  console.log(`🔌 监听端口: ${WS_PORT}`);
  console.log('');
  
  // 创建飞书 SDK 客户端（用于发送消息）
  const client = new Client({
    appId: config.appId,
    appSecret: config.appSecret,
  });
  
  console.log('✅ 飞书 SDK 客户端已创建');
  console.log('');
  
  // 创建 WebSocket 服务器
  const wss = new WebSocket.Server({ port: WS_PORT });
  
  console.log('✅ WebSocket 服务器已启动');
  console.log('');
  console.log('📝 请在飞书开放平台配置：');
  console.log('====================================');
  console.log('1. 事件订阅 → 切换到长连接模式');
  console.log(`2. WebSocket URL: ws://122.51.14.70:${WS_PORT}`);
  console.log('3. 启用事件推送');
  console.log('4. 保存配置');
  console.log('');
  console.log('⏳ 等待飞书连接...');
  console.log('');
  
  const connections = new Map();
  
  wss.on('connection', (ws, req) => {
    console.log('🔗 新连接建立');
    
    ws.on('message', async (data) => {
      try {
        const message = JSON.parse(data.toString());
        console.log('📨 收到消息:', message.type || message.event_type || 'unknown');
        
        // 处理事件
        if (message.header && message.header.event_type) {
          const eventType = message.header.event_type;
          
          if (eventType === 'im.message.receive_v1') {
            const event = message.event;
            const content = JSON.parse(event.message.content);
            const text = content.text;
            const senderId = event.sender.sender_id.open_id;
            const chatId = event.message.chat_id;
            
            console.log(`💬 消息: ${text.substring(0, 50)}...`);
            
            // 调用 Gateway
            try {
              const gatewayResponse = await axios.post('http://127.0.0.1:3000/chat', {
                message: text,
                session_id: chatId,
              }, {
                timeout: 60000,
              });
              
              const reply = gatewayResponse.data.response;
              console.log(`✅ Gateway 回复: ${reply.substring(0, 50)}...`);
              
              // 使用 SDK 发送回复
              try {
                await client.im.message.create({
                  data: {
                    receive_id: senderId,
                    content: JSON.stringify({ text: reply }),
                    msg_type: 'text',
                  },
                  params: {
                    receive_id_type: 'open_id',
                  },
                });
                console.log('✅ 回复已发送（使用 SDK）');
              } catch (error) {
                console.error('❌ 发送回复失败（SDK）:', error.message);
              }
            } catch (error) {
              console.error('❌ 调用 Gateway 失败:', error.message);
            }
          }
        }
      } catch (error) {
        console.error('❌ 处理消息失败:', error.message);
      }
    });
    
    ws.on('close', () => {
      console.log('🔌 连接已关闭');
    });
    
    ws.on('error', (error) => {
      console.error('❌ WebSocket 错误:', error.message);
    });
  });
  
  process.on('SIGINT', () => {
    console.log('\n👋 正在关闭...');
    wss.clients.forEach(client => client.close());
    wss.close(() => process.exit(0));
  });
}

main().catch(console.error);
