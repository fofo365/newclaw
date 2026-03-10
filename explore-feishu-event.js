#!/usr/bin/env node
/**
 * NewClaw Feishu SDK Event 模块探索
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

async function exploreEventModule() {
  console.log('🔍 探索飞书 SDK Event 模块');
  console.log('=============================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) process.exit(1);
  
  const client = new Client({
    appId: config.appId,
    appSecret: config.appSecret,
  });
  
  console.log('Event 模块结构:');
  console.log(JSON.stringify(client.event, null, 2));
  console.log('');
  
  console.log('Event.v1 模块结构:');
  if (client.event.v1) {
    console.log(JSON.stringify(Object.keys(client.event.v1), null, 2));
  }
  console.log('');
  
  // 测试获取事件订阅信息
  console.log('📡 测试: 获取事件订阅信息');
  try {
    // 尝试不同的方法
    const methods = [
      'getUserEventSubscription',
      'getEventSubscription',
      'queryEventSubscription',
      'listEventSubscription',
    ];
    
    for (const method of methods) {
      if (client.event.v1[method]) {
        console.log(`✅ 找到方法: ${method}`);
        try {
          const result = await client.event.v1[method]({
            headers: {
              Authorization: `Bearer ${await client.auth.tenantAccessToken.internalGet({
                data: {
                  app_id: config.appId,
                  app_secret: config.appSecret,
                },
              }).then(r => r.tenant_access_token)}`,
            },
          });
          console.log(`   结果:`, JSON.stringify(result, null, 2));
        } catch (error) {
          console.log(`   调用失败:`, error.message);
        }
      }
    }
  } catch (error) {
    console.error('❌ 测试失败:', error.message);
  }
}

exploreEventModule().catch(console.error);
