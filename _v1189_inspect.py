"""Inspect V1189 (V0.6.3 baseline) full dim state."""
import json

with open('artifacts/v1189_v1182_integration.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

print('V1189 (V0.6.3 integration):')
print(f"  v1182 baseline: {data['v1182_asi_baseline']}")
print(f"  v1189 lifted:   {data['v1189_asi_lifted']}")
print(f"  delta:          {data['delta_asi']}")
print(f"  vs north_star:  {data['vs_north_star']}")
print()
dims = data['dims']
print('21-dim values (sorted low→high):')
sorted_dims = []
for d, v in dims.items():
    if isinstance(v, dict):
        val = v.get('value', 0)
        weight = v.get('weight', '?')
        source = v.get('source', '?')[:40]
        print(f"  {d:30} weight={weight} value={val:.4f} source={source}")
    else:
        print(f"  {d:30} value={v:.4f}")
        sorted_dims.append((d, v))