#!/usr/bin/env bash
# =============================================================================
# packaging/brew/install-brew.sh
#
# Homebrew formula 的 user-side install helper (per task spec 1.0 release #4)
# vs packaging/brew/build.sh: 后者是 release engineer 用的 (推 formula 到 tap)
#                            前者是 end user 用的 (从 tap 装)
#
# 决策: D-06 (8 包齐发)
# 公式: packaging/brew/apeireth.rb
# 兄弟: scripts/install/install-brew.sh (跨包统一入口)
#
# 用法:
#   ./packaging/brew/install-brew.sh                  # 装正式版
#   APEIRETH_TAP=apeireth/tap ./packaging/brew/install-brew.sh
# 卸载: brew uninstall apeireth
# =============================================================================

set -euo pipefail

VERSION="${APEIRETH_VERSION:-1.0.0}"
TAP="${APEIRETH_TAP:-apeireth/tap}"

echo "=== apeireth brew install v${VERSION} (tap=${TAP}) ==="

# 1. brew 检测
if ! command -v brew >/dev/null 2>&1; then
    echo "❌ brew 未装, 先装: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
    exit 1
fi

# 2. tap
if ! brew tap | grep -q "^${TAP%%/*}/tap$"; then
    echo "[1/3] brew tap ${TAP}..."
    brew tap "${TAP}"
fi

# 3. install
echo "[2/3] brew install ${TAP}/apeireth..."
brew install "${TAP}/apeireth"

# 4. services start
echo "[3/3] brew services start apeireth..."
brew services start apeireth
sleep 2

# 5. 报告
HEALTH=$(curl -fsS -m 5 http://localhost:8080/health 2>/dev/null || echo "FAILED")
if [[ "${HEALTH}" != "FAILED" ]]; then
    echo "    ✅ /health: ${HEALTH}"
else
    echo "    ⚠️  /health 未响应, 看: brew services list | grep apeireth"
fi

echo "✅ 安装完成 (详细见 docs/installation/macos-brew-install.md)"
