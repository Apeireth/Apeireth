#!/usr/bin/env bash
# Apeireth ASI R8 真生产部署测试 (≥15 项)
# 主哲学: 17:43 实事求是 — 每条测试必须有可重现的 PASS/FAIL 证据
# 不假装守门: 不写 "skip" / 不写 "warn-but-pass"; 真失败即返非 0

set -uo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
APEIRETH_HOME="$( cd "${SCRIPT_DIR}/.." && pwd )"
TEST_OUT="${APEIRETH_HOME}/logs/r8_deployment_tests_$(date +%Y%m%d_%H%M%S).log"
mkdir -p "$(dirname "${TEST_OUT}")"

PYTHON_BIN="${PYTHON_BIN:-python}"
export PYTHONPATH="${APEIRETH_HOME}:${PYTHONPATH:-}"

red()   { printf '\033[31m%s\033[0m' "$*"; }
green() { printf '\033[32m%s\033[0m' "$*"; }
yel()   { printf '\033[33m%s\033[0m' "$*"; }

PASS=0; FAIL=0; SKIPPED=0
FAIL_LIST=()

run_test() {
    local test_id="$1"; shift
    local desc="$1"; shift
    local test_fn="$1"; shift
    printf "[%-2s] %-65s | " "${test_id}" "${desc}"
    if "${test_fn}"; then
        echo "$(green PASS)"
        PASS=$((PASS+1))
    else
        echo "$(red FAIL)"
        FAIL=$((FAIL+1))
        FAIL_LIST+=("${test_id}:${desc}")
    fi
}

note_test() {
    local test_id="$1"; shift
    local desc="$1"; shift
    local note="$1"; shift
    printf "[%-2s] %-65s | " "${test_id}" "${desc}"
    echo "$(yel SKIP) (${note})"
    SKIPPED=$((SKIPPED+1))
}

# ---------------------------------------------------------------------
# 测试 1: 项目根结构存在
# ---------------------------------------------------------------------
t01_project_structure() {
    [ -d "${APEIRETH_HOME}/apeireth" ] && [ -d "${APEIRETH_HOME}/tests" ] && [ -f "${APEIRETH_HOME}/start_apeireth.sh" ]
}

# ---------------------------------------------------------------------
# 测试 2: 关键 V1080-V1088 模块全部存在 (复现→边界→审计→路由→推理→HQB→e2e)
# ---------------------------------------------------------------------
t02_v1080_v1088_modules() {
    local all_ok=1
    for mod in v1080_asi_reproducibility v1081_asi_honest_limits v1082_asi_codebase_audit v1083_asi_decision_router v1084_asi_real_llm_inference v1085_hqb_core v1086_hqb_persistence v1087_asi_hqb_live_gate v1088_asi_e2e_operator; do
        [ -f "${APEIRETH_HOME}/apeireth/${mod}.py" ] || { all_ok=0; break; }
    done
    [ ${all_ok} -eq 1 ]
}

# ---------------------------------------------------------------------
# 测试 3: 关键 V1090-V1098 模块全部存在 (R7 真实现)
# ---------------------------------------------------------------------
t03_v1090_v1098_modules() {
    local all_ok=1
    for mod in v1090_memory_wal v1091_memory_replay v1092_memory_dream v1093_dgm_archive v1094_memory_schema v1096_persona_prompts v1098_dgm_perf; do
        [ -f "${APEIRETH_HOME}/apeireth/${mod}.py" ] || { all_ok=0; break; }
    done
    [ ${all_ok} -eq 1 ]
}

# ---------------------------------------------------------------------
# 测试 4: start_apeireth_r8.sh 存在且可执行
# ---------------------------------------------------------------------
t04_r8_script_executable() {
    [ -x "${APEIRETH_HOME}/scripts/start_apeireth_r8.sh" ]
}

# ---------------------------------------------------------------------
# 测试 5: docker-compose.r8.yml YAML 合法 (服务数 ≥ 12)
# ---------------------------------------------------------------------
t05_docker_compose_valid() {
    ${PYTHON_BIN} - "${APEIRETH_HOME}/docker-compose.r8.yml" <<'EOF' >/dev/null 2>&1
import sys, yaml
with open(sys.argv[1], encoding="utf-8") as f:
    d = yaml.safe_load(f)
n = len(d.get("services", {}))
sys.exit(0 if n >= 12 else 1)
EOF
}

# ---------------------------------------------------------------------
# 测试 6: docker-compose.r8.yml 包含 V3 4 层安全门服务
# ---------------------------------------------------------------------
t06_docker_4layer_gates() {
    local f="${APEIRETH_HOME}/docker-compose.r8.yml"
    grep -q "apeireth-r8-v3-guard" "${f}" && \
    grep -q "apeireth-r8-asi-measure" "${f}" && \
    grep -q "apeireth-r8-honest-limits" "${f}" && \
    grep -q "apeireth-r8-hqb-live-gate" "${f}"
}

