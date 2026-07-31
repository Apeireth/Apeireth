import json
from pathlib import Path
import time

log_path = Path('cron-research-runs.jsonl')
if log_path.exists():
    lines = log_path.read_text(encoding='utf-8').strip().split('\n')
    for line in lines[-12:]:
        try:
            d = json.loads(line)
            ts = d.get('ts', '?')
            if isinstance(ts, (int, float)):
                ts_str = time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(ts))
            else:
                ts_str = str(ts)
            note = d.get('note','')
            print(ts_str, '| round=', d.get('round','?'), '| action=', d.get('action','?'), '| note=', note[:80])
        except Exception as e:
            print('parse err:', e, ':', line[:100])
else:
    print('no log yet')