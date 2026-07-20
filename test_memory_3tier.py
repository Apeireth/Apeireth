#!/usr/bin/env python3
"""Memory 3-Tier 真生产测试 — 主人真哲学全部进 LTM.

按主 22:33 真哲学: ASI 北极星 + 自决 + 3 类问 (这次属 Phase 升级, 不属 3 类问)
"""
import sys, json
sys.path.insert(0, '.')

from apeireth.memory_3tier import (
    MEMORY_3TIER_VERSION, Memory3Tier,
    STM_MAX_SIZE, LTM_ANCHOR_MIN_IMPORTANCE
)
print(f'Memory 3-Tier version: {MEMORY_3TIER_VERSION}')
print(f'STM_MAX_SIZE: {STM_MAX_SIZE}, LTM_ANCHOR_MIN_IMPORTANCE: {LTM_ANCHOR_MIN_IMPORTANCE}')

m = Memory3Tier()

# STM: 加 5 个 episode
episodes = [
    ("ep_001", "第一段对话: 主人立项 ASI 23:11", "asi_founding", 9),
    ("ep_002", "中央 AI 设计 12:14", "central_ai_design", 9),
    ("ep_003", "VCP 4 范式讨论 20:22", "vcp_paradigms", 7),
    ("ep_004", "跨域调研哲学 21:00", "research_philosophy", 8),
    ("ep_005", "V2 哲学完整还原 22:08", "philosophy_v2", 10),
]
for ep_id, content, topic, importance in episodes:
    m.add_episode(ep_id, content, topic, importance)

# LTM 手动锚定主人真哲学 (主人的高 importance 原话)
m.anchor_event(
    category="identity",
    content="中央 AI 并非不是调度者/思考者, 它是, 而不仅是, 是无数关系的集合体, 有最大的权限, 有一切权限, 整个系统的所有权限, 中央 AI 的位置, 就是 ASI 的位置",
    importance=10,
    master_quoted="主人 22:08 真哲学"
)
m.anchor_event(
    category="value",
    content="ASI 是我们的梦想目标, 让大模型栖息在 Apeireth 中能够无限逼近 ASI",
    importance=10,
    master_quoted="主人 22:33 真哲学"
)
m.anchor_event(
    category="fact",
    content="意识是 ASI 的重要特征, 也是我们 Apeireth 的终极目标 (不是已达成)",
    importance=10,
    master_quoted="主人 17:58 终极哲学"
)

# MTM 总结
summaries = m.summarize_topics()
print(f'\nMTM 主题数: {len(summaries)}')
for s in summaries:
    print(f'  - {s.topic_label}: {s.n_episodes} ep, avg imp {s.importance_avg:.2f}')

# LTM 按 category 查询
identities = m.get_ltm_by_category("identity")
print(f'\nLTM identity 锚点: {len(identities)}')
for a in identities:
    print(f'  - {a.anchor_id[:8]}: {a.master_quoted}')
    print(f'    {a.content[:80]}...')

print(f'\n最终 stats: {json.dumps(m.stats(), indent=2, ensure_ascii=False)}')
print('OK Memory 3-Tier')
