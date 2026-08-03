"""Quick endpoint probe — find which API endpoint works."""
import json
import urllib.request
import urllib.error
import ssl
import time

try:
    import certifi
    ca = certifi.where()
except ImportError:
    ca = None

api_key = open('.minimax_key', encoding='utf-8-sig').read().strip().split('\n')[0].strip()
print(f'API key prefix: {api_key[:18]}... (len={len(api_key)})')

endpoints = [
    ('https://api.minimaxi.com/v1/text/chatcompletion_v2', 'MiniMax'),
    ('https://api.MiniMax.chat/v1/chat/completions', 'MiniMax2'),
    ('https://api.deepseek.com/v1/chat/completions', 'deepseek'),
    ('https://api.openai.com/v1/chat/completions', 'openai'),
]

for url, name in endpoints:
    body = json.dumps({
        'model': 'MiniMax-M3',
        'messages': [{'role': 'user', 'content': 'ping'}],
        'max_tokens': 5,
        'temperature': 0
    }).encode('utf-8')
    req = urllib.request.Request(url, data=body, headers={
        'Authorization': 'Bearer ' + api_key,
        'Content-Type': 'application/json'
    }, method='POST')
    t0 = time.perf_counter()
    try:
        ctx = ssl.create_default_context(cafile=ca) if ca else None
        with urllib.request.urlopen(req, timeout=8, context=ctx) as r:
            data = r.read().decode('utf-8', 'replace')
            print(f'{name}: HTTP {r.status} ({int((time.perf_counter()-t0)*1000)}ms) -> {data[:120]}')
    except urllib.error.HTTPError as e:
        try:
            body_txt = e.read().decode('utf-8', 'replace')[:200] if e.fp else ''
        except Exception:
            body_txt = ''
        print(f'{name}: HTTP {e.code} ({int((time.perf_counter()-t0)*1000)}ms) -> {body_txt}')
    except Exception as e:
        print(f'{name}: ERR {type(e).__name__}: {str(e)[:200]}')