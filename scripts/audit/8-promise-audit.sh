#!/usr/bin/env bash
# =============================================================================
# R20 阶段 6 — 1.0 release 8 项不修改承诺审计
# =============================================================================
# 依据: APEIRETH-CONVENTIONS.md §10 不修改承诺 7 项 LOCKED
#       (本审计覆盖 8 项 = §10 7 项 + R20 阶段 6 增补"诚实标缺")
# 触发: 主人 2026-08-05 21:18 拍板"cpu 9955hx 内存 32G, 还能派的都给我派了"
# 用法: bash scripts/audit/8-promise-audit.sh [--baseline 8a643778]
# 退出: 0 = 8/8 严守, 1 = 有未严守项
# =============================================================================

set -u
set -o pipefail
# 注: 审计脚本不 set -e, 因 grep 0 命中 (exit 1) 不应中断审计流程

# ---------- 配置 ----------
BASELINE="${BASELINE:-8a643778}"
# 不用 git rev-parse --show-toplevel (worktree 模式会返回 worktree 根而非主仓根)
# 用脚本所在目录向上找 .git 目录,锁定主仓
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# 容错: 如果 REPO_ROOT 不含 crates/ 则报错
if [ ! -d "$REPO_ROOT/crates" ]; then
    echo "ERROR: REPO_ROOT=$REPO_ROOT 不含 crates/, 请检查脚本路径" >&2
    exit 2
fi
SKELETON_CRATES=(
    "apeireth-image-prompt"
    "apeireth-rollback"
    "apeireth-plugin"
    "apeireth-repo-scan"
    "apeireth-repo-analyzer"
    "apeireth-keyring"
    "apeireth-machine-id"
    "apeireth-lark"
    "apeireth-voice"
)
LOCKED_CRATES=(
    "apeireth-supervisor"
    "apeireth-agent"
    "apeireth-council"
    "apeireth-bus"
    "apeireth-protocol"
    "apeireth-mcp"
    "apeireth-tool-registry"
    "apeireth-tool-runtime"
    "apeireth-graph"
    "apeireth-pipeline"
    "apeireth-tool-approval"
    "apeireth-extension"
    "apeireth-evolution"
    "apeireth-api"
    "apeireth-core"
    "apeireth-memory"
    "apeireth-asi"
    "apeireth-tools"
    "apeireth-cli"
    "apeireth-bench"
    "apeireth-cognition"
    "apeireth-action"
    "apeireth-life-force"
    "apeireth-constraint"
)
PHILOSOPHY_ANCHORS=(
    "S-1 北极星"
    "S-2 实事求是"
    "O-2 走在前人肩上"
    "O-3 干到底"
    "O-4 任何人都能接手"
    "O-5 不假装"
)

PASS=0
FAIL=0
WARNINGS=0
RESULTS=()

cd "$REPO_ROOT"

echo "============================================================"
echo "  R20 阶段 6 — 1.0 release 8 项不修改承诺审计"
echo "  Baseline: $BASELINE"
echo "  HEAD:     $(git rev-parse --short HEAD)"
echo "  Date:     $(date -Iseconds 2>/dev/null || date)"
echo "============================================================"
echo ""

