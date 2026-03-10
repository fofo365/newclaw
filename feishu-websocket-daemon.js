#!/usr/bin/env node

/**
 * NewClaw Feishu WebSocket 连接守护进程
 *
 * 维持与飞书服务器的长连接，用于接收事件推送
 */

const WebSocket = require('ws');

// 从配置文件读取
const toml = require('@iarna/toml');
const fs = require('fs');

function loadConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    return toml.parse(content);
  } catch (error) {
    console.error('❌ Failed to load config:', error.message);
    process.exit(1);
  }
}

function getFeishuConfig(config) {
  const accounts = config.feishu?.accounts || {};
  const account = accounts['feishu-mind'] || Object.values(accounts)[0];
  
  if (!account) {
    console.error('❌ No Feishu account found in config');
    process.exit(1);
  }
  
  return {
    appId: account.app_id,
    appSecret: account.app_secret,
    enabled: account.enabled !== false,
  };
}

// 连接到飞书 WebSocket
function connectToFeishu(appId, appSecret) {
  // 飞书 WebSocket 端点（需要先通过 HTTP API 获取）
  const wsUrl = 'wss://open.feishu.cn/open-apis/event-bridge/v3/ connect';
  
  console.log(`🔌 Connecting to Feishu WebSocket...`);
  console.log(`   App ID: ${appId}`);
  
  // 注意：真实的飞书 WebSocket 连接需要：
  // 1. 先调用 HTTP API 获取 ticket
  // 2. 使用 ticket 建立 WebSocket 连接
  // 3. 定期发送心跳包保持连接
  
  // 这里只是一个占位符实现
  // 真实的实现需要参考飞书文档
  
  const ws = new WebSocket(wsUrl, {
    headers: {
      'X-Feishu-App-Id': appId,
      'X-Feishu-App-Secret': appSecret,
    },
  });
  
  ws.on('open', () => {
    console.log('✅ Connected to Feishu WebSocket');
    
    // 发送心跳包
    const heartbeatInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.ping();
      }
    }, 30000); // 每30秒发送一次心跳
    
    ws.on('close', () => {
      clearInterval(heartbeatInterval);
    });
  });
  
  ws.on('message', (data) => {
    console.log('📨 Received message:', data.toString());
  });
  
  ws.on('error', (error) => {
    console.error('❌ WebSocket error:', error.message);
  });
  
  ws.on('close', () => {
    console.log('🔌 Connection closed, reconnecting in 5 seconds...');
    setTimeout(() => {
      connectToFeishu(appId, appSecret);
    }, 5000);
  });
  
  return ws;
}

// 主函数
function main() {
  console.log('🚀 NewClaw Feishu WebSocket Daemon starting...');
  
  const config = loadConfig();
  const feishuConfig = getFeishuConfig(config);
  
  if (!feishuConfig.enabled) {
    console.log('⚠️  Feishu is disabled in config, exiting');
    process.exit(0);
  }
  
  console.log(`✅ Loaded Feishu config for app: ${feishuConfig.appId}`);
  
  // 连接到飞书
  const ws = connectToFeishu(feishuConfig.appId, feishuConfig.appSecret);
  
  // 保持进程运行
  process.on('SIGINT', () => {
    console.log('\n👋 Shutting down...');
    ws.close();
    process.exit(0);
  });
}

main();
