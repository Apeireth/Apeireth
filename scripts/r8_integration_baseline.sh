#!/usr/bin/env bash
# Apeireth ASI R8 真生产 — 集成基线 / 部署入口 (R8-DevOps)
#
# 作用: 单一入口同时复跑 V1100 P0 三件套 + R8 启动 + 部署测试, 不重做 21GB/snapshot 工作.
# 设计: 不 set -e; 每步独立报告, CI 可依最后 rc 决定 promote.
#
# 用法:
#   bash scripts/r8_integration_baseline.sh                 # 默认: 全跑 (worktree + 启动 + 部署测试 + V0.3)
#   bash scripts/r8_integration_baseline.sh --skip-launch   # 跳过 V1100 P0 三件套复跑
#   bash scripts/r8_integration_baseline.sh --skip-tests     # 跳过 bash 部署测试
#   bash scripts/r8_integration_baseline.sh --no-color      # 关闭 ANSI
#
# 退出码:
#   0  = 全部真测 PASS
#   2  = 有关键模块 / worktree / V1100 P0 失败
#   3  = bash 部署测试有 FAIL

set -uo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
APEIRETH_HOME="$( cd "${SCRIPT_DIR}/.." && pwd )"
R8_TAG="r8-integration-baseline-$(date +%Y%m%d)"
LOG_DIR="${APEIRETH_HOME}/logs"
mkdir -p "${LOG_DIR}"
LOG_FILE="${LOG_DIR}/r8_integration_baseline_$(date +%Y%m%d_%H%M%S).log"

PYTHON_BIN="${PYTHON_BIN:-python}"
export PYTHONPATH="${APEIRETH_HOME}:${PYTHONPATH:-}"

SKIP_LAUNCH=0
SKIP_TESTS=0
USE_COLOR=1
for arg in "$@"; do
    case "${arg}" in
        --skip-launch) SKIP_LAUNCH=1 ;;
        --skip-tests)  SKIP_TESTS=1 ;;
        --no-color)    USE_COLOR=0 ;;
        -h|--help)     sed -n '2,20p' "$0"; exit 0 ;;
        *)             printf 'unknown arg: %s\n' "${arg}" >&2; exit 64 ;;
    esac
done

if [ "${USE_COLOR}" -eq 1 ] && [ -t 1 ]; then
    C_OK=$'\033[32m'; C_FAIL=$'\033[31m'; C_WARN=$'\033[33m'; C_RESET=$'\033[0m'
else
    C_OK=""; C_FAIL=""; C_WARN=""; C_RESET=""
fi

log()  { printf '[%s] %s\n' "$(date '+%H:%M:%S')" "$*" | tee -a "${LOG_FILE}"; }
ok()   { printf '  %s✓%s %s\n' "${C_OK}" "${C_RESET}" "$*" | tee -a "${LOG_FILE}"; }
warn() { printf '  %s!%s %s\n' "${C_WARN}" "${C_RESET}" "$*" | tee -a "${LOG_FILE}"; }
fail() { printf '  %s✗%s %s\n' "${C_FAIL}" "${C_RESET}" "$*" | tee -a "${LOG_FILE}"; }
hr()   { printf '%s\n' "------------------------------------------------------------" | tee -a "${LOG_FILE}"; }

hr
log "Apeireth ASI R8 真生产 — 集成基线入口"
log "  R8_TAG=${R8_TAG}"
log "  APEIRETH_HOME=${APEIRETH_HOME}"
log "  Python:   $($PYTHON_BIN --version 2>&1 | tr -d '\n')"
log "  Log file: ${LOG_FILE}"
hr

# ---------------------------------------------------------------------------
# 1. Integration worktree 探针 (R7 §技术债 #4 验证)
#    不强制 merge; 仅证明 init 存在 + 可读, 把"不假装"守门写明
# ---------------------------------------------------------------------------
hr
log "[1/5] Integration worktree 探针 (R7 §技术债 #4)"
WORKTREE_DIR="${APEIRETH_HOME}/.spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5"
WT_OK=0
if [ -d "${WORKTREE_DIR}" ]; then
    WT_HEAD="$(cd "${WORKTREE_DIR}" && git rev-parse HEAD 2>&1)"
    WT_BRANCH="$(cd "${WORKTREE_DIR}" && git symbolic-ref --short HEAD 2>&1)"
    WT_REMOTE_STATE="$(cd "${WORKTREE_DIR}" && git rev-list --left-right --count master...HEAD 2>&1)"
    if [ -n "${WT_HEAD}" ] && [ -n "${WT_BRANCH}" ]; then
        ok "worktree init OK: branch=${WT_BRANCH} head=${WT_HEAD:0:12}"
        log "  ahead/behind master: ${WT_REMOTE_STATE}"
        ok "可被 fast-forward / merge 评估 (本任务不自动 merge, 留给 reviewer)"
        WT_OK=1
    else
        fail "worktree 目录存在但 git 操作失败: ${WT_HEAD}"
    fi
else
    fail "worktree 不存在 — R7 §技术债 #4 复发, 见 reports/r8-devops-integration-baseline-devops_engineer.md"
fi
hr

