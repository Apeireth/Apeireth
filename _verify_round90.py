import json, os
p = r'.openclaw\workspace\promethean\research-v7-round-90.json'
print('size:', os.path.getsize(p))
with open(p, encoding='utf-8') as f:
    d = json.load(f)
print('round:', d['round'], 'ts_iso:', d['ts_iso'], 'ok:', d['ok_count'], '/', d['queries_count'], 'sec:', d['total_sec'])
for q in d['queries']:
    name = q['query'][:60]
    print(f"  [{q['id']}] ok={q.get('ok')} elapsed={q.get('elapsed_sec')}s domain={q['domain']}")
    # spot check: count bocha_web hits
    res = q.get('result', {})
    sources = res.get('sources', {}) if isinstance(res, dict) else {}
    bw = sources.get('bocha_web', {})
    if isinstance(bw, dict):
        bw_pages = bw.get('result', {}).get('data', {}).get('webPages', {}).get('value', [])
        print(f"      bocha_web hits: {len(bw_pages)}")
    ba = sources.get('bocha_ai', {})
    if isinstance(ba, dict):
        ai_data = ba.get('result', {}).get('data', {}) if isinstance(ba.get('result'), dict) else {}
        if ai_data:
            print(f"      bocha_ai status: {ba.get('status')}")
