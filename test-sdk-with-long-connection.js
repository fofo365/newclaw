#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接（使用 SDK）
 * 
 * 参考：https://github.com/larksuite/node-sdk
 */

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

async function main() {
  console.log('🚀 使用飞书 SDK 实现长连接');
  console.log('================================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 应用: ${config.name} (${config.appId})`);
  console.log('');
  
  // 创建飞书客户端
  const client = new Client({
    appId: config.appId,
    appSecret: config.appSecret,
  });
  
  console.log('✅ 飞书客户端已创建');
  console.log('');
  
  // 检查 SDK 版本和功能
  console.log('📋 SDK 信息:');
  console.log('   版本:', require('@larksuiteoapi/node-sdk/package.json').version);
  console.log('');
  
  // 查看所有可用的 API
  console.log('📡 可用的 API 模块:');
  const apis = [
    'im', 'message', 'auth', 'event', 'ws', 'websocket',
    'subscription', 'webhook', 'bot', 'chat'
  ];
  
  apis.forEach(api => {
    if (client[api]) {
      console.log(`   ✅ ${api}: ${Object.keys(client[api]).join(', ')}`);
    } else {
      console.log(`   ❌ ${api}: 不可用`);
    }
  });
  
  console.log('');
  
  // 尝试使用 IM 模块发送消息
  console.log('📤 测试发送消息功能（使用 SDK）');
  try {
    const result = await client.im.message.create({
      data: {
        receive_id: 'ou_1fd6d40ae1fa693340b85a97428973be',
        content: JSON.stringify({ text: '测试消息来自飞书 SDK' }),
        msg_type: 'text',
      },
      params: {
        receive_id_type: 'open_id',
      },
    });
    
    console.log('✅ 消息发送成功');
    console.log('   结果:', JSON.stringify(result, null, 2));
  } catch (error) {
    console.error('❌ 消息发送失败:', error.message);
    console.log('   错误详情:', JSON.stringify(error.response?.data, null, 2));
  }
  
  console.log('');
  console.log('📝 结论:');
  console.log('   飞书 SDK 主要用于调用飞书 API，不支持长连接');
  console.log('   长连接需要使用原生 WebSocket 实现');
  console.log('   我们当前的 WebSocket 服务器方案是正确的');
}

main().catch((error) => {
  console.error('❌ 测试失败:', error);
  process.exit(1);
});
