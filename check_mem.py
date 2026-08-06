import os, sys
sys.stdout.reconfigure(encoding='utf-8')
with open(r'.openclaw\workspace\memory\2026-07-30.md', 'r', encoding='utf-8') as f:
    content = f.read()
print(f'File size: {len(content)} chars')
print(f'Last 3000 chars:')
print(content[-3000:])
