"""Quick V1447 summary."""
import json

data = json.load(open('.v1447-asi-cross-modular-audit-report.json', encoding='utf-8'))
print('overall_closure_rate:', data['overall_closure_rate'])
print('overall_cross_link_density:', data['overall_cross_link_density'])
print()
print('per_kind:')
for k, v in data['per_kind_closure_rate'].items():
    print(f'  {k}: {v:.4f}')
print()
print('per_position:')
for k, v in data['per_position_closure_rate'].items():
    print(f'  {k}: {v:.4f}')
print()
print('per_problem:')
for k, v in data['per_problem_closure_rate'].items():
    print(f'  {k}: {v:.4f}')
print()
print('compositional:', len(data['compositional_pairs']))
print('anti_modular:', len(data['anti_modular_pairs']))
print('substitutable:', len(data['substitutable_pairs']))
print()
print('Sample pair stats (first 10):')
for ps in data['pair_stats'][:10]:
    prob = ps['problem']
    pos = ps['position']
    rate = ps['closure_rate']
    closed = ps['n_closed']
    total = ps['n_probes']
    broken = ps['broken_kinds']
    print(f'  ({prob}, {pos}): rate={rate:.2f} closed={closed}/{total} broken={broken}')
print()
# Distribution
print('Pair closure rate distribution:')
high = sum(1 for p in data['pair_stats'] if p['closure_rate'] >= 0.8)
mid = sum(1 for p in data['pair_stats'] if 0.4 <= p['closure_rate'] < 0.8)
low = sum(1 for p in data['pair_stats'] if 0 < p['closure_rate'] < 0.4)
zero = sum(1 for p in data['pair_stats'] if p['closure_rate'] == 0)
print(f'  >= 0.8: {high}')
print(f'  0.4-0.8: {mid}')
print(f'  0-0.4: {low}')
print(f'  == 0: {zero}')