"""Test bocha API."""
import httpx
import json
import sys

sys.stdout.reconfigure(encoding='utf-8')

key = 'sk-905b4204278848a08e84e2de0b570271'
r = httpx.post(
    'https://api.bochaai.com/v1/ai-search',
    headers={'Authorization': f'Bearer {key}', 'Content-Type': 'application/json'},
    json={'query': 'ASI artificial superintelligence safety alignment', 'summary': True, 'count': 3},
    timeout=15,
)
data = r.json()
print('=== Bocha response keys ===')
for k, v in data.items():
    if isinstance(v, (str, int, float, bool)):
        print(f'  {k}: {repr(v)[:200]}')
    elif isinstance(v, list):
        print(f'  {k}: list[{len(v)}]')
    elif isinstance(v, dict):
        print(f'  {k}: dict[{list(v.keys())[:10]}]')
print()
print('=== messages ===')
msgs = data.get('messages', [])
for m in msgs[:5]:
    if isinstance(m, dict):
        role = m.get('role')
        content = m.get('content', '')
        print(f'  role={role}, content[:300]={repr(content)[:300]}')