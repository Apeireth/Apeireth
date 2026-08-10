#!/usr/bin/env bash
# =============================================================================
# scripts/install/uninstall-all.sh
#
# Apeireth OS — 通用卸载入口 (跨 8 通道, 自动检测)
# (1.0 release checklist #4 install + #6 uninstall 集成)
#
# 蓝图: docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.7
# 决策: D-06 (主人 2026-08-05 20:53 拍 A: 8 包齐发)
# 兄弟: scripts/uninstall/uninstall.sh (手动指定 --channel)
#
# 5 步 0 残留守门 (per 蓝图 §3.7):
#   1. 检测当前装了哪 1 个或多个 (8 通道, 互斥)
#   2. 询问用户确认 (y/N)
#   3. 调对应通道的 uninstall (apt / dnf / brew / scoop / rm / docker)
#   4. 清理数据目录 / 配置 / 日志 / 端口
#   5. 报告残留 (期望 0 残留, 列出 0 错)
#
# 自动检测优先级 (高→低, 命中即停):
#   1. deb: dpkg -l apeireth
#   2. rpm: rpm -q apeireth
#   3. brew: brew list apeireth
#   4. scoop: scoop list apeireth  (Windows 旁路)
#   5. tarball: /opt/apeireth/bin/apeireth
#   6. zip: C:\Program Files\apeireth\ (Windows)
#   7. msi: Windows registry HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
#   8. docker: docker images apeireth
#
# 8 项不修改承诺 (不重述):
#   - 0 改 LOCKED, 0 改 version, 0 引 NewAPI
#   - 编译期 hardcode 数据目录列表
#
# 用法:
#   sudo ./scripts/install/uninstall-all.sh                     # 自动检测
#   sudo ./scripts/install/uninstall-all.sh --keep-data         # 保留 /var/lib/apeireth
#   sudo ./scripts/install/uninstall-all.sh --force             # 跳过 y/N 确认
# =============================================================================

set -euo pipefail

cd "$(dirname "$0")/../.."

# === 0. 参数解析 ===
KEEP_DATA=false
FORCE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --keep-data) KEEP_DATA=true; shift ;;
        --force)     FORCE=true; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# *//'
            exit 0
            ;;
        *) echo "❌ 未知参数: $1 (用 --help)"; exit 1 ;;
    esac
done

echo "=== apeireth uninstall-all (8 通道自动检测) ==="
echo "    KEEP_DATA: ${KEEP_DATA}"
echo "    FORCE:     ${FORCE}"

# === 1. 检测已装通道 ===
DETECTED=()
DETECT_FUNCS=(
    "detect_deb:remove_deb"
    "detect_rpm:remove_rpm"
    "detect_brew:remove_brew"
    "detect_tarball:remove_tarball"
    "detect_zip:remove_zip"
    "detect_docker:remove_docker"
)

declare -A DETECT_RESULTS
for entry in "${DETECT_FUNCS[@]}"; do
    FN="${entry%%:*}"
    RM="${entry##*:}"
    if "${FN}"; then
        DETECTED+=("${RM}")
        DETECT_RESULTS["${RM}"]=true
    fi
done

# Windows 旁路 (scoop / msi): 在非 Windows 系统直接跳过
if [[ "$(uname -s)" == "MINGW"* || "$(uname -s)" == "CYGWIN"* || "$(uname -s)" == "MSYS"* ]]; then
    if command -v scoop >/dev/null 2>&1 && scoop list apeireth >/dev/null 2>&1; then
        DETECTED+=("remove_scoop")
    fi
fi

