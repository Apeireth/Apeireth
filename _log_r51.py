import json
import time
from datetime import datetime

ts = time.time()
ts_str = datetime.fromtimestamp(ts).strftime('%Y-%m-%dT%H:%M:%S+08:00')

log_entry = {
    'ts': ts,
    'ts_str': ts_str,
    'cron_id': 'd8c2b3c8-bb4a-466a-86fc-0fe95ae8bc1b',
    'round': 51,
    'action': 'running',
    'note': 'round-51 start, next=51 from round_auto_naming.py, no conflict, r50 done 9h55m ago, fs healthy',
    'prev_round': 50,
    'prev_done_ts': 1785379907,
    'elapsed_h': (ts - 1785379907) / 3600,
}

with open('cron-research-runs.jsonl', 'a', encoding='utf-8') as f:
    f.write(json.dumps(log_entry, ensure_ascii=False) + '\n')

print(f'Logged running: {log_entry}')
