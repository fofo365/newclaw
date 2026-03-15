#!/bin/bash
# NewClaw 版本号同步脚本
# 从 Cargo.toml 读取版本号并同步到其他文件

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 从 Cargo.toml 读取版本号
VERSION=$(grep -m1 '^version = "' "$PROJECT_ROOT/Cargo.toml" | sed 's/version = "\(.*\)"/\1/')

if [ -z "$VERSION" ]; then
    echo "❌ 无法从 Cargo.toml 读取版本号"
    exit 1
fi

echo "📋 检测到版本号: $VERSION"

# 同步到 package.json
PACKAGE_JSON="$PROJECT_ROOT/dashboard-ui/package.json"
if [ -f "$PACKAGE_JSON" ]; then
    CURRENT_PKG_VERSION=$(grep -m1 '"version":' "$PACKAGE_JSON" | sed 's/.*"version": *"\([^"]*\)".*/\1/')
    if [ "$CURRENT_PKG_VERSION" != "$VERSION" ]; then
        sed -i "s/\"version\": *\"[^\"]*\"/\"version\": \"$VERSION\"/" "$PACKAGE_JSON"
        echo "✅ 更新 package.json: $CURRENT_PKG_VERSION -> $VERSION"
    else
        echo "ℹ️  package.json 已是最新版本: $VERSION"
    fi
fi

# 同步到 Dashboard UI 标题
MAIN_LAYOUT="$PROJECT_ROOT/dashboard-ui/src/components/MainLayout.tsx"
if [ -f "$MAIN_LAYOUT" ]; then
    if grep -q "v0\.[0-9]\+\.[0-9]\+" "$MAIN_LAYOUT"; then
        sed -i "s/v0\.[0-9]\+\.[0-9]\+/v$VERSION/g" "$MAIN_LAYOUT"
        echo "✅ 更新 MainLayout.tsx 版本显示"
    fi
fi

# 同步到 systemd 服务文件
SERVICE_FILE="$PROJECT_ROOT/deploy/newclaw.service"
DASHBOARD_SERVICE="$PROJECT_ROOT/deploy/newclaw-dashboard.service"
if [ -f "$SERVICE_FILE" ]; then
    sed -i "s/v0\.[0-9]\+\.[0-9]\+/v$VERSION/g" "$SERVICE_FILE"
fi
if [ -f "$DASHBOARD_SERVICE" ]; then
    sed -i "s/v0\.[0-9]\+\.[0-9]\+/v$VERSION/g" "$DASHBOARD_SERVICE"
fi
echo "✅ 更新 systemd 服务文件版本"

# 同步到安装脚本
INSTALL_SCRIPT="$PROJECT_ROOT/deploy/install.sh"
if [ -f "$INSTALL_SCRIPT" ]; then
    sed -i "s/VERSION=\"0\.[0-9]\+\.[0-9]\+\"/VERSION=\"$VERSION\"/" "$INSTALL_SCRIPT"
    echo "✅ 更新安装脚本版本"
fi

echo ""
echo "========================================"
echo "  ✅ 版本号同步完成: v$VERSION"
echo "========================================"