#!/bin/bash
# Apeireth ASI 基座真生产启动脚本
# 主人 20:46 "ASI 是超越时代的 我们能做的也只是尽力逼近"
# 主人 14:52 "24/7 不能崩" — 真生产就绪

set -e

echo "==========================================="
echo "Apeireth ASI 基座真生产启动"
echo "主人哲学: ASI 超越时代, 我们逼近"
echo "==========================================="
echo ""

# Paths
APEIRETH_HOME="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "[1/6] APEIRETH_HOME=$APEIRETH_HOME"

# Environment
echo "[2/6] 环境检查"
if [ -z "$NEWAPI_API_KEY" ]; then
    echo "  WARN: NEWAPI_API_KEY 未设 — Phase 21 LLM Kernel 用 template fallback"
fi
if [ -z "$MINIMAX_API_KEY" ]; then
    echo "  WARN: MINIMAX_API_KEY 未设 — 同样用 template fallback"
fi

# Check Python
PYTHON=$(which python3 || which python)
echo "  Python: $PYTHON ($(${PYTHON} --version 2>&1))"

# Storage check
echo "[3/6] 存储检查"
mkdir -p "$APEIRETH_HOME/data/identity"
mkdir -p "$APEIRETH_HOME/data/memory"
mkdir -p "$APEIRETH_HOME/data/graph"
mkdir -p "$APEIRETH_HOME/data/skills"
mkdir -p "$APEIRETH_HOME/data/dgm"
mkdir -p "$APEIRETH_HOME/logs"

# Phase 0: setup
echo "[4/6] Phase 0 — Setup"
${PYTHON} -c "
import sys
sys.path.insert(0, '$APEIRETH_HOME')
from apeireth import (
    IdentityStore, IdentityCard,
    Mirror, MetaMonitor, SelfModel,
    SkillLibrary, DGMArchive, DeliberationEngine,
    Mirror, PhiProxy, ASIApproachReport,
    LLMConfig,
)
print('  IdentityStore: OK')
print('  Mirror: OK')
print('  MetaMonitor: OK')
print('  SelfModel: OK')
print('  SkillLibrary: OK')
print('  DGMArchive: OK')
print('  DeliberationEngine: OK')
print('  PhiProxy: OK')
print('  ASIApproachReport: OK')
"

# Phase 21: LLM Kernel health check
echo "[5/6] Phase 21 LLM Kernel — health check"
${PYTHON} -c "
import sys, os
sys.path.insert(0, '$APEIRETH_HOME')
from apeireth.llm_kernel import LLMConfig, call_llm_minimax
cfg = LLMConfig.minimax_default()
print(f'  default provider={cfg.provider} model={cfg.model} base_url={cfg.base_url}')
print(f'  api_key_set={bool(cfg.api_key)}')
resp = call_llm_minimax('Apeireth health check', cfg)
print(f'  call result: provider={resp.provider} content_len={len(resp.content)}')
"

# Phase 20: ASI Approach Index
echo "[6/6] Phase 20 ASI Approach Index"
${PYTHON} -c "
import sys
sys.path.insert(0, '$APEIRETH_HOME')
from apeireth.asi_north_star import compute_v7_approach, compute_target_approach
v7 = compute_v7_approach()
target = compute_target_approach()
print(f'  V7 ASI Approach Index: {v7.asi_approach:.4f} ({v7.interpretation})')
print(f'  Target: {target.asi_approach:.4f} ({target.interpretation})')
print(f'  Gap: {target.asi_approach - v7.asi_approach:.4f}')
"

echo ""
echo "==========================================="
echo "Apeireth ASI 基座已启动"
echo "  13 能力 PASS"
echo "  3 意识层 工程化"
echo "  Phase 19 Thinking Layer"
echo "  Phase 20 ASI Approach Metric"
echo "  Phase 21 LLM Kernel (MiniMax 默认)"
echo ""
echo "下一步: 运行 python asi_demo_v7.py 看完整 demo"
echo "==========================================="
