#!/bin/bash
# NewClaw v0.7.0 安装脚本
# 用于自动安装和配置 NewClaw 服务

set -e

VERSION="0.7.0"
INSTALL_DIR="/usr/local"
CONFIG_DIR="/etc/newclaw"
DATA_DIR="/var/lib/newclaw"
LOG_DIR="/var/log/newclaw"
STATIC_DIR="${DATA_DIR}/static"

echo "========================================"
echo "  NewClaw v${VERSION} 安装脚本"
echo "========================================"

# 检查是否以 root 运行
if [ "$EUID" -ne 0 ]; then
    echo "❌ 请以 root 用户运行此脚本"
    exit 1
fi

# 创建目录
echo "📁 创建目录..."
mkdir -p ${CONFIG_DIR}
mkdir -p ${DATA_DIR}
mkdir -p ${LOG_DIR}
mkdir -p ${STATIC_DIR}

# 复制二进制文件
echo "📦 安装二进制文件..."
if [ -f "./target/release/newclaw" ]; then
    cp ./target/release/newclaw ${INSTALL_DIR}/bin/newclaw
    chmod +x ${INSTALL_DIR}/bin/newclaw
elif [ -f "./target/debug/newclaw" ]; then
    cp ./target/debug/newclaw ${INSTALL_DIR}/bin/newclaw
    chmod +x ${INSTALL_DIR}/bin/newclaw
else
    echo "❌ 找不到 newclaw 二进制文件，请先运行 cargo build --release"
    exit 1
fi

# 复制静态文件（Dashboard UI）
echo "🌐 安装 Dashboard 静态文件..."
if [ -d "./static" ] && [ "$(ls -A ./static 2>/dev/null)" ]; then
    cp -r ./static/* ${STATIC_DIR}/
    echo "✅ Dashboard 静态文件已安装到 ${STATIC_DIR}"
elif [ -d "./dashboard-ui/dist" ] && [ "$(ls -A ./dashboard-ui/dist 2>/dev/null)" ]; then
    cp -r ./dashboard-ui/dist/* ${STATIC_DIR}/
    echo "✅ Dashboard 静态文件已安装到 ${STATIC_DIR}"
else
    echo "⚠️  警告: 找不到 Dashboard 静态文件"
    echo "   请先运行: cd dashboard-ui && npm run build"
    echo "   然后运行: cp -r dashboard-ui/dist static"
fi

# 创建默认配置文件
echo "📝 创建配置文件..."
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    cat > ${CONFIG_DIR}/config.toml << 'EOF'
# NewClaw v0.7.0 配置文件

[server]
host = "0.0.0.0"
port = 3000

[llm]
provider = "glm"
model = "glm-4"
temperature = 0.7
max_tokens = 4096

[llm.glm]
# 从环境变量读取: GLM_API_KEY
region = "international"
provider_type = "glm"

[feishu]
enabled = false
EOF
    echo "✅ 创建默认配置文件: ${CONFIG_DIR}/config.toml"
else
    echo "ℹ️  配置文件已存在，跳过"
fi

# 创建环境变量文件
echo "🔐 创建环境变量文件..."
if [ ! -f "/etc/default/newclaw" ]; then
    cat > /etc/default/newclaw << 'EOF'
# NewClaw 环境变量配置
# 请在此处设置您的 API 密钥

# GLM API Key（必填）
GLM_API_KEY="your-glm-api-key-here"

# 飞书配置（可选）
FEISHU_APP_ID=""
FEISHU_APP_SECRET=""
FEISHU_ENCRYPT_KEY=""
FEISHU_VERIFICATION_TOKEN=""

# 日志级别
RUST_LOG=info
EOF
    echo "✅ 创建环境变量文件: /etc/default/newclaw"
    echo "⚠️  请编辑 /etc/default/newclaw 设置您的 API 密钥"
else
    echo "ℹ️  环境变量文件已存在，跳过"
fi

# 安装 systemd 服务
echo "🔧 安装 systemd 服务..."
cp ./deploy/newclaw.service /etc/systemd/system/
cp ./deploy/newclaw-dashboard.service /etc/systemd/system/

systemctl daemon-reload
systemctl enable newclaw
systemctl enable newclaw-dashboard

echo "✅ systemd 服务已安装"

# 安装完成提示
echo ""
echo "========================================"
echo "  ✅ 安装完成！"
echo "========================================"
echo ""
echo "下一步操作："
echo ""
echo "1. 编辑配置文件设置 API 密钥:"
echo "   nano /etc/default/newclaw"
echo ""
echo "2. 启动服务:"
echo "   systemctl start newclaw           # 启动 Gateway (端口 3000)"
echo "   systemctl start newclaw-dashboard # 启动 Dashboard (端口 8080)"
echo ""
echo "3. 检查服务状态:"
echo "   systemctl status newclaw"
echo "   systemctl status newclaw-dashboard"
echo ""
echo "4. 访问 Dashboard:"
echo "   http://localhost:8080"
echo ""
echo "5. 获取登录配对码:"
echo "   newclaw paircode"
echo ""
echo "========================================"