#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接注册脚本
 * 
 * 功能：
 * 1. 获取访问令牌
 * 2. 调用飞书 API 注册长连接
 * 3. 返回 WebSocket 连接信息
 */

const axios = require("axios");
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
async function registerLongConnection(appId, appSecret) {
  console.log('🔌 Registering Feishu long connection...');
  console.log(`   App ID: ${appId}`);
  console.log('');
  
  try {
    // 步骤 1: 获取访问令牌
    const accessToken = await getAccessToken(appId, appSecret);
    console.log('✅ Got access token');
    console.log(`   Token: ${accessToken.substring(0, 20)}...`);
    console.log('');
    
    // 步骤 2: 尝试不同的 API 端点注册长连接
    
    // 方式 1: 获取事件订阅信息
    console.log('📡 Trying to get event subscription info...');
    try {
      const response = await axios.get(
        'https://open.feishu.cn/open-apis/event/v1/user/event_subscription',
        {
          headers: {
            'Authorization': `Bearer ${accessToken}`,
          },
        }
      );
      
      console.log('✅ Got event subscription info:');
      console.log(JSON.stringify(response.data, null, 2));
      console.log('');
      
      // 检查是否有长连接信息
      if (response.data.data && response.data.data.websocket) {
        console.log('🎉 Found WebSocket connection info:');
        console.log(`   URL: ${response.data.data.websocket.url}`);
        return response.data.data.websocket;
      }
    } catch (error) {
      console.error('❌ Error getting event subscription:', error.message);
    }
    
    // 方式 2: 尝试用户长连接 API
    console.log('📡 Trying user websocket API...');
    try {
      const response = await axios.get(
        'https://open.feishu.cn/open-apis/event/v2/user/websocket',
        {
          headers: {
            'Authorization': `Bearer ${accessToken}`,
            'X-Feishu-App-Id': appId,
          },
        }
      );
      
      console.log('✅ Got user websocket info:');
      console.log(JSON.stringify(response.data, null, 2));
      console.log('');
      
      if (response.data.data && response.data.data.url) {
        console.log('🎉 Found WebSocket connection info:');
        console.log(`   URL: ${response.data.data.url}`);
        console.log('');
        console.log('📝 请将以下 WebSocket URL 配置到飞书开放平台：');
        console.log(`   ${response.data.data.url}`);
        return response.data.data;
      }
    } catch (error) {
      console.error('❌ Error getting user websocket:', error.message);
      console.error('   Response:', error.response?.data);
    }
    
    // 方式 3: 尝试机器人长连接 API
    console.log('📡 Trying bot websocket API...');
    try {
      const response = await axios.get(
        'https://open.feishu.cn/open-apis/event/v2/bot/websocket',
        {
          headers: {
            'Authorization': `Bearer ${accessToken}`,
            'X-Feishu-App-Id': appId,
          },
        }
      );
      
      console.log('✅ Got bot websocket info:');
      console.log(JSON.stringify(response.data, null, 2));
      console.log('');
      
      if (response.data.data && response.data.data.url) {
        console.log('🎉 Found WebSocket connection info:');
        console.log(`   URL: ${response.data.data.url}`);
        console.log('');
        console.log('📝 请将以下 WebSocket URL 配置到飞书开放平台：');
        console.log(`   ${response.data.data.url}`);
        return response.data.data;
      }
    } catch (error) {
      console.error('❌ Error getting bot websocket:', error.message);
      console.error('   Response:', error.response?.data);
    }
    
    console.log('');
    console.log('⚠️  所有方式都失败了');
    console.log('');
    console.log('📝 可能的原因：');
    console.log('   1. 应用未在飞书开放平台启用');
    console.log('   2. 应用权限不足');
    console.log('   3. API 端点已变更');
    console.log('');
    console.log('💡 建议：');
    console.log('   1. 检查飞书开放平台的应用配置');
    console.log('   2. 确认应用权限（获取信息、接收消息等）');
    console.log('   3. 联系飞书技术支持');
    
    return null;
  } catch (error) {
    console.error('❌ Fatal error:', error.message);
    return null;
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw 飞书长连接注册工具');
  console.log('========================================');
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
  const result = await registerLongConnection(config.appId, config.appSecret);
  
  if (result) {
    console.log('');
    console.log('✅ 长连接注册成功！');
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
