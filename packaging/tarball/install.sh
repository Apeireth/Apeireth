#!/usr/bin/env bash
# =============================================================================
# packaging/tarball/install.sh
#
# 通用 tarball 的 user-side install helper (per task spec 1.0 release #4)
# vs packaging/tarball/build.sh: 后者是 release engineer 用的 (打 tarball)
#                                 前者是 end user 用的 (装 tarball, 不依赖其他工具)
#
# 决策: D-06 (8 包齐发)
# 适用: 任何 Linux / Unix (有 tar / curl / sha256sum 即可, 不需 apt/dnf/brew)
# 兄弟: scripts/install/install-tarball.sh (跨包统一入口, 多了 sha256 校验 + systemd)
#
# 这个 install.sh 是 self-contained, 跟着 tarball 一起发, 让用户能装在没 apt/dnf 的系统
#
# 用法 (在 tarball 解包后):
#   sudo ./install.sh
# 卸载: sudo rm -rf /opt/apeireth /usr/local/bin/apeireth
# =============================================================================

set -euo pipefail

VERSION="${APEIRETH_VERSION:-1.0.0}"
INSTALL_DIR="${APEIRETH_INSTALL_DIR:-/opt/apeireth}"

# 0. root 守门
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (写入 /opt + /usr/local/bin): sudo $0"
    exit 1
fi

echo "=== apeireth tarball install v${VERSION} ==="
echo "    安装到: ${INSTALL_DIR}"

# 1. 找 apeireth 二进制 (当前目录或 bin/ 子目录)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [[ -x "${SCRIPT_DIR}/apeireth" ]]; then
    BIN_PATH="${SCRIPT_DIR}/apeireth"
elif [[ -x "${SCRIPT_DIR}/bin/apeireth" ]]; then
    BIN_PATH="${SCRIPT_DIR}/bin/apeireth"
else
    echo "❌ 未找到 apeireth 二进制 (${SCRIPT_DIR}/apeireth 或 bin/apeireth)"
    exit 1
fi

# 2. 拷到 /opt/apeireth
echo "[1/4] 部署二进制..."
mkdir -p "${INSTALL_DIR}/bin"
cp "${BIN_PATH}" "${INSTALL_DIR}/bin/apeireth"
chmod +x "${INSTALL_DIR}/bin/apeireth"

# 3. symlink
echo "[2/4] symlink /usr/local/bin/apeireth..."
ln -sf "${INSTALL_DIR}/bin/apeireth" /usr/local/bin/apeireth

# 4. systemd (optional)
echo "[3/4] systemd 集成 (如有 systemd)..."
if [[ -f "${SCRIPT_DIR}/systemd/apeireth.service" ]] && command -v systemctl >/dev/null 2>&1; then
    cp "${SCRIPT_DIR}/systemd/apeireth.service" /etc/systemd/system/apeireth.service
    systemctl daemon-reload
    systemctl enable apeireth.service
    systemctl restart apeireth.service
    sleep 2
    echo "    ✅ systemd 启用"
elif [[ -f "${SCRIPT_DIR}/systemd/apeireth.service" ]]; then
    echo "    ⚠️  无 systemctl (Alpine / Devuan), 跳过 systemd 部署"
fi

# 5. 报告
echo "[4/4] ✅ 安装完成"
echo "    二进制: ${INSTALL_DIR}/bin/apeireth"
echo "    symlink: /usr/local/bin/apeireth"
echo "    验证: apeireth --version"
echo "    启动: apeireth serve (前台) 或 systemctl status apeireth"
echo "    卸载: sudo rm -rf ${INSTALL_DIR} /usr/local/bin/apeireth /etc/systemd/system/apeireth.service"
