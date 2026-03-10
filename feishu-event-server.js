#!/usr/bin/env node

/**
 * NewClaw Feishu 事件接收服务器
 *
 * 接收飞书的事件推送（HTTP 回调模式）
 * 并转发给 Gateway 处理
 */

const express = require('express');
const axios = require('axios');
const toml = require('@iarna/toml');
const fs = require('fs');
const crypto = require('crypto');

const app = express();
app.use(express.json());

// 读取配置
function loadConfig() {
  const configPath = '/etc/newclaw/config.toml';
  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    return toml.parse(content);
  } catch (error) {
    console.error('❌ Failed to load config:', error.message);
    return null;
  }
}

function getFeishuConfig(config) {
  const accounts = config.feishu?.accounts || {};
  const account = accounts['feishu-mind'] || Object.values(accounts)[0];
  
  if (!account) {
    console.error('❌ No Feishu account found in config');
    return null;
  }
  
  return {
    appId: account.app_id,
    appSecret: account.app_secret,
    encryptKey: account.encrypt_key || '',
    verificationToken: account.verification_token || '',
    enabled: account.enabled !== false,
  };
}

// 验证飞书签名
function verifySignature(timestamp, nonce, body, signature, encryptKey) {
  if (!encryptKey) {
    // 如果没有配置加密密钥，跳过验证
    return true;
  }
  
  const signStr = timestamp + nonce + encryptKey + body;
  const hash = crypto.createHash('sha256').update(signStr).digest('hex');
  return hash === signature;
}

// 飞书事件回调端点
app.post('/feishu/events', async (req, res) => {
  try {
    console.log('📨 Received Feishu event:', JSON.stringify(req.body).substring(0, 500));

    const { type, challenge, token, event, header } = req.body;

    // URL 验证请求（旧格式）
    if (type === 'url_verification') {
      console.log('✅ URL verification challenge:', challenge);
      return res.json({
        challenge: challenge,
      });
    }

    // 飞书事件推送（新格式）
    if (header && header.event_type) {
      const eventType = header.event_type;
      console.log(`📬 Event type: ${eventType}`);

      // 处理消息接收事件
      if (eventType === 'im.message.receive_v1') {
        const content = JSON.parse(event.message.content);
        const text = content.text;
        const senderId = event.sender.sender_id.open_id;
        const chatId = event.message.chat_id;

        console.log(`💬 Message received from ${senderId}: ${text}`);

        // 转发给 Gateway 处理
        try {
          const gatewayResponse = await axios.post('http://127.0.0.1:3000/chat', {
            message: text,
            session_id: chatId,
            metadata: {
              feishu_event: req.body,
              sender_id: senderId,
              chat_id: chatId,
            },
          }, {
            timeout: 60000,
          });

          console.log('✅ Gateway response:', gatewayResponse.data.response.substring(0, 100));

          // 发送回复到飞书
          try {
            // 获取访问令牌
            const tokenResponse = await axios.post(
              'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal',
              {
                app_id: 'cli_a921727d9838dbef',
                app_secret: '0W5xSCyz4EMIAAyIqHsKNdU5qnOGZLtw',
              }
            );
            
            const accessToken = tokenResponse.data.tenant_access_token;
            
            // 发送消息到飞书
            const messageResponse = await axios.post(
              `https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id`,
              {
                receive_id: senderId,
                content: JSON.stringify({ text: gatewayResponse.data.response }),
                msg_type: 'text',
              },
              {
                headers: {
                  'Authorization': `Bearer ${accessToken}`,
                },
              }
            );
            
            console.log('✅ Reply sent to Feishu:', messageResponse.data.code);
          } catch (error) {
            console.error('❌ Error sending reply to Feishu:', error.message);
          }

        } catch (error) {
          console.error('❌ Error calling Gateway:', error.message);
        }
      }

      return res.status(200).send('ok');
    }

    // 事件推送（旧格式兼容）
    if (type === 'event' || type === 'im.message.receive_v1') {
      console.log('📬 Event received (legacy format)');
      
      // 转发给 Gateway 处理
      try {
        await axios.post('http://127.0.0.1:3000/feishu/events', req.body, {
          timeout: 5000,
        });
        console.log('✅ Event forwarded to Gateway');
      } catch (error) {
        console.error('⚠️  Failed to forward event to Gateway:', error.message);
      }

      return res.status(200).send('ok');
    }

    console.log('⚠️  Unknown request type:', type);
    res.status(400).json({ error: 'Unknown request type' });
  } catch (error) {
    console.error('❌ Error processing event:', error);
    res.status(500).send('Internal server error');
  }
});

// 健康检查
app.get('/health', (req, res) => {
  res.json({ status: 'ok', service: 'feishu-event-receiver' });
});

// 启动服务器
const PORT = 3002;
const config = loadConfig();
const feishuConfig = getFeishuConfig(config);

if (!feishuConfig || !feishuConfig.enabled) {
  console.error('❌ Feishu is not configured or disabled');
  process.exit(1);
}

app.listen(PORT, () => {
  console.log(`🚀 Feishu Event Receiver listening on port ${PORT}`);
  console.log(`   App ID: ${feishuConfig.appId}`);
  console.log(`   Event URL: http://122.51.14.70:${PORT}/feishu/events`);
  console.log('');
  console.log('📝 请在飞书开放平台配置以下信息：');
  console.log(`   - 请求 URL: http://122.51.14.70:${PORT}/feishu/events`);
  console.log('   - 选择：HTTP 回调模式');
  console.log('   - 勾选：启用事件推送');
});
