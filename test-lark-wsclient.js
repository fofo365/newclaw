#!/usr/bin/env node
/**
 * 测试飞书 SDK 的 WSClient
 */

const Lark = require('@larksuiteoapi/node-sdk');
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
  console.log('🔍 测试飞书 SDK 的 WSClient');
  console.log('==============================');
  console.log('');
  
  const config = loadFeishuConfig();
  if (!config) {
    process.exit(1);
  }
  
  console.log(`📱 应用: ${config.name} (${config.appId})`);
  console.log(`🌐 Domain: ${config.domain}`);
  console.log('');
  
  // 检查 WSClient 是否存在
  console.log('📦 检查 Lark.WSClient:');
  if (Lark.WSClient) {
    console.log('✅ Lark.WSClient 存在！');
    console.log('   类型:', typeof Lark.WSClient);
    console.log('');
    
    // 创建 WSClient
    console.log('🔌 创建 WSClient...');
    try {
      const wsClient = new Lark.WSClient({
        appId: config.appId,
        appSecret: config.appSecret,
        domain: config.domain === 'lark' ? Lark.Domain.Lark : Lark.Domain.Feishu,
        loggerLevel: Lark.LoggerLevel.info,
      });
      
      console.log('✅ WSClient 创建成功！');
      console.log('   对象:', wsClient);
      console.log('');
      
      // 检查方法
      console.log('📋 WSClient 方法:');
      console.log(Object.getOwnPropertyNames(Object.getPrototypeOf(wsClient)).join(', '));
      console.log('');
      
      // 检查是否有 start 方法
      if (typeof wsClient.start === 'function') {
        console.log('✅ 发现 start 方法');
        
        // 创建事件分发器
        console.log('');
        console.log('📦 创建 EventDispatcher...');
        const eventDispatcher = new Lark.EventDispatcher({});
        console.log('✅ EventDispatcher 创建成功');
        console.log('   方法:', Object.getOwnPropertyNames(Object.getPrototypeOf(eventDispatcher)).join(', '));
        console.log('');
        
        console.log('🚀 启动 WebSocket 连接...');
        console.log('');
        console.log('⏳ 连接中...');
        
        wsClient.start({ eventDispatcher });
        
        // 保持进程运行
        process.on('SIGINT', () => {
          console.log('\n👋 正在关闭...');
          process.exit(0);
        });
        
      } else {
        console.log('❌ 未找到 start 方法');
      }
      
    } catch (error) {
      console.error('❌ 创建 WSClient 失败:', error.message);
      console.error('   错误详情:', error);
    }
  } else {
    console.log('❌ Lark.WSClient 不存在');
    console.log('');
    console.log('📦 Lark 对象的属性:');
    console.log(Object.keys(Lark).join(', '));
  }
}

main().catch(console.error);
