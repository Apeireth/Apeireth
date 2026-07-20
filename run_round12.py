#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""V7 round-12 真生产细节调研 — 失败恢复/沙盒/协议/评测/成本/世界模型."""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

# 12 query 真生产细节角度 (round-12, 主人 00:25 真务实修)
QUERIES = [
    # 失败/恢复
    'production agent failure modes recovery retry idempotency',
    'agent loop timeout stuck detection circuit breaker',
    # 沙盒/权限
    'LLM agent tool sandboxing permission policy 2026',
    'code execution sandbox gVisor firecracker agent',
    # 通信协议
    'Model Context Protocol MCP specification 2025',
    'agent to agent A2A protocol Google interoperability',
    # 评测
    'SWE-bench verified production agent benchmark 2026',
    'agent evaluation harness llm-as-judge framework',
    # 成本/能耗
    'cost aware LLM inference routing cascade',
    'energy efficient agent compute budget allocation',
    # 世界模型 + 自我模型
    'world model agent Sora Genie embodiment 2025',
    'agent self-model metacognition production architecture',
]

def main():
    t0 = time.time()
    results = []
    for i, q in enumerate(QUERIES, 1):
        r = dual_research(q, top_k=3)
        results.append(r)
        elapsed = time.time() - t0
        print(f'\n=== [{i}/12] {q[:60]} ({elapsed:.1f}s) ===')
        print(f'  Bocha web: {len(r["bocha_web"])}, AnySearch: {len(r["anysearch"])}, merged: {len(r["merged_sources"])}')
        if r['bocha_ai_answer']:
            print(f'  AI answer: {r["bocha_ai_answer"][:160]}')
    out = Path(r'.openclaw\workspace\promethean\research-v7-round-12.json')
    out.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - t0
    total_sources = sum(len(r['merged_sources']) for r in results)
    print(f'\nsaved {len(results)} queries / {total_sources} merged sources / {total:.1f}s')
    print(f'output: {out}')

if __name__ == '__main__':
    main()