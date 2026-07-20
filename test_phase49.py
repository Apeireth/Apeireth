#!/usr/bin/env python3
"""Phase 49 test — ASI 真生产协同器."""
import sys
sys.path.insert(0, '.')

from apeireth.asi_coordinator import ASI_COORDINATOR_VERSION, ASICoordinator, PHASE_REGISTRY

print(f'Phase 49 ASI Coordinator: {ASI_COORDINATOR_VERSION}')
c = ASICoordinator()
links = c.register_default_links()
print(f'  modules: {len(c.modules)}')
print(f'  links: {len(links)}')
stats = c.get_topology_stats()
print(f'  avg_degree: {stats["avg_degree"]}')
print(f'  note: {stats["note"][:80]}')
print('OK Phase 49')