import json, sys, os, re
sys.stdout.reconfigure(encoding='utf-8')

# Look at all rounds to see coverage of key topics
keywords_to_check = [
    ('mechanotransduction', 'R7 应激'),
    ('Piezo', 'R7 应激'),
    ('proprioception', 'R7 应激'),
    ('focal adhesion', 'R7 应激'),
    ('apoptosis', 'R3 死亡'),
    ('caspase', 'R3 死亡'),
    ('efferocytosis', 'R3 死亡'),
    ('programmed cell death', 'R3 死亡'),
    ('Hox', 'R2 发育'),
    ('homeotic', 'R2 发育'),
    ('bicoid', 'R2 发育'),
    ('cytoplasmic determinants', 'R2 发育'),
    ('epigenetic', 'R9 遗传'),
    ('transgenerational', 'R9 遗传'),
    ('imprinting', 'R9 遗传'),
    ('planarian', 'R5 修复'),
    ('hydra', 'R5 修复'),
    ('neoblast', 'R5 修复'),
    ('morphallaxis', 'R5 修复'),
    ('epimorphosis', 'R5 修复'),
    ('niche construction', 'R12 生态'),
    ('Odling-Smee', 'R12 生态'),
    ('extended phenotype', 'R12 生态'),
    ('flagellar motor', 'R8 运动'),
    ('kinesin', 'R8 运动'),
    ('molecular motors', 'R8 运动'),
    ('global workspace', 'R11 意识'),
    ('Baars', 'R11 意识'),
    ('Dehaene', 'R11 意识'),
    ('claude-agent-sdk', 'GitHub'),
    ('mem0', 'GitHub'),
    ('HarnessAgent', 'GitHub'),
    ('multiagent_LLM', 'GitHub'),
    ('telomere', 'R4 衰老'),
    ('Hayflick', 'R4 衰老'),
    ('senescence', 'R4 衰老'),
    ('chemolithotrophy', 'R0 新陈代谢'),
]

for kw, label in keywords_to_check:
    hits = []
    for r in range(1, 59):
        p = f'research-v7-round-{r}.json'
        if not os.path.exists(p): continue
        try:
            d = json.load(open(p, encoding='utf-8'))
        except: continue
        for q in d:
            if kw.lower() in q['query'].lower():
                hits.append(r)
                break
    status = f"covered r{hits}" if hits else "❌ MISSING"
    print(f'{kw:35} {label:15} {status}')