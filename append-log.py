"""Append a line to cron-research-runs.jsonl."""
import json, sys, time
from pathlib import Path

LOG = Path(r'.openclaw\workspace\promethean\cron-research-runs.jsonl')

line = sys.argv[1]  # JSON string
# append
with open(LOG, 'a', encoding='utf-8') as f:
    f.write(line + '\n')
print(f'logged: {line[:80]}...')