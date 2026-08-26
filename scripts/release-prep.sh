#!/usr/bin/env bash
# Apeireth release-prep — 切 release tag 前的全维度自检
#
# 把分散的 CI 守门 + 现有 checklist 串成一个本地可跑的脚本。
# 设计: 在 master HEAD 跑 = 模拟 "tag 触发 release-1.0.0.yml" 前的状态。
#
# 覆盖 3 个维度:
#   A. 8 硬墙守门 (per CONTRIBUTING.md §0)
#   B. PII 关键词 (per .github/workflows/pii-leak-detection.yml)
#   C. Release 12 项 checklist (复用 scripts/release-1.0-checklist.sh)
#
# 用法:
#   ./scripts/release-prep.sh               # 全跑, 1 P0 fail 阻塞 release
#   ./scripts/release-prep.sh --dry-run    #不阻塞, 全 PASS 报"可切 tag"
#   ./scripts/release-prep.sh --skip pii   #跳过 PII 段 (CI 已验)
#
# 输出: stdout + 可选 reports/release-prep-<date>.md
#
# 注: 跟 .github/workflows/release-1.0.0.yml 互补 (release pipeline 跑全平台
# build + 5/5 gate, 本脚本侧重"tag 前最后一道本地自检")

set -uo pipefail
cd "$(dirname "$0")/.."

VERSION="${APEIRETH_VERSION:-1.0.0}"
DRY_RUN=false
SKIP=""
if [[ "${1:-}" == "--dry-run" ]]; then DRY_RUN=true; fi
if [[ "${1:-}" == "--skip" ]]; then SKIP="${2:-}"; fi

DATE=$(date -u +%Y-%m-%d)
REPORT="reports/release-prep-${VERSION}-${DATE}.md"
mkdir -p reports

PASS=0
FAIL=0
SKIPPED=0
RESULTS=()

