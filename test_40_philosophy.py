#!/usr/bin/env python3
"""Phase 40 + Philosophy Guard test."""
import sys
sys.path.insert(0, '.')

from apeireth.small_world import SMALL_WORLD_VERSION, SmallWorldGraph
from apeireth.philosophy import PHILOSOPHY_VERSION, check_philosophy

print(f'Phase 40 SmallWorldGraph: {SMALL_WORLD_VERSION}')
print(f'Philosophy Guard: {PHILOSOPHY_VERSION}')

# Phase 40
sw = SmallWorldGraph(rewire_prob=0.3, k=4, n_nodes=20)
sw.build_ring_lattice()
n_rewired = sw.rewire_links()
stats = sw.stats()
print(f'  SmallWorld: {stats}')

# Philosophy Guard tests
print('\n=== Philosophy Guard Tests ===')
check_clean = check_philosophy(
    'SmallWorldGraph',
    '用于跨域借鉴 Watts 网络组织模式, 优化 Phase 38-39 节点连通性',
)
print(f'  Clean module: passed={check_clean.passed}, deviations={len(check_clean.deviations)}')

check_bad = check_philosophy(
    'BadMock',
    '中央 AI 是调度者, 我有 Phenomenal consciousness, 复刻生态学哲学',
)
print(f'  Bad module:  passed={check_bad.passed}, deviations={len(check_bad.deviations)}')
for d in check_bad.deviations:
    print(f'    line={d["line"]} pattern={d["pattern_matched"]}')

print('OK Phase 40 + Philosophy Guard')
