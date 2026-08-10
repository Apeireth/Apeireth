"""Fetch VCP __init__.py."""
import urllib.request
import json
import base64

url = 'https://api.github.com/repos/Creed-Space/VCP-SDK/contents/python/src/vcp/__init__.py'
req = urllib.request.Request(url, headers={'User-Agent': 'apeireth-v1432'})
try:
    r = urllib.request.urlopen(req, timeout=30)
    data = json.loads(r.read())
    content = base64.b64decode(data['content']).decode('utf-8')
    print(content[:5000])
    print(f'\\n\\n=== Total size: {len(content)} chars ===')
except Exception as e:
    print('fail:', type(e).__name__, str(e)[:200])
