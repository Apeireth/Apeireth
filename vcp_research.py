#!/usr/bin/env python3
"""VCPtoolbox 深挖 — 主人 23:18 真哲学: VCPtoolbox 记忆算法是核心."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 主人 23:18 真哲学: VCPtoolbox 自研算法, 记忆方面
# 主 20:22 "也别忽视 vcptoolbox" — 主人明示
# 主 23:10 "真研究代码" — 不只 README
# 主人 23:18 "互联网上的优质资源, 博查 AI 搜索" — 主 21:05 双端点
QUERIES = [
    'VCPToolBox vcptoolbox github memory algorithm',
    'VCPtoolbox 自研 记忆算法 源码',
    'VCPToolBox VCP AI memory architecture',
    'vcptoolbox 4 paradigms continuous_existence',
    'VCPtoolbox plugin memory DND mode',
    'VCPtoolbox FactTimeLine fact timeline memory',
    'VCPtoolbox GravityMemory gravity retrieval',
    'VCPtoolbox architecture continuous existence natural perception',
    'vcptoolbox 实现原理 自主生活 一体生态',
    'VCPtoolbox vcp tool box memory persistence',
    'vcptoolbox manifest plugin spec',
    'VCPtoolBox GitHub stars implementation detail',
]

results = []
for q in QUERIES:
    try:
        r = dual_research(q, top_k=5)
        results.append(r)
        print(f'OK: {q[:50]}')
    except Exception as e:
        print(f'ERR: {q[:50]} -- {e}')
        results.append({'query': q, 'error': str(e)})

with open('research-vcp-deep.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} VCP deep queries')