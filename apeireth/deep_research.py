"""主人 14:52 '不计成本' — 立刻深度调研 8 个新 arXiv 论文 + 论文互引网络.

主人 14:48 '边写边搜论文, 联网查, GitHub钻研, 要聚集全人类的智慧'
主人 14:52 '最高深度, 最深刻优先'

策略:
1. 抓 8 个新 arXiv 论文 abstract
2. 找它们的 references/citations (互引网络)
3. 找 GitHub 上对应的官方实现
4. 综合写出我们的借鉴图谱
"""
import sys
sys.stdout.reconfigure(encoding='utf-8')
import re
from pathlib import Path
from apeireth import AnySearch, GitHubResearch

s = AnySearch()
g = GitHubResearch()

papers = [
    ('2603.07670', 'memory substrate Rust'),
    ('2605.18226', 'agent memory'),
    ('2602.21600', 'knowledge graph'),
    ('2604.11544', 'cognitive architecture'),
    ('2607.00151', 'emergence'),
    ('2605.30785', 'reasoning'),
    ('2602.11443', 'reflection metacognition'),
    ('2501.13956', 'Zep temporal knowledge graph'),
]

base = Path(r'.openclaw\workspace\promethean\arxiv-deep')
base.mkdir(exist_ok=True)

findings = []
for arxiv_id, tag in papers:
    print(f'\n=== {arxiv_id} ({tag}) ===')
    # 抓 abstract
    r = s.extract(f'https://arxiv.org/abs/{arxiv_id}')
    abstract = ''
    if r['ok']:
        d = r['data']
        if isinstance(d, dict):
            for c in d.get('content', []):
                if c.get('type') == 'text':
                    abstract = c['text']
                    break
    if not abstract:
        # fallback to abs 直接抓
        r2 = s.extract(f'https://arxiv.org/abs/{arxiv_id}.json')
        if r2['ok']:
            d = r2['data']
            if isinstance(d, dict):
                for c in d.get('content', []):
                    if c.get('type') == 'text':
                        abstract = c['text']
                        break
    if abstract:
        # 保存
        (base / f'{arxiv_id}.md').write_text(abstract, encoding='utf-8')
        # 摘要 (前 800 字)
        print(abstract[:800])
        findings.append({'id': arxiv_id, 'tag': tag, 'abstract': abstract[:2000]})
    else:
        print(f'  FAIL {arxiv_id}')

# Save 索引
(base / 'INDEX.json').write_text(
    __import__('json').dumps(findings, ensure_ascii=False, indent=2),
    encoding='utf-8'
)
print(f'\n\nSaved {len(findings)} papers to {base}')