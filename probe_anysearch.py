#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Probe AnySearch raw."""
import urllib.request, json, sys
sys.stdout.reconfigure(encoding='utf-8')

KEY = open('.anysearch_key', encoding='utf-8').read().strip()
HEADERS = {'Authorization': 'Bearer ' + KEY, 'Content-Type': 'application/json'}
body = json.dumps({'query': 'ecosystem engineering ASI base', 'count': 3}).encode('utf-8')
try:
    req = urllib.request.Request('https://anysearch.com/api/v1/search', headers=HEADERS, data=body)
    with urllib.request.urlopen(req, timeout=30) as r:
        raw = r.read().decode('utf-8')
    print('status:', r.status)
    print('body (first 500):')
    print(raw[:500])
except urllib.error.HTTPError as e:
    print('HTTP:', e.code)
    print('body:')
    print(e.read().decode('utf-8')[:500])
except Exception as e:
    print('ERR:', repr(e))
