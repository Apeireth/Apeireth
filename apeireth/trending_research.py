"""主人 16:38 — GitHub 周榜月榜 + 大模型前沿 + Hermes/Codex/Claude Code/vcptoolbox 深度调研

策略:
1. GitHub trending repos (general + AI/ML + Rust + Python)
2. 4 个指定项目深调研: Hermes Agent / Codex / Claude Code / vcptoolbox
3. arXiv 2026 最新大模型论文
4. VCP ecosystem 2026 进展
"""
import sys
import json
from pathlib import Path
sys.stdout.reconfigure(encoding='utf-8')
from apeireth import AnySearch, GitHubResearch

s = AnySearch()
g = GitHubResearch()

base = Path(r'.openclaw\workspace\apeireth\research-trending-2026')
base.mkdir(exist_ok=True)

findings = {}

# === A. GitHub 周榜月榜 ===
print('=' * 70)
print('A. GitHub Trending 周榜月榜 — 大模型 + AI Agent')
print('=' * 70)

trending_queries = [
    'GitHub trending weekly AI agent 2026',
    'GitHub trending monthly large language model 2026',
    'GitHub trending Rust AI agent 2026',
    'GitHub most starred AI agent framework 2026',
    'GitHub trending Anthropic Claude Code July 2026',
    'GitHub OpenAI Codex CLI agent 2026',
    'GitHub Hermes Agent Nous Research 2026',
    'GitHub vcptoolbox VCP 2026',
    'GitHub trending AI agent scaffold July 2026',
]
for q in trending_queries:
    print(f'\n--- Q: {q} ---')
    r = s.search(q, max_results=5)
    if r['ok']:
        d = r['data']
        if isinstance(d, dict):
            for c in d.get('content', []):
                if c.get('type') == 'text':
                    print(c['text'][:2000])
                    findings[q] = c['text'][:3000]
                    break

# === B. 4 个项目深调研 — 抓 README 实际内容 ===
print()
print('=' * 70)
print('B. 4 个项目深调研 — Hermes / Codex / Claude Code / vcptoolbox')
print('=' * 70)

projects = [
    ('Lumio-Research/hermes-agent-rs', 'Rust Hermes Agent (70⭐, 17 crates)'),
    ('NousResearch/hermes-agent', 'Python Hermes Agent'),
    ('openai/codex', 'OpenAI Codex CLI'),
    ('anthropics/claude-code', 'Claude Code (Anthropic)'),
    ('anthropics/skills', 'Claude Skills'),
    ('lmanhes/vcptoolbox', 'vcptoolbox'),
]
for repo, desc in projects:
    print(f'\n--- {desc} ({repo}) ---')
    md = g.fetch_readme(repo)
    if md and len(md) > 500:
        # 保存
        safe_name = repo.replace('/', '_')
        (base / f'{safe_name}_README.md').write_text(md, encoding='utf-8')
        print(f'  saved {len(md)} chars')
        # 显示前 1500
        print(md[:1500])
    else:
        print(f'  FAILED (len={len(md) if md else 0})')

# === C. arXiv 2026 大模型前沿 ===
print()
print('=' * 70)
print('C. arXiv 2026 大模型前沿论文')
print('=' * 70)

arxiv_qs = [
    'arxiv 2026 best LLM agent benchmark July',
    'arxiv 2026 frontier model reasoning capability',
    'arxiv 2026 Anthropic Claude technical report',
    'arxiv 2026 OpenAI o3 reasoning agent',
    'arxiv 2026 large language model agent survey',
]
for q in arxiv_qs:
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
(base / 'findings.json').write_text(json.dumps(findings, ensure_ascii=False, indent=2), encoding='utf-8')
print(f'\n\nSaved {len(findings)} findings to {base}/findings.json')
print(f'Saved README files: {sorted(p.name for p in base.glob("*README.md"))}')