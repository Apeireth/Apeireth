#!/usr/bin/env python3
"""Verify final state of round-56."""
import json, subprocess
from pathlib import Path
print('=== Final State Verification Round-56 ===')
print()
print('1. Output file:')
out = Path('research-v7-round-56.json')
print(f'   {out.name}: {out.stat().st_size} bytes')
print()
print('2. Memory sync:')
mem = Path(r'.openclaw\workspace\memory\2026-08-01.md')
print(f'   {mem.name}: {mem.stat().st_size} bytes')
print()
print('3. Cron log:')
log = Path('cron-research-runs.jsonl')
lines = log.read_text(encoding='utf-8').strip().split('\n')
r56_entries = [l for l in lines if '"round": 56' in l]
print(f'   Total lines: {len(lines)}')
print(f'   Round-56 entries: {len(r56_entries)}')
for entry in r56_entries:
    j = json.loads(entry)
    print(f'     action={j["action"]}, ts={j["ts"]}')
print()
print('4. Git:')
r = subprocess.run(['git', 'log', '--oneline', '-1'], capture_output=True, text=True)
print(f'   latest: {r.stdout.strip()}')