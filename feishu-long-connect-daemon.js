#!/usr/bin/env node
/**
 * NewClaw Feishu 长连接守护进程
 * 
 * 功能：
 * 1. 从配置文件读取飞书配置
 * 2. 连接飞书 WebSocket
 * 3. 接收消息
 * 4. 转发给 Gateway
 * 5. 发送回复
 */

const WebSocket = require('ws');
const axios = require('axios');
const toml = require('@iarna/toml');
const fs = require('fs');

// 从配置文件读取飞书配置
function loadFeishuConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    const config = toml.parse(content);
    
    // 提取飞书配置
    if (config.feishu && config.feishu.accounts) {
      // 遍历所有账号，找到第一个启用的
      for (const [key, account] of Object.entries(config.feishu.accounts)) {
        if (account.enabled) {
          console.log(`✅ Found Feishu account: ${key}`);
          return {
            appId: account.app_id,
            appSecret: account.app_secret,
            encryptKey: account.encrypt_key || '',
            verificationToken: account.verification_token || '',
            enabled: account.enabled !== false,
            connectionMode: account.connection_mode || 'websocket',
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

// 发送消息到飞书
async function sendReplyToFeishu(appId, appSecret, receiveId, content) {
  try {
    console.log(`📤 Sending reply to ${receiveId}: ${content.substring(0, 50)}...`);
    
    // 获取访问令牌
    const tokenResponse = await axios.post(
      'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal',
      {
        app_id: appId,
        app_secret: appSecret,
      }
    );
    
    const accessToken = tokenResponse.data.tenant_access_token;
    
    // 发送消息
    const response = await axios.post(
      `https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id`,
      {
        receive_id: receiveId,
        content: content,
        msg_type: 'text',
      },
      {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
      }
    );
    
    if (response.data.code === 0) {
      console.log('✅ Reply sent successfully');
      return true;
    } else {
      console.error('❌ Failed to send reply:', response.data.msg);
      return false;
    }
  } catch (error) {
    console.error('❌ Error sending reply to Feishu:', error.message);
    return false;
  }
}

// 主函数
async function main() {
  console.log('🚀 NewClaw Feishu 长连接守护进程启动...');
  console.log('');
  
  // 加载飞书配置
  const config = loadFeishuConfig();
  
  if (!config) {
    console.error('❌ 无法加载飞书配置');
    process.exit(1);
  }
  
  if (!config.enabled) {
    console.log('⚠️  飞书账号在配置中已禁用');
    process.exit(0);
  }
  
  console.log(`📱 App ID: ${config.appId}`);
  console.log(`🔗 连接模式: ${config.connectionMode}`);
  console.log('');
  
  // 获取访问令牌并测试
  try {
    const accessToken = await getAccessToken(config.appId, config.appSecret);
    console.log('✅ 飞书访问令牌获取成功');
    console.log(`   Token: ${accessToken.substring(0, 20)}...`);
    console.log('');
    
    console.log('⚠️  注意：当前是简化版本，仅测试访问令牌获取');
    console.log('   飞书长连接需要先在飞书开放平台配置启用');
    console.log('');
    console.log('📝 配置步骤：');
    console.log('   1. 进入飞书开放平台');
    console.log(`   2. 找到应用 ${config.appId}`);
    console.log('   3. 事件订阅 → 切换到长连接模式');
    console.log('   4. 启用事件推送');
    console.log('   5. 保存配置');
    
  } catch (error) {
    console.error('❌ 测试失败:', error.message);
    process.exit(1);
  }
}

main().catch((error) => {
  console.error('❌ 启动失败:', error);
  process.exit(1);
});
