import json
import time
from datetime import datetime

runs = []
with open('cron-research-runs.jsonl', 'r', encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        try:
            runs.append(json.loads(line))
        except Exception:
            pass

print(f'Total runs: {len(runs)}')
print()
print('Last 20 runs:')
for r in runs[-20:]:
    ts = r.get('ts', '?')
    if isinstance(ts, (int, float)):
        ts_str = datetime.fromtimestamp(ts).strftime('%Y-%m-%d %H:%M:%S')
    else:
        ts_str = str(ts)
    note = r.get('note', '')[:80]
    print(f'  - {ts_str} | round={r.get("round", "?")} | action={r.get("action", "?")} | {note}')

print()
now = time.time()
last_done_ts = None
for r in reversed(runs):
    if r.get('action') == 'done' and isinstance(r.get('ts'), (int, float)):
        last_done_ts = r['ts']
        break

if last_done_ts:
    elapsed_h = (now - last_done_ts) / 3600
    print(f'Hours since last done: {elapsed_h:.2f}h')
    print(f'  Last done at: {datetime.fromtimestamp(last_done_ts).strftime("%Y-%m-%d %H:%M:%S")}')
    print(f'  Now: {datetime.fromtimestamp(now).strftime("%Y-%m-%d %H:%M:%S")}')
