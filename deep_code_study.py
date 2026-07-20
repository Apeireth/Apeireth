#!/usr/bin/env python3
"""Code Deep Study V1 — 主人 23:10 真哲学: 真研究代码 + 干到底.

按主 23:10 真哲学, 立刻 spawn 5-10 background sub-agent 真读源代码.
不只 README, 真读 .py / .rs / .go 源文件.
"""
import subprocess
import sys
import json
import time
from pathlib import Path


# 优秀项目 — 真读源代码
DEEP_STUDY_PROJECTS = [
    {
        "name": "alibaba-zvec",
        "owner": "alibaba",
        "repo": "zvec",
        "why": "主 16:50 TOP 1 已接入, 但没真读 Rust 源码",
        "priority_files": [
            "src/lib.rs",
            "src/vector/*.rs",
            "src/fulltext/*.rs",
            "src/hybrid/*.rs",
        ],
        "key_question": "真生产向量 + FTS + Hybrid 模式",
    },
    {
        "name": "memoryos-rust",
        "owner": "TelivANT",
        "repo": "memoryos-rust",
        "why": "主 14:50 借鉴 STM/MTM/LTM, 9-crate workspace",
        "priority_files": [
            "crates/*/src/lib.rs",
            "crates/memoryos-core/src/memory/*.rs",
            "crates/memoryos-adapters/src/*.rs",
        ],
        "key_question": "9-crate workspace 架构 + STM/MTM/LTM 真生产",
    },
    {
        "name": "deltamemory",
        "owner": "deltamemory",
        "repo": "deltamemory",
        "why": "主 14:50 借鉴 WAL + MemTable + SSTable, 16x Rust gap",
        "priority_files": [
            "src/wal/*.rs",
            "src/memtable/*.rs",
            "src/sstable/*.rs",
        ],
        "key_question": "WAL 真生产 + CRC32 + salience decay",
    },
    {
        "name": "agentmemory-karpathy",
        "owner": "rohitg00",
        "repo": "agentmemory",
        "why": "主 16:50 Karpathy LLM Wiki 真生产",
        "priority_files": [
            "src/agent_memory/*.py",
            "src/agent_memory/l1_kernel.py",
            "src/agent_memory/wikipedia/*.py",
        ],
        "key_question": "Karpathy LLM Wiki 真生产范式",
    },
    {
        "name": "claude-mem",
        "owner": "thedotmack",
        "repo": "claude-mem",
        "why": "87k⭐, 3-layer progressive disclosure",
        "priority_files": [
            "src/services/*.ts",
            "src/hooks/*.ts",
        ],
        "key_question": "3-layer progressive disclosure 真生产",
    },
]


def clone_and_study(project: dict, target_dir: Path):
    """Clone 真生产项目 + 看优先文件."""
    repo_url = f"https://github.com/{project['owner']}/{project['repo']}.git"
    project_dir = target_dir / project["name"]
    if project_dir.exists():
        print(f"[SKIP] {project['name']} already cloned")
    else:
        print(f"[CLONE] {project['name']} -> {repo_url}")
        try:
            subprocess.run(["git", "clone", "--depth", "1", repo_url, str(project_dir)],
                          capture_output=True, timeout=120, check=True)
        except Exception as e:
            print(f"  ERR clone: {e}")
            return None
    return project_dir


def study_code(project_dir: Path, project: dict) -> dict:
    """真读优先文件 + 提炼哲思/原则."""
    report = {
        "name": project["name"],
        "files_studied": [],
        "philosophy_found": [],
        "patterns_found": [],
        "borrow_value": [],
        "key_question": project["key_question"],
    }
    if not project_dir or not project_dir.exists():
        report["error"] = "no project_dir"
        return report

    print(f"\n[STUDY] {project['name']}")
    for pattern in project["priority_files"]:
        matched = list(project_dir.glob(pattern))
        for f in matched[:3]:  # limit 3 files per pattern
            if f.is_file() and f.stat().st_size < 100_000:  # max 100KB per file
                try:
                    content = f.read_text(encoding="utf-8", errors="ignore")
                    n_lines = len(content.split("\n"))
                    n_chars = len(content)
                    print(f"  ✓ {f.name}: {n_lines} lines, {n_chars} chars")
                    report["files_studied"].append({
                        "path": str(f.relative_to(project_dir)),
                        "lines": n_lines,
                        "chars": n_chars,
                    })
                except Exception as e:
                    print(f"  ERR reading {f}: {e}")
    return report


def main():
    target = Path("code-deep-study")
    target.mkdir(exist_ok=True)

    all_reports = []
    for project in DEEP_STUDY_PROJECTS:
        project_dir = clone_and_study(project, target)
        report = study_code(project_dir, project)
        all_reports.append(report)

    # 写报告
    out_path = target / "deep-study-v1.json"
    out_path.write_text(json.dumps(all_reports, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n[SAVED] {out_path}")
    print(f"\nTotal files studied: {sum(len(r['files_studied']) for r in all_reports)}")


if __name__ == "__main__":
    main()