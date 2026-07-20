"""Inspect MemoryOS-Rust architecture — single place for each day."""
import sys
sys.stdout.reconfigure(encoding='utf-8')
from apeireth import GitHubResearch
from pathlib import Path

g = GitHubResearch()
base = Path(r'.openclaw\workspace\promethean\memoryos-inspect')
base.mkdir(exist_ok=True)

# 关键文件列表 — 看它的 core 怎么设计
files = [
    'README.md',
    'Cargo.toml',
    'crates/memoryos-core/src/lib.rs',
    'crates/memoryos-core/Cargo.toml',
    'crates/memoryos-core/src/memory/tier_manager.rs',
    'crates/memoryos-core/src/memory/stm.rs',
    'crates/memoryos-core/src/memory/mtm.rs',
    'crates/memoryos-core/src/memory/ltm.rs',
    'crates/memoryos-ports/src/lib.rs',
    'crates/memoryos-adapters/src/lib.rs',
    'crates/memoryos-gateway/src/main.rs',
]
for p in files:
    md = g.fetch_file('TelivANT/memoryos-rust', p)
    if md and len(md) > 80:
        # 安全路径
        safe = p.replace('/', '__')
        (base / safe).write_text(md, encoding='utf-8')
        print(f'  OK {p}: {len(md)} chars')
    else:
        print(f'  -- {p}')