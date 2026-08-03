"""Inspect V1189 (V0.6.3 baseline) full dim state v2."""
import json

with open('artifacts/v1189_v1182_integration.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

dims = data['dims']
print('21-dim values (sorted low→high):')
sorted_dims = []
for d, v in dims.items():
    if isinstance(v, dict):
        val = v.get('value', 0)
    else:
        val = v
    sorted_dims.append((d, val))

sorted_dims.sort(key=lambda x: x[1])
for d, v in sorted_dims:
    print(f"  {d:30} value={v:.4f}")