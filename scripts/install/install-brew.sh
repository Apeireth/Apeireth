#!/usr/bin/env bash
# =============================================================================
# scripts/install/install-brew.sh
#
# Apeireth OS — macOS Homebrew 安装入口
# (1.0 release checklist #4 install, D-06 8 包齐发)
#
# 蓝图: docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4
# 决策: D-06 (主人 2026-08-05 20:53 拍 A: 8 包齐发)
# 公式: packaging/brew/apeireth.rb
#
# 4 步标准安装流 (per 蓝图 §3.4, brew 走 tap 仓库而非本地 .rb):
#   1. 检测 brew 是否装
#   2. tap apeireth/tap (per packaging/brew/build.sh 推送到 homebrew-tap 仓库)
#   3. brew install apeireth/tap/apeireth
#   4. brew services start apeireth + 健康检查
#
# 8 项不修改承诺 (同 install-deb.sh, 不重述):
#   - 0 改 24 LOCKED, 0 改 workspace version, 0 引 NewAPI
#   - 编译期 hardcode VERSION=1.0.0, TAP_REPO=apeireth/homebrew-tap
#   - 不重复造轮子: 调 packaging/brew/apeireth.rb, 不重写 formula
#
# 用法:
#   ./scripts/install/install-brew.sh                              # 装正式版
#   APEIRETH_TAP_REPO=apeireth/homebrew-tap-test ./scripts/install/install-brew.sh
# 卸载: brew uninstall apeireth && brew services stop apeireth
# =============================================================================

set -euo pipefail

cd "$(dirname "$0")/../.."
VERSION="${APEIRETH_VERSION:-1.0.0}"
TAP_REPO="${APEIRETH_TAP_REPO:-apeireth/homebrew-tap}"

echo "=== apeireth brew install v${VERSION} ==="
echo "    tap: ${TAP_REPO}"

# === 0. macOS 守门 ===
if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "❌ 此脚本仅在 macOS 跑 (其他平台: Linuxbrew 见 packaging/brew/ 注释)"
    echo "   Linuxbrew 路径:"
    echo "     1. apt install linuxbrew-wrapper  # Debian/Ubuntu"
    echo "     2. ./scripts/install/install-brew.sh  # 同脚本兼容"
    exit 1
fi

# === 1. brew 检测 ===
if ! command -v brew >/dev/null 2>&1; then
    echo "❌ brew 未装, 先装: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
    exit 1
fi
echo "[1/4] ✅ brew 已装: $(brew --version | head -1)"

# === 2. tap apeireth/tap ===
echo "[2/4] brew tap ${TAP_REPO}..."
if brew tap | grep -q "^${TAP_REPO%%/*}/tap$"; then
    echo "    ✅ tap 已存在"
else
    brew tap "${TAP_REPO}"
fi

# === 3. brew install ===
echo "[3/4] brew install apeireth/tap/apeireth..."
brew install apeireth/tap/apeireth
# 注: 装过的会自动跳过, 如需重装: brew reinstall apeireth/tap/apeireth

# === 4. services start + 健康检查 ===
echo "[4/4] brew services start apeireth + 健康检查..."
brew services start apeireth
sleep 3  # launchd 启动 launchd plist 需要时间

HEALTH=$(curl -fsS -m 5 http://localhost:8080/health || echo "FAILED")
if [[ "${HEALTH}" == "FAILED" ]]; then
    echo "⚠️  /health 未响应, 查看: brew services info apeireth"
    echo "    或: tail -f $(brew --prefix)/var/log/apeireth*.log"
    exit 0
fi
echo "    ✅ /health: ${HEALTH}"

# === 5. 完成 ===
echo ""
echo "✅ apeireth ${VERSION} 安装完成"
echo "    状态: brew services list | grep apeireth"
echo "    日志: $(brew --prefix)/var/log/apeireth.log"
echo "    卸载: brew services stop apeireth && brew uninstall apeireth"
echo "          或: scripts/uninstall/uninstall.sh --channel brew"
