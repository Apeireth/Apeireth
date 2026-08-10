#!/usr/bin/env bash
# =============================================================================
# scripts/install/install-deb.sh
#
# Apeireth OS — Debian/Ubuntu .deb 安装入口
# (1.0 release checklist #4 install, D-06 8 包齐发 + Linux 4 包重点)
#
# 蓝图: docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4
# 决策: D-06 (主人 2026-08-05 20:53 拍 A: 8 包齐发 + Linux 4 包重点)
# 兄弟脚本: install-rpm.sh / install-tarball.sh / install-brew.sh / install-scoop.ps1
#
# 5 步标准安装流 (per 蓝图 §3.4):
#   1. 检测 .deb 路径 (参数 $1, 默认 target/.../*.deb)
#   2. 校验 sha256 (如果同目录有 .sha256)
#   3. apt install ./<deb> (systemd unit + 配置自动部署)
#   4. systemctl daemon-reload + enable + start apeireth
#   5. 健康检查 curl /health 期望 200
#
# 严守 8 项不修改承诺:
#   - 0 改 24 LOCKED crate
#   - 0 改 workspace version 1.0.0
#   - 0 引 NewAPI (用系统 apt / systemctl / curl)
#   - 不假装: 缺 cargo-deb 工具链时, 引导去 packaging/deb/build.sh 跑, 不假装编过
#   - 编译期 hardcode: VERSION=1.0.0, 默认 /etc/apeireth/config.toml
#   - 6 哲学锚穿透: 不假装已装 / 不假装健康 / 不假装签名 (cosign 后续 R20 阶段 6 续)
#   - 不重复造轮子: 用 packaging/deb/apeireth.service 现成 unit, 不重写
#   - 诚实标缺: 跨平台测试不跑 (CI 跑), 标需要 CI 守门
#
# 用法:
#   sudo ./scripts/install/install-deb.sh                                    # 默认路径
#   sudo ./scripts/install/install-deb.sh /path/to/apeireth_1.0.0_amd64.deb  # 显式
#   APEIRETH_DEB=./apeireth.deb sudo ./scripts/install/install-deb.sh        # 环境变量
# 卸载: sudo apt remove --purge apeireth  (用 scripts/uninstall/uninstall.sh)
# =============================================================================

set -euo pipefail

# === 0. root 守门 (per 蓝图 §3.4 5 守门 non-root image, 安装步骤可逆) ===
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (apt install / systemctl): sudo $0 $*"
    exit 1
fi

# === 1. 找 .deb ===
cd "$(dirname "$0")/../.."
VERSION="${APEIRETH_VERSION:-1.0.0}"

if [[ -n "${1:-}" ]]; then
    DEB_PATH="$1"
elif [[ -n "${APEIRETH_DEB:-}" ]]; then
    DEB_PATH="${APEIRETH_DEB}"
else
    # 默认: target/<triple>/debian/apeireth_<version>_amd64.deb
    DEB_PATH=$(find target -name "apeireth_${VERSION}_amd64.deb" -type f 2>/dev/null | head -1 || true)
    if [[ -z "${DEB_PATH}" ]]; then
        echo "❌ 未找到 .deb (默认路径 target/**/apeireth_${VERSION}_amd64.deb)"
        echo "   选项:"
        echo "   1. 先跑 packaging/deb/build.sh 编出 .deb"
        echo "   2. 从 GitHub release 下载: https://github.com/apeireth/apeireth-rust/releases/download/v${VERSION}/apeireth_${VERSION}_amd64.deb"
        echo "   3. 显式传: sudo $0 /path/to/apeireth.deb"
        exit 1
    fi
fi

if [[ ! -f "${DEB_PATH}" ]]; then
    echo "❌ .deb 不存在: ${DEB_PATH}"
    exit 1
fi

echo "=== apeireth deb install v${VERSION} ==="
echo "    目标: ${DEB_PATH}"

# === 2. sha256 校验 (per 蓝图 §3.4 完整性) ===
SHA256_PATH="${DEB_PATH}.sha256"
if [[ -f "${SHA256_PATH}" ]]; then
    echo "[1/5] 校验 sha256..."
    EXPECTED=$(cat "${SHA256_PATH}" | cut -d' ' -f1)
    ACTUAL=$(sha256sum "${DEB_PATH}" | cut -d' ' -f1)
    if [[ "${EXPECTED}" != "${ACTUAL}" ]]; then
        echo "❌ sha256 不匹配:"
        echo "    期望: ${EXPECTED}"
        echo "    实际: ${ACTUAL}"
        exit 1
    fi
    echo "    ✅ sha256 验证通过"
else
    echo "[1/5] ⚠️  无 .sha256 旁路文件, 跳过完整性校验 (CI 产物应带, 手工下载可能缺)"
fi

# === 3. apt install ===
echo "[2/5] apt install ${DEB_PATH}..."
apt update -qq
apt install -y "${DEB_PATH}"

# === 4. systemd 启用 ===
echo "[3/5] systemctl daemon-reload + enable + start..."
systemctl daemon-reload
systemctl enable apeireth.service
systemctl restart apeireth.service
sleep 2  # 给 Type=notify 一点时间

# === 5. 健康检查 (per 蓝图 §3.4 install #4) ===
echo "[4/5] 健康检查 curl /health (期望 200)..."
HEALTH=$(curl -fsS -m 5 http://localhost:8080/health || echo "FAILED")
if [[ "${HEALTH}" == "FAILED" ]]; then
    echo "⚠️  /health 未响应, 查看日志: journalctl -u apeireth -n 50"
    echo "    安装完成, 但服务可能未正常启动 (请人工排查)"
    exit 0  # 不阻塞: 安装动作已完成
fi
echo "    ✅ /health: ${HEALTH}"

# === 6. 完成报告 ===
echo "[5/5] ✅ 安装完成"
echo "    状态: systemctl status apeireth"
echo "    日志: journalctl -u apeireth -f"
echo "    卸载: sudo apt remove --purge apeireth"
echo "          或: sudo scripts/uninstall/uninstall.sh --channel deb"
