#!/usr/bin/env bash
# ============================================================================
# scripts/vet.sh — TP20-S5 塞缝批
# ----------------------------------------------------------------------------
# 三件套: cargo vet + cargo audit + cargo deny, 串成发布前供应链验证。
#
# 触发: release tag push / `make audit` / 主人手动 `bash scripts/vet.sh`
#
# 退出码:
#   0  = 三件套全 pass (实际极少, 通常至少 cargo-audit 有 known 项)
#   1  = cargo vet 失败
#   2  = cargo audit 失败 (有未处理 vuln 或 advisory-db error)
#   3  = cargo deny 失败 (advisories/bans/licenses/sources)
#
# 工具链要求 (0 装 PASS 边界):
#   - cargo (Rust stable, per rust-toolchain.toml)
#   - cargo-audit    : 已装在 ~/.cargo/bin, 若缺 → 此步 SKIP, exit 0
#   - cargo-deny     : 已装在 ~/.cargo/bin, 若缺 → 此步 SKIP, exit 0
#   - cargo-vet      : 可选 (TP20-S5 后置); 若缺 → 此步 SKIP, exit 0, 但
#                      CI release.yml 的 security-and-sbom job 必须装
#
# 安装 fallback (主人拍板, 工具失败时文档标注):
#   cargo install cargo-vet   --locked --root tools/
#   cargo install cargo-audit --locked
#   cargo install cargo-deny  --locked
#   cargo install cargo-cyclonedx --locked
#
# 哲学锚点:
#   - 机制而非补丁: 把供应链验证做成 first-class 脚本, 而不是临时人肉跑
#   - 集成而非分立: vet + audit + deny 三件套共享 workspace, 一次 cargo
#     metadata 解析就够, 不重复 fetch crates.io index
#   - 安全底线: 任一失败硬阻断 (--no-fail-on-warnings 不传), audit known
#     项时主人需在 audit-report.json 里手动 ack, 然后再跑 (不静默放行)
#
# 与 scripts/audit/run-cargo-audit-deny.sh 的关系:
#   - 老脚本 (R20 阶段 6): 只 audit + deny, 已入库, 保留不动 (向后兼容)
#   - 本脚本 (TP20-S5): vet + audit + deny, 增量, 8 包 release 全跑
#   - Makefile 的 `make audit` target 调本脚本
#
# 用法:
#   bash scripts/vet.sh                  # 三件套全跑 (release gate)
#   APEIRETH_FAST=1 bash scripts/vet.sh  # 跳过 audit (本地快速, CI 必跑)
# ============================================================================

set -u
set -o pipefail

# 不 set -e: vet/audit/deny 的非零退出码是**期望信号**, 不是错误

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOLS_DIR="${REPO_ROOT}/tools"
REPORTS_DIR="${REPO_ROOT}/reports"
DATE="$(date -u +%Y-%m-%d)"
mkdir -p "${REPORTS_DIR}"

cd "${REPO_ROOT}"

# 把 tools/bin 加入 PATH (cargo install --root tools/ 时路径约定)
if [[ -d "${TOOLS_DIR}/bin" ]]; then
    export PATH="${TOOLS_DIR}/bin:${PATH}"
fi

echo "============================================================"
echo "  TP20-S5 — cargo vet + audit + deny"
echo "  Repo:   ${REPO_ROOT}"
echo "  Date:   ${DATE}"
echo "  HEAD:   $(git rev-parse --short HEAD 2>/dev/null || echo 'no-git')"
echo "============================================================"
echo ""

VET_EXIT=0
AUDIT_EXIT=0
DENY_EXIT=0

# ----------------------------------------------------------------------------
# [1/3] cargo vet — 第三方依赖审计 (Mozilla cargo-vet 规范)
# ----------------------------------------------------------------------------
echo ">>> [1/3] cargo vet (.cargo/vet.toml)"
if command -v cargo-vet >/dev/null 2>&1; then
    cargo vet 2>&1 | tee "${REPORTS_DIR}/tp20-s5-cargo-vet-stdout-${DATE}.txt" > /dev/null
    VET_EXIT=${PIPESTATUS[0]}
    echo "  exit=${VET_EXIT} (0=all pass, 1=unvetted dependency)"
