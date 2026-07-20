#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test IIT Φ-proxy."""
import sys
sys.path.insert(0, '.')
from apeireth.phi_proxy import compute_phi_proxy, PHI_PROXY_VERSION
from apeireth.mirror import SelfState

print(f'PHI_PROXY version: {PHI_PROXY_VERSION}')

state = SelfState(
    self_name='apeireth',
    memory_episode_count=100,
    memory_note_count=50,
    identity_card_count=10,
    team_card_count=7,
    graph_node_count=20,
    graph_edge_count=26,
    proactive_actions_total=4,
    awareness_level='Layer 4 (SMM)',
)
result = compute_phi_proxy(state)
print(f'Phi-proxy: {result["phi_proxy"]}')
print(f'interpretation: {result["interpretation"]}')
for k, v in result['components'].items():
    print(f'  {k}: {v}')
print('OK IIT Phi-proxy works')