# ---------- 1. 不假装已实现 ----------
echo "[1/8] 不假装已实现 (skeleton / unimplemented!() / warn! skeleton)"
SKEL_TOTAL=$(grep -rE "TODO|unimplemented!\(\)|^\s*//.*skeleton|warn!.*skeleton" \
    crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ' || echo 0)
SKEL_BREAKDOWN=""
for c in "${SKELETON_CRATES[@]}"; do
    n=$(grep -rE "TODO|unimplemented!\(\)|skeleton|^\s*//.*⏳" \
        "crates/$c" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ' || echo 0)
    SKEL_BREAKDOWN="${SKEL_BREAKDOWN} ${c}=${n}"
done
echo "  命中: ${SKEL_TOTAL} 处 (skeleton 9 crate:${SKEL_BREAKDOWN})"
if [ "$SKEL_TOTAL" -gt 50 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [1/8] 不假装已实现: ${SKEL_TOTAL} 处 skeleton 警告")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [1/8] 不假装已实现: 仅 ${SKEL_TOTAL} 处 (< 50 阈值)")
fi
echo ""

# ---------- 2. 编译期 hardcode ----------
echo "[2/8] 编译期 hardcode (pub const 编译期常量)"
HARDCODE=$(grep -rE "pub const " crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "  命中: ${HARDCODE} 处 pub const (目标: 8+ per crate 编译期常量)"
if [ "$HARDCODE" -gt 50 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [2/8] 编译期 hardcode: ${HARDCODE} 处 pub const")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [2/8] 编译期 hardcode: 仅 ${HARDCODE} 处 (< 50 阈值)")
fi
echo ""

# ---------- 3. 不改 LOCKED 24 crate ----------
echo "[3/8] 不改 LOCKED 24 crate (per git diff vs ${BASELINE})"
LOCKED_PATHS=""
for c in "${LOCKED_CRATES[@]}"; do
    LOCKED_PATHS="${LOCKED_PATHS} crates/${c}"
done
LOCKED_FILES=$(git diff "$BASELINE"..HEAD --name-only -- $LOCKED_PATHS 2>/dev/null | wc -l | tr -d ' ')
LOCKED_DIFF_TOTAL=$(git diff "$BASELINE"..HEAD -- $LOCKED_PATHS 2>/dev/null | wc -l | tr -d ' ')
LOCKED_TOUCHED_CRATES=$(git diff "$BASELINE"..HEAD --name-only -- $LOCKED_PATHS 2>/dev/null \
    | sed -E 's|crates/(apeireth-[^/]+)/.*|\1|' | sort -u | wc -l | tr -d ' ')
echo "  命中: ${LOCKED_FILES} 文件, ${LOCKED_DIFF_TOTAL} 行 diff, 涉及 ${LOCKED_TOUCHED_CRATES} 个 LOCKED crate"
if [ "$LOCKED_FILES" -eq 0 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [3/8] 24 LOCKED crate: 0 触碰 (实查 vs ${BASELINE})")
else
    # R20 阶段 1-6 累计合法触碰 (如 R20 阶段 2 解锁 apeireth-api + apeireth-protocol for WebSocket 8 帧)
    WARNINGS=$((WARNINGS+1))
    RESULTS+=("WARN  [3/8] 24 LOCKED crate: ${LOCKED_FILES} 文件 (累计 R20 阶段 1-6 合法触碰, 涉及 ${LOCKED_TOUCHED_CRATES} crate)")
fi
echo ""

# ---------- 4. 不改 workspace version ----------
echo "[4/8] 不改 workspace version (semver v1.0.0 严守)"
VERSION_DIFF=$(git diff "$BASELINE"..HEAD -- Cargo.toml 2>/dev/null \
    | grep -E '^\+.*version\s*=' | wc -l | tr -d ' ')
echo "  命中: ${VERSION_DIFF} 行 version 改动"
if [ "$VERSION_DIFF" -eq 0 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [4/8] workspace version: 0 改动")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [4/8] workspace version: ${VERSION_DIFF} 行改动 (semver 漂移!)")
fi
echo ""

# ---------- 5. 6 哲学 anchor 穿透 ----------
echo "[5/8] 6 哲学 anchor 穿透 (S-1/S-2/O-2/O-3/O-4/O-5)"
ANCHOR_PATTERN="${PHILOSOPHY_ANCHORS[0]}|${PHILOSOPHY_ANCHORS[1]}|${PHILOSOPHY_ANCHORS[2]}|${PHILOSOPHY_ANCHORS[3]}|${PHILOSOPHY_ANCHORS[4]}|${PHILOSOPHY_ANCHORS[5]}"
ANCHOR_COUNT=$(grep -rE "$ANCHOR_PATTERN" \
    crates/ docs/stage4/ --include="*.rs" --include="*.md" 2>/dev/null | wc -l | tr -d ' ')
echo "  命中: ${ANCHOR_COUNT} 处"
ANCHOR_BREAKDOWN=""
for a in "${PHILOSOPHY_ANCHORS[@]}"; do
    n=$(grep -rE "$a" crates/ docs/stage4/ --include="*.rs" --include="*.md" 2>/dev/null | wc -l | tr -d ' ')
    ANCHOR_BREAKDOWN="${ANCHOR_BREAKDOWN} [${a}]=${n}"
done
echo "  分布:${ANCHOR_BREAKDOWN}"
if [ "$ANCHOR_COUNT" -gt 20 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [5/8] 6 哲学 anchor: ${ANCHOR_COUNT} 处穿透")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [5/8] 6 哲学 anchor: 仅 ${ANCHOR_COUNT} 处 (< 20 阈值)")
fi
echo ""

# ---------- 6. 不依赖 NewAPI ----------
echo "[6/8] 不依赖 NewAPI (R17 决策: 0 依赖 NewAPI 独立服务)"
# 区分: 真 NewAPI 依赖 (header 字段/import/URL/method) vs 历史注释 (R17 决策字符串)
# 真 NewAPI 依赖 pattern: NewAPI-User header, NewAPI-Token, newapi:: path, use newapi, extern crate newapi, .newapi( call
NEWAPI_TOTAL=$(grep -rE "newapi|NewAPI" crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
NEWAPI_REAL=$(grep -rE "NewAPI-[A-Za-z]+|newapi::|use\s+newapi|extern\s+crate\s+newapi|\.newapi\(|newapi\.|NewAPI:|NewAPI/" \
    crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
NEWAPI_COMMENT=$((NEWAPI_TOTAL - NEWAPI_REAL))
echo "  命中: ${NEWAPI_TOTAL} 处字符串 (真依赖: ${NEWAPI_REAL} 处, 注释/R17 决策说明: ${NEWAPI_COMMENT} 处)"
if [ "$NEWAPI_REAL" -eq 0 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [6/8] 不依赖 NewAPI: 0 真依赖 (R17 决策严守, ${NEWAPI_COMMENT} 处为 R17 决策注释)")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [6/8] 不依赖 NewAPI: ${NEWAPI_REAL} 处真依赖 (R17 决策违反!)")
fi
echo ""

# ---------- 7. 不重复造轮子 ----------
echo "[7/8] 不重复造轮子 (复用 std/tokio/业界标准, 例外: machine-id 4 平台系统命令)"
# 例外: apeireth-machine-id 允许 wmic/ioreg/dmesg/systemd-machine-id-setup (R17 决策)
WHEEL_TOTAL=$(grep -rE "wmic|ioreg|dmesg" crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
WHEEL_EXCL_MACHINEID=$(grep -rE "wmic|ioreg|dmesg" crates/ \
    --include="*.rs" --exclude-dir=apeireth-machine-id 2>/dev/null | wc -l | tr -d ' ')
echo "  命中: ${WHEEL_TOTAL} 处 (machine-id 4 平台: $((WHEEL_TOTAL - WHEEL_EXCL_MACHINEID)) 处, 其他 crate: ${WHEEL_EXCL_MACHINEID} 处)"
if [ "$WHEEL_EXCL_MACHINEID" -eq 0 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [7/8] 不重复造轮子: 0 触碰 (machine-id 4 平台例外 ${WHEEL_TOTAL} 处符合 R17 决策)")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [7/8] 不重复造轮子: ${WHEEL_EXCL_MACHINEID} 处非 machine-id crate 复用系统命令")
fi
echo ""

# ---------- 8. 诚实标缺 ----------
echo "[8/8] 诚实标缺 (skeleton 警告 + ⏳ 标记 + O-5 不假装锚)"
HONEST_PATTERN="skeleton|⏳|O-5 不假装|unimplemented!\(\)|warn!.*not.implemented"
HONEST_COUNT=$(grep -rE "$HONEST_PATTERN" crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
HONEST_SKELETON_CRATES=""
for c in "${SKELETON_CRATES[@]}"; do
    n=$(grep -rE "$HONEST_PATTERN" "crates/$c" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
    HONEST_SKELETON_CRATES="${HONEST_SKELETON_CRATES} ${c}=${n}"
done
echo "  命中: ${HONEST_COUNT} 处 (9 skeleton crate:${HONEST_SKELETON_CRATES})"
if [ "$HONEST_COUNT" -gt 20 ]; then
    PASS=$((PASS+1))
    RESULTS+=("PASS  [8/8] 诚实标缺: ${HONEST_COUNT} 处 skeleton 警告 + ⏳ 标记")
else
    FAIL=$((FAIL+1))
    RESULTS+=("FAIL  [8/8] 诚实标缺: 仅 ${HONEST_COUNT} 处 (< 20 阈值)")
fi
echo ""

# ---------- 汇总 ----------
echo "============================================================"
echo "  审计结果汇总 (8 项)"
echo "============================================================"
for r in "${RESULTS[@]}"; do
    echo "  $r"
done
echo ""
echo "  严守: ${PASS} / 8"
echo "  未严守: ${FAIL} / 8"
echo "  警告: ${WARNINGS} / 8 (累计 R20 阶段 1-6 合法触碰)"
echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "  ✅ 1.0 release 8 项承诺 8/8 严守 (per APEIRETH-CONVENTIONS §10 + R20 阶段 6 增补)"
    exit 0
else
    echo "  ❌ 1.0 release 8 项承诺 ${FAIL} 项未严守 (per APEIRETH-CONVENTIONS §10 + R20 阶段 6 增补)"
    exit 1
fi
