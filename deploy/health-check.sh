#!/bin/bash
# NewClaw 健康检查脚本

set -e

# 颜色
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; }
warn() { echo -e "${YELLOW}!${NC} $1"; }

echo "NewClaw 健康检查"
echo "================"
echo ""

# 1. 检查进程
echo -n "检查进程... "
if pgrep -x "newclaw" > /dev/null; then
    pass "运行中"
else
    fail "未运行"
    exit 1
fi

# 2. 检查端口
echo -n "检查端口 3000... "
if netstat -tlnp 2>/dev/null | grep -q ":3000"; then
    pass "监听中"
else
    fail "未监听"
fi

# 3. 检查 Redis
echo -n "检查 Redis... "
if redis-cli ping 2>/dev/null | grep -q "PONG"; then
    pass "正常"
else
    fail "无法连接"
fi

# 4. 检查健康端点
echo -n "检查 /health 端点... "
HEALTH=$(curl -s http://localhost:3000/health 2>/dev/null || echo '{"status":"error"}')
if echo "$HEALTH" | grep -q '"status":"ok"'; then
    pass "正常"
    echo "$HEALTH" | python3 -m json.tool 2>/dev/null || echo "$HEALTH"
else
    fail "异常"
    echo "$HEALTH"
fi

# 5. 检查配置文件
echo -n "检查配置文件... "
if [ -f "/opt/newclaw/newclaw.toml" ]; then
    pass "存在"
else
    warn "不存在（使用默认配置）"
fi

# 6. 检查日志错误
echo -n "检查最近错误... "
ERRORS=$(journalctl -u newclaw -p err -n 5 --no-pager 2>/dev/null | wc -l)
if [ "$ERRORS" -eq 0 ]; then
    pass "无错误"
else
    warn "发现 $ERRORS 条错误"
    journalctl -u newclaw -p err -n 5 --no-pager
fi

# 7. 检查内存
echo -n "检查内存使用... "
MEM=$(ps aux | grep "[n]ewclaw" | awk '{print $4}')
if [ ! -z "$MEM" ]; then
    if (( $(echo "$MEM < 50" | bc -l) )); then
        pass "${MEM}%"
    else
        warn "${MEM}% (偏高)"
    fi
else
    fail "无法获取"
fi

# 8. 检查磁盘
echo -n "检查磁盘空间... "
DISK=$(df -h /opt/newclaw 2>/dev/null | tail -1 | awk '{print $5}' | tr -d '%')
if [ ! -z "$DISK" ]; then
    if [ "$DISK" -lt 80 ]; then
        pass "${DISK}% 已用"
    else
        warn "${DISK}% 已用 (空间不足)"
    fi
else
    warn "无法获取"
fi

echo ""
echo "================"
echo "健康检查完成"
