import json
from pathlib import Path

log = Path(r'.openclaw\workspace\promethean\cron-research-runs.jsonl')
lines = log.read_text(encoding='utf-8').strip().split('\n')
last_two = lines[-2:]
for ln in last_two:
    e = json.loads(ln)
    print(f"r{e.get('round')} {e.get('action'):8s} {e.get('ts', 'N/A')}")

print()
print('Round 54 output:')
out = Path(r'.openclaw\workspace\promethean\research-v7-round-54.json')
print(f'  size: {out.stat().st_size} bytes')
data = json.loads(out.read_text(encoding='utf-8'))
print(f'  entries: {len(data)}')
print(f'  total merged sources: {sum(len(d["merged_sources"]) for d in data)}')

# 验证 12 生命特征覆盖
print()
print('Memory synced:')
mem = Path(r'.openclaw\workspace\memory\2026-07-31.md')
print(f'  memory size: {mem.stat().st_size} bytes')
content = mem.read_text(encoding='utf-8')
print(f'  Round 54 section present: {"Round-54 完成" in content}')