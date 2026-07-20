import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

QUERIES = [
    'autonomous AI agent foundation model research 2026',
    'long context LLM production memory architecture',
    'APEIRETH ASI agent 2026',
    'multi agent system coordination research 2026',
    'AI agent open source production 2026',
    'memory augmented neural network research 2026',
    'LLM agentic workflow foundation 2026',
    'reasoning model emergent 2026',
    'agent memory consolidation foundation 2026',
    'AI tool use function calling benchmark 2026',
    'real world AI deployment production 2026',
    'AI alignment safety 2026 frontier',
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

with open('research-v7-round-9.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'saved {len(results)} V7 round-9')