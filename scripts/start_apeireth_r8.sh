#!/usr/bin/env bash
# Apeireth ASI R8 真生产一键启动脚本 (主哲学: 23:44 干到底 + 22:33 ASI 北极星 + 17:43 实事求是)
# 覆盖 R7 真实现阶段 (V1080-V1098): subprocess deploy / honest limits / codebase audit /
#                                  decision router / real LLM / HQB core / HQB persistence /
#                                  HQB Live Gate / e2e operator + 9 个 R8 真实现模块
#                                  (memory wal / replay / dream / dgm archive / memory schema /
#                                   r8 hero role / r8 learn / r8 evolve / persona prompts / dgm perf)
# 主哲学护栏: V3 哲学守门 + 4 层安全门 + 真生产不停 + 不绑单模型 + 不刷 KPI
# 上一篇 R7 启动脚本: start_apeireth.sh (Phase 0-21, 2026-07)
# 当前的: R8 真实现 (V1080-V1098)

set -uo pipefail  # 不 set -e; 我们要逐项报告健康状态

# ---------------------------------------------------------------------------
# 0. 路径 / 元信息
# ---------------------------------------------------------------------------
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
APEIRETH_HOME="$( cd "${SCRIPT_DIR}/.." && pwd )"
R8_TAG="apeireth-r8-launcher-2026-07-29"
LAUNCH_LOG="${APEIRETH_HOME}/logs/r8_start_${R8_TAG}_$(date +%Y%m%d_%H%M%S).log"

PYTHON_BIN="${PYTHON_BIN:-python}"
PY_ARGS=("-m")

mkdir -p "${APEIRETH_HOME}/logs"
mkdir -p "${APEIRETH_HOME}/data/identity"
mkdir -p "${APEIRETH_HOME}/data/memory"
mkdir -p "${APEIRETH_HOME}/data/graph"
mkdir -p "${APEIRETH_HOME}/data/skills"
mkdir -p "${APEIRETH_HOME}/data/dgm"
mkdir -p "${APEIRETH_HOME}/artifacts"

log() { printf '[%s] %s\n' "$(date '+%H:%M:%S')" "$*" | tee -a "${LAUNCH_LOG}"; }
ok()  { printf '  \033[32m✓\033[0m %s\n' "$*"; }
warn(){ printf '  \033[33m!\033[0m %s\n' "$*"; }
fail(){ printf '  \033[31m✗\033[0m %s\n' "$*"; }
hr()  { printf '%s\n' "------------------------------------------------------------"; }

hr
log "Apeireth ASI R8 真生产一键启动"
log "  R8_TAG=${R8_TAG}"
log "  APEIRETH_HOME=${APEIRETH_HOME}"
log "  Python: $($PYTHON_BIN --version 2>&1 | tr -d '\n')"
log "  Launch log: ${LAUNCH_LOG}"
hr

# ---------------------------------------------------------------------------
# 1. 环境探针
# ---------------------------------------------------------------------------
PYTHONPATH_ARG="PYTHONPATH=${APEIRETH_HOME}:${PYTHONPATH:-}"
export PYTHONPATH="${APEIRETH_HOME}:${PYTHONPATH:-}"

log "[1/9] 环境探针"

ENV_PROBE_OUT="$($PYTHON_BIN - <<'EOF' 2>&1)
import sys, os, platform
try:
    import yaml
    yaml_ok = True
except Exception:
    yaml_ok = False
try:
    import sqlite3
    sqlite_ok = True
except Exception:
    sqlite_ok = False
print(f"python={sys.version.split()[0]} os={platform.system()}-{platform.release()} "
      f"pyyaml={yaml_ok} sqlite={sqlite_ok} "
      f"cwd_ok={os.access(os.getcwd(), os.W_OK)}")
EOF
)"
echo "${ENV_PROBE_OUT}" | tee -a "${LAUNCH_LOG}"

API_KEY_PRESENT=0
if [ -n "${NEWAPI_API_KEY:-}" ] || [ -n "${MINIMAX_API_KEY:-}" ] || [ -n "${OPENAI_API_KEY:-}" ]; then
    API_KEY_PRESENT=1
fi
if [ "${API_KEY_PRESENT}" -eq 0 ]; then
    warn "未设 NEWAPI_API_KEY / MINIMAX_API_KEY / OPENAI_API_KEY — LLM 调用 fallback 到 template"
else
    ok "外部 LLM Key 已就绪"
fi

