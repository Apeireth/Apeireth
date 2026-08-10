#!/usr/bin/env bash
# =============================================================================
# packaging/rpm/install-rpm.sh
#
# rpm 的 user-side install helper (per task spec 1.0 release #4)
# vs packaging/rpm/build.sh: 后者是 release engineer 用的 (cargo-rpm 编译)
#                             前者是 end user 用的 (从 .rpm 装)
#
# 决策: D-06 (8 包齐发)
# Spec: packaging/rpm/apeireth.spec (手写, 含 %pre 创建用户 / %post systemd)
# 兄弟: scripts/install/install-rpm.sh (跨包统一入口, 多了 sha256 校验)
#
# 用法:
#   sudo ./packaging/rpm/install-rpm.sh                       # 默认 target/ 路径
#   sudo ./packaging/rpm/install-rpm.sh /path/to/apeireth.rpm # 显式
# 卸载: sudo dnf remove apeireth
# =============================================================================

set -euo pipefail

VERSION="${APEIRETH_VERSION:-1.0.0}"
cd "$(dirname "$0")/../.."

# 0. root 守门
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root: sudo $0 $*"
    exit 1
fi

# 1. 找 .rpm
if [[ -n "${1:-}" ]]; then
    RPM_PATH="$1"
else
    RPM_PATH=$(find target -name "apeireth-${VERSION}-*.rpm" -type f 2>/dev/null | head -1 || true)
    if [[ -z "${RPM_PATH}" ]]; then
        echo "❌ 未找到 apeireth-${VERSION}-*.rpm, 先跑 packaging/rpm/build.sh"
        exit 1
    fi
fi

if [[ ! -f "${RPM_PATH}" ]]; then
    echo "❌ .rpm 不存在: ${RPM_PATH}"
    exit 1
fi

echo "=== apeireth rpm install v${VERSION} (${RPM_PATH}) ==="

# 2. dnf install (兼容 yum / rpm)
echo "[1/3] dnf install ${RPM_PATH}..."
if command -v dnf >/dev/null 2>&1; then
    dnf install -y "${RPM_PATH}"
elif command -v yum >/dev/null 2>&1; then
    yum install -y "${RPM_PATH}"
else
    rpm -Uvh "${RPM_PATH}"
fi

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

echo "✅ 安装完成 (详细见 docs/installation/rpm-install.md)"
