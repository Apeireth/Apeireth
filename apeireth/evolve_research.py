"""主人 16:33 '时刻搜索' — 搜 2026 最前沿 Self-Evolving / Harness / Meta-cognition

策略:
1. 搜 arXiv 2026 meta-cognition + self-evolving + harness
2. 搜 GitHub trending Rust AI agent 2026
3. 找 Darwin Gödel Machine / OpenSage / Voyager / Self-Harness 真证据
"""
import sys
import json
from pathlib import Path
sys.stdout.reconfigure(encoding='utf-8')
from apeireth import AnySearch, GitHubResearch

s = AnySearch()
g = GitHubResearch()

base = Path(r'.openclaw\workspace\apeireth\research-2026')
base.mkdir(exist_ok=True)

# === Phase A: arXiv 2026 论文调研 ===
print('=' * 70)
print('Phase A: arXiv 2026 self-evolving / harness / meta-cognition')
print('=' * 70)

queries = [
    'arxiv 2026 self-evolving LLM agent harness meta-cognition',
    'arxiv 2026 Darwin Godel Machine recursive self-improvement',
    'arxiv 2026 metacognition LLM agent reflection',
    'arxiv 2026 OpenSage LLM self-create agent',
    'arxiv 2026 self-harness LLM safety constraint',
    'arxiv 2026 AHE adaptive harness evolution agent',
    'arxiv 2026 continual learning agent without catastrophic forgetting',
    'arxiv 2026 Voyager skill library Minecraft agent',
]
all_papers = []
for q in queries:
    print(f'\n--- Q: {q} ---')
    r = s.search(q, max_results=5)
    if r['ok']:
        d = r['data']
        if isinstance(d, dict):
            for c in d.get('content', []):
                if c.get('type') == 'text':
                    text = c['text']
                    print(text[:1500])
                    all_papers.append({'q': q, 'text': text[:3000]})
                    break

# === Phase B: GitHub Rust AI 2026 ===
print()
print('=' * 70)
print('Phase B: GitHub trending Rust AI agent 2026')
print('=' * 70)

github_queries = [
    'github 2026 Rust self-evolving agent open source',
    'github Rust agent harness Claude production',
    'github Darwin Godel Machine implementation',
    'github Voyager Minecraft Rust port',
]
for q in github_queries:
    print(f'\n--- Q: {q} ---')
    r = s.search(q, max_results=3)
    if r['ok']:
        d = r['data']
        if isinstance(d, dict):
            for c in d.get('content', []):
                if c.get('type') == 'text':
                    print(c['text'][:1500])
                    break

# Save findings
(base / 'findings.json').write_text(json.dumps(all_papers, ensure_ascii=False, indent=2), encoding='utf-8')
print(f'\n\nSaved {len(all_papers)} papers to {base}/findings.json')