# ---------------------------------------------------------------------------
# 2. 模块清单自检 (V1080-V1088 + V1090-V1098, 共 16 个生产模块)
# ---------------------------------------------------------------------------
hr
log "[2/9] R8 模块清单自检 (V1080-V1088 + V1090-V1098)"

R8_MODULES=(
    "v1080_asi_real_subprocess_deploy"
    "v1080_asi_reproducibility"
    "v1081_asi_honest_limits"
    "v1082_asi_codebase_audit"
    "v1083_asi_decision_router"
    "v1084_asi_real_llm_inference"
    "v1085_hqb_core"
    "v1086_hqb_persistence"
    "v1087_asi_hqb_live_gate"
    "v1088_asi_e2e_operator"
    "v1090_memory_wal"
    "v1091_memory_replay"
    "v1092_memory_dream"
    "v1093_dgm_archive"
    "v1094_memory_schema"
    "v1096_persona_prompts"
    "v1098_dgm_perf"
)
PRESENT=0; MISSING=0; MISSING_LIST=()
for mod in "${R8_MODULES[@]}"; do
    if [ -f "${APEIRETH_HOME}/apeireth/${mod}.py" ]; then
        ok "apeireth/${mod}.py"
        PRESENT=$((PRESENT+1))
    else
        fail "apeireth/${mod}.py (missing)"
        MISSING=$((MISSING+1))
        MISSING_LIST+=("${mod}")
    fi
done
log "  R8 module presence: ${PRESENT}/${#R8_MODULES[@]} present, ${MISSING} missing"
if [ "${MISSING}" -gt 0 ]; then
    warn "Missing modules: ${MISSING_LIST[*]}"
fi

# ---------------------------------------------------------------------------
# 3. V1081 honest-limits + V3 哲学 4 层守门 (L1/L3)
# ---------------------------------------------------------------------------
hr
log "[3/9] V1081 honest-limits + V3 哲学 4 层守门 (L1/L3)"

V1081_HEALTH=$($PYTHON_BIN -m apeireth.v1081_asi_honest_limits --probe --report 2>&1 | tail -50)
echo "${V1081_HEALTH}" | tail -20 | tee -a "${LAUNCH_LOG}"
if echo "${V1081_HEALTH}" | grep -qiE "ALL OK\s*[:=]\s*TRUE|GUARD.*PASS|philosophy.*PASS"; then
    ok "V1081 V3 哲学 4 层守门 (L1/L3) PASS"
else
    warn "V1081 V3 哲学 4 层守门 (L1/L3) — 检查 ${LAUNCH_LOG}"
fi

# ---------------------------------------------------------------------------
# 4. V1082 codebase audit (R7 backlog + module presence)
# ---------------------------------------------------------------------------
hr
log "[4/9] V1082 codebase audit --audit (无 --lift, 避免误改 inventory)"

V1082_AUDIT=$($PYTHON_BIN -m apeireth.v1082_asi_codebase_audit --audit 2>&1 | tail -30)
echo "${V1082_AUDIT}" | tail -15 | tee -a "${LAUNCH_LOG}"
log "  V1082 audit 跑完; lift 留给 reviewer 单独决定"

# ---------------------------------------------------------------------------
# 5. V1085 HQB core quick health (L4 安全门)
# ---------------------------------------------------------------------------
hr
log "[5/9] V1085 HQB 核心 — 真生产 4 维 (SC/NR/EV/CDT) quick health"

HQB_HEALTH=$($PYTHON_BIN - <<'EOF' 2>&1
import sys
from apeireth.v1085_hqb_core import (
    HonestDecisionModule, DecisionContext, Verdict
)
m = HonestDecisionModule()
ctx = DecisionContext(
    task="r8_launch_healthcheck",
    latency_ms=80,
    cost=0.002,
    scope="read_only",
    phi_proxy=0.5,
)
v = m.evaluate(ctx)
print(f"verdict={v.verdict.value} confidence={v.confidence:.3f} reason={v.reason[:80]}")
EOF
)
echo "${HQB_HEALTH}" | tee -a "${LAUNCH_LOG}"
if echo "${HQB_HEALTH}" | grep -qE "verdict=(ALLOW|WARN)"; then
    ok "HQB core 真生产可用"
else
    warn "HQB core — review logs"
fi

# ---------------------------------------------------------------------------
# 6. V1087 HQB Live Gate (L4 PDP-PEP 门序)
# ---------------------------------------------------------------------------
hr
log "[6/9] V1087 HQB Live Gate — 一行命令 self-check"
$PYTHON_BIN -m apeireth.v1087_asi_hqb_live_gate --self-check 2>&1 | tee -a "${LAUNCH_LOG}" | tail -10 || warn "V1087 self-check 返回非 0 — 检查 HQB 状态"