# === 8 硬墙守门 (per CONTRIBUTING.md §0) ===
check_hard_walls() {
    local section="A"
    echo "=================================================="
    echo "  [A] 8 硬墙守门 (per CONTRIBUTING.md §0)"
    echo "=================================================="

    # A1: workspace.version
    local CUR VER REASON
    CUR=$(grep -E '^version\s*=' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    REASON=""
    if [[ "$CUR" != "1.2.0" ]]; then
        REASON="workspace.version = $CUR (期望 1.2.0)"
    fi
    if [[ -n "$REASON" ]]; then
        fail "$section.1" "workspace.version 1.2.0 不变" "$REASON"
    else
        pass "$section.1" "workspace.version = $CUR"
    fi

    # A2: 0 触碰 24 LOCKED crate (per 8 硬墙 + CODEOWNERS 1:1)
    # 注: 之前漏 apeireth-graph, 修于 commit 71984e03 + 后续 hard-walls job 修
    local LOCKED="apeireth-supervisor apeireth-agent apeireth-council apeireth-bus apeireth-protocol apeireth-mcp apeireth-tool-registry apeireth-tool-runtime apeireth-graph apeireth-pipeline apeireth-tool-approval apeireth-extension apeireth-evolution apeireth-api apeireth-core apeireth-memory apeireth-asi apeireth-tools apeireth-cli apeireth-bench apeireth-cognition apeireth-action apeireth-life-force apeireth-constraint"
    local LOCKED_REGEX
    LOCKED_REGEX=$(echo "$LOCKED" | tr ' ' '|')
    local HITS
    HITS=$(git diff --name-only HEAD~1...HEAD -- 'crates/*.rs' 'crates/*/Cargo.toml' 2>/dev/null | grep -E "^crates/($LOCKED_REGEX)/" || true)
    if [[ -n "$HITS" ]]; then
        fail "$section.2" "0 触碰 24 LOCKED crate" "触碰: $HITS"
    else
        pass "$section.2" "0 触碰 24 LOCKED crate (master HEAD)"
    fi

    # A3: R11 baseline 3 值 (0.8682 / 0.8532 / 0.9063)
    # 实际定义在 legacy/donor/apeireth-asi/tests/integration_r_measure.rs (编译期 hardcode const)
    # 也出现在 legacy/donor/apeireth-blueprint-impl/src/r_measure.rs (RMeasureDelta 字段)
    local TARGET="legacy/donor/apeireth-asi/tests/integration_r_measure.rs"
    local MISS=""
    for VAL in 0.8682 0.8532 0.9063; do
        if ! grep -q "$VAL" "$TARGET" 2>/dev/null; then
            MISS="$MISS $VAL"
        fi
    done
    if [[ -n "$MISS" ]]; then
        fail "$section.3" "R11 baseline 3 值完整" "缺失:$MISS (在 $TARGET)"
    else
        pass "$section.3" "R11 baseline (0.8682/0.8532/0.9063) 在 $TARGET"
    fi

    # A4: companion-desktop 不进 root workspace
    if grep -q 'frontend/companion-desktop' Cargo.toml; then
        fail "$section.4" "companion-desktop 独立 workspace" "root Cargo 含 frontend/companion-desktop"
    else
        pass "$section.4" "companion-desktop 是独立 workspace"
    fi
}

# === PII 守门 (per .github/workflows/pii-leak-detection.yml) ===
check_pii() {
    local section="B"
    if [[ ",${SKIP}," == *",pii,"* ]]; then
        skip "$section" "PII 关键词 (CI 已验, --skip)"
        return
    fi
    echo "=================================================="
    echo "  [B] PII 关键词 (per pii-leak-detection.yml)"
    echo "=================================================="

    local PII_PATTERNS=(
        "警号" "警校" "东乡族" "甘肃农村" "甘肃养老"
        "31683" "东乡语" "治安学"
    )

    # 排除 self + 合法元提及 (CHANGELOG, ci-fix-log, release-prep.sh 自身含 PII_PATTERNS 定义)
    local EXCLUDES=(
        ':(exclude)docs/archive/**'
        ':(exclude)research/source/**'
        ':(exclude).git/**'
        ':(exclude)Cargo.lock'
        ':(exclude)frontend/companion-desktop/pnpm-lock.yaml'
        ':(exclude)frontend/companion-desktop/src-tauri/Cargo.lock'
        ':(exclude).github/workflows/pii-leak-detection.yml'
        ':(exclude)scripts/release-prep.sh'
        ':(exclude)CHANGELOG.md'
        ':(exclude)docs/04-internal/ci-fix-log-2026-08.md'
        ':(exclude)**/target/**'
        ':(exclude)**/node_modules/**'
        ':(exclude)**/dist/**'
        ':(exclude)**/build/**'
    )

    local HITS_TOTAL=0
    local HITS_DETAIL=""
    for PATTERN in "${PII_PATTERNS[@]}"; do
        local HITS
        HITS=$(git grep -lI "$PATTERN" -- . "${EXCLUDES[@]}" 2>/dev/null || true)
        if [[ -n "$HITS" ]]; then
            HITS_TOTAL=$((HITS_TOTAL + 1))
            HITS_DETAIL="${HITS_DETAIL}${PATTERN}: ${HITS}; "
        fi
    done

    if [[ $HITS_TOTAL -gt 0 ]]; then
        fail "$section" "PII 关键词 0 命中" "命中: $HITS_DETAIL"
    else
        pass "$section" "PII 0 命中 (8 关键词全过)"
    fi
}

# === Release 12 项 checklist (复用 scripts/release-1.0-checklist.sh) ===
check_release_checklist() {
    local section="C"
    if [[ ! -x "scripts/release-1.0-checklist.sh" ]]; then
        skip "$section" "scripts/release-1.0-checklist.sh 不存在或不可执行"
        return
    fi

    echo "=================================================="
    echo "  [C] Release 12 项 checklist (per 蓝图 §3.5)"
    echo "=================================================="

    # 跑 dry-run, 捕获输出, 看是否有真 FAIL
    # (排除 summary 行的 "0 PASS / 0 FAIL" 字串误判)
    local OUT
    OUT=$(bash scripts/release-1.0-checklist.sh --dry-run 2>&1 || true)
    local FAIL_COUNT
    FAIL_COUNT=$(echo "$OUT" | grep -E '^\s*❌' | wc -l)
    if [[ $FAIL_COUNT -gt 0 ]]; then
        fail "$section" "12 项 checklist (dry-run)" "$FAIL_COUNT 个 FAIL (跑 scripts/release-1.0-checklist.sh 详情)"
    else
        pass "$section" "12 项 checklist dry-run OK"
    fi
}

# === 工具函数 ===
pass() {
    local id="$1"; local title="$2"
    RESULTS+=("| ${id} | ${title} | ✅ PASS | — |")
    PASS=$((PASS+1))
    echo "  ✅ ${id}: ${title}"
}
fail() {
    local id="$1"; local title="$2"; local reason="$3"
    RESULTS+=("| ${id} | ${title} | ❌ FAIL | ${reason} |")
    FAIL=$((FAIL+1))
    echo "  ❌ ${id}: ${title} -- ${reason}"
}
skip() {
    local id="$1"; local reason="$2"
    RESULTS+=("| ${id} | (skipped) | ⏭ SKIP | ${reason} |")
    SKIPPED=$((SKIPPED+1))
    echo "  ⏭ ${id}: ${reason}"
}

# === 主流程 ===
echo "=================================================="
echo "  Apeireth release-prep v${VERSION}"
echo "  Date: ${DATE}"
echo "  Mode: $(if ${DRY_RUN}; then echo "DRY-RUN"; else echo "BLOCKING (1 P0 fail → exit 1)"; fi)"
echo "  Skip: ${SKIP:-none}"
echo "=================================================="
echo ""

check_hard_walls
echo ""
check_pii
echo ""
check_release_checklist
echo ""

# === 报告回写 ===
cat > "${REPORT}" <<EOF
# Apeireth release-prep — v${VERSION}

**Date**: ${DATE}
**HEAD**: \`$(git rev-parse --short HEAD)\`
**Mode**: $(if ${DRY_RUN}; then echo "DRY-RUN (1 fail 不阻塞)"; else echo "BLOCKING (1 fail → exit 1)"; fi)
**Skip**: ${SKIP:-none}

## 3 维度结果

| # | 类别 | 状态 | 备注 |
|---|------|------|------|
EOF
printf '%s\n' "${RESULTS[@]}" >> "${REPORT}"

cat >> "${REPORT}" <<EOF

## 汇总

- PASS:    ${PASS}
- FAIL:    ${FAIL}
- SKIPPED: ${SKIPPED}

## 切 tag 前清单 (per CONTRIBUTING.md)

- [ ] 3 维度全 PASS (本脚本)
- [ ] release-1.0-checklist.sh 12 项 PASS (含 docs/test/security/perf/observability/license)
- [ ] release-1.0.0.yml pipeline dispatch + 5/5 gate green
- [ ] companion-desktop-ci.yml (Tauri shell + pnpm svelte-check) green
- [ ] pii-leak-detection.yml daily cron PASS (next 24h 内会跑)
- [ ] git tag v${VERSION} -m 'release ${VERSION}'
- [ ] GitHub release page 写 release notes

EOF

echo "=================================================="
echo "  ${PASS} PASS / ${FAIL} FAIL / ${SKIPPED} SKIP"
echo "  Report: ${REPORT}"
echo "=================================================="

# BLOCKING mode: 任何 FAIL → exit 1
if [[ "${DRY_RUN}" == "false" ]]; then
    if [[ ${FAIL} -gt 0 ]]; then
        echo ""
        echo "❌ BLOCKING mode: ${FAIL} 个 FAIL, 阻塞 release tag"
        exit 1
    fi
fi

exit 0