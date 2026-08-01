import json, sys
sys.stdout.reconfigure(encoding='utf-8')
lines = open('cron-research-runs.jsonl', encoding='utf-8').readlines()
print(f'Total log lines: {len(lines)}')
print('Last 4 log entries:')
for l in lines[-4:]:
    d = json.loads(l)
    rid = d.get('round')
    act = d.get('action')
    ts = d.get('ts_iso', 'N/A')[:19]
    print(f'- r{rid} action={act} ts={ts}')