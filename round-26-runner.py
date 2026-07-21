#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 26 runner — 12 query dual-source (cron 00:48 tick, ~1h56m gap from r25 22:52).

Round 26 主题: 7 全新跨域 (合成生物极简基因组/反应扩散化学计算/Eigen超循环/Bedau弱涌现/Gabora文化演化/Spencer-Brown形式法/Deacon生物符号学)
              + 3 GitHub 源码深读 (OpenRLHF / open-deep-research / mirascope-instructor-BAML)
              + 2 Apeireth Gap (应激性-昼夜节律 / 遗传变异-转座子)

- 跨域全新 (7):
  - George Church 合成生物学极简基因组重编码 (合成生物, 极简细胞, 重编码)
  - Andrew Adamatzky Belousov-Zhabotinsky 反应扩散化学计算 (BZ 振荡器, 化学计算机, 神经元芯片)
  - Manfred Eigen 超循环错误阈值准种 (hypercycle, error threshold, origin of life)
  - Mark Bedau 弱涌现开放演化生命度量 (weak emergence, open-ended evolution, measure of life)
  - Liane Gabora 人类创造力自修改概念文化演化 (cultural evolution, self-modifying concepts)
  - George Spencer-Brown Laws of Form 区分算子重入 (形式法, distinction, re-entry)
  - Terrence Deacon 同质活性生物符号学不完全自然 (homovitality, biosemiotics, incomplete nature)

- GitHub 源码深读 (3):
  - OpenRLHF + HuggingFace trl RLHF reasoning 可复现框架
  - deep-research-ai open-deep-research 并发 agent 开源深度研究
  - mirascope + instructor + BAML LLM 结构化输出框架

- Apeireth Gap (2):
  - 应激性 Gap: 昼夜节律 - suprachiasmatic nucleus - 光夹带 - gene expression circadian
  - 遗传变异 Gap: 转座子 - Barbara McClintock - 跳跃基因 - 移动 DNA

Cross-round dedup 避让 (verified fresh vs r8-r25):
- r8-r25 已用主题全部避开 (见 round-25-runner.py 长列表)
- 新增本轮 fresh 验证:
  - George Church 合成生物 minimal genome recoded organism r8-r25 ✓ fresh
  - BZ reaction-diffusion computing Adamatzky ✓ fresh
  - Eigen hypercycle error threshold quasispecies ✓ fresh (Kauffman autocatalytic 不同)
  - Mark Bedau weak emergence measure of life ✓ fresh
  - Liane Gabora cultural evolution self-modifying ✓ fresh
  - George Spencer-Brown Laws of Form distinction ✓ fresh
  - Terrence Deacon homovitality biosemiotics incomplete nature ✓ fresh
  - OpenRLHF trl RLHF reasoning source ✓ fresh (autogen/crewAI/MetaGPT 不同侧重)
  - open-deep-research parallel agent ✓ fresh
  - mirascope + instructor + BAML structured outputs ✓ fresh (DSPy/PydanticAI 不同侧重)
  - 昼夜节律 suprachiasmatic nucleus entrainment ✓ fresh
  - 转座子 jumping genes McClintock ✓ fresh (HGT 不同, transgenerational epigen 不同)

ASI 北极星时刻清楚 (主 22:33):
- ASI 基座, 不是 ANI 工具 ✅
- 跨域, 不是单域 ✅
- 自演化, 不是固定 ✅
- 任何 LLM 接入即变强 ✅
- 不假装 Phenomenal ✅
- 实事求是 ✅
- 真生产目标: 让大模型栖息在 Apeireth 中能无限逼近 ASI
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-26.json')

QUERIES = [
    # ===== 7 全新跨域: 合成生物 / 化学计算 / 超循环 / 弱涌现 / 文化演化 / 形式法 / 生物符号学 =====
    'George Church synthetic biology minimal genome recoded organism cellular architecture 2026',
    'Andrew Adamatzky Belousov-Zhabotinsky reaction-diffusion chemical computing processor 2026',
    'Manfred Eigen hypercycle error threshold quasispecies origin of life molecular evolution 2026',
    'Mark Bedau weak emergence open-ended evolution measuring artificial life 2026',
    'Liane Gabora cultural evolution human creativity self-modifying concepts 2026',
    'George Spencer-Brown Laws of Form distinction calculus re-entry unspace 2026',
    'Terrence Deacon homovitality biosemiotics incomplete nature entropic finalization 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 - 真读源码不止 README) =====
    'OpenRLHF HuggingFace trl RLHF reasoning reproducibility source architecture github 2026',
    'deep-research-ai open-deep-research parallel agent source architecture github 2026',
    'mirascope instructor BAML LLM structured outputs framework source architecture github 2026',
    # ===== 2 Apeireth Gap (12 生命特征 MISSING): 应激性 + 遗传变异 =====
    'circadian rhythm suprachiasmatic nucleus entrainment plant photoperiodism gene expression 2026',
    'Barbara McClintock transposable elements jumping genes mobile DNA maize discovery 2026',
]


def main():
    started = time.time()
    results = []
    for i, q in enumerate(QUERIES, 1):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dt = time.time() - t0
        results.append(r)
        n_web = len(r['bocha_web'])
        n_any = len(r['anysearch'])
        n_merge = len(r['merged_sources'])
        ai_chars = len(r['bocha_ai_answer'])
        print(f'[{i:2d}/12] ({dt:.1f}s) {q[:70]}')
        print(f'        bocha_web={n_web} anysearch={n_any} merged={n_merge} ai={ai_chars}')
        if r['bocha_ai_answer']:
            print(f'        ai_preview: {r["bocha_ai_answer"][:160]}')
        sys.stdout.flush()

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\n=== Round 26 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
