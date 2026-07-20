"""主人 14:48 边写边搜 - 真调研 Rust agent / memory 生态

采集:
- arXiv papers on Rust memory + agent
- GitHub trending Rust AI projects
- Qdrant / Tantivy / DeltaMemory 源码深读
- Cargo crates 实测 benchmark
"""
import sys
sys.stdout.reconfigure(encoding='utf-8')
from apeireth import AnySearch, GitHubResearch
import json
from pathlib import Path

s = AnySearch()
g = GitHubResearch()

# 1) arXiv 2025-2026 真论文: Rust + memory + agent
print("=" * 60)
print("Phase A: 调研 arXiv 上 Rust + memory + agent 论文")
print("=" * 60)

queries = [
    'arxiv 2026 Rust memory agent substrate',
    'arxiv 2026 vector database HNSW performance comparison',
    'arxiv 2026 temporal knowledge graph agent memory',
    'arxiv 2026 LLM agent context management production',
]
findings = []
for q in queries:
    r = s.search(q, max_results=3)
    if r['ok']:
        data = r['data']
        if isinstance(data, dict):
            text = ''
            for c in data.get('content', []):
                if c.get('type') == 'text':
                    text = c['text']
            findings.append({'q': q, 'text': text[:500]})
            # 找 arxiv 链接
            import re
            arxiv_links = re.findall(r'arxiv\.org/[\w/.-]+', text)
            for link in arxiv_links[:2]:
                print(f"  - {link}")

# 2) GitHub trending Rust AI projects (不是 Generic, 必须是 2026)
print()
print("=" * 60)
print("Phase B: 搜 GitHub 2026 Rust AI 项目")
print("=" * 60)

# 用 general search 找 Rust AI 项目
r = s.search('github 2026 Rust AI agent memory production framework', max_results=5)
if r['ok']:
    data = r['data']
    if isinstance(data, dict):
        for c in data.get('content', []):
            if c.get('type') == 'text':
                text = c['text']
                print(text[:2000])

# 3) 重点深读 Qdrant / Tantivy / DeltaMemory 源码结构 (不看 README, 看 Cargo.toml + lib.rs)
print()
print("=" * 60)
print("Phase C: 深读 Cargo.toml + lib.rs 决定我们要不要 fork")
print("=" * 60)

for repo in ['qdrant/qdrant', 'quickwit-oss/tantivy']:
    cargo = g.fetch_file(repo, 'Cargo.toml')
    if cargo:
        # 抓核心依赖
        deps = [line.strip() for line in cargo.split('\n') if line.strip().startswith('[') or '=' in line and 'version' in line]
        print(f"--- {repo} ---")
        print('\n'.join(deps[:25]))

# Save findings
out = Path('research-rust-deep-dive.json')
out.write_text(json.dumps(findings, ensure_ascii=False, indent=2), encoding='utf-8')
print(f"\nSaved findings: {len(findings)} queries → research-rust-deep-dive.json")