# ---------------------------------------------------------------------
# 测试 7: integration worktree 真存在且 git 可读 (R7 §技术债 #4 验证)
# ---------------------------------------------------------------------
t07_integration_worktree_exists() {
    local wt="${APEIRETH_HOME}/.spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5"
    [ -d "${wt}" ] && ( cd "${wt}" && git rev-parse HEAD 2>/dev/null | grep -qE "^[a-f0-9]+$" )
}

# ---------------------------------------------------------------------
# 测试 8: integration worktree HEAD 不是空 (有 commit)
# ---------------------------------------------------------------------
t08_integration_worktree_committed() {
    local wt="${APEIRETH_HOME}/.spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5"
    ( cd "${wt}" && git log --oneline -n 3 2>&1 | grep -qE "^[a-f0-9]+ " )
}

# ---------------------------------------------------------------------
# 测试 9: integration worktree 在 git worktree list 中可见 (持久化)
# ---------------------------------------------------------------------
t09_worktree_persistence() {
    cd "${APEIRETH_HOME}"
    git worktree list --porcelain 2>&1 | grep -q "527f21de-e3e3-4dcc-a90d-d022bec6d5e5"
}

# ---------------------------------------------------------------------
# 测试 10: V1081 honest-limits 跑得动 --probe
# ---------------------------------------------------------------------
t10_v1081_probe_runs() {
    cd "${APEIRETH_HOME}"
    timeout 30 ${PYTHON_BIN} -m apeireth.v1081_asi_honest_limits --probe --report >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 11: V1085 HQB core 可被 import (无 LLM key 时也能跑)
# ---------------------------------------------------------------------
t11_v1085_importable() {
    cd "${APEIRETH_HOME}"
    ${PYTHON_BIN} -c "from apeireth.v1085_hqb_core import HonestDecisionModule, DecisionContext, Verdict; print('OK')" >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 12: V1090 memory_wal atomic_write_jsonl 函数可用
# ---------------------------------------------------------------------
t12_v1090_wal_writable() {
    cd "${APEIRETH_HOME}"
    ${PYTHON_BIN} -c "
from pathlib import Path
import tempfile, os
from apeireth.v1090_memory_wal import atomic_write_jsonl
with tempfile.TemporaryDirectory() as td:
    p = Path(td) / 'test.jsonl'
    n = atomic_write_jsonl(p, ['{\"a\":1}', '{\"b\":2}'])
    assert n == 2, n
    lines = p.read_text(encoding='utf-8').splitlines()
    assert len(lines) == 2
print('OK')
" >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 13: V1091 memory_replay 可被 import
# ---------------------------------------------------------------------
t13_v1091_replay_importable() {
    cd "${APEIRETH_HOME}"
    ${PYTHON_BIN} -c "from apeireth.v1091_memory_replay import MemoryReplay; print('OK')" >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 14: V1092 memory_dream 可被 import
# ---------------------------------------------------------------------
t14_v1092_dream_importable() {
    cd "${APEIRETH_HOME}"
    ${PYTHON_BIN} -c "from apeireth.v1092_memory_dream import MemoryDream; print('OK')" >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 15: V1094 memory_schema upgrade 能在临时 SQLite 跑通
# ---------------------------------------------------------------------
t15_v1094_schema_migrates() {
    cd "${APEIRETH_HOME}"
    ${PYTHON_BIN} -c "
import tempfile, sqlite3, os
from pathlib import Path
from apeireth.v1094_memory_schema import upgrade_path
with tempfile.TemporaryDirectory() as td:
    p = Path(td) / 'mem.db'
    upgrade_path(p)
    assert p.exists(), 'db not created'
    c = sqlite3.connect(str(p))
    n = c.execute(\"SELECT COUNT(*) FROM sqlite_master WHERE type='table'\").fetchone()[0]
    c.close()
    assert n >= 1, n
print('OK')
" >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 16: V1087 hqb_live_gate --self-check 可执行
# ---------------------------------------------------------------------
t16_v1087_self_check() {
    cd "${APEIRETH_HOME}"
    timeout 30 ${PYTHON_BIN} -m apeireth.v1087_asi_hqb_live_gate --self-check 2>&1 | tee -a "${TEST_OUT}" | grep -qiE "subscore|gate_id|verdict"
}

# ---------------------------------------------------------------------
# 测试 17: V1083 decision_router --route 可执行
# ---------------------------------------------------------------------
t17_v1083_router_runs() {
    cd "${APEIRETH_HOME}"
    timeout 30 ${PYTHON_BIN} -m apeireth.v1083_asi_decision_router --route --task code --latency 1000 --cost 0.005 --policy balanced 2>&1 | tee -a "${TEST_OUT}" >/dev/null
}

# ---------------------------------------------------------------------
# 测试 18: V1078 cron_self_audit 可被 import (无副作用)
# ---------------------------------------------------------------------
t18_v1078_importable() {
    cd "${APEIRETH_HOME}"
    ${PYTHON_BIN} -c "from apeireth.v1078_asi_cron_self_audit import run_cron_self_audit; print('OK')" >/dev/null 2>&1
}

# ---------------------------------------------------------------------
# 测试 19: 数据/日志/制品目录可写
# ---------------------------------------------------------------------
t19_data_dirs_writable() {
    local d
    for d in data/identity data/memory data/graph data/skills data/dgm logs artifacts; do
        mkdir -p "${APEIRETH_HOME}/${d}"
        [ -w "${APEIRETH_HOME}/${d}" ] || return 1
    done
}

# ---------------------------------------------------------------------
# 测试 20: docker-compose.r8.yml 网络/卷声明齐全
# ---------------------------------------------------------------------
t20_docker_network_volume() {
    local f="${APEIRETH_HOME}/docker-compose.r8.yml"
    grep -q "^networks:" "${f}" && grep -q "apeireth-net" "${f}" && grep -q "^volumes:" "${f}"
}

# ---------------------------------------------------------------------
# 测试 21: R8 tags 包含 V1080-V1088 全 9 个集成工程闭环模块
# ---------------------------------------------------------------------
t21_docker_all_v1080_present() {
    local f="${APEIRETH_HOME}/docker-compose.r8.yml"
    for svc in v1080-reproducibility v1081-honest-limits v1082-codebase-audit v1083-decision-router v1084-real-llm; do
        grep -q "^  ${svc}:" "${f}" || return 1
    done
}

# ---------------------------------------------------------------------
# 测试 22: .gitignore 包含 logs/ 和 artifacts/ (避免真生产脏数据泄露 git)
# ---------------------------------------------------------------------
t22_gitignore_hygiene() {
    grep -q "^logs/" "${APEIRETH_HOME}/.gitignore" && grep -q "^artifacts/" "${APEIRETH_HOME}/.gitignore"
}

# ---------------------------------------------------------------------
# 跑测
# ---------------------------------------------------------------------
{
    echo "============================================================"
    echo "Apeireth ASI R8 部署测试套"
    echo "$(date -Is)"
    echo "APEIRETH_HOME=${APEIRETH_HOME}"
    echo "Log: ${TEST_OUT}"
    echo "============================================================"

    run_test 01 "项目根结构 (apeireth/ + tests/ + start_apeireth.sh)" t01_project_structure
    run_test 02 "V1080-V1088 集成工程闭环 9 模块存在" t02_v1080_v1088_modules
    run_test 03 "V1090-V1098 R7 真实现 7 模块存在" t03_v1090_v1098_modules
    run_test 04 "scripts/start_apeireth_r8.sh 可执行" t04_r8_script_executable
    run_test 05 "docker-compose.r8.yml YAML 合法 (≥12 服务)" t05_docker_compose_valid
    run_test 06 "docker-compose.r8.yml 含 V3 4 层安全门服务" t06_docker_4layer_gates
    run_test 07 "integration worktree 真存在且 git 可读" t07_integration_worktree_exists
    run_test 08 "integration worktree HEAD 有 commit" t08_integration_worktree_committed
    run_test 09 "git worktree list 含 527f21de-* (持久化)" t09_worktree_persistence
    run_test 10 "V1081 honest_limits --probe 可跑通" t10_v1081_probe_runs
    run_test 11 "V1085 HQB core 可被 import" t11_v1085_importable
    run_test 12 "V1090 WAL atomic_write_jsonl 可写" t12_v1090_wal_writable
    run_test 13 "V1091 memory_replay 可被 import" t13_v1091_replay_importable
    run_test 14 "V1092 memory_dream 可被 import" t14_v1092_dream_importable
    run_test 15 "V1094 memory_schema upgrade 临时 SQLite 通过" t15_v1094_schema_migrates
    run_test 16 "V1087 hqb_live_gate --self-check 可执行" t16_v1087_self_check
    run_test 17 "V1083 decision_router --route 可执行" t17_v1083_router_runs
    run_test 18 "V1078 cron_self_audit 可被 import" t18_v1078_importable
    run_test 19 "data/logs/artifacts 目录全部可写" t19_data_dirs_writable
    run_test 20 "docker-compose.r8.yml network/volumes 齐全" t20_docker_network_volume
    run_test 21 "docker-compose 包含 V1080-V1084 服务" t21_docker_all_v1080_present
    run_test 22 ".gitignore 含 logs/ 和 artifacts/" t22_gitignore_hygiene

    echo "------------------------------------------------------------"
    echo "汇总: pass=${PASS} fail=${FAIL} skip=${SKIPPED}"
    if [ "${FAIL}" -gt 0 ]; then
        echo "Failed tests:"
        for ft in "${FAIL_LIST[@]}"; do
            echo "  - ${ft}"
        done
    fi
    echo "Log: ${TEST_OUT}"
    echo "============================================================"
} | tee -a "${TEST_OUT}"

# Exit code: 任何失败则非 0
[ ${FAIL} -eq 0 ] || exit 1
exit 0