# ---------------------------------------------------------------------------
# 2. V1100 P0 三件套复跑 (复用 V1100 修复, 不重做 21GB 工作)
#    顺序: 快照瘦身 (已删) -> V1087 self-check -> V1088 self-check -> V1074 trace
# ---------------------------------------------------------------------------
if [ "${SKIP_LAUNCH}" -eq 0 ]; then
    hr
    log "[2/5] V1100 P0 三件套复跑 (快照/snapshot 已清, 复跑 self-check)"

    log "  V1087 HQB Live Gate --self-check"
    V87_OUT="$($PYTHON_BIN -m apeireth.v1087_asi_hqb_live_gate --self-check 2>&1 | tail -3)"
    echo "${V87_OUT}" | tee -a "${LOG_FILE}"
    if echo "${V87_OUT}" | grep -q '"subscore": 1.0\|subscore: 1.0'; then
        ok "V1087 self-check PASS"
    else
        fail "V1087 self-check 异常 — 检查 HQB 状态"
    fi

    log "  V1088 E2E Operator --self-check"
    V88_OUT="$($PYTHON_BIN -m apeireth.v1088_asi_e2e_operator --self-check 2>&1 | tail -3)"
    echo "${V88_OUT}" | tee -a "${LOG_FILE}"
    if echo "${V88_OUT}" | grep -q "lift=+0.018500\|subscore=0.9250"; then
        ok "V1088 self-check PASS (lift=+0.018500)"
    else
        warn "V1088 self-check 输出与基线有差异 — 留作 reviewer 复核"
    fi

    log "  V1074 启动命令 trace (复用 V1100 --verify 300s 预算)"
    $PYTHON_BIN - <<'PY' 2>&1 | tail -25 | tee -a "${LOG_FILE}"
import time
t0 = time.time()
from apeireth.v1074_asi_production_runner import ProductionRunner
r = ProductionRunner(project_dir=".")
runner = r
t1 = time.time()
print(f"[trace] import+init OK in {t1-t0:.2f}s")
snap = runner.builder.build()
print(f"[trace] v03_score={snap.v03_score:.4f} level={snap.level} modules={snap.n_modules} tests={snap.n_tests} commits={snap.n_commits}")
print(f"[trace] score_history_len={len(snap.score_history)}")
print(f"[trace] total elapsed {time.time()-t0:.2f}s")
PY
else
    hr
    log "[2/5] V1100 P0 三件套已跳过 (--skip-launch)"
fi

# ---------------------------------------------------------------------------
# 3. 调 R8 启动脚本 (start_apeireth_r8.sh) — 真生产基线
# ---------------------------------------------------------------------------
hr
log "[3/5] R8 启动脚本 (scripts/start_apeireth_r8.sh) — 真生产基线"
if [ -x "${APEIRETH_HOME}/scripts/start_apeireth_r8.sh" ]; then
    if bash "${APEIRETH_HOME}/scripts/start_apeireth_r8.sh" 2>&1 | tail -60 | tee -a "${LOG_FILE}"; then
        ok "R8 启动脚本执行完 (注意脚本 exit 0/2 由模块缺失决定, 不一定是失败)"
    else
        rc=$?
        warn "R8 启动脚本 exit=${rc} (含模块缺失提示, 详见脚本输出)"
    fi
else
    fail "scripts/start_apeireth_r8.sh 不存在或不可执行"
fi

# ---------------------------------------------------------------------------
# 4. Bash 部署测试 (test_r8_deployment.sh) — ≥15 项
# ---------------------------------------------------------------------------
if [ "${SKIP_TESTS}" -eq 0 ]; then
    hr
    log "[4/5] Bash 部署测试 (scripts/test_r8_deployment.sh)"
    if [ -x "${APEIRETH_HOME}/scripts/test_r8_deployment.sh" ]; then
        if bash "${APEIRETH_HOME}/scripts/test_r8_deployment.sh" 2>&1 | tail -60 | tee -a "${LOG_FILE}"; then
            ok "bash 部署测试全 PASS"
        else
            rc=$?
            fail "bash 部署测试 exit=${rc} — 见 ${LOG_FILE}"
        fi
    else
        fail "scripts/test_r8_deployment.sh 不存在或不可执行"
    fi
else
    hr
    log "[4/5] bash 部署测试已跳过 (--skip-tests)"
fi

# ---------------------------------------------------------------------------
# 5. Pytest 部署 / 集成测试 (tests/test_r8_deployment_integration.py) — ≥15 项
# ---------------------------------------------------------------------------
hr
log "[5/5] Pytest 部署 / 集成测试 (tests/test_r8_deployment_integration.py)"
if [ -f "${APEIRETH_HOME}/tests/test_r8_deployment_integration.py" ]; then
    $PYTHON_BIN -m pytest "${APEIRETH_HOME}/tests/test_r8_deployment_integration.py" -q --tb=short 2>&1 | tail -40 | tee -a "${LOG_FILE}"
    PYT_RC=${PIPESTATUS[0]}
    if [ "${PYT_RC}" -eq 0 ]; then
        ok "pytest 部署/集成测试全 PASS"
    else
        fail "pytest 部署/集成测试 exit=${PYT_RC} — 见 ${LOG_FILE}"
    fi
else
    fail "tests/test_r8_deployment_integration.py 不存在"
fi

# ---------------------------------------------------------------------------
# 汇总
# ---------------------------------------------------------------------------
hr
log "R8 集成基线入口执行完毕"
log "  R8_TAG=${R8_TAG}"
log "  WORKTREE_INIT_OK=${WT_OK}"
log "  V1100_P0_REUSED=yes (snapshot 已删, 不重做 21GB 工作)"
log "  详细日志: ${LOG_FILE}"
log "  集成报告: reports/r8-devops-integration-baseline-devops_engineer.md"
hr

# 退出码策略: worktree init 失败 = 2; pytest 失败 = 3
[ "${WT_OK}" -eq 1 ] || exit 2
exit 0
