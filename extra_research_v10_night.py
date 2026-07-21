import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

QUERIES = [
    'self-improving AI agent 2026 foundation model',
    'Apeireth ASI base architecture 2026',
    'production grade AI agent real deployment 2026',
    'multi-agent coordination real world 2026',
    'long horizon planning AI 2026',
    'human level reasoning AI 2026',
    'agentic workflow real production 2026',
    'tool use foundation model 2026',
    'context window million token 2026',
    'memory augmented AI agent 2026',
    'AI agent enterprise deployment 2026',
    'reasoning model real benchmark 2026',
]

results = []
for q in QUERIES:
    try:
        r = dual_research(q, top_k=3)
        results.append(r)
        print(f'OK: {q[:50]}')
    except Exception as e:
        results.append({'query': q, 'error': str(e)})
        print(f'ERR: {q[:50]}')

with open('research-v7-round-10.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'saved {len(results)} V7 round-10 queries')