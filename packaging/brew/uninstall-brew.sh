#!/usr/bin/env bash
# =============================================================================
# packaging/brew/uninstall-brew.sh
#
# Homebrew formula 的 user-side uninstall helper (per task spec 1.0 release #4)
# vs packaging/brew/install-brew.sh: 后者装, 本脚本卸 (brew uninstall + services stop)
#
# 决策: D-06 (8 包齐发)
# 公式: packaging/brew/apeireth.rb
# 兄弟: packaging/brew/install-brew.sh (装) / scripts/install/uninstall-all.sh (跨包)
#
# 6 哲学锚穿透:
#   1. 不假装已卸 — brew list 验证
#   2. 守门 — macOS + brew 命令守门, 确认提示
#   3. 真理 — brew services stop + brew uninstall 真清
#   4. 进化 — 保留 tap 选项 (--keep-tap), 保留数据选项 (--keep-data)
#   5. 择善 — 用 brew 而非手 rm Cellar
#   6. 不从 — 用户确认前不动手
#
# 8 项不修改承诺:
#   - 0 改 24 LOCKED crate
#   - 0 改 workspace version 1.0.0
#   - 0 引 NewAPI (用系统 brew)
#   - 不假装: 缺 brew / 未装时显式报
#   - 编译期 hardcode: 包名 apeireth, tap apeireth/tap, log 路径
#   - 6 哲学锚穿透
#   - 不重复造轮子: 用 brew services stop, 不手 kill
#   - 诚实标缺: bottle 卸载后 SHA 不一致标 TODO
#
# 用法:
#   ./packaging/brew/uninstall-brew.sh                 # 卸 + 删 tap + 清数据
#   ./packaging/brew/uninstall-brew.sh --keep-tap     # 卸, 保留 tap
#   ./packaging/brew/uninstall-brew.sh --keep-data    # 卸, 保留 ~/.apeireth
#   ./packaging/brew/uninstall-brew.sh --force        # 跳过 y/N 确认
# =============================================================================

set -euo pipefail

# === 0. macOS + brew 守门 ===
if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "❌ 此脚本仅在 macOS 跑 (其他平台: 见 scripts/install/uninstall-all.sh)"
    exit 1
fi
if ! command -v brew >/dev/null 2>&1; then
    echo "❌ brew 未装, 无需卸载"
    exit 1
fi

# === 1. 参数 ===
KEEP_TAP=false
KEEP_DATA=false
FORCE=false
TAP="${APEIRETH_TAP:-apeireth/tap}"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --keep-tap)  KEEP_TAP=true; shift ;;
        --keep-data) KEEP_DATA=true; shift ;;
        --force)     FORCE=true; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# *//' | head -40
            exit 0
            ;;
        *) echo "❌ 未知参数: $1"; exit 1 ;;
    esac
done

echo "=== apeireth brew uninstall ==="
echo "    TAP:       ${TAP}"
echo "    KEEP_TAP:  ${KEEP_TAP}"
echo "    KEEP_DATA: ${KEEP_DATA}"
echo "    FORCE:     ${FORCE}"

# === 2. 检查是否装了 ===
if ! brew list apeireth >/dev/null 2>&1; then
    echo "❌ brew 未装 apeireth (brew list 无返回)"
    echo "   手动查: brew list | grep apeireth"
    exit 1
fi

# === 3. 确认 ===
if [[ "${FORCE}" != "true" ]]; then
    read -rp "确认卸载 apeireth (brew)? (y/N) " CONFIRM
    if [[ "${CONFIRM}" != "y" && "${CONFIRM}" != "Y" ]]; then
        echo "已取消"
        exit 0
    fi
fi

# === 4. brew services stop (per 蓝图 §3.7 step 3) ===
echo "[1/4] brew services stop..."
brew services stop apeireth 2>/dev/null || echo "    (服务未启动, 跳过 stop)"

# === 5. brew uninstall ===
echo "[2/4] brew uninstall apeireth..."
brew uninstall apeireth

# === 6. tap 清理 ===
if [[ "${KEEP_TAP}" != "true" ]]; then
    echo "[3/4] brew untap ${TAP}..."
    if brew tap | grep -q "^${TAP%%/*}/tap$"; then
        brew untap "${TAP}"
    else
        echo "    (tap 不存在, 跳过)"
    fi
else
    echo "[3/4] ⚠️  保留 tap (--keep-tap): ${TAP}"
fi

# === 7. 数据清理 ===
if [[ "${KEEP_DATA}" != "true" ]]; then
    echo "[4/4] drop data + log..."
    rm -rf "${HOME}/.apeireth" 2>/dev/null || true
    # Homebrew log
    BREW_PREFIX="$(brew --prefix)"
    rm -f "${BREW_PREFIX}/var/log/apeireth.log" 2>/dev/null || true
    rm -f "${BREW_PREFIX}/var/log/apeireth.err" 2>/dev/null || true
else
    echo "[4/4] ⚠️  保留数据 (--keep-data): ~/.apeireth"
fi

# === 8. 验证 0 残留 ===
RESIDUE=0
if brew list apeireth >/dev/null 2>&1; then
    echo "    ❌ brew list 仍有 apeireth"
    RESIDUE=$((RESIDUE + 1))
fi
if [[ -e "${HOME}/.apeireth" && "${KEEP_DATA}" != "true" ]]; then
    echo "    ❌ ~/.apeireth 残留"
    RESIDUE=$((RESIDUE + 1))
fi

if [[ ${RESIDUE} -eq 0 ]]; then
    echo "    ✅ 0 残留, 卸载完成"
    echo ""
    echo "重装: ./packaging/brew/install-brew.sh"
    exit 0
else
    echo ""
    echo "⚠️  ${RESIDUE} 项残留, 手动清: brew uninstall --force apeireth; rm -rf ~/.apeireth"
    exit 1
fi
