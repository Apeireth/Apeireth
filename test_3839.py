#!/usr/bin/env python3
"""Phase 38 + 39 test."""
import sys
sys.path.insert(0, '.')

from apeireth.game_theory import NASH_VERSION, IncentiveEngine
from apeireth.metaphor import LAKOFF_VERSION, MetaphorEngine

print(f'Phase 38: {NASH_VERSION}')
print(f'Phase 39: {LAKOFF_VERSION}')

# Phase 38 Nash
ie = IncentiveEngine()
def payoff_a(a):
    return 2.0 if a == 'cooperate' else 1.0
def payoff_b(a):
    return 2.0 if a == 'cooperate' else 1.0

a1 = ie.add_agent('调度者', ['cooperate', 'defect'], payoff_a)
a2 = ie.add_agent('学习者', ['cooperate', 'defect'], payoff_b)
nash = ie.find_nash()
print(f'  Nash: {nash.profile}, is_nash={nash.is_nash}, iter={nash.iterations_to_reach}')

# Phase 39 Metaphor
me = MetaphorEngine()
seeds = me.apeireth_seed_metaphors()
print(f'  seed metaphors: {len(seeds)}')
hits = me.find_by_target('中央 AI')
print(f'  find target 中央 AI: {len(hits)} hit')
print(f'  stats: {me.stats()}')
print('OK Phase 38+39 work')
