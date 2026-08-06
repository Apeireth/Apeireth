import sys, re
sys.stdout.reconfigure(encoding='utf-8')
with open(r'.openclaw\workspace\memory\2026-07-30.md', 'r', encoding='utf-8') as f:
    content = f.read()

idx = content.find('round-51')
print(f'r51 idx: {idx}')
for m in re.finditer(r'(round-51|round-50|Round 51|Round 50)', content):
    s = max(0, m.start()-100)
    e = m.start()+200
    print(f'Found "{m.group()}" at {m.start()}: {content[s:e]}')
    print()
