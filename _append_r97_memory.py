import json, time
# build round-97 summary section
d = json.load(open('research-v7-round-97.json',encoding='utf-8'))
lines = []
lines.append('')
lines.append('## Round-97 (2026-08-09 10:58 Asia/Shanghai, cron-every-2h)')
lines.append('')
lines.append('Self-decision: round-96 done ~2h3min ago (>30min), no conflict, run round-97. 23.2 sec, 12/12 OK.')
lines.append('')
lines.append('Theme: ASI substrate cross-domain — 7 cross-domain + 3 GitHub deep + 2 Gap (繁殖/可塑)')
lines.append('')
lines.append('### Queries')
for q in d.get('queries',[]):
    qid = q.get('id','?')
    domain = q.get('domain','?')
    gap = q.get('gap','?')
    qd = q.get('result',{})
    bw = qd.get('bocha_web', qd.get('web', []))
    if isinstance(bw, list):
        nw = len(bw)
    else:
        nw = 0
    ai_ans = qd.get('bocha_ai_answer', qd.get('ai_answer',''))
    if isinstance(ai_ans, str):
        nh = len(ai_ans)
        ans_preview = ai_ans[:200].replace('\n',' ')
    else:
        nh = 0; ans_preview = ''
    lines.append(f'- **{qid}** [{domain}] gap={gap}: web={nw} ai_chars={nh}')
    if ans_preview:
        lines.append(f'  - AI preview: {ans_preview}...')
lines.append('')
lines.append('### Key insights (preliminary)')
lines.append('- Turritopsis: biological immortality via transdifferentiation, life cycle reversal substrate')
lines.append('- Tardigrade: vitrification / trehalose as suspended animation substrate')
lines.append('- Ctenophora: independent nervous system origin, challenges neural-essentialist substrate')
lines.append('- PT-symmetric: balanced loss/gain extends QM to non-Hermitian regimes')
lines.append('- Topos: sheaf-based logic, internal language, category foundation for generalized reasoning')
lines.append('- Rewilding: top-down ecosystem restoration via keystone reintroduction')
lines.append('- Transgenerational epigenetic: 4 dimensions of heredity, beyond DNA-centric substrate')
lines.append('- nanoGPT: minimal clean GPT training substrate reference')
lines.append('- crewAI: role-based multi-agent orchestration substrate')
lines.append('- browser-use: DOM-grounded browser agent substrate')
lines.append('- Armadillo polyembryony: clonal mammalian reproduction substrate (gap R6)')
lines.append('- Astrocyte tripartite synapse: non-neuronal plasticity substrate (gap R11)')
lines.append('')
lines.append('### ASI north-star self-check')
lines.append('- ASI substrate (not ANI tool) yes')
lines.append('- Cross-domain (not single-domain) yes')
lines.append('- Self-evolving (not fixed) yes')
lines.append('- Any-LLM-strengthens yes (Apeireth DNA pattern)')
lines.append('- no Phenomenal pretense yes')
lines.append('- fact-based yes')
lines.append('- metaphor as tool yes')
lines.append('')

with open(r'.openclaw\workspace\memory\2026-08-09.md','a',encoding='utf-8') as f:
    f.write('\n'.join(lines))
print('appended round-97 section to memory/2026-08-09.md')
print('added',len(lines),'lines')
