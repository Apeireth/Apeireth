"""Show ASI V0.2 score breakdown."""
import json
from apeireth.v1048_asi_v02_real_measure import measure_asi_v02_real

r = measure_asi_v02_real()
print('total:', r['total'])
print('level:', r['level'])
print()
print('Component scores:')
for k, v in sorted(r['component_scores'].items()):
    print(f'  {k:30s} {v:.4f}')
print()
print('Top contributions:')
top = sorted(r['contributions'].items(), key=lambda x: -x[1]['contribution'])[:6]
for k, v in top:
    raw = v['raw_score']
    w = v['weight']
    c = v['contribution']
    print(f'  {k:30s} raw={raw:.3f}  w={w:.2f}  c={c:.4f}')