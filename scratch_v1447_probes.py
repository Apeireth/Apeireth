"""Inspect specific probes in V1447 report."""
import json

data = json.load(open('.v1447-asi-cross-modular-audit-report.json', encoding='utf-8'))
print('All probes for time/scheduler pair:')
for p in data['probes']:
    if p['problem'] == 'time' and p['position'] == 'scheduler':
        kind = p['kind']
        closed = p['closed']
        ev = p['evidence']
        print(f'  {kind}: closed={closed}')
        print(f'    evidence: {ev[:300]}')
print()
print('--- forward probes for various pairs ---')
for p in data['probes']:
    if p['kind'] == 'forward':
        prob = p['problem']
        pos = p['position']
        closed = p['closed']
        print(f'  ({prob}, {pos}): closed={closed}')
        if closed == 0:
            print(f'    evidence: {p["evidence"][:200]}')