elif [[ -x "${TOOLS_DIR}/bin/cargo-vet.exe" ]] || [[ -x "${TOOLS_DIR}/bin/cargo-vet" ]]; then
    "${TOOLS_DIR}/bin/cargo-vet" vet 2>&1 | tee "${REPORTS_DIR}/tp20-s5-cargo-vet-stdout-${DATE}.txt" > /dev/null
    VET_EXIT=${PIPESTATUS[0]}
    echo "  exit=${VET_EXIT} (local tools/)"
else
    echo "  ⚠️  cargo-vet 未装, SKIP 这一步 (CI release.yml 必须装)"
    echo "  fallback install: cargo install cargo-vet --locked --root ${TOOLS_DIR}"
    VET_EXIT=0  # 本地 SKIP 不算失败; release gate 由 CI 守门
fi
echo ""

# ----------------------------------------------------------------------------
# [2/3] cargo audit — RustSec advisory-db 离线扫描
# ----------------------------------------------------------------------------
if [[ "${APEIRETH_FAST:-0}" != "1" ]]; then
    echo ">>> [2/3] cargo audit (RustSec advisory-db)"
    if command -v cargo-audit >/dev/null 2>&1; then
        cargo audit --json > "${REPO_ROOT}/audit-report.json" 2> "${REPO_ROOT}/audit-audit.stderr.txt"
        AUDIT_EXIT=$?
        cargo audit 2>&1 | tee "${REPORTS_DIR}/tp20-s5-cargo-audit-stdout-${DATE}.txt" > /dev/null
        echo "  exit=${AUDIT_EXIT} (0=clean, 1=vuln found, 2=advisory-db error)"
    else
        echo "  ❌ cargo-audit 未装 (TP20-S5 必须装, 不 fallback)"
        AUDIT_EXIT=2
    fi
    echo ""
else
    echo ">>> [2/3] cargo audit SKIP (APEIRETH_FAST=1)"
    echo ""
fi

# ----------------------------------------------------------------------------
# [3/3] cargo deny — advisories + bans + licenses + sources
# ----------------------------------------------------------------------------
echo ">>> [3/3] cargo deny check (deny.toml)"
if command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check 2>&1 | tee "${REPORTS_DIR}/tp20-s5-cargo-deny-stdout-${DATE}.txt" > /dev/null
    DENY_EXIT=${PIPESTATUS[0]}
    echo "  exit=${DENY_EXIT} (0=all pass, 3=fail)"
else
    echo "  ❌ cargo-deny 未装 (TP20-S5 必须装, 不 fallback)"
    DENY_EXIT=3
fi
echo ""

# ----------------------------------------------------------------------------
# 汇总
# ----------------------------------------------------------------------------
echo "============================================================"
echo "  TP20-S5 三件套汇总"
echo "============================================================"
VULN_COUNT="$(grep -oE '"count":[0-9]+' "${REPO_ROOT}/audit-report.json" 2>/dev/null | head -1 | cut -d: -f2)"
DENY_LINE="$(tail -1 "${REPORTS_DIR}/tp20-s5-cargo-deny-stdout-${DATE}.txt" 2>/dev/null)"
echo "  cargo vet:   exit=${VET_EXIT}"
echo "  cargo audit: ${VULN_COUNT:-?} vulnerabilities, exit=${AUDIT_EXIT}"
echo "  cargo deny:  ${DENY_LINE:-?}, exit=${DENY_EXIT}"
echo "  报告:        reports/tp20-s5-cargo-{vet,audit,deny}-stdout-${DATE}.txt"
echo "  JSON:        audit-report.json"
echo "============================================================"

# 退出码: 取最严重
if [[ "${AUDIT_EXIT}" -ne 0 ]]; then exit "${AUDIT_EXIT}"; fi
if [[ "${DENY_EXIT}" -ne 0 ]]; then exit "${DENY_EXIT}"; fi
if [[ "${VET_EXIT}" -ne 0 ]]; then exit "${VET_EXIT}"; fi
exit 0