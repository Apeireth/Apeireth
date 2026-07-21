#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Deep Research Dual-Source — Bocha AI + AnySearch."""
from __future__ import annotations
import json, sys
sys.stdout.reconfigure(encoding='utf-8')
import urllib.request, urllib.error
from pathlib import Path

# UTF-8 no BOM 读 keys (避免 latin-1 codec 错)
def read_key(path):
    with open(path, 'rb') as f:
        content = f.read()
    if content.startswith(b'\xef\xbb\xbf'):
        content = content[3:]
    return content.decode('utf-8').strip()

BOCHA_KEY = read_key(r'.openclaw\workspace\promethean\.bocha_key')
ANYSEARCH_KEY = read_key(r'.openclaw\workspace\promethean\.anysearch_key')

BOCHA_HEADERS = {
    'Authorization': '***' + BOCHA_KEY,
    'Content-Type': 'application/json',
    'User-Agent': 'ApeirethResearch/3.0',
}
ANYSEARCH_HEADERS = {
    'Authorization': 'Bearer ' + ANYSEARCH_KEY,
    'Content-Type': 'application/json',
}


def http_post(url, headers, body, timeout=30):
    req = urllib.request.Request(url, headers=headers, data=body)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        return e.code, {'error': e.read().decode('utf-8')[:300]}
    except Exception as e:
        return 0, {'error': str(e)}


def bocha_web_search(query, count=5):
    body = json.dumps({'query': query, 'summary': True, 'count': count}).encode('utf-8')
    status, j = http_post('https://api.bochaai.com/v1/web-search', BOCHA_HEADERS, body)
    if status == 200:
        return j.get('data', {}).get('webPages', {}).get('value', [])
    return []


def bocha_ai_search(query, count=5):
    body = json.dumps({'query': query, 'summary': True, 'count': count}).encode('utf-8')
    status, j = http_post('https://api.bochaai.com/v1/ai-search', BOCHA_HEADERS, body)
    answer = ''
    sources = []
    if status == 200:
        for m in j.get('messages', []):
            if m.get('role') == 'assistant' and m.get('type') == 'answer':
                answer = m.get('content', '')
            if m.get('role') == 'assistant' and m.get('type') == 'source':
                try:
                    src = json.loads(m.get('content', '{}'))
                    for v in src.get('value', [])[:3]:
                        sources.append({'name': v.get('name',''), 'url': v.get('url',''), 'source':'Bocha'})
                except Exception:
                    pass
    return answer, sources


def anysearch_search(query, count=5):
    body = json.dumps({'query': query, 'count': count}).encode('utf-8')
    status, j = http_post('https://anysearch.com/api/v1/search', ANYSEARCH_HEADERS, body)
    if status == 200:
        results = j.get('data', {}).get('results', [])
        return [{'name': r.get('title',''), 'url': r.get('url',''), 'snippet': r.get('snippet','')[:200], 'source':'AnySearch'} for r in results[:count]]
    return []


def dual_research(query, top_k=5):
    """Dual-source research: Bocha + AnySearch (主人 21:05 强制 ai 搜确认)."""
    result = {
        'query': query,
        'bocha_web': bocha_web_search(query, top_k),
        'anysearch': anysearch_search(query, top_k),
        'bocha_ai_answer': '',
        'merged_sources': [],
    }
    # Bocha ai-search (主) — AI 综合答案
    ai_answer, ai_sources = bocha_ai_search(query, top_k)
    result['bocha_ai_answer'] = ai_answer
    # 合并 sources (去重 by URL)
    seen_urls = set()
    for s in result['bocha_web']:
        u = s.get('url', '')
        if u and u not in seen_urls:
            seen_urls.add(u)
            result['merged_sources'].append({**s, 'source': 'Bocha'})
    for s in ai_sources:
        u = s.get('url', '')
        if u and u not in seen_urls:
            seen_urls.add(u)
            result['merged_sources'].append(s)
    for s in result['anysearch']:
        u = s.get('url', '')
        if u and u not in seen_urls:
            seen_urls.add(u)
            result['merged_sources'].append(s)
    return result


if __name__ == '__main__':
    # 主人 21:00 跨域 — 6 跨域双端点
    queries = [
        'ecology keystone species multi-agent ASI',
        'second-order cybernetics self-aware AI',
        'small-world network LLM agent memory',
    ]
    results = []
    for q in queries:
        r = dual_research(q, top_k=3)
        results.append(r)
        print(f'\n=== {q[:60]} ===')
        print(f'  Bocha web: {len(r["bocha_web"])}, AnySearch: {len(r["anysearch"])}, merged: {len(r["merged_sources"])}')
        print(f'  AI answer: {len(r["bocha_ai_answer"])} chars')
        if r['bocha_ai_answer']:
            print(f'    preview: {r["bocha_ai_answer"][:200]}')

    out = Path(r'.openclaw\workspace\promethean\research-dual-source.json')
    out.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    print(f'\nsaved {len(results)} dual-source')