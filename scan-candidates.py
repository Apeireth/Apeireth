import json
from pathlib import Path
base = Path(r'.openclaw\workspace\promethean')
candidates = ['Cheney', 'Seyfarth', 'Heinrich', 'Clayton', 'corvid', 'baboon', 'Hauser',
              'Crutchfield', 'Shalizi', 'D Arcy', 'Thompson On Growth', 'On Growth and Form',
              'René Thom', 'structural stability', 'morphogenesis', 'Niles Eldredge',
              'punctuated equilibrium', 'Lewontin', 'Lewontin gene', 'Sterelny', 'Gould',
              'Vrba', 'exaptation', 'Lenski', 'LTEE', 'Long-Term Evolution', 'Brian Goodwin',
              'How the Leopard', 'structuralist biology', 'Marcello Barbieri', 'code biology',
              'Christopher Zeeman', 'mirror neuron', 'Rizzolatti', 'Gallese', 'Frans de Waal',
              'MAP-Elites', 'Quality-Diversity', 'Mouret', 'steel-dev', 'Composio', 'composio',
              'AgentOps', 'agentops']
for jf in sorted(base.glob('research-v7-round-*.json')):
    with open(jf, 'r', encoding='utf-8') as f:
        data = json.load(f)
    rn = jf.stem.replace('research-v7-round-', '')
    queries = ' '.join(q['query'] for q in data).lower()
    found = [c for c in candidates if c.lower() in queries]
    if found:
        print(f'r{rn}: {found}')