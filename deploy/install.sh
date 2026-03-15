#!/bin/bash
# NewClaw 一键部署脚本
# 用法: sudo ./install.sh

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 检查 root 权限
if [ "$EUID" -ne 0 ]; then
    log_error "请使用 sudo 运行此脚本"
    exit 1
fi

# 配置
NEWCLAW_VERSION="0.5.0"
INSTALL_DIR="/opt/newclaw"
BIN_FILE="/opt/newclaw/newclaw"
SERVICE_FILE="/etc/systemd/system/newclaw.service"

log_info "开始安装 NewClaw v${NEWCLAW_VERSION}..."

# 1. 创建用户
if ! id -u newclaw &>/dev/null; then
    log_info "创建 newclaw 用户..."
    useradd -r -s /bin/false newclaw
fi

# 2. 创建目录
log_info "创建目录..."
mkdir -p ${INSTALL_DIR}
mkdir -p /var/log/newclaw
mkdir -p /var/run/newclaw

# 3. 检查二进制文件
if [ -f "./target/release/newclaw" ]; then
    log_info "使用本地编译的二进制文件..."
    cp ./target/release/newclaw ${BIN_FILE}
elif [ -f "./newclaw" ]; then
    log_info "使用当前目录的二进制文件..."
    cp ./newclaw ${BIN_FILE}
else
    log_error "未找到 newclaw 二进制文件"
    log_error "请先编译或下载二进制文件"
    exit 1
fi

chmod +x ${BIN_FILE}

# 4. 配置文件
if [ ! -f "${INSTALL_DIR}/newclaw.toml" ]; then
    log_info "创建配置文件..."
    if [ -f "./config/newclaw.example.toml" ]; then
        cp ./config/newclaw.example.toml ${INSTALL_DIR}/newclaw.toml
        log_warn "请编辑 ${INSTALL_DIR}/newclaw.toml 配置 API Key"
    else
        log_error "未找到配置模板"
        exit 1
    fi
else
    log_info "配置文件已存在，跳过..."
fi

# 5. 设置权限
log_info "设置权限..."
chown -R newclaw:newclaw ${INSTALL_DIR}
chown -R newclaw:newclaw /var/log/newclaw
chown -R newclaw:newclaw /var/run/newclaw

# 6. 安装 systemd 服务
log_info "安装 systemd 服务..."
if [ -f "./deploy/newclaw.service" ]; then
    cp ./deploy/newclaw.service ${SERVICE_FILE}
else
    cat > ${SERVICE_FILE} << 'EOF'
[Unit]
Description=NewClaw AI Agent Gateway
After=network.target redis.service
Requires=redis.service

[Service]
Type=simple
User=newclaw
Group=newclaw
WorkingDirectory=/opt/newclaw
ExecStart=/opt/newclaw/newclaw gateway
Restart=on-failure
RestartSec=10
NoNewPrivileges=true
PrivateTmp=true
Environment="RUST_LOG=info"
Environment="NEWCLAW_CONFIG=/opt/newclaw/newclaw.toml"

[Install]
WantedBy=multi-user.target
EOF
fi

systemctl daemon-reload

# 7. 检查 Redis
log_info "检查 Redis..."
if ! command -v redis-cli &> /dev/null; then
    log_warn "Redis 未安装，正在安装..."
    apt update && apt install -y redis-server
fi

if ! redis-cli ping &> /dev/null; then
    log_warn "Redis 未运行，正在启动..."
    systemctl start redis-server
    systemctl enable redis-server
fi

log_info "Redis 状态: $(redis-cli ping)"

# 8. 启动服务
read -p "是否立即启动 NewClaw 服务？[y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    log_info "启动服务..."
    systemctl enable newclaw
    systemctl start newclaw
    sleep 2
    systemctl status newclaw --no-pager
fi

# 9. 验证
log_info "验证安装..."
sleep 2
if curl -s http://localhost:3000/health | grep -q "ok"; then
    log_info "✅ NewClaw 安装成功！"
else
    log_warn "服务可能未正常启动，请检查日志: journalctl -u newclaw -f"
fi

echo ""
echo "=========================================="
echo "安装完成！"
echo "=========================================="
echo ""
echo "配置文件: ${INSTALL_DIR}/newclaw.toml"
echo "日志查看: journalctl -u newclaw -f"
echo "服务管理: systemctl {start|stop|restart|status} newclaw"
echo ""
echo "下一步："
echo "1. 编辑配置文件: vim ${INSTALL_DIR}/newclaw.toml"
echo "2. 重启服务: systemctl restart newclaw"
echo "3. 查看日志: journalctl -u newclaw -f"
echo ""
