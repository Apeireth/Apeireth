"""Inspect V1153 baseline 21-dim."""
import json
from pathlib import Path

with open('artifacts/v1153_v06_spec.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

spec = data['spec']
results = spec['dim_results']

print(f"21-dim results: {type(results).__name__} len={len(results)}")
print(f"first entry: {results[0]}")
print()
print("=== 21-dim sorted (low → high) ===")
sorted_dims = sorted(results, key=lambda x: x.get('value', 0))
for d in sorted_dims:
    dim = d.get('dim', d.get('name', '?'))
    weight = d.get('weight', 0)
    value = d.get('value', 0)
    status = d.get('status', '?')
    source = d.get('source', '?')[:40]
    print(f"  {dim:40} weight={weight:.4f} value={value:.4f} status={status} source={source}")

print()
print(f"n_dims: {spec['n_dims']}")
print(f"asi_v06_score: {spec['asi_v06_score']}")
print(f"north_star: {spec['north_star']}")
print(f"gap: {spec['gap']}")

# Save the dim dict for downstream use
dim_dict = {d.get('dim', d.get('name', '?')): d.get('value', 0) for d in results}
print()
print("dim_dict written:")
print(json.dumps(dim_dict, indent=2))