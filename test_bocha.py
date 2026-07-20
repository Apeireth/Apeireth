#!/usr/bin/env python3
"""Test Bocha API with real key + 6 cross-domain research."""
import sys
import json
import urllib.request
import urllib.error
from pathlib import Path

KEY = 'sk-905b4204278848a08e84e2de0b570271'  # 35 chars 真 key!
HEADERS = {
    'Authorization': 'Bearer ' + KEY,
    'Content-Type': 'application/json',
    'User-Agent': 'ApeirethResearch/3.0'
}


def search(endpoint, query, count=5, mode='web'):
    body = json.dumps({'query': query, 'summary': True, 'count': count}).encode('utf-8')
    req = urllib.request.Request(f'https://api.bochaai.com/v1/{endpoint}', headers=HEADERS, data=body)
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read().decode('utf-8'))


def ai_search(query, count=5):
    """Get AI-generated answer from Bocha ai-search."""
    body = json.dumps({'query': query, 'summary': True, 'count': count}).encode('utf-8')
    req = urllib.request.Request('https://api.bochaai.com/v1/ai-search', headers=HEADERS, data=body)
    with urllib.request.urlopen(req, timeout=30) as r:
        result = json.loads(r.read().decode('utf-8'))
    ai_answer = None
    sources = []
    for m in result.get('messages', []):
        if m.get('role') == 'assistant' and m.get('type') == 'answer':
            ai_answer = m.get('content', '')
        if m.get('role') == 'assistant' and m.get('type') == 'source':
            try:
                src = json.loads(m.get('content', '{}'))
                for v in src.get('value', [])[:3]:
                    sources.append(f"{v.get('name','')[:60]} — {v.get('url','')[:60]}")
            except Exception:
                pass
    return ai_answer, sources


if __name__ == '__main__':
    # 1. 验证双端点
    print('=== Test Bocha 双端点 (web + ai) ===')
    try:
        r = search('web-search', 'ecosystem engineering ASI', 3)
        print(f'  web-search OK: {len(r["data"]["webPages"]["value"])} hits')
    except Exception as e:
        print(f'  web-search ERR: {e}')

    # 2. 主人 21:00 "跨多个界" — 6 跨域深度调研,每个 web + ai 双端点
    queries = [
        ('ecology_engineering', 'How does ecosystem engineering principles apply to ASI base architecture? Self-organization, niche construction, keystone species.'),
        ('second_order_cybernetics', 'How does von Foerster second-order cybernetics (observing systems observing themselves) apply to self-aware AI agents?'),
        ('game_theory_multi_agent', 'How does game theory (Nash equilibrium, mechanism design) inform multi-agent cooperation in ASI?'),
        ('cognitive_linguistics_metaphor', 'How does Lakoff embodied cognition and metaphor theory apply to LLM agent design?'),
        ('network_science_small_world', 'How does Watts small-world network (6 degrees) apply to LLM agent memory graph?'),
        ('ecology_keystone_species', 'What is a keystone species in ecology and how does it translate to ASI agent ecosystem design?'),
    ]

    results = []
    for slug, q in queries:
        print(f'\n=== {slug} ===')
        # ai-search first (主人 21:00 "博查ai搜索回答更智能")
        ai_ans, sources = ai_search(q, count=4)
        print(f'  AI answer: {len(ai_ans) if ai_ans else 0} chars')
        if ai_ans:
            print(f'    {ai_ans[:600]}')
        print(f'  Sources: {len(sources)}')
        for s in sources[:2]:
            print(f'    - {s}')
        results.append({
            'slug': slug,
            'query': q,
            'ai_answer': ai_ans,
            'sources': sources,
        })

    # save
    out = Path(r'.openclaw\workspace\promethean\research-cross-domain-2026-07-20.json')
    out.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    print(f'\nsaved {len(results)} queries to {out}')
