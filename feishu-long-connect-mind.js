#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接（cli_a921727d9838dbef）
 */

const Lark = require('@larksuiteoapi/node-sdk');
const axios = require('axios');

const config = {
  name: 'feishu-mind',
  appId: 'cli_a921727d9838dbef',
  appSecret: '0W5xSCyz4EMIAAyIqHsKNdU5qnOGZLtw',
  domain: 'feishu',
  encryptKey: '',
  verificationToken: '',
};

async function main() {
  console.log('🚀 NewClaw Feishu 长连接（cli_a921727d9838dbef）');
  console.log('===========================================');
  console.log('');
  console.log(`📱 应用: ${config.name}`);
  console.log(`📱 App ID: ${config.appId}`);
  console.log(`🌐 Domain: ${config.domain}`);
  console.log('');
  
  // 创建 WSClient
  console.log('🔌 创建 WSClient...');
  const wsClient = new Lark.WSClient({
    appId: config.appId,
    appSecret: config.appSecret,
    domain: Lark.Domain.Feishu,
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
        console.log('');
        console.log('📬 ========== 收到消息事件 ==========');
        console.log(`   时间: ${new Date().toISOString()}`);
        
        const content = JSON.parse(data.message.content);
        const text = content.text;
        const senderId = data.sender.sender_id.open_id;
        const chatId = data.message.chat_id;
        
        console.log(`💬 消息内容: ${text}`);
        console.log(`👤 发送者: ${senderId}`);
        console.log(`💬 会话ID: ${chatId}`);
        
        // 调用 Gateway
        try {
          console.log('🔄 正在调用 Gateway...');
          const gatewayResponse = await axios.post('http://127.0.0.1:3000/chat', {
            message: text,
            session_id: chatId,
          }, {
            timeout: 60000,
          });
          
          const reply = gatewayResponse.data.response;
          console.log(`✅ Gateway 回复: ${reply.substring(0, 100)}...`);
          
          // 创建客户端发送回复
          const client = new Lark.Client({
            appId: config.appId,
            appSecret: config.appSecret,
            appType: Lark.AppType.SelfBuild,
            domain: Lark.Domain.Feishu,
          });
          
          console.log('📤 正在发送回复到飞书...');
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
          console.log('====================================');
          console.log('');
        } catch (error) {
          console.error('❌ 处理消息失败:', error.message);
          console.error('====================================');
          console.log('');
        }
      } catch (error) {
        console.error('❌ 处理事件失败:', error.message);
        console.error('====================================');
        console.log('');
      }
    },
  });
  
  console.log('✅ 事件处理器已注册');
  console.log('');
  
  console.log('📝 飞书开放平台配置：');
  console.log('====================================');
  console.log(`1. 找到应用 ${config.appId}`);
  console.log('2. 事件与回调 → 订阅方式');
  console.log('3. 选择：使用长连接接收事件/回调');
  console.log('4. 启用事件推送');
  console.log('5. 保存配置');
  console.log('');
  
  console.log('🚀 启动 WebSocket 长连接...');
  console.log('⏳ 正在连接到飞书服务器...');
  console.log('');
  
  try {
    await wsClient.start({ eventDispatcher });
    console.log('');
    console.log('✅ WebSocket 长连接已建立！');
    console.log('   现在可以在飞书开放平台保存配置了');
    console.log('');
    console.log('⏳ 等待接收消息...');
    console.log('');
  } catch (error) {
    console.error('❌ 启动长连接失败:', error.message);
    process.exit(1);
  }
  
  // 定期检查连接状态
  setInterval(() => {
    const reconnectInfo = wsClient.getReconnectInfo();
    console.log(`📊 ${new Date().toISOString()} - 连接正常`);
  }, 60000);
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 正在关闭...');
    wsClient.close();
    process.exit(0);
  });
}

main().catch(console.error);
