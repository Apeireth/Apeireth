#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Fast dual-source research: Bocha web (primary) + AnySearch (timeout-fallback).

AnySearch is currently slow (~28s/query). Round-79 timing optimization:
- Bocha web primary (1.15s reliable)
- AnySearch with shorter timeout (5s) - fallback only
- Skip Bocha AI (403 quota exhausted)
"""
from __future__ import annotations
import json, sys, urllib.request, urllib.error
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')

def read_key(path):
    with open(path, 'rb') as f:
        content = f.read()
    if content.startswith(b'\xef\xbb\xbf'):
        content = content[3:]
    return content.decode('utf-8').strip()

BOCHA_KEY = read_key(r'.openclaw\workspace\promethean\.bocha_key')
ANYSEARCH_KEY = read_key(r'.openclaw\workspace\promethean\.anysearch_key')

BOCHA_HEADERS = {
    'Authorization': 'Bearer ' + BOCHA_KEY,
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


def bocha_web_search(query, count=5, timeout=15):
    body = json.dumps({'query': query, 'summary': True, 'count': count}).encode('utf-8')
    status, j = http_post('https://api.bochaai.com/v1/web-search', BOCHA_HEADERS, body, timeout=timeout)
    if status == 200:
        return j.get('data', {}).get('webPages', {}).get('value', [])
    return []


def anysearch_search(query, count=5, timeout=8):
    """AnySearch with shorter timeout - if slow, return []."""
    body = json.dumps({'query': query, 'count': count}).encode('utf-8')
    status, j = http_post('https://anysearch.com/api/v1/search', ANYSEARCH_HEADERS, body, timeout=timeout)
    if status == 200:
        results = j.get('data', {}).get('results', [])
        return [{'name': r.get('title',''), 'url': r.get('url',''), 'snippet': r.get('snippet','')[:200], 'source':'AnySearch'} for r in results[:count]]
    return []


def dual_research_fast(query, top_k=5):
    """Bocha web primary, AnySearch short-timeout fallback."""
    bw = bocha_web_search(query, top_k)
    any_n = anysearch_search(query, top_k, timeout=8)
    
    # Merge sources (dedup by URL)
    seen_urls = set()
    merged = []
    for s in bw:
        u = s.get('url', '')
        if u and u not in seen_urls:
            seen_urls.add(u)
            merged.append({**s, 'source': 'Bocha'})
    for s in any_n:
        u = s.get('url', '')
        if u and u not in seen_urls:
            seen_urls.add(u)
            merged.append(s)
    
    return {
        'query': query,
        'bocha_web': bw,
        'anysearch': any_n,
        'bocha_ai_answer': '',
        'merged_sources': merged,
    }