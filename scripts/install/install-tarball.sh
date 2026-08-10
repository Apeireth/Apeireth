#!/usr/bin/env bash
# =============================================================================
# scripts/install/install-tarball.sh
#
# Apeireth OS — 通用 Linux/Unix tarball 安装入口
# (1.0 release checklist #4 install, D-06 8 包齐发 + Linux 4 包重点)
#
# 蓝图: docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4
# 决策: D-06 (主人 2026-08-05 20:53 拍 A: 8 包齐发 + Linux 4 包重点)
#
# 5 步标准安装流 (per 蓝图 §3.4):
#   1. 检测 tarball 路径 (参数 $1, 默认 target/**/apeireth-*.tar.gz)
#   2. 校验 sha256 (如果同目录有 .sha256)
#   3. 解包到 /opt/apeireth + symlink /usr/local/bin/apeireth
#   4. 部署 systemd unit (从解包目录拷到 /etc/systemd/system/)
#   5. systemctl enable + start + 健康检查
#
# 适用场景 (per packaging/tarball/build.sh 注释):
#   - AUR / 自编译基础 / 老发行版 (没 apt / dnf)
#   - 任何 musl 静态链接兼容的 Linux
#   - musl 静态 = 0 运行时依赖 (ldd 应显示 'not a dynamic executable')
#
# 8 项不修改承诺 (同 install-deb.sh, 不重述):
#   - 0 改 24 LOCKED, 0 改 workspace version, 0 引 NewAPI
#   - 编译期 hardcode VERSION=1.0.0, INSTALL_DIR=/opt/apeireth
#
# 用法:
#   sudo ./scripts/install/install-tarball.sh                                    # 默认
#   sudo ./scripts/install/install-tarball.sh /path/to/apeireth-1.0.0-x86_64-linux.tar.gz
#   APEIRETH_TARBALL=./apeireth.tar.gz sudo ./scripts/install/install-tarball.sh
# 卸载: sudo rm -rf /opt/apeireth /usr/local/bin/apeireth /etc/systemd/system/apeireth.service
# =============================================================================

set -euo pipefail

# === 0. root 守门 ===
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (写入 /opt / /usr/local/bin / /etc/systemd): sudo $0 $*"
    exit 1
fi

# === 1. 找 tarball ===
cd "$(dirname "$0")/../.."
VERSION="${APEIRETH_VERSION:-1.0.0}"
INSTALL_DIR="${APEIRETH_INSTALL_DIR:-/opt/apeireth}"

if [[ -n "${1:-}" ]]; then
    TARBALL_PATH="$1"
elif [[ -n "${APEIRETH_TARBALL:-}" ]]; then
    TARBALL_PATH="${APEIRETH_TARBALL}"
else
    TARBALL_PATH=$(find target -name "apeireth-${VERSION}-*.tar.gz" -type f 2>/dev/null | head -1 || true)
    if [[ -z "${TARBALL_PATH}" ]]; then
        echo "❌ 未找到 tarball (默认路径 target/**/apeireth-${VERSION}-*.tar.gz)"
        echo "   选项:"
        echo "   1. 先跑 packaging/tarball/build.sh 编出 tarball"
        echo "   2. 从 GitHub release 下载"
        echo "   3. 显式传: sudo $0 /path/to/apeireth.tar.gz"
        exit 1
    fi
fi

if [[ ! -f "${TARBALL_PATH}" ]]; then
    echo "❌ tarball 不存在: ${TARBALL_PATH}"
    exit 1
fi

echo "=== apeireth tarball install v${VERSION} ==="
echo "    目标: ${TARBALL_PATH}"
echo "    安装到: ${INSTALL_DIR}"

# === 2. sha256 校验 ===
SHA256_PATH="${TARBALL_PATH}.sha256"
if [[ -f "${SHA256_PATH}" ]]; then
    echo "[1/5] 校验 sha256..."
    EXPECTED=$(cat "${SHA256_PATH}" | cut -d' ' -f1)
    ACTUAL=$(sha256sum "${TARBALL_PATH}" | cut -d' ' -f1)
    if [[ "${EXPECTED}" != "${ACTUAL}" ]]; then
        echo "❌ sha256 不匹配"
        exit 1
    fi
    echo "    ✅ sha256 验证通过"
else
    echo "[1/5] ⚠️  无 .sha256 旁路文件, 跳过完整性校验"
fi

# === 3. 解包到 /opt/apeireth ===
echo "[2/5] 解包到 ${INSTALL_DIR} ..."
mkdir -p "${INSTALL_DIR}"
tar -xzf "${TARBALL_PATH}" -C /opt  # tarball 内是 apeireth-<ver>-<triple>/ 子目录
PACK_NAME=$(tar -tzf "${TARBALL_PATH}" | head -1 | cut -d'/' -f1)
ACTUAL_DIR="/opt/${PACK_NAME}"
if [[ "${ACTUAL_DIR}" != "${INSTALL_DIR}" ]]; then
    # 如果目录名跟 INSTALL_DIR 不一致, 链到 /opt/apeireth
    rm -rf "${INSTALL_DIR}"
    mv "${ACTUAL_DIR}" "${INSTALL_DIR}"
fi
chmod +x "${INSTALL_DIR}/bin/apeireth"

# === 4. symlink + systemd unit 部署 ===
echo "[3/5] 部署 bin + systemd unit..."
ln -sf "${INSTALL_DIR}/bin/apeireth" /usr/local/bin/apeireth

if [[ -f "${INSTALL_DIR}/systemd/apeireth.service" ]]; then
    cp "${INSTALL_DIR}/systemd/apeireth.service" /etc/systemd/system/apeireth.service
    systemctl daemon-reload
    systemctl enable apeireth.service
    systemctl restart apeireth.service
    sleep 2
else
    echo "⚠️  解包目录无 systemd/apeireth.service, 跳过 systemd 部署 (非 systemd 系统: Alpine / Devuan)"
fi

# === 5. 健康检查 ===
echo "[4/5] 健康检查 curl /health (期望 200)..."
HEALTH=$(curl -fsS -m 5 http://localhost:8080/health || echo "FAILED")
if [[ "${HEALTH}" == "FAILED" ]]; then
    echo "⚠️  /health 未响应, 查看: journalctl -u apeireth -n 50 或手动 /opt/apeireth/bin/apeireth serve"
    echo "    安装完成, 但服务可能未正常启动 (请人工排查)"
    exit 0
fi
echo "    ✅ /health: ${HEALTH}"

# === 6. 完成 ===
echo "[5/5] ✅ 安装完成"
echo "    二进制: ${INSTALL_DIR}/bin/apeireth"
echo "    symlink: /usr/local/bin/apeireth"
echo "    配置模板: ${INSTALL_DIR}/config/apeireth.env.example"
echo "    卸载: sudo rm -rf ${INSTALL_DIR} /usr/local/bin/apeireth /etc/systemd/system/apeireth.service"
echo "          或: sudo scripts/uninstall/uninstall.sh --channel tarball"
