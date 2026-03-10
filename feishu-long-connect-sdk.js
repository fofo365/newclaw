#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接（使用飞书 SDK 的 WSClient）
 * 
 * 参考 OpenClaw 的实现
 */

const Lark = require('@larksuiteoapi/node-sdk');
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
            encryptKey: account.encrypt_key || '',
            verificationToken: account.verification_token || '',
            domain: account.domain || 'feishu',
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

async function main() {
  console.log('🚀 NewClaw Feishu 长连接（使用 SDK WSClient）');
  console.log('===============================================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 应用: ${config.name} (${config.appId})`);
  console.log(`🌐 Domain: ${config.domain}`);
  console.log('');
  
  // 创建 WSClient
  console.log('🔌 创建 WSClient...');
  const wsClient = new Lark.WSClient({
    appId: config.appId,
    appSecret: config.appSecret,
    domain: config.domain === 'lark' ? Lark.Domain.Lark : Lark.Domain.Feishu,
    loggerLevel: Lark.LoggerLevel.info,
  });
  
  console.log('✅ WSClient 已创建');
  console.log('');
  
  // 创建事件分发器
  console.log('📦 创建 EventDispatcher...');
  const eventDispatcher = new Lark.EventDispatcher({
    encryptKey: config.encryptKey,
    verificationToken: config.verificationToken,
  });
  
  console.log('✅ EventDispatcher 已创建');
  console.log('');
  
  // 注册事件处理器
  eventDispatcher.register({
    'im.message.receive_v1': async (data) => {
      try {
        console.log('📬 收到消息事件');
        console.log('   数据:', JSON.stringify(data).substring(0, 200));
        
        const content = JSON.parse(data.message.content);
        const text = content.text;
        const senderId = data.sender.sender_id.open_id;
        const chatId = data.message.chat_id;
        
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
          
          // 创建客户端发送回复
          const client = new Lark.Client({
            appId: config.appId,
            appSecret: config.appSecret,
            appType: Lark.AppType.SelfBuild,
            domain: config.domain === 'lark' ? Lark.Domain.Lark : Lark.Domain.Feishu,
          });
          
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
      } catch (error) {
        console.error('❌ 处理事件失败:', error.message);
      }
    },
  });
  
  console.log('✅ 事件处理器已注册');
  console.log('');
  
  console.log('📝 飞书开放平台配置要求：');
  console.log('====================================');
  console.log('1. 登录飞书开放平台');
  console.log(`2. 找到应用 ${config.appId}`);
  console.log('3. 事件与回调 → 订阅方式');
  console.log('4. 选择：使用长连接接收事件/回调');
  console.log('5. 启用事件推送');
  console.log('6. 保存配置');
  console.log('');
  
  console.log('🚀 启动 WebSocket 长连接...');
  console.log('');
  
  wsClient.start({ eventDispatcher });
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 正在关闭...');
    wsClient.close();
    process.exit(0);
  });
}

main().catch(console.error);
