#!/bin/bash
# NewClaw Gateway 启动脚本（带飞书长连接）
#
# 功能：
# 1. 启动 NewClaw Gateway (Rust)
# 2. 启动飞书长连接服务 (Node.js)
# 3. 监控两个进程，任一失败则重启

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 配置
NEWCLAW_BINARY="/usr/local/bin/newclaw"
NEWCLAW_CONFIG="/etc/newclaw/config.toml"
GATEWAY_PORT=3000
FEISHU_SERVICE="/root/newclaw/feishu-long-connect-mind.js"
LOG_DIR="/tmp"
MAX_RESTART=5
RESTART_DELAY=10

# PID 文件
GATEWAY_PID_FILE="/tmp/newclaw-gateway.pid"
FEISHU_PID_FILE="/tmp/newclaw-feishu-long-connect.pid"

# 日志函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."
    
    if [ ! -f "$NEWCLAW_BINARY" ]; then
        log_error "NewClaw binary not found: $NEWCLAW_BINARY"
        exit 1
    fi
    
    if [ ! -f "$NEWCLAW_CONFIG" ]; then
        log_error "NewClaw config not found: $NEWCLAW_CONFIG"
        exit 1
    fi
    
    if [ ! -f "$FEISHU_SERVICE" ]; then
        log_error "Feishu service not found: $FEISHU_SERVICE"
        exit 1
    fi
    
    if ! command -v node &> /dev/null && ! command -v /root/.nvm/versions/node/v22.22.0/bin/node &> /dev/null; then
        log_error "Node.js not found"
        exit 1
    fi
    
    # 设置 Node.js 路径
    if command -v /root/.nvm/versions/node/v22.22.0/bin/node &> /dev/null; then
        export PATH="/root/.nvm/versions/node/v22.22.0/bin:$PATH"
    fi
    
    log_info "✅ 依赖检查通过"
}

# 启动 Gateway
start_gateway() {
    log_info "启动 NewClaw Gateway..."
    
    cd /root/newclaw
    
    nohup "$NEWCLAW_BINARY" gateway \
        --config "$NEWCLAW_CONFIG" \
        --port "$GATEWAY_PORT" > "$LOG_DIR/newclaw-gateway.log" 2>&1 &
    
    echo $! > "$GATEWAY_PID_FILE"
    
    sleep 2
    
    if pgrep -F "$GATEWAY_PID_FILE" > /dev/null; then
        log_info "✅ Gateway 已启动 (PID: $(cat $GATEWAY_PID_FILE))"
    else
        log_error "❌ Gateway 启动失败"
        return 1
    fi
}

# 启动飞书长连接
start_feishu() {
    log_info "启动飞书长连接服务..."
    
    local NODE_BIN="/root/.nvm/versions/node/v22.22.0/bin/node"
    
    nohup "$NODE_BIN" "$FEISHU_SERVICE" > "$LOG_DIR/feishu-long-connect-mind.log" 2>&1 &
    
    echo $! > "$FEISHU_PID_FILE"
    
    sleep 3
    
    if pgrep -F "$FEISHU_PID_FILE" > /dev/null; then
        log_info "✅ 飞书长连接已启动 (PID: $(cat $FEISHU_PID_FILE))"
    else
        log_error "❌ 飞书长连接启动失败"
        return 1
    fi
}

# 停止所有服务
stop_all() {
    log_info "停止所有服务..."
    
    if [ -f "$GATEWAY_PID_FILE" ]; then
        kill $(cat "$GATEWAY_PID_FILE") 2>/dev/null || true
        rm -f "$GATEWAY_PID_FILE"
        log_info "Gateway 已停止"
    fi
    
    if [ -f "$FEISHU_PID_FILE" ]; then
        kill $(cat "$FEISHU_PID_FILE") 2>/dev/null || true
        rm -f "$FEISHU_PID_FILE"
        log_info "飞书长连接已停止"
    fi
}

# 监控服务
monitor_services() {
    local restart_count=0
    
    log_info "开始监控服务..."
    log_info "Gateway 日志: tail -f $LOG_DIR/newclaw-gateway.log"
    log_info "飞书日志:   tail -f $LOG_DIR/feishu-long-connect-mind.log"
    log_info ""
    
    while true; do
        sleep 10
        
        # 检查 Gateway
        if ! pgrep -F "$GATEWAY_PID_FILE" > /dev/null 2>&1; then
            log_warn "Gateway 已停止，尝试重启..."
            
            if [ $restart_count -ge $MAX_RESTART ]; then
                log_error "重启次数过多，退出"
                stop_all
                exit 1
            fi
            
            restart_count=$((restart_count + 1))
            
            if start_gateway; then
                log_info "Gateway 重启成功"
                restart_count=0
            else
                sleep $RESTART_DELAY
            fi
        fi
        
        # 检查飞书长连接
        if ! pgrep -F "$FEISHU_PID_FILE" > /dev/null 2>&1; then
            log_warn "飞书长连接已停止，尝试重启..."
            
            if start_feishu; then
                log_info "飞书长连接重启成功"
            else
                sleep $RESTART_DELAY
            fi
        fi
    done
}

# 主函数
main() {
    log_info "========================================"
    log_info "NewClaw Gateway + 飞书长连接"
    log_info "========================================"
    log_info ""
    
    # 清理之前的进程
    stop_all
    sleep 2
    
    # 检查依赖
    check_dependencies
    
    # 启动服务
    if ! start_gateway; then
        log_error "无法启动 Gateway"
        exit 1
    fi
    
    if ! start_feishu; then
        log_warn "飞书长连接启动失败，但 Gateway 继续运行"
        log_warn "可以通过 HTTP API 使用 Gateway"
    fi
    
    log_info ""
    log_info "========================================"
    log_info "所有服务已启动"
    log_info "========================================"
    log_info ""
    log_info "🌐 Gateway API: http://0.0.0.0:$GATEWAY_PORT"
    log_info "📊 健康检查:     http://0.0.0.0:$GATEWAY_PORT/health"
    log_info "💬 聊天 API:     http://0.0.0.0:$GATEWAY_PORT/chat"
    log_info ""
    log_info "按 Ctrl+C 停止所有服务"
    log_info ""
    
    # 信号处理
    trap stop_all SIGINT SIGTERM
    
    # 监控服务
    monitor_services
}

# 运行
main "$@"
