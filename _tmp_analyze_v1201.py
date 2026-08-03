# -*- coding: utf-8 -*-
import json

v1182 = json.load(open('artifacts/v1182_asi_v06_recomputed_baseline.json', encoding='utf-8'))
print('=== V1182 dim_breakdown 全部 (field=value) ===')
total = 0
for d in v1182.get('dim_breakdown', []):
    val = d.get('value', 0)
    w = d.get('weight', 0)
    contrib = val * w
    total += contrib
    print(f"{d['dim']:30s}  weight={w:.4f}  value={val:.4f}  contrib={contrib:.5f}")
print(f"Total: {total:.4f}  (north_star=0.98)")
print(f"V1182 sum: {v1182.get('v1182_total', 'NA')}")

print()
v1153 = json.load(open('artifacts/v1153_v06_spec.json', encoding='utf-8'))
print('V1153 keys:', list(v1153.keys()))
