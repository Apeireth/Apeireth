"""Inspect V1182 (V0.6.2) dim breakdown v2."""
import json

with open('artifacts/v1182_asi_v06_recomputed_baseline.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

print(f"V1182 total: {data['total']}")
print(f"n_dims: {data['n_dims']}")
print()

print('=== 23-dim (sorted low→high) ===')
breakdown = data['dim_breakdown']
sorted_bd = sorted(breakdown, key=lambda x: x.get('value', 0))
for d in sorted_bd:
    dim = d['dim']
    weight = d['weight']
    val = d['value']
    status = d['status']
    basis = d['data_basis']
    source = d['source'][:40]
    print(f"  {dim:30} weight={weight:.4f} value={val:.4f} status={status} basis={basis} source={source}")