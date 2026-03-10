#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接（直接连接版）
 * 
 * 功能：
 * 1. 获取访问令牌
 * 2. 直接连接到飞书 WebSocket
 * 3. 发送认证消息
 * 4. 处理事件
 */

const WebSocket = require('ws');
const axios = require('axios');
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

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu 长连接测试');
  console.log('================================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 App ID: ${config.appId}`);
  console.log('');
  
  // 获取访问令牌
  const accessToken = await getAccessToken(config.appId, config.appSecret);
  console.log('✅ Got access token');
  console.log('');
  
  // 尝试不同的 WebSocket URL
  const wsUrls = [
    'wss://open.feishu.cn/open-apis/ws/v2',
    'wss://open.feishu.cn/open-apis/event/v2/ws',
    'wss://push.feishu.cn/open-apis/ws/v2',
    'wss://push.feishu.cn/open-apis/event/v2/ws',
  ];
  
  for (const wsUrl of wsUrls) {
    console.log(`🔍 Trying WebSocket URL: ${wsUrl}`);
    
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
          console.log('✅ Connected successfully!');
          console.log('');
          console.log('📝 WebSocket URL 配置成功！');
          console.log(`   URL: ${wsUrl}`);
          console.log('');
          console.log('📝 请将此 URL 配置到飞书开放平台：');
          console.log(`   1. 登录飞书开放平台`);
          console.log(`   2. 找到应用 ${config.appId}`);
          console.log('   3. 事件订阅 → 切换到长连接模式');
          console.log(`   4. WebSocket URL: ${wsUrl}`);
          console.log('   5. 启用事件推送');
          console.log('   6. 保存配置');
          
          ws.close();
          resolve();
        });
        
        ws.on('error', (error) => {
          clearTimeout(timeout);
          reject(error);
        });
      });
      
      // 如果连接成功，退出
      process.exit(0);
    } catch (error) {
      console.error(`❌ Failed: ${error.message}`);
      console.log('');
    }
  }
  
  console.log('⚠️  所有 WebSocket URL 都失败了');
  console.log('');
  console.log('📝 可能的原因：');
  console.log('   1. 飞书长连接功能需要先在开放平台配置');
  console.log('   2. 需要先启用事件推送功能');
  console.log('   3. WebSocket URL 可能需要认证参数');
  console.log('');
  console.log('💡 建议步骤：');
  console.log('   1. 登录飞书开放平台');
  console.log(`   2. 找到应用 ${config.appId}`);
  console.log('   3. 事件订阅 → 添加事件 → 订阅所需事件');
  console.log('   4. 先配置 HTTP 回调模式测试');
  console.log('   5. 确认可以正常接收消息后，再切换到长连接模式');
}

main().catch((error) => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});
