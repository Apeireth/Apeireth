import sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')

import httpx, codecs

# UTF-8-sig reads and strips BOM
with open(r'.openclaw\workspace\promethean\.minimax_key', 'r', encoding='utf-8-sig') as f:
    keys = [k.strip() for k in f.read().split() if k.strip()]

print(f'Found {len(keys)} keys')
for i, k in enumerate(keys):
    print(f'KEY{i+1} starts with: {k[:20]!r}')

base = 'https://api.minimax.chat/v1'

def try_request(label, hdr, url, method='GET', body=None):
    try:
        if method == 'GET':
            r = httpx.get(url, headers=hdr, timeout=10)
        else:
            r = httpx.post(url, headers=hdr, json=body, timeout=20)
        return f'{label}: HTTP {r.status_code} body={r.content[:300]!r}'
    except Exception as e:
        return f'{label}: ERR {type(e).__name__} {repr(str(e))[:200]}'

out = []
out.append(try_request('KEY1-GET-models', {'Authorization': 'Bearer ' + keys[0]}, base + '/models'))
out.append(try_request('KEY1-POST-text-minimax', {'Authorization': 'Bearer ' + keys[0]}, base + '/text/chatcompletion_v2', method='POST',
                  body={'model': 'MiniMax-Text-01', 'messages': [{'role': 'user', 'content': 'say hi'}], 'max_tokens': 20}))
out.append(try_request('KEY1-POST-chat-minimax', {'Authorization': 'Bearer ' + keys[0]}, base + '/chat/completions', method='POST',
                  body={'model': 'MiniMax-Text-01', 'messages': [{'role': 'user', 'content': 'say hi'}], 'max_tokens': 20}))
out.append(try_request('KEY1-POST-chat-minimax-M3', {'Authorization': 'Bearer ' + keys[0]}, base + '/chat/completions', method='POST',
                  body={'model': 'MiniMax-M3', 'messages': [{'role': 'user', 'content': 'say hi'}], 'max_tokens': 20}))
out.append(try_request('KEY1-POST-chat-abab6.5', {'Authorization': 'Bearer ' + keys[0]}, base + '/chat/completions', method='POST',
                  body={'model': 'abab6.5s-chat', 'messages': [{'role': 'user', 'content': 'say hi'}], 'max_tokens': 20}))
out.append(try_request('KEY2-GET-models', {'Authorization': 'Bearer ' + keys[1] if len(keys) > 1 else 'Bearer ' + keys[0]}, base + '/models'))

result = '\n'.join(out)
with open(r'.openclaw\workspace\promethean\probe_api.out', 'w', encoding='utf-8') as f:
    f.write(result)
print(result)
