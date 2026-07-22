import json, sys
p = sys.argv[1]
d = json.load(open(p, encoding='utf-8'))
print(f'round {p}: {len(d)} queries')
for i, q in enumerate(d, 1):
    print(f' {i:2d}. {q["query"]}')
    # Show source counts
    bw = len(q.get('bocha_web') or [])
    asrc = len(q.get('anysearch') or [])
    ba = (q.get('bocha_ai_answer') or '')[:60]
    print(f'     bocha_web={bw} anysearch={asrc} ai_answer_len={len(q.get("bocha_ai_answer") or "")}')