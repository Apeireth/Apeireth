"""Quick check."""
import json
with open('artifacts/v1201_asi_v0611_dual_dim_lift.json', encoding='utf-8') as f:
    d = json.load(f)
print('V1201 dim_lifts:')
for k, v in d.get('dim_lifts', {}).items():
    print(f"  {k}: {v.get('baseline')} -> {v.get('new_value')}")
print()
print('V1201 asi_recompute_lifted:', d.get('asi_recompute_lifted'))
