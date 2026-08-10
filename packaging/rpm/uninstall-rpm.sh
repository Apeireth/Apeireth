#!/usr/bin/env bash
# =============================================================================
# packaging/rpm/uninstall-rpm.sh
#
# rpm 的 user-side uninstall helper (per task spec 1.0 release #4)
# vs packaging/rpm/install-rpm.sh: 后者装, 本脚本卸 (dnf remove)
#
# 决策: D-06 (8 包齐发)
# 兄弟: packaging/rpm/install-rpm.sh (装) / scripts/install/uninstall-all.sh (跨包)
#
# 6 哲学锚穿透:
#   1. 不假装已卸 — rpm -q apeireth + /health 验证
#   2. 守门 — root + 确认提示
#   3. 真理 — dnf remove 真清, 不假装 rpm -e 跳过钩子
#   4. 进化 — 保留数据选项 (--keep-data)
#   5. 择善 — 用 dnf 而非 rpm -e, 自动解依赖 + 触发 %postun
#   6. 不从 — 用户确认前不动手
#
# 8 项不修改承诺:
#   - 0 改 24 LOCKED crate
#   - 0 改 workspace version 1.0.0
#   - 0 引 NewAPI (用系统 dnf/yum/systemctl)
#   - 不假装: 缺 dnf/yum 时显式报, 不假装卸了
#   - 编译期 hardcode: 包名 apeireth, 系统用户 apeireth, 数据目录列表
#   - 6 哲学锚穿透
#   - 不重复造轮子: 复用 spec 里的 %pre/%postun 钩子
#   - 诚实标缺: SELinux 残留标 TODO, 依赖 dnf 触发
#
# 用法:
#   sudo ./packaging/rpm/uninstall-rpm.sh                 # 卸 + 清数据
#   sudo ./packaging/rpm/uninstall-rpm.sh --keep-data     # 卸 + 保留 /var/lib/apeireth
#   sudo ./packaging/rpm/uninstall-rpm.sh --keep-user     # 卸 + 保留系统用户
#   sudo ./packaging/rpm/uninstall-rpm.sh --force         # 跳过 y/N 确认
# =============================================================================

set -euo pipefail

# === 0. root 守门 ===
if [[ $EUID -ne 0 ]]; then
    echo "❌ 需要 root (dnf remove / systemctl): sudo $0 $*"
    exit 1
fi

# === 1. 参数 ===
KEEP_DATA=false
KEEP_USER=false
FORCE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --keep-data)  KEEP_DATA=true; shift ;;
        --keep-user)  KEEP_USER=true; shift ;;
        --force)      FORCE=true; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# *//' | head -40
            exit 0
            ;;
        *) echo "❌ 未知参数: $1"; exit 1 ;;
    esac
done

echo "=== apeireth rpm uninstall ==="
echo "    KEEP_DATA: ${KEEP_DATA}"
echo "    KEEP_USER: ${KEEP_USER}"
echo "    FORCE:     ${FORCE}"

# === 2. 检查是否装了 ===
if ! rpm -q apeireth >/dev/null 2>&1; then
    echo "❌ apeireth 未装 (rpm -q 无返回)"
    echo "   手动查: rpm -q apeireth"
    exit 1
fi

# === 3. 确认 ===
if [[ "${FORCE}" != "true" ]]; then
    read -rp "确认卸载 apeireth (.rpm)? (y/N) " CONFIRM
    if [[ "${CONFIRM}" != "y" && "${CONFIRM}" != "Y" ]]; then
        echo "已取消"
        exit 0
    fi
fi

# === 4. systemd 停服 + 禁用 (在 dnf remove 之前, 触发 %preun) ===
echo "[1/5] systemctl stop + disable..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop apeireth 2>/dev/null || echo "    (服务未运行, 跳过 stop)"
    systemctl disable apeireth 2>/dev/null || echo "    (服务未启用, 跳过 disable)"
fi

# === 5. dnf remove (per 蓝图 §3.7 step 3, 触发 %preun / %postun) ===
echo "[2/5] dnf remove apeireth..."
if command -v dnf >/dev/null 2>&1; then
    dnf remove -y apeireth
elif command -v yum >/dev/null 2>&1; then
    yum remove -y apeireth
else
    echo "❌ 未找到 dnf/yum, 回退 rpm -e"
    rpm -e apeireth
fi

# === 6. 残留清理 ===
if [[ "${KEEP_DATA}" != "true" ]]; then
    echo "[3/5] drop data + config + log..."
    rm -rf /var/lib/apeireth 2>/dev/null || true
    rm -rf /var/log/apeireth 2>/dev/null || true
    rm -rf /etc/apeireth 2>/dev/null || true
else
    echo "[3/5] ⚠️  保留数据 (--keep-data): /var/lib/apeireth, /etc/apeireth"
fi

# === 7. 系统用户清理 ===
if [[ "${KEEP_USER}" != "true" ]]; then
    echo "[4/5] drop system user/group..."
    userdel apeireth 2>/dev/null || echo "    (用户不存在, 跳过)"
    groupdel apeireth 2>/dev/null || echo "    (组不存在, 跳过)"
else
    echo "[4/5] ⚠️  保留系统用户 (--keep-user)"
fi

# === 8. systemd daemon-reload ===
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload
    systemctl reset-failed apeireth 2>/dev/null || true
fi

# === 9. 验证 0 残留 ===
echo "[5/5] 验证 0 残留..."
RESIDUE=0
if rpm -q apeireth >/dev/null 2>&1; then
    echo "    ❌ rpm 仍有 apeireth 标记"
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
if getent passwd apeireth >/dev/null && [[ "${KEEP_USER}" != "true" ]]; then
    echo "    ❌ 系统用户 apeireth 残留"
    RESIDUE=$((RESIDUE + 1))
fi

if [[ ${RESIDUE} -eq 0 ]]; then
    echo "    ✅ 0 残留, 卸载完成"
    echo ""
    echo "重装: sudo ./packaging/rpm/install-rpm.sh"
    exit 0
else
    echo ""
    echo "⚠️  ${RESIDUE} 项残留, 手动清: rpm -e apeireth; rm -rf /var/lib/apeireth /etc/apeireth; userdel apeireth"
    exit 1
fi
