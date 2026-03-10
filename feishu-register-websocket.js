#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接注册工具（完整版）
 * 
 * 功能：
 * 1. 获取访问令牌
 * 2. 调用飞书 API 注册长连接
 * 3. 返回连接信息
 */

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

// 注册长连接
async function registerWebSocketConnection(appId, appSecret) {
  console.log('🔌 Registering WebSocket long connection with Feishu...');
  console.log(`   App ID: ${appId}`);
  console.log('');
  
  try {
    // 步骤 1: 获取访问令牌
    const accessToken = await getAccessToken(appId, appSecret);
    console.log('✅ Got access token');
    console.log('');
    
    // 步骤 2: 尝试不同的 API 端点
    
    const endpoints = [
      {
        name: 'User WebSocket Subscription (v2)',
        method: 'GET',
        url: 'https://open.feishu.cn/open-apis/event/v2/user/websocket',
      },
      {
        name: 'Bot WebSocket Subscription (v2)',
        method: 'GET',
        url: 'https://open.feishu.cn/open-apis/event/v2/bot/websocket',
      },
      {
        name: 'Event Subscription Info (v1)',
        method: 'GET',
        url: 'https://open.feishu.cn/open-apis/event/v1/user/event_subscription',
      },
      {
        name: 'Event Subscription Info (v2)',
        method: 'GET',
        url: 'https://open.feishu.cn/open-apis/event/v2/subscription',
      },
      {
        name: 'Subscribe Events',
        method: 'POST',
        url: 'https://open.feishu.cn/open-apis/event/v4/subscription',
        data: {
          event_type: ['im.message.receive_v1'],
        },
      },
    ];
    
    for (const endpoint of endpoints) {
      console.log(`📡 Trying: ${endpoint.name}`);
      console.log(`   URL: ${endpoint.url}`);
      
      try {
        let response;
        
        if (endpoint.method === 'GET') {
          response = await axios.get(endpoint.url, {
            headers: {
              'Authorization': `Bearer ${accessToken}`,
              'Content-Type': 'application/json',
            },
          });
        } else if (endpoint.method === 'POST') {
          response = await axios.post(endpoint.url, endpoint.data || {}, {
            headers: {
              'Authorization': `Bearer ${accessToken}`,
              'Content-Type': 'application/json',
            },
          });
        }
        
        console.log('✅ Success!');
        console.log('   Response:', JSON.stringify(response.data, null, 2));
        console.log('');
        
        // 检查是否有有用的信息
        if (response.data.code === 0 && response.data.data) {
          const data = response.data.data;
          
          if (data.websocket) {
            console.log('🎉 Found WebSocket info!');
            console.log(`   URL: ${data.websocket.url || data.websocket.address || data.websocket.endpoint}`);
            console.log('');
            console.log('📝 请将此 URL 配置到飞书开放平台的长连接设置中');
            return data.websocket;
          }
          
          if (data.url || data.address || data.endpoint) {
            console.log('🎉 Found connection info!');
            console.log(`   URL: ${data.url || data.address || data.endpoint}`);
            console.log('');
            console.log('📝 请将此 URL 配置到飞书开放平台的长连接设置中');
            return data;
          }
          
          if (data.subscriptions) {
            console.log('📋 Current subscriptions:');
            data.subscriptions.forEach(sub => {
              console.log(`   - ${sub.event_type || sub.remark || sub.id}`);
            });
          }
        }
        
        console.log('');
      } catch (error) {
        console.error(`❌ Failed: ${error.message}`);
        if (error.response && error.response.data) {
          console.error(`   Error details:`, JSON.stringify(error.response.data, null, 2));
        }
        console.log('');
      }
    }
    
    console.log('⚠️  未找到 WebSocket 连接信息');
    console.log('');
    console.log('📝 可能的原因：');
    console.log('   1. 应用权限不足（需要"接收消息"权限）');
    console.log('   2. 应用未在飞书开放平台启用');
    console.log('   3. API 端点已变更或需要特定版本');
    console.log('   4. 长连接功能需要企业版或特殊申请');
    console.log('');
    console.log('💡 建议：');
    console.log('   1. 检查飞书开放平台的权限配置');
    console.log('   2. 查看"事件订阅"页面，确认应用是否支持长连接');
    console.log('   3. 先使用 HTTP 回调模式测试基本功能');
    console.log('   4. 联系飞书技术支持确认长连接的使用方式');
    
    return null;
  } catch (error) {
    console.error('❌ Fatal error:', error.message);
    return null;
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu 长连接注册工具');
  console.log('==========================================');
  console.log('');
  
  // 加载飞书配置
  const config = loadFeishuConfig();
  
  if (!config) {
    console.error('❌ 无法加载飞书配置');
    process.exit(1);
  }
  
  console.log(`📱 应用名称: ${config.name}`);
  console.log(`📱 应用 ID: ${config.appId}`);
  console.log('');
  
  // 注册长连接
  const result = await registerWebSocketConnection(config.appId, config.appSecret);
  
  if (result) {
    console.log('');
    console.log('✅ 注册成功！');
    console.log('');
    console.log('📝 下一步：');
    console.log('   1. 登录飞书开放平台');
    console.log(`   2. 找到应用 ${config.appId}`);
    console.log('   3. 事件订阅 → 切换到长连接模式');
    console.log('   4. 启用事件推送');
    console.log('   5. 保存配置');
  }
}

main().catch((error) => {
  console.error('❌ 启动失败:', error);
  process.exit(1);
});
