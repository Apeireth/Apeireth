#!/usr/bin/env bash
# =============================================================================
# packaging/deb/install-deb.sh
#
# deb 的 user-side install helper (per task spec 1.0 release #4)
# vs packaging/deb/build.sh: 后者是 release engineer 用的 (cargo-deb 编译)
#                             前者是 end user 用的 (从 .deb 装)
#
# 决策: D-06 (8 包齐发)
# 公式: packaging/deb/build.sh 出 .deb; packaging/deb/apeireth.service 是 systemd unit
# 兄弟: scripts/install/install-deb.sh (跨包统一入口, 多了 sha256 校验)
#
# 用法:
#   sudo ./packaging/deb/install-deb.sh                       # 默认 target/ 路径
#   sudo ./packaging/deb/install-deb.sh /path/to/apeireth.deb # 显式
# 卸载: sudo apt remove --purge apeireth
# =============================================================================

set -euo pipefail

VERSION="${APEIRETH_VERSION:-1.0.0}"
cd "$(dirname "$0")/../.."

# 0. root 守门
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root: sudo $0 $*"
    exit 1
fi

# 1. 找 .deb
if [[ -n "${1:-}" ]]; then
    DEB_PATH="$1"
else
    DEB_PATH=$(find target -name "apeireth_${VERSION}_amd64.deb" -type f 2>/dev/null | head -1 || true)
    if [[ -z "${DEB_PATH}" ]]; then
        echo "❌ 未找到 apeireth_${VERSION}_amd64.deb, 先跑 packaging/deb/build.sh"
        exit 1
    fi
fi

if [[ ! -f "${DEB_PATH}" ]]; then
    echo "❌ .deb 不存在: ${DEB_PATH}"
    exit 1
fi

echo "=== apeireth deb install v${VERSION} (${DEB_PATH}) ==="

# 2. apt install (systemd 单元自动部署)
echo "[1/3] apt install ${DEB_PATH}..."
apt update -qq
apt install -y "${DEB_PATH}"

# 3. systemd 启用
echo "[2/3] systemctl enable + start..."
systemctl daemon-reload
systemctl enable apeireth.service
systemctl restart apeireth.service
sleep 2

# 4. 健康检查
echo "[3/3] 健康检查..."
HEALTH=$(curl -fsS -m 5 http://localhost:8080/health 2>/dev/null || echo "FAILED")
if [[ "${HEALTH}" != "FAILED" ]]; then
    echo "    ✅ /health: ${HEALTH}"
else
    echo "    ⚠️  /health 未响应, 看: journalctl -u apeireth -n 50"
fi

echo "✅ 安装完成 (详细见 docs/installation/deb-install.md)"
