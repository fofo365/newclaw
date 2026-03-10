#!/usr/bin/env node
/**
 * NewClaw Feishu SDK 测试
 * 
 * 使用飞书官方 SDK 实现长连接
 */

const { Client } = require('@larksuiteoapi/node-sdk');
const toml = require('@iarna/toml');

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

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu SDK 测试');
  console.log('==============================');
  console.log('');
  
  const config = loadFeishuConfig();
  
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 应用名称: ${config.name}`);
  console.log(`📱 应用 ID: ${config.appId}`);
  console.log('');
  
  // 创建飞书客户端
  const client = new Client({
    appId: config.appId,
    appSecret: config.appSecret,
  });
  
  console.log('✅ 飞书客户端已创建');
  console.log('');
  
  // 测试 1: 获取访问令牌
  console.log('📡 测试 1: 获取访问令牌');
  try {
    const tokenResponse = await request.auth.tenantAccessToken.internalGet({
      data: {
        app_id: config.appId,
        app_secret: config.appSecret,
      },
    });
    
    console.log('✅ 访问令牌获取成功');
    console.log(`   Token: ${tokenResponse.tenant_access_token.substring(0, 20)}...`);
  } catch (error) {
    console.error('❌ 获取访问令牌失败:', error.message);
  }
  
  console.log('');
  
  // 测试 2: 尝试使用 SDK 的事件订阅功能
  console.log('📡 测试 2: 检查 SDK 的事件订阅功能');
  
  // 查看 client 对象有哪些方法
  console.log('可用的 API 模块:');
  Object.keys(client).forEach(key => {
    console.log(`  - ${key}`);
  });
  
  console.log('');
  
  // 测试 3: 检查是否有 WebSocket 相关的方法
  console.log('📡 测试 3: 检查 WebSocket 支持');
  
  if (client.ws) {
    console.log('✅ 发现 WebSocket 模块');
    console.log('  方法:', Object.keys(client.ws));
  } else {
    console.log('⚠️  未找到 WebSocket 模块');
  }
  
  if (client.event) {
    console.log('✅ 发现 Event 模块');
    console.log('  方法:', Object.keys(client.event));
  } else {
    console.log('⚠️  未找到 Event 模块');
  }
  
  console.log('');
  console.log('📝 SDK 对象结构:');
  console.log(JSON.stringify(Object.keys(client), null, 2));
}

main().catch((error) => {
  console.error('❌ 测试失败:', error);
  process.exit(1);
});
