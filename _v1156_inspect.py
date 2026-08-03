"""Inspect V1156-V1169 dim values."""
import json
from pathlib import Path

# Map v115x → dim
v_to_dim = {
    'v1156': 'cognitive_core',
    'v1157': 'self_improving_core',
    'v1158': 'plugin_core',
    'v1159': 'engineering',
    'v1160': 'rubric_open',
    'v1161': 'v2_philosophy',
    'v1162': 'world_model',
    'v1163': 'real_production',
    'v1164': 'world_model_patched',
    'v1165': 'self_organizing_core',
    'v1166': 'real_llm_benchmark',
    'v1167': 'streamlit_real_startup',
    'v1168': 'philosophy_5gaps',
    'v1169': 'reinforcement_learning',
}

print("=== V1156-V1169 V0.6 real dim ===")
for v, expected_dim in v_to_dim.items():
    matching = list(Path('artifacts').glob(f'{v}_*.json'))
    if not matching:
        print(f'{v}: NOT FOUND')
        continue
    f = matching[0]
    try:
        with open(f, 'r', encoding='utf-8') as fp:
            data = json.load(fp)
        # 找 dim / value
        dim = data.get('dim', '?')
        value = data.get('value', data.get('dim_value', data.get('score', data.get('total', '?'))))
        n_sub = data.get('n_sub_dims', data.get('n_dims', '?'))
        print(f'{v}: file={f.name} dim={dim} value={value} n_sub={n_sub}')
    except Exception as e:
        print(f'{v}: err={e}')