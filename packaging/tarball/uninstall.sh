#!/usr/bin/env bash
# =============================================================================
# packaging/tarball/uninstall.sh
#
# 通用 tarball 的 user-side uninstall helper (per task spec 1.0 release #4)
# vs packaging/tarball/install.sh: 后者装, 本脚本卸
#
# 决策: D-06 (8 包齐发)
# 适用: 任何 Linux / Unix (不需 apt/dnf/brew, 跟 install.sh 一样 0 系统依赖)
# 兄弟: packaging/tarball/install.sh (装) / scripts/install/uninstall-all.sh (跨包)
#
# 6 哲学锚穿透:
#   1. 不假装已卸 — ls + systemctl 验证
#   2. 守门 — root + 确认提示
#   3. 真理 — rm 真清, 不假装硬链忽略
#   4. 进化 — 保留数据选项 (--keep-data)
#   5. 择善 — 手起/系统服务两条路都清
#   6. 不从 — 用户确认前不动手
#
# 8 项不修改承诺:
#   - 0 改 24 LOCKED crate
#   - 0 改 workspace version 1.0.0
#   - 0 引 NewAPI (用系统 rm/systemctl)
#   - 不假装: 缺 /opt/apeireth 时显式报未装
#   - 编译期 hardcode: INSTALL_DIR=/opt/apeireth, symlink 路径
#   - 6 哲学锚穿透
#   - 不重复造轮子: 跟 install.sh 用同样路径常量
#   - 诚实标缺: 无 systemd 系统跳过 unit 清理 (Alpine/Devuan)
#
# 用法:
#   sudo ./packaging/tarball/uninstall.sh                 # 卸 + 清数据
#   sudo ./packaging/tarball/uninstall.sh --keep-data     # 卸 + 保留 /var/lib/apeireth
#   sudo ./packaging/tarball/uninstall.sh --force         # 跳过 y/N 确认
# =============================================================================

set -euo pipefail

# === 0. root 守门 ===
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (写入 /opt / /usr/local/bin / /etc/systemd): sudo $0 $*"
    exit 1
fi

# === 1. 参数 ===
KEEP_DATA=false
FORCE=false
INSTALL_DIR="${APEIRETH_INSTALL_DIR:-/opt/apeireth}"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --keep-data)  KEEP_DATA=true; shift ;;
        --force)      FORCE=true; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# *//' | head -40
            exit 0
            ;;
        *) echo "❌ 未知参数: $1"; exit 1 ;;
    esac
done

echo "=== apeireth tarball uninstall ==="
echo "    INSTALL_DIR: ${INSTALL_DIR}"
echo "    KEEP_DATA:   ${KEEP_DATA}"
echo "    FORCE:       ${FORCE}"

# === 2. 检查是否装了 ===
RESIDUE_DETECT=0
if [[ -d "${INSTALL_DIR}" ]]; then RESIDUE_DETECT=$((RESIDUE_DETECT + 1)); fi
if [[ -L /usr/local/bin/apeireth ]]; then RESIDUE_DETECT=$((RESIDUE_DETECT + 1)); fi
if [[ -f /etc/systemd/system/apeireth.service ]]; then RESIDUE_DETECT=$((RESIDUE_DETECT + 1)); fi
if [[ ${RESIDUE_DETECT} -eq 0 ]]; then
    echo "❌ 未检测到 apeireth (${INSTALL_DIR} 不存在, /usr/local/bin/apeireth 无, systemd unit 无)"
    exit 1
fi
echo "    检测到 ${RESIDUE_DETECT} 项安装痕迹"

# === 3. 确认 ===
if [[ "${FORCE}" != "true" ]]; then
    read -rp "确认卸载 apeireth (tarball)? (y/N) " CONFIRM
    if [[ "${CONFIRM}" != "y" && "${CONFIRM}" != "Y" ]]; then
        echo "已取消"
        exit 0
    fi
fi

# === 4. systemd 停服 (如有) ===
echo "[1/4] systemctl stop + disable (如有)..."
if command -v systemctl >/dev/null 2>&1 && [[ -f /etc/systemd/system/apeireth.service ]]; then
    systemctl stop apeireth 2>/dev/null || echo "    (服务未运行, 跳过 stop)"
    systemctl disable apeireth 2>/dev/null || echo "    (服务未启用, 跳过 disable)"
fi

# === 5. 删 systemd unit ===
echo "[2/4] 删 systemd unit..."
if [[ -f /etc/systemd/system/apeireth.service ]]; then
    rm -f /etc/systemd/system/apeireth.service
    if command -v systemctl >/dev/null 2>&1; then
        systemctl daemon-reload
        systemctl reset-failed apeireth 2>/dev/null || true
    fi
else
    echo "    (无 systemd unit, 跳过 — Alpine/Devuan/WSL2 预期)"
fi

# === 6. 删安装目录 + symlink ===
echo "[3/4] 删 ${INSTALL_DIR} + /usr/local/bin/apeireth..."
if [[ -d "${INSTALL_DIR}" ]]; then
    rm -rf "${INSTALL_DIR}"
fi
if [[ -L /usr/local/bin/apeireth ]] || [[ -f /usr/local/bin/apeireth ]]; then
    rm -f /usr/local/bin/apeireth
fi

# === 7. 数据/配置/日志清理 ===
if [[ "${KEEP_DATA}" != "true" ]]; then
    echo "[4/4] drop data + config + log..."
    rm -rf /var/lib/apeireth 2>/dev/null || true
    rm -rf /var/log/apeireth 2>/dev/null || true
    rm -rf /etc/apeireth 2>/dev/null || true
else
    echo "[4/4] ⚠️  保留数据 (--keep-data): /var/lib/apeireth, /etc/apeireth"
fi

# === 8. 验证 0 残留 ===
RESIDUE=0
[[ -d "${INSTALL_DIR}" ]] && { echo "    ❌ ${INSTALL_DIR} 残留"; RESIDUE=$((RESIDUE + 1)); }
[[ -e /usr/local/bin/apeireth ]] && { echo "    ❌ /usr/local/bin/apeireth 残留"; RESIDUE=$((RESIDUE + 1)); }
[[ -f /etc/systemd/system/apeireth.service ]] && { echo "    ❌ systemd unit 残留"; RESIDUE=$((RESIDUE + 1)); }
if [[ ${RESIDUE} -eq 0 ]]; then
    echo "    ✅ 0 残留, 卸载完成"
    echo ""
    echo "重装: sudo ./packaging/tarball/install.sh"
    exit 0
else
    echo ""
    echo "⚠️  ${RESIDUE} 项残留, 手动清: rm -rf ${INSTALL_DIR} /usr/local/bin/apeireth /etc/systemd/system/apeireth.service"
    exit 1
fi
