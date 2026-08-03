"""Probe MiniMax V2 endpoint with different model names."""
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
url = 'https://api.minimaxi.com/v1/text/chatcompletion_v2'

models = [
    'MiniMax-Text-01',
    'MiniMax-M',
    'MiniMax-M3',
    'MiniMax-01',
    'abab6.5s-chat',
    'abab6.5-chat',
    'abab5.5-chat',
]

for model in models:
    body = json.dumps({
        'model': model,
        'messages': [{'role': 'user', 'content': 'Reply with one word: hello'}],
        'max_tokens': 30,
        'temperature': 0,
    }).encode('utf-8')
    req = urllib.request.Request(url, data=body, headers={
        'Authorization': 'Bearer ' + api_key,
        'Content-Type': 'application/json'
    }, method='POST')
    t0 = time.perf_counter()
    try:
        ctx = ssl.create_default_context(cafile=ca) if ca else None
        with urllib.request.urlopen(req, timeout=10, context=ctx) as r:
            data = r.read().decode('utf-8', 'replace')
            d = json.loads(data)
            choices = d.get('choices', [])
            content = ''
            if choices:
                msg = choices[0].get('message', {})
                content = msg.get('content', '')
            base = d.get('base_resp', {})
            print(f'{model}: HTTP {r.status} ({int((time.perf_counter()-t0)*1000)}ms) content={content[:60]!r} base_resp_status_code={base.get("status_code")}')
    except urllib.error.HTTPError as e:
        try:
            txt = e.read().decode('utf-8', 'replace')[:200] if e.fp else ''
        except Exception:
            txt = ''
        print(f'{model}: HTTP {e.code} -> {txt}')
    except Exception as e:
        print(f'{model}: ERR {type(e).__name__}: {str(e)[:200]}')