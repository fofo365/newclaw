#!/bin/bash
# 测试飞书长连接和 Gateway 集成

echo "🧪 测试 NewClaw Gateway + 飞书长连接"
echo "======================================"
echo ""

# 等待服务启动
sleep 3

# 检查 Gateway
echo "1️⃣ 检查 Gateway 状态..."
for i in {1..5}; do
    if curl -s http://127.0.0.1:3000/health | grep -q "OK\|ok"; then
        echo "   ✅ Gateway 运行正常"
        break
    else
        if [ $i -eq 5 ]; then
            echo "   ❌ Gateway 未响应"
            exit 1
        fi
        echo "   ⏳ 等待 Gateway 启动... ($i/5)"
        sleep 2
    fi
done

echo ""

# 检查飞书长连接
echo "2️⃣ 检查飞书长连接状态..."
if pgrep -f "feishu-long-connect-mind" > /dev/null; then
    echo "   ✅ 飞书长连接运行中 (PID: $(pgrep -f 'feishu-long-connect-mind' | head -1))"
    
    # 显示连接状态
    echo ""
    echo "   📊 最近日志："
    tail -10 /tmp/feishu-long-connect-mind.log 2>/dev/null | grep -E "连接状态|ws connect|ws client ready|收到消息|Gateway 回复" | sed 's/^/      /' || echo "      (暂无日志)"
else
    echo "   ❌ 飞书长连接未运行"
fi

echo ""

# 测试 Gateway API
echo "3️⃣ 测试 Gateway Chat API..."
response=$(curl -s -X POST http://127.0.0.1:3000/chat \
    -H "Content-Type: application/json" \
    -d '{"message":"你好","session_id":"test"}')

if echo "$response" | grep -q "response"; then
    echo "   ✅ Chat API 正常工作"
    echo "   📝 回复: $(echo "$response" | jq -r '.response' 2>/dev/null | head -c 50)..."
else
    echo "   ⚠️  Chat API 响应异常"
    echo "   响应: $(echo "$response" | head -c 100)..."
fi

echo ""
echo "======================================"
echo "✅ 集成测试完成"
echo ""
echo "📝 日志查看："
echo "   Gateway: tail -f /tmp/newclaw-gateway.log"
echo "   飞书:   tail -f /tmp/feishu-long-connect-mind.log"
echo ""
echo "🔧 管理命令："
echo "   启动: systemctl start newclaw-full"
echo "   停止: systemctl stop newclaw-full"
echo "   重启: systemctl restart newclaw-full"
echo "   状态: systemctl status newclaw-full"
echo ""
echo "📱 飞书机器人配置："
echo "   App ID: cli_a921727d9838dbef"
echo "   事件订阅: 使用长连接接收事件/回调"
