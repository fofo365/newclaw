#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接客户端（主动连接飞书）
 * 
 * 功能：
 * 1. 主动连接到飞书的 WebSocket 服务器
 * 2. 进行认证
 * 3. 接收事件
 * 4. 转发给 Gateway
 * 5. 使用 SDK 发送回复
 */

const WebSocket = require('ws');
const { Client } = require('@larksuiteoapi/node-sdk');
const toml = require('@iarna/toml');
const axios = require('axios');

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

// 主动连接到飞书
async function connectToFeishu(config, client) {
  console.log('🔌 正在连接到飞书...');
  
  try {
    // 获取访问令牌
    const accessToken = await getAccessToken(config.appId, config.appSecret);
    console.log('✅ 访问令牌已获取');
    
    // 尝试连接到飞书 WebSocket
    // 根据飞书文档，WebSocket URL 可能需要先通过 API 获取
    console.log('📡 尝试获取 WebSocket 连接信息...');
    
    // 方式 1: 尝试直接连接
    const wsUrls = [
      'wss://open.feishu.cn/open-apis/ws/v2',
      'wss://push.feishu.cn/open-apis/ws/v2',
    ];
    
    for (const wsUrl of wsUrls) {
      console.log(`🔗 尝试连接: ${wsUrl}`);
      
      try {
        const ws = new WebSocket(wsUrl, {
          headers: {
            'Authorization': `Bearer ${accessToken}`,
            'X-Feishu-App-Id': config.appId,
          },
        });
        
        await new Promise((resolve, reject) => {
          const timeout = setTimeout(() => {
            ws.close();
            reject(new Error('Connection timeout'));
          }, 10000);
          
          ws.on('open', () => {
            clearTimeout(timeout);
            console.log('✅ WebSocket 连接成功！');
            
            // 发送认证消息
            ws.send(JSON.stringify({
              app_id: config.appId,
              access_token: accessToken,
            }));
            
            resolve(ws);
          });
          
          ws.on('error', (error) => {
            clearTimeout(timeout);
            reject(error);
          });
          
          ws.on('message', async (data) => {
            try {
              const message = JSON.parse(data.toString());
              console.log('📨 收到消息:', message);
              
              // 处理事件
              if (message.event || (message.header && message.header.event_type)) {
                const event = message.event || message;
                const eventType = event.type || message.header.event_type;
                
                if (eventType === 'im.message.receive_v1') {
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
                    console.log('✅ 回复已发送');
                  } catch (error) {
                    console.error('❌ 处理消息失败:', error.message);
                  }
                }
              }
            } catch (error) {
              console.error('❌ 解析消息失败:', error.message);
            }
          });
          
          ws.on('close', () => {
            console.log('🔌 连接已关闭，10秒后重连...');
            setTimeout(() => {
              connectToFeishu(config, client);
            }, 10000);
          });
          
          ws.on('error', (error) => {
            console.error('❌ WebSocket 错误:', error.message);
          });
        });
        
        console.log('✅ 长连接已建立');
        return;
      } catch (error) {
        console.error(`❌ 连接失败: ${error.message}`);
      }
    }
    
    console.log('⚠️  所有连接方式都失败了');
    console.log('');
    console.log('📝 可能的原因：');
    console.log('   1. 飞书长连接需要先在开放平台配置');
    console.log('   2. 需要特殊的权限或企业版');
    console.log('   3. API 端点已变更');
    console.log('');
    console.log('💡 建议：继续使用 HTTP 回调模式（已修复）');
    
  } catch (error) {
    console.error('❌ 连接失败:', error.message);
    console.log('⏳ 10秒后重试...');
    setTimeout(() => {
      connectToFeishu(config, client);
    }, 10000);
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu 长连接客户端');
  console.log('==================================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 应用: ${config.name} (${config.appId})`);
  console.log('');
  
  // 创建飞书 SDK 客户端
  const client = new Client({
    appId: config.appId,
    appSecret: config.appSecret,
  });
  
  console.log('✅ 飞书 SDK 客户端已创建');
  console.log('');
  
  // 开始连接
  await connectToFeishu(config, client);
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 正在关闭...');
    process.exit(0);
  });
}

main().catch(console.error);
