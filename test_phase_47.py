#!/usr/bin/env python3
"""Test Phase 47 — Neural Darwinism (Edelman NGST)."""
import sys
sys.path.insert(0, '.')

from apeireth.neural_darwinism import (
    NEURAL_DARWINISM_VERSION,
    NeuronalGroup,
    SelectionRound,
    DynamicCoreReport,
    NeuralDarwinismSystem,
)

print(f'NEURAL_DARWINISM_VERSION = {NEURAL_DARWINISM_VERSION}')

# Phase 1: 创建 system + 发育期选择
nd = NeuralDarwinismSystem(n_groups=20)
nd.developmental_selection()
assert nd.developmental_selection_done
assert len(nd.groups) == 20
print(f'Phase 1: developmental_selection OK, {len(nd.groups)} groups')

# Phase 2: 经验期选择 (5 cycles)
for i in range(5):
    sr = nd.experiential_selection(input_signal=0.5 + i * 0.1)
    assert isinstance(sr, SelectionRound)
    assert len(sr.selected_group_ids) > 0
print(f'Phase 2: 5 experiential_selection rounds, mean_fitness last={sr.mean_fitness:.3f}')

# Phase 3: Reentry step
coh = nd.reentry_step()
print(f'Phase 3: reentry_step coherence = {coh:.3f}')

# Dynamic Core report
report = nd.dynamic_core_report()
print(f'DynamicCoreReport:')
print(f'  n_active_groups = {report.n_active_groups}')
print(f'  mean_fitness = {report.mean_fitness:.3f}')
print(f'  reentry_coherence = {report.reentry_coherence:.3f}')
print(f'  diversity_index = {report.diversity_index:.3f}')
print(f'  is_dynamic_core = {report.is_dynamic_core}')

# Stats
s = nd.stats()
print(f'\nstats keys: {list(s.keys())}')
print(f'V2 philosophy: {s["v2_philosophy"][:100]}...')

print('\n✅ Phase 47 Neural Darwinism — ALL OK')