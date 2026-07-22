"""Test MiniMax API endpoints."""
import httpx
import sys

sys.stdout.reconfigure(encoding='utf-8')

# Read both keys
with open(r'.openclaw\workspace\promethean\.minimax_key') as f:
    keys = [line.strip() for line in f if line.strip()]

print(f"Found {len(keys)} keys")

# Try MiniMax with different endpoints and headers
endpoints = [
    ('MiniMax v2', 'https://api.minimax.chat/v1/text/chatcompletion_v2', 'MiniMax-M3'),
    ('MiniMax v1', 'https://api.minimax.chat/v1/text/chatcompletion', 'MiniMax-M3'),
    ('MiniMax new', 'https://api.minimaxi.chat/v1/text/chatcompletion_v2', 'MiniMax-M3'),
]

for key_idx, key in enumerate(keys):
    print(f'\n=== Key {key_idx}: {key[:20]}...{key[-8:]} ===')
    for name, url, model in endpoints:
        try:
            r = httpx.post(url,
                headers={'Authorization': f'Bearer {key}'},
                json={'model': model, 'messages': [{'role': 'user', 'content': 'ping'}]},
                timeout=8)
            print(f'  {name}: status={r.status_code} body[:200]={r.text[:200]!r}')
        except Exception as e:
            print(f'  {name}: err={type(e).__name__}: {str(e)[:200]}')