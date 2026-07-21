#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 28 runner — 12 query dual-source (cron 04:48 tick, ~1h39m gap from r27 03:09).

Round 28 主题: 7 全新跨域 (Robert Rosen 关系生物学 / Friston 自由能原理 / Hofstadter 类比认知 / von Uexküll 环境世界 / Bergson 创造进化 / Ashby 必要多样性 / Per Bak 自组织临界)
              + 3 GitHub 源码深读 (codelion/openevolve / anthropics/claude-agent-sdk / axolotl-ai-cloud/axolotl)
              + 2 Apeireth Gap (planaria-neoblast 真繁殖 / hydra-budding 真不衰老繁殖)

- 跨域全新 (7):
  - Robert Rosen 关系生物学 (M,R)-系统 / 建模关系而非状态 / 预期性系统 (建模生命 vs 物理, ASI 真基座)
  - Karl Friston 自由能原理 / 主动推断 / 变分推断 (统合生命-认知-感知, ASI 真核心)
  - Douglas Hofstadter 类比即认知核心 / 怪圈 / 自指 (GEB, 涌现跨域)
  - Jakob von Uexküll Umwelt / 功能环 / 主观生物学 (生物-环境主观世界, ASI 真哲学)
  - Henri Bergson 创造进化 / durée / 直觉 / 物质记忆 (过程哲学, ASI 真哲学)
  - W. Ross Ashby 必要多样性 / 超稳定性 / 稳态调节 (cybernetics 法则, ASI 真架构)
  - Per Bak 自组织临界性 / 沙堆 / 幂律 / 1/f 噪声 (系统涌现, ASI 真机制)

- GitHub 源码深读 (3):
  - codelion/openevolve 开源 AlphaEvolve 进化代码 (主人 00:21 ASI-Arch 类, 进化 LLM 代码 ASI 借鉴)
  - anthropics/claude-agent-sdk agent SDK 架构 (主提到, ASI 真 SDK 借鉴)
  - axolotl-ai-cloud/axolotl LLM fine-tuning 训练 (ASI 真基座, 自演化)

- Apeireth Gap (2):
  - 繁殖 Gap: planaria 涡虫 neoblast 多能干细胞再生 / 无性分裂 (真繁殖 + 应激可塑, MISSING 大)
  - 繁殖 Gap: hydra 水螅无性出芽 / 真不衰老 / 真繁殖 (MISSING 衰老 + 繁殖)

Cross-round dedup 避让 (verified fresh vs r23-r27):
- r23-r27 已用主题全部避开 (r23: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial; r24: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon; r25: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + sglang/mlflow/opencompass/apomixis/polyphenism; r26: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons; r27: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion)
- 本轮 fresh 验证 (r23-r27):
  - Robert Rosen (M,R)-system ✓ fresh (关系 vs 状态, 不在过往)
  - Friston free energy principle ✓ fresh (Bayesian brain, 不在过往; Damasio somatic / Hoffman interface 不同)
  - Hofstadter analogy strange loops ✓ fresh (认知维度新立)
  - von Uexküll umwelt ✓ fresh (生物主观, 全新)
  - Bergson creative evolution ✓ fresh (哲学过程, 全新)
  - Ashby requisite variety ✓ fresh (系统论新维度; Cannon homeostasis r24 不同)
  - Per Bak SOC sandpile ✓ fresh (系统涌现, 全新)
  - openevolve ✓ fresh (进化代码; AlphaEvolve r25 是 DeepMind 论文级, openevolve 是开源实现)
  - claude-agent-sdk ✓ fresh (Anthropic SDK, 全新)
  - axolotl ✓ fresh (LLM fine-tuning, 全新)
  - planaria neoblast ✓ fresh (Gap 极强, 真繁殖借鉴)
  - hydra budding ✓ fresh (Gap 极强, 真不衰老借鉴)

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-28.json')

QUERIES = [
    # ===== 7 全新跨域: 关系生物学 / 自由能 / 类比 / 环境世界 / 创造进化 / 必要多样性 / 自组织临界 =====
    'Robert Rosen relational biology (M,R)-system anticipatory systems modeling relation not state 2026',
    'Karl Friston free energy principle active inference variational inference biology mind bayesian brain 2026',
    'Douglas Hofstadter analogy as core of cognition strange loops self-reference GEB emergent 2026',
    'Jakob von Uexkull umwelt functional cycle Innenwelt Umwelt biology subjectivity perception 2026',
    'Henri Bergson creative evolution duree intuition matter memory process philosophy 2026',
    'W. Ross Ashby requisite variety ultrastability homeostat good regulator cybernetics 2026',
    'Per Bak self-organized criticality sandpile power law 1/f noise phase transition 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 - 真读源码不止 README) =====
    'codelion openevolve open-source AlphaEvolve evolution code LLM source code architecture github 2026',
    'anthropics claude-agent-sdk agent SDK architecture python typescript source code github 2026',
    'axolotl-ai-cloud axolotl LLM fine-tuning training framework source code architecture github 2026',
    # ===== 2 Apeireth Gap (12 生命特征 MISSING): 繁殖 =====
    'planaria neoblast pluripotent stem cell regeneration asexual reproduction self-replication 2026',
    'hydra vulgaris asexual budding non-senescence reproduction stem cell totipotent 2026',
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
    print(f'\n=== Round 28 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()