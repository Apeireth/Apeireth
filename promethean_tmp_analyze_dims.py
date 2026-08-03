"""Quick dim analysis script."""
import json
import os

base = ".openclaw/workspace/promethean/artifacts/"
artifacts = [
    'v1194_asi_v066_3dim_lift.json',
    'v1195_asi_v067_3dim_lift.json',
    'v1196_asi_v068_3dim_lift.json',
    'v1197_asi_v069_3dim_recover.json',
    'v1200_asi_v0610_dual_dim_lift.json',
    'v1201_asi_v0611_dual_dim_lift.json',
]
for a in artifacts:
    p = base + a
    if not os.path.exists(p):
        print(f"  MISSING: {a}")
        continue
    with open(p, encoding='utf-8') as f:
        d = json.load(f)
    print(f"=== {a} ===")
    print(f"  formula_2_recompute: {d.get('formula_2_recompute', '?')}")
    print(f"  asi_recompute_lifted: {d.get('asi_recompute_lifted', '?')}")
    for k, v in d.get('dim_lifts', {}).items():
        print(f"    {k}: {v.get('baseline', '?')} -> {v.get('new_value', v.get('value', '?'))} (delta={v.get('delta', '?')})")
    # show all dim states
    if 'all_dims' in d:
        print(f"  all_dims ({len(d['all_dims'])}):")
        for k, v in d['all_dims'].items():
            print(f"    {k}: {v}")
