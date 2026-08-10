#!/usr/bin/env bash
# =============================================================================
# scripts/install/install-rpm.sh
#
# Apeireth OS — RHEL/Fedora/CentOS .rpm 安装入口
# (1.0 release checklist #4 install, D-06 8 包齐发 + Linux 4 包重点)
#
# 蓝图: docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4
# 决策: D-06 (主人 2026-08-05 20:53 拍 A: 8 包齐发 + Linux 4 包重点)
#
# 5 步标准安装流 (per 蓝图 §3.4):
#   1. 检测 .rpm 路径 (参数 $1, 默认 target/rpm/...)
#   2. 校验 sha256 (如果同目录有 .sha256)
#   3. dnf install ./<rpm> (systemd unit + 配置自动部署)
#   4. systemctl daemon-reload + enable + start apeireth
#   5. 健康检查 curl /health 期望 200
#
# 8 项不修改承诺 (同 install-deb.sh, 不重述):
#   - 0 改 24 LOCKED, 0 改 workspace version, 0 引 NewAPI
#   - 编译期 hardcode VERSION=1.0.0
#   - 诚实标缺: 缺 cargo-rpm 工具链时引导去 packaging/rpm/build.sh
#
# 用法:
#   sudo ./scripts/install/install-rpm.sh                                    # 默认路径
#   sudo ./scripts/install/install-rpm.sh /path/to/apeireth-1.0.0-1.x86_64.rpm
#   APEIRETH_RPM=./apeireth.rpm sudo ./scripts/install/install-rpm.sh
# 卸载: sudo dnf remove apeireth
# =============================================================================

set -euo pipefail

# === 0. root 守门 ===
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (dnf install / systemctl): sudo $0 $*"
    exit 1
fi

# === 1. 找 .rpm ===
cd "$(dirname "$0")/../.."
VERSION="${APEIRETH_VERSION:-1.0.0}"

if [[ -n "${1:-}" ]]; then
    RPM_PATH="$1"
elif [[ -n "${APEIRETH_RPM:-}" ]]; then
    RPM_PATH="${APEIRETH_RPM}"
else
    # 默认: target/rpm/RPMS/x86_64/apeireth-<version>-1.x86_64.rpm (cargo-rpm 默认输出)
    RPM_PATH=$(find target -name "apeireth-${VERSION}-*.rpm" -type f 2>/dev/null | head -1 || true)
    if [[ -z "${RPM_PATH}" ]]; then
        echo "❌ 未找到 .rpm (默认路径 target/**/apeireth-${VERSION}-*.rpm)"
        echo "   选项:"
        echo "   1. 先跑 packaging/rpm/build.sh 编出 .rpm"
        echo "   2. 从 GitHub release 下载: https://github.com/apeireth/apeireth-rust/releases/download/v${VERSION}/apeireth-${VERSION}-1.x86_64.rpm"
        echo "   3. 显式传: sudo $0 /path/to/apeireth.rpm"
        exit 1
    fi
fi

if [[ ! -f "${RPM_PATH}" ]]; then
    echo "❌ .rpm 不存在: ${RPM_PATH}"
    exit 1
fi

echo "=== apeireth rpm install v${VERSION} ==="
echo "    目标: ${RPM_PATH}"

# === 2. sha256 校验 ===
SHA256_PATH="${RPM_PATH}.sha256"
if [[ -f "${SHA256_PATH}" ]]; then
    echo "[1/5] 校验 sha256..."
    EXPECTED=$(cat "${SHA256_PATH}" | cut -d' ' -f1)
    ACTUAL=$(sha256sum "${RPM_PATH}" | cut -d' ' -f1)
    if [[ "${EXPECTED}" != "${ACTUAL}" ]]; then
        echo "❌ sha256 不匹配:"
        echo "    期望: ${EXPECTED}"
        echo "    实际: ${ACTUAL}"
        exit 1
    fi
    echo "    ✅ sha256 验证通过"
else
    echo "[1/5] ⚠️  无 .sha256 旁路文件, 跳过完整性校验"
fi

# === 3. dnf install (兼容 yum / dnf / rpm) ===
echo "[2/5] dnf install ${RPM_PATH}..."
if command -v dnf >/dev/null 2>&1; then
    dnf install -y "${RPM_PATH}"
elif command -v yum >/dev/null 2>&1; then
    yum install -y "${RPM_PATH}"
else
    echo "❌ 未找到 dnf/yum, RHEL/CentOS 应有 dnf (8+); 老版本用 rpm -Uvh"
    rpm -Uvh "${RPM_PATH}"
fi

# === 4. systemd 启用 ===
echo "[3/5] systemctl daemon-reload + enable + start..."
systemctl daemon-reload
systemctl enable apeireth.service
systemctl restart apeireth.service
sleep 2

# === 5. 健康检查 ===
echo "[4/5] 健康检查 curl /health (期望 200)..."
HEALTH=$(curl -fsS -m 5 http://localhost:8080/health || echo "FAILED")
if [[ "${HEALTH}" == "FAILED" ]]; then
    echo "⚠️  /health 未响应, 查看日志: journalctl -u apeireth -n 50"
    echo "    安装完成, 但服务可能未正常启动 (请人工排查)"
    exit 0
fi
echo "    ✅ /health: ${HEALTH}"

# === 6. 完成 ===
echo "[5/5] ✅ 安装完成"
echo "    状态: systemctl status apeireth"
echo "    日志: journalctl -u apeireth -f"
echo "    卸载: sudo dnf remove apeireth"
echo "          或: sudo scripts/uninstall/uninstall.sh --channel rpm"