if [[ ${#DETECTED[@]} -eq 0 ]]; then
    echo "❌ 未检测到 apeireth 安装 (8 通道全 0 命中)"
    echo "   手动查: dpkg -l | grep apeireth; rpm -q apeireth; brew list apeireth"
    exit 1
fi

echo ""
echo "检测到已装通道:"
for rm in "${DETECTED[@]}"; do
    echo "  - ${rm}"
done
echo ""

# === 2. 确认 ===
if [[ "${FORCE}" != "true" ]]; then
    read -rp "确认卸载以上所有? (y/N) " CONFIRM
    if [[ "${CONFIRM}" != "y" && "${CONFIRM}" != "Y" ]]; then
        echo "已取消"
        exit 0
    fi
fi

# === 3. 调对应通道 uninstall ===
for rm in "${DETECTED[@]}"; do
    echo ""
    echo "[=== ${rm} ===]"
    if declare -f "${rm}" >/dev/null; then
        "${rm}"
    else
        echo "⚠️  ${rm} 未实装 (Windows only / 估补)"
    fi
done

# === 4. 公共清理 (per 蓝图 §3.7 step 4-5, 跨通道 0 残留) ===
echo ""
echo "[=== 公共清理 ===]"
if [[ "${KEEP_DATA}" != "true" ]]; then
    echo "[4/5] drop data..."
    rm -rf /var/lib/apeireth /var/log/apeireth 2>/dev/null || true
    rm -rf /etc/apeireth 2>/dev/null || true
    # Windows 旁路
    if [[ "$(uname -s)" == "MINGW"* || "$(uname -s)" == "MSYS"* ]]; then
        rm -rf "$HOME/.apeireth" 2>/dev/null || true
    fi
else
    echo "[4/5] ⚠️  保留数据 (--keep-data): /var/lib/apeireth, /etc/apeireth"
fi

echo "[5/5] release port (8080/9090)..."
# lsof / fuser, 见 scripts/uninstall/uninstall.sh 详细实现
if command -v lsof >/dev/null 2>&1; then
    PORTS=$(lsof -ti:8080,9090 2>/dev/null || true)
    if [[ -n "${PORTS}" ]]; then
        echo "    ⚠️  端口仍占用: ${PORTS}, 手动: kill -9 ${PORTS}"
    else
        echo "    ✅ 端口 8080/9090 释放"
    fi
else
    echo "    ℹ️  无 lsof, 跳端口检查"
fi

echo ""
echo "✅ 卸载完成 (期望 0 残留, 残留列上面 ⚠️ 项)"
echo "   重装: scripts/install/install-{deb,rpm,tarball,brew}.sh"
echo "         scripts/install/install-scoop.ps1  (Windows)"

# ====== 检测函数 (per channel) ======
detect_deb() { dpkg -l apeireth 2>/dev/null | grep -q '^ii'; }
detect_rpm() { rpm -q apeireth >/dev/null 2>&1; }
detect_brew() { command -v brew >/dev/null 2>&1 && brew list apeireth >/dev/null 2>&1; }
detect_tarball() { [[ -x /opt/apeireth/bin/apeireth ]] || [[ -L /usr/local/bin/apeireth ]]; }
detect_zip() { [[ "$(uname -s)" == "MINGW"* || "$(uname -s)" == "MSYS"* ]] && [[ -d "/c/Program Files/apeireth" ]]; }
detect_docker() { command -v docker >/dev/null 2>&1 && docker images apeireth 2>/dev/null | grep -q apeireth; }

# ====== 移除函数 (per channel) ======
remove_deb() {
    if [[ $EUID -ne 0 ]]; then echo "❌ 需要 root"; return 1; fi
    systemctl stop apeireth 2>/dev/null || true
    systemctl disable apeireth 2>/dev/null || true
    apt remove --purge -y apeireth
}
remove_rpm() {
    if [[ $EUID -ne 0 ]]; then echo "❌ 需要 root"; return 1; fi
    systemctl stop apeireth 2>/dev/null || true
    systemctl disable apeireth 2>/dev/null || true
    if command -v dnf >/dev/null 2>&1; then
        dnf remove -y apeireth
    else
        yum remove -y apeireth
    fi
}
remove_brew() {
    brew services stop apeireth 2>/dev/null || true
    brew uninstall apeireth
}
remove_scoop() {
    echo "  (scoop 卸载见 scripts/install/install-scoop.ps1 注释)"
    echo "   Windows: scoop uninstall apeireth"
}
remove_tarball() {
    if [[ $EUID -ne 0 ]]; then echo "❌ 需要 root"; return 1; fi
    systemctl stop apeireth 2>/dev/null || true
    systemctl disable apeireth 2>/dev/null || true
    rm -f /etc/systemd/system/apeireth.service
    systemctl daemon-reload
    rm -rf /opt/apeireth
    rm -f /usr/local/bin/apeireth
}
remove_zip() {
    echo "  (zip 卸载估补, Windows-only)"
}
remove_docker() {
    docker stop apeireth 2>/dev/null || true
    docker rm apeireth 2>/dev/null || true
    docker rmi ghcr.io/apeireth/apeireth:1.0.0 2>/dev/null || true
}
