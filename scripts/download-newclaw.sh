#!/bin/bash
# NewClaw 自动下载脚本
# 用法: ./download-newclaw.sh [VERSION]
# 示例: ./download-newclaw.sh v0.4.1

set -e

# 配置
REPO="fofo365/newclaw"
INSTALL_DIR="/root/newclaw"
BINARY_NAME="newclaw"

# 获取版本
if [ -z "$1" ]; then
    echo "正在获取最新版本..."
    VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo "❌ 无法获取最新版本"
        exit 1
    fi
else
    VERSION="$1"
fi

echo "📦 准备下载 NewClaw ${VERSION}"

# 构建下载 URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/newclaw-linux-x64-${VERSION}.tar.gz"
FALLBACK_URL="https://github.com/${REPO}/releases/download/${VERSION}/newclaw"

# 创建临时目录
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cd "$TEMP_DIR"

# 尝试下载 tarball
echo "⬇️  下载中..."
if curl -fsSL --progress-bar -o newclaw.tar.gz "$DOWNLOAD_URL"; then
    echo "✅ 下载完成（tarball）"
    tar -xzf newclaw.tar.gz
    chmod +x newclaw
elif curl -fsSL --progress-bar -o newclaw "$FALLBACK_URL"; then
    echo "✅ 下载完成（二进制）"
    chmod +x newclaw
else
    echo "❌ 下载失败"
    echo "请检查版本号是否正确: $VERSION"
    echo "访问 https://github.com/${REPO}/releases 查看可用版本"
    exit 1
fi

# 备份旧版本
if [ -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
    BACKUP_NAME="${BINARY_NAME}.backup.$(date +%Y%m%d_%H%M%S)"
    echo "💾 备份旧版本到 ${BACKUP_NAME}"
    mv "${INSTALL_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BACKUP_NAME}"
fi

# 安装新版本
mkdir -p "$INSTALL_DIR"
mv newclaw "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo ""
echo "✅ NewClaw ${VERSION} 安装成功！"
echo "   安装位置: ${INSTALL_DIR}/${BINARY_NAME}"
echo "   文件大小: $(du -h "${INSTALL_DIR}/${BINARY_NAME}" | cut -f1)"
echo ""
echo "验证安装:"
echo "  ${INSTALL_DIR}/${BINARY_NAME} --version"
echo ""
echo "如果需要，可以恢复旧版本:"
echo "  ls -la ${INSTALL_DIR}/${BINARY_NAME}.backup.*"
