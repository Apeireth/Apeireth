import json
import time
import os
from datetime import datetime

ts = time.time()
ts_str = datetime.fromtimestamp(ts).strftime('%Y-%m-%dT%H:%M:%S+08:00')

# File sizes
size_51 = os.path.getsize('research-v7-round-51.json')
size_runner = os.path.getsize('round-51-runner.py')

log_entry = {
    'ts': ts,
    'ts_str': ts_str,
    'cron_id': 'd8c2b3c8-bb4a-466a-86fc-0fe95ae8bc1b',
    'round': 51,
    'action': 'done',
    'note': f'round-51 done, 12/12 hits via AnySearch, bw=0/ba=0/any=60 (173.6s). R6 繁殖 MISSING 接力 (gametogenesis) + R11 意识终极目标 (Penrose Orch-OR + Godel) + 7 跨域 (Bateson/Ashby/Penrose/Bohm/Bergson/Whitehead/Prigogine-Stengers) + 3 GitHub (openai-agents-python/browser-use/computer-use). Size: {size_51}B, runner: {size_runner}B. commit c80bab8.',
    'size': size_51,
    'runner_size': size_runner,
    'duration_s': 173.6,
    'commit_hash': 'c80bab8',
}

with open('cron-research-runs.jsonl', 'a', encoding='utf-8') as f:
    f.write(json.dumps(log_entry, ensure_ascii=False) + '\n')

print(f'Logged done: action=done, round=51, ts={ts_str}')
print(f'  size: {size_51}B, runner: {size_runner}B, duration: 173.6s, commit: c80bab8')
