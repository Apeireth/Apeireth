import json
r=json.load(open(r'.openclaw\workspace\promethean\research-v7-round-62.json',encoding='utf-8'))
print('=== round-62 queries ===')
for i, item in enumerate(r, 1):
    q = item.get('query','')
    core = q.split(' substrate ASI')[0].split(' canonical ')[0].split(' 第')[0]
    bw = len(item.get('bocha_web',[]))
    any_ = len(item.get('anysearch',[]))
    ba = 'yes' if item.get('bocha_ai_answer','') else 'no'
    print(f'{i:2d}. [{bw}bw/{any_}any/ba={ba}] {core}')
