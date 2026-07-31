import json
import os
import sys
import subprocess
sys.stdout.reconfigure(encoding='utf-8')

print('=== Round-51 Verification ===')
print()

# 1. research-v7-round-51.json
size = os.path.getsize('research-v7-round-51.json')
print(f'1. research-v7-round-51.json: {size} bytes')

with open('research-v7-round-51.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print(f'   entries: {len(data)}')

# 2. round-51-runner.py
runner_size = os.path.getsize('round-51-runner.py')
print(f'2. round-51-runner.py: {runner_size} bytes')

# 3. cron-research-runs.jsonl last 2
print(f'3. cron-research-runs.jsonl last 2:')
with open('cron-research-runs.jsonl', 'r', encoding='utf-8') as f:
    lines = [l for l in f if l.strip()]

for line in lines[-2:]:
    r = json.loads(line)
    ts_str = r.get('ts_str', '?')
    r_num = r.get('round', '?')
    action = r.get('action', '?')
    print(f'   - {ts_str} | round={r_num} | action={action}')

# 4. memory
mem_size = os.path.getsize('../memory/2026-07-30.md')
print(f'4. ../memory/2026-07-30.md: {mem_size} bytes (was 9618)')

# 5. git
out = subprocess.check_output(['git', 'log', '--oneline', '-3'], cwd='.').decode()
print('5. git log -3:')
print(out)
