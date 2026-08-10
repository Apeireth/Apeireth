"""Quick VCP GitHub fetch for V1432 source-code deep read."""
import urllib.request
import json

url = 'https://api.github.com/repos/Creed-Space/VCP-SDK/contents/'
req = urllib.request.Request(url, headers={'User-Agent': 'apeireth-v1432'})
try:
    r = urllib.request.urlopen(req, timeout=15)
    data = json.loads(r.read())
    for item in data:
        print(f"{item['type']:10s} {item['name']:50s} size={item.get('size', 'N/A')}")
except Exception as e:
    print('fail:', type(e).__name__, str(e)[:200])
