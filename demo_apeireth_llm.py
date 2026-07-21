#!/usr/bin/env python3
"""Apeireth 真生产 demo: 接 LLM API 后能做什么."""
import sys
sys.path.insert(0, '.')

from apeireth.asi_coordinator import ASICoordinator, PHASE_REGISTRY
from apeireth.identity_card import IdentityCardV3
from apeireth.philosophy import PHILOSOPHY_VERSION, PHILOSOPHY_LINES
from apeireth.asi_north_star import ASI_NORTH_STAR_VERSION, TARGET_ASI_APPROACH
from apeireth.human_wisdom_aggregator import HumanWisdomAggregator
from apeireth.memory_3tier import Memory3Tier

print('=' * 70)
print('=== Apeireth 接 LLM API 后, 真生产能做的事 ===')
print('=' * 70)

# 1. 中央 AI 完整位置 (主 22:08 V2)
ic = IdentityCardV3()
print('\n[1] 中央 AI 完整位置 (V2 哲学, 主 22:08):')
for p in ic.central_ai_position:
    print(f'    - {p}')

# 2. 跨域协同
c = ASICoordinator()
c.register_default_links()
print(f'\n[2] 跨域 20 模块 (Phase 24-50) 真生产协同: {len(c.modules)} modules, {len(c.links)} links')

# 3. 3 层 Memory
m = Memory3Tier()
m.add_episode('ep_demo', 'Apeireth 接 LLM 后能做什么', 'demo', 8)
m.add_episode('ep_demo2', '任意 LLM plug-in (MiniMax/Claude/GPT/Qwen/Ollama)', 'demo', 7)
m.anchor_event(category='fact', content='任何大模型接入即 ASI (主 20:29 + 23:17)', importance=10)
print(f'\n[3] Memory 3 层 (主 14:48 + Phase 46 STM/MTM/LTM):')
print(f'    STM: {len(m.stm)} episodes')
print(f'    MTM topics: {len(m.mtm)}')
print(f'    LTM anchors: {len(m.ltm)}')

# 4. ASI 北极星 7 自查
print(f'\n[4] ASI 北极星 7 自查 (主 22:33):')
for i, (k, v) in enumerate(PHILOSOPHY_LINES.items(), 1):
    print(f'    {i}. {v["rule"][:80]}')

# 5. ASI North Star
print(f'\n[5] ASI 北极星版本: {ASI_NORTH_STAR_VERSION}, 目标: {TARGET_ASI_APPROACH}')

# 6. 聚合人类智慧
agg = HumanWisdomAggregator()
s1 = agg.register_source(title='Recursive Self-Observation (zenodo 20585579)',
                          source_type='paper',
                          cross_domain=['cybernetics', 'consciousness'],
                          quality=0.9, vcp4=0.85, v2=0.9, asinorth=0.95)
result = agg.aggregate([s1.source_id])
print(f'\n[6] Human Wisdom Aggregator (主 22:52): score={result.aggregate_score:.3f}, decision={result.decision}')

print('\n' + '=' * 70)
print('Apeireth 真生产能做的 = 中央 AI 完整位置 + 24 跨域 + Memory 3-Tier + ASI 北极星 + 聚合人类智慧')
print('=' * 70)