# ---------------------------------------------------------------------------
# 7. Integration Worktree 健康 (R7 §技术债 #4 验证)
# ---------------------------------------------------------------------------
hr
log "[7/9] Integration Worktree 健康 (R7 §技术债 #4 验证)"

WORKTREE_DIR="${APEIRETH_HOME}/.spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5"
WORKTREE_STATE=0
if [ -d "${WORKTREE_DIR}" ]; then
    HEAD_ON_WT="$(cd "${WORKTREE_DIR}" && git rev-parse HEAD 2>&1)"
    BRANCH_ON_WT="$(cd "${WORKTREE_DIR}" && git symbolic-ref --short HEAD 2>&1)"
    if [ -n "${HEAD_ON_WT}" ] && [ -n "${BRANCH_ON_WT}" ]; then
        ok "Integration worktree 已 init: branch=${BRANCH_ON_WT} head=${HEAD_ON_WT:0:12}"
        WORKTREE_STATE=1
    else
        fail "Integration worktree 目录存在但 git 操作异常"
    fi
else
    fail "Integration worktree 不存在 — R7 §技术债 #4 切实存在，需手动初始化"
fi

log "  WORKTREE_HEALTHY=${WORKTREE_STATE}"

# ---------------------------------------------------------------------------
# 8. 关键子命令注册 (供运维/CI 调用)
# ---------------------------------------------------------------------------
hr
log "[8/9] R8 关键子命令注册表"
cat <<'EOF'
  ┌────────────────────────────────────────────────────────────────────┐
  │ R8 CLI 一键命令 (供运维 / CI / Leader 调度)                     │
  ├────────────────────────────────────────────────────────────────────┤
EOF
for mod in "${R8_MODULES[@]}"; do
    printf "  │ %-50s --help             │\n" "-m apeireth.${mod}"
done
cat <<'EOF'
  ├────────────────────────────────────────────────────────────────────┤
  │ 推荐调用顺序 (按 R7 启动 5 步改造):                              │
  │   1. v1081_asi_honest_limits --probe        (L1/L3 守门)         │
  │   2. v1082_asi_codebase_audit --audit       (backlog 扫描)       │
  │   3. v1083_asi_decision_router --route      (router 自检)        │
  │   4. v1084_asi_real_llm_inference --ping    (LLM ping, 可选)    │
  │   5. v1085_hqb_core --eval self-test        (HQB core)          │
  │   6. v1087_asi_hqb_live_gate --self-check   (Live Gate)         │
  │   7. v1088_asi_e2e_operator --smoke         (e2e operator)      │
  │   8. v1090_memory_wal --smoke              (Memory WAL)         │
  │   9. v1091_memory_replay --smoke           (Memory Replay)      │
  │  10. v1092_memory_dream --smoke            (Memory Dream)       │
  │  11. v1075_asi_real_deployment_run --run    (真部署)            │
  │  12. v1074_asi_production_runner --report   (ASI V0.3 真测量)    │
  └────────────────────────────────────────────────────────────────────┘
EOF

# ---------------------------------------------------------------------------
# 9. 启动汇总 + 返回码 (供 CI 调用时依据)
# ---------------------------------------------------------------------------
hr
log "[9/9] R8 启动汇总"

SUMMARY=$(cat <<EOF
R8_TAG=${R8_TAG}
APEIRETH_HOME=${APEIRETH_HOME}
PYTHON=$($PYTHON_BIN --version 2>&1 | tr -d '\n')
MODULES_PRESENT=${PRESENT}/${#R8_MODULES[@]}
MODULES_MISSING=${MISSING}
WORKTREE_HEALTHY=${WORKTREE_STATE}
LLM_KEY_PRESENT=${API_KEY_PRESENT}
EOF
)
echo "${SUMMARY}" | tee -a "${LAUNCH_LOG}"

hr
log "Apeireth ASI R8 基座已启动"
log "  V1080-V1088: 集成工程闭环 (复现→边界→审计→路由→推理→HQB核→HQB持→e2e)"
log "  V1090-V1098: R7 真实现 Phase-1 (WAL→Replay→Dream) + DGM perf"
log "  下一步: bash scripts/test_r8_deployment.sh 跑 ≥15 个部署测试"
hr

# 退出码: 必有 module 缺失或 worktree 不健康则非 0
if [ "${PRESENT}" -lt "${#R8_MODULES[@]}" ] || [ "${WORKTREE_STATE}" -ne 1 ]; then
    exit 2
fi
exit 0
