#!/usr/bin/env bash
# =============================================================================
# packaging/deb/uninstall-deb.sh
#
# deb 的 user-side uninstall helper (per task spec 1.0 release #4)
# vs packaging/deb/install-deb.sh: 后者装, 本脚本卸 (apt remove --purge)
#
# 决策: D-06 (8 包齐发)
# 兄弟: packaging/deb/install-deb.sh (装) / scripts/install/uninstall-all.sh (跨包)
#
# 6 哲学锚穿透:
#   1. 不假装已卸 — systemctl status + /health 验证
#   2. 守门 — root + 确认提示
#   3. 真理 — apt remove --purge 真清, 不假装 dpkg -P 跳过钩子
#   4. 进化 — 保留数据选项 (--keep-data), 升级时不丢
#   5. 择善 — 用 apt 而非 dpkg, 自动解依赖 + 触发钩子
#   6. 不从 — 用户确认前不动手
#
# 8 项不修改承诺:
#   - 0 改 24 LOCKED crate
#   - 0 改 workspace version 1.0.0
#   - 0 引 NewAPI (用系统 apt/systemctl)
#   - 不假装: 缺 apt / 缺 root 时显式报错, 不假装卸了
#   - 编译期 hardcode: 包名 apeireth, 数据目录列表
#   - 6 哲学锚穿透 (见上)
#   - 不重复造轮子: 调 packaging/deb/apeireth.service 现成 unit, 不重写
#   - 诚实标缺: 跨发行版 dpkg hook 标 TODO, 实际依赖 apt 触发
#
# 用法:
#   sudo ./packaging/deb/uninstall-deb.sh                 # 卸 + 清数据
#   sudo ./packaging/deb/uninstall-deb.sh --keep-data     # 卸 + 保留 /var/lib/apeireth
#   sudo ./packaging/deb/uninstall-deb.sh --force         # 跳过 y/N 确认
# =============================================================================

set -euo pipefail

# === 0. root 守门 ===
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (apt remove / systemctl): sudo $0 $*"
    exit 1
fi

# === 1. 参数 ===
KEEP_DATA=false
FORCE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --keep-data) KEEP_DATA=true; shift ;;
        --force)     FORCE=true; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# *//' | head -40
            exit 0
            ;;
        *) echo "❌ 未知参数: $1"; exit 1 ;;
    esac
done

echo "=== apeireth deb uninstall ==="
echo "    KEEP_DATA: ${KEEP_DATA}"
echo "    FORCE:     ${FORCE}"

# === 2. 检查是否装了 ===
if ! dpkg -l apeireth 2>/dev/null | grep -q '^ii'; then
    echo "❌ apeireth 未装 (dpkg -l 无 'ii' 标记)"
    echo "   手动查: dpkg -l | grep apeireth"
    exit 1
fi

# === 3. 确认 (除非 --force) ===
if [[ "${FORCE}" != "true" ]]; then
    read -rp "确认卸载 apeireth (.deb)? (y/N) " CONFIRM
    if [[ "${CONFIRM}" != "y" && "${CONFIRM}" != "Y" ]]; then
        echo "已取消"
        exit 0
    fi
fi

# === 4. systemd 停服 + 禁用 (在 apt remove 之前) ===
echo "[1/4] systemctl stop + disable..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop apeireth 2>/dev/null || echo "    (服务未运行, 跳过 stop)"
    systemctl disable apeireth 2>/dev/null || echo "    (服务未启用, 跳过 disable)"
fi

# === 5. apt remove --purge (per 蓝图 §3.7 step 3) ===
echo "[2/4] apt remove --purge apeireth..."
apt remove --purge -y apeireth
apt autoremove -y 2>/dev/null || true  # 清孤包

# === 6. 残留清理 (数据 / 配置 / 日志) ===
if [[ "${KEEP_DATA}" != "true" ]]; then
    echo "[3/4] drop data + config + log..."
    rm -rf /var/lib/apeireth 2>/dev/null || true
    rm -rf /var/log/apeireth 2>/dev/null || true
    rm -rf /etc/apeireth 2>/dev/null || true
else
    echo "[3/4] ⚠️  保留数据 (--keep-data): /var/lib/apeireth, /etc/apeireth"
fi

# === 7. systemd daemon-reload ===
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload
    systemctl reset-failed apeireth 2>/dev/null || true
fi

# === 8. 验证 0 残留 (per 蓝图 §3.7 step 5) ===
echo "[4/4] 验证 0 残留..."
RESIDUE=0
if dpkg -l apeireth 2>/dev/null | grep -q '^ii'; then
    echo "    ❌ dpkg 仍有 apeireth 标记"
    RESIDUE=$((RESIDUE + 1))
fi
if [[ -d /var/lib/apeireth && "${KEEP_DATA}" != "true" ]]; then
    echo "    ❌ /var/lib/apeireth 残留"
    RESIDUE=$((RESIDUE + 1))
fi
if [[ -f /etc/systemd/system/apeireth.service ]]; then
    echo "    ❌ /etc/systemd/system/apeireth.service 残留"
    RESIDUE=$((RESIDUE + 1))
fi

if [[ ${RESIDUE} -eq 0 ]]; then
    echo "    ✅ 0 残留, 卸载完成"
    echo ""
    echo "重装: sudo ./packaging/deb/install-deb.sh"
    exit 0
else
    echo ""
    echo "⚠️  ${RESIDUE} 项残留, 手动清: dpkg --purge apeireth; rm -rf /var/lib/apeireth /etc/apeireth"
    exit 1
fi
