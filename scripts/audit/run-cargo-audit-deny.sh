#!/usr/bin/env bash
# ============================================================================
# R20 阶段 6 — cargo audit + cargo deny 一键扫描 (1.0 release #3 security)
# ============================================================================
# 用途: 把 cargo-audit + cargo-deny 串联成可重复扫描, 任何 commit 跑一次
# 触发: 主人 2026-08-05 21:18 拍板"真派"
# 现实: cargo-audit 默认 exit 0 (没 vuln 时), 4 vuln 时 exit 1
#       cargo-deny 默认 exit 0 (全 pass 时), fail 时 exit 3
#       本脚本**不** --deny warnings, 让 CI 看实际告警
# 用法: bash scripts/audit/run-cargo-audit-deny.sh
# 退出: 0 = 全部 0 violation (极少), 1 = cargo audit 有 vuln, 3 = cargo deny fail
# ============================================================================

set -u
set -o pipefail

# 不 set -e: cargo-audit / cargo-deny 的非零退出码是**期望信号**, 不是错误

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
REPORTS_DIR="${REPO_ROOT}/reports"
DATE="$(date -u +%Y-%m-%d)"
mkdir -p "${REPORTS_DIR}"

cd "${REPO_ROOT}"

echo "============================================================"
echo "  R20 阶段 6 — cargo audit + cargo deny"
echo "  Repo:    ${REPO_ROOT}"
echo "  Date:    ${DATE}"
echo "  HEAD:    $(git rev-parse --short HEAD 2>/dev/null || echo 'no-git')"
echo "============================================================"
echo ""

# ---------- 1. cargo audit ----------
echo ">>> [1/2] cargo audit (RustSec advisory-db)"
ADVISORY_DB_PATH="${HOME}/.cargo/advisory-db"
if [ ! -d "${ADVISORY_DB_PATH}/.git" ] && [ -d "${ADVISORY_DB_PATH}-stale" ]; then
    echo "  [i] advisory-db stale, 切换到最新"
    if [ -d "${ADVISORY_DB_PATH}" ]; then
        mv "${ADVISORY_DB_PATH}/db.lock.stale" "${ADVISORY_DB_PATH}/db.lock.stale.bak" 2>/dev/null || true
    fi
fi

cargo audit --json > "${REPO_ROOT}/audit-report.json" 2> "${REPO_ROOT}/audit-audit.stderr.txt"
AUDIT_EXIT=$?
cargo audit 2>&1 | tee "${REPORTS_DIR}/r20-cargo-audit-stdout-${DATE}.txt" > /dev/null
echo "  exit=${AUDIT_EXIT} (0=clean, 1=vuln found, 2=advisory-db error)"
echo ""

# ---------- 2. cargo deny ----------
echo ">>> [2/2] cargo deny check (advisories + bans + licenses + sources)"
cargo deny check 2>&1 | tee "${REPORTS_DIR}/r20-cargo-deny-stdout-${DATE}.txt" > /dev/null
DENY_EXIT=$?
echo "  exit=${DENY_EXIT} (0=all pass, 3=advisories/bans/licenses/sources fail)"
echo ""

# ---------- 汇总 ----------
echo "============================================================"
echo "  扫描结果汇总"
echo "============================================================"
VULN_COUNT="$(grep -oE '"count":[0-9]+' "${REPO_ROOT}/audit-report.json" 2>/dev/null | head -1 | cut -d: -f2)"
DENY_LINE="$(tail -1 "${REPORTS_DIR}/r20-cargo-deny-stdout-${DATE}.txt" 2>/dev/null)"
echo "  cargo audit: ${VULN_COUNT:-?} vulnerabilities, exit=${AUDIT_EXIT}"
echo "  cargo deny:  ${DENY_LINE:-?}, exit=${DENY_EXIT}"
echo "  报告:        reports/r20-cargo-audit-stdout-${DATE}.txt"
echo "               reports/r20-cargo-deny-stdout-${DATE}.txt"
echo "  JSON:        audit-report.json"
echo "============================================================"

# 退出码: 保留更严重的退出码
if [ "${AUDIT_EXIT}" -ne 0 ]; then exit "${AUDIT_EXIT}"; fi
if [ "${DENY_EXIT}" -ne 0 ]; then exit "${DENY_EXIT}"; fi
exit 0
