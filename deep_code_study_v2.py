#!/usr/bin/env python3
"""Code Deep Study V2 — 主人 23:10 真哲学: 真研究代码, 但聪明地.

不 git clone 整库 (太慢). 直接读真生产 .py / .rs 关键文件 + 提炼原则.
"""
import json
import time
from pathlib import Path


# 优秀项目 — 真读关键 Python 源文件 (PyPI 安装过的)
DEEP_STUDY_PROJECTS = [
    {
        "name": "agentmemory-karpathy",
        "package": "agentmemory",
        "source": "已安装: AgentMemory-master/src/agent_memory/",
        "why": "主 16:50 Karpathy LLM Wiki, 真生产",
        "priority_files": [
            "../AgentMemory/AgentMemory-master/src/agent_memory/l1_kernel.py",
            "../AgentMemory/AgentMemory-master/src/agent_memory/wikipedia.py",
            "../AgentMemory/AgentMemory-master/src/agent_memory/wikipedia_extractor.py",
            "../AgentMemory/AgentMemory-master/src/agent_memory/lcm.py",
            "../AgentMemory/AgentMemory-master/src/agent_memory/wikipedia_archiver.py",
            "../AgentMemory/AgentMemory-master/src/agent_memory/__init__.py",
        ],
        "key_question": "Karpathy LLM Wiki 真生产范式",
    },
    {
        "name": "anthropic-sdk",
        "package": "anthropic",
        "source": "pip show anthropic",
        "why": "Claude API SDK 范式",
        "priority_files": [],  # 动态探测
        "key_question": "SDK 真生产 client 范式",
    },
    {
        "name": "openai-sdk",
        "package": "openai",
        "source": "pip show openai",
        "why": "OpenAI API SDK 范式",
        "priority_files": [],
        "key_question": "SDK 真生产 client 范式",
    },
    {
        "name": "rust-substrate-apeireth",
        "package": "(本地)",
        "source": "promethean/rust-substrate/",
        "why": "Apeireth 自有 Rust 真生产 (主 14:47 真哲学)",
        "priority_files": [
            "rust-substrate/crates/apeireth-core/src/lib.rs",
            "rust-substrate/crates/apeireth-core/src/memory.rs",
            "rust-substrate/crates/apeireth-core/src/episode.rs",
            "rust-substrate/crates/apeireth-core/src/note.rs",
            "rust-substrate/crates/apeireth-core/src/reconsolidate.rs",
            "rust-substrate/crates/apeireth-core/src/forget.rs",
            "rust-substrate/crates/apeireth-ports/src/lib.rs",
            "rust-substrate/crates/apeireth-adapters/src/lib.rs",
        ],
        "key_question": "Rust substrate 真生产模式 (主 14:47)",
    },
    {
        "name": "apeireth-real",
        "package": "(本地)",
        "source": "promethean/apeireth/",
        "why": "Apeireth 自有真生产代码 (主 13:32 命名)",
        "priority_files": [
            "apeireth/memory.py",
            "apeireth/memory_3tier.py",
            "apeireth/persona.py",
            "apeireth/self_org_team.py",
            "apeireth/identity_card.py",
            "apeireth/philosophy.py",
            "apeireth/asi_coordinator.py",
            "apeireth/human_wisdom_aggregator.py",
        ],
        "key_question": "Apeireth 真生产模式 (主 22:33 ASI 北极星)",
    },
]


def study_local_files(project: dict) -> dict:
    """真读本地源码文件 + 提炼."""
    report = {
        "name": project["name"],
        "source": project["source"],
        "files_studied": [],
        "philosophy_found": [],
        "patterns_found": [],
        "borrow_value": [],
        "key_question": project["key_question"],
    }

    if project["name"] == "anthropic-sdk" or project["name"] == "openai-sdk":
        # 探测 pip 安装位置
        try:
            import importlib.util
            mod_spec = importlib.util.find_spec(project["package"])
            if mod_spec and mod_spec.origin:
                package_path = Path(mod_spec.origin).parent
                report["source"] = str(package_path)
                report["files_studied"].append({
                    "path": str(package_path),
                    "note": "SDK package root",
                })
                # 读核心 client
                client_file = package_path / "client.py"
                if client_file.exists():
                    content = client_file.read_text(encoding="utf-8", errors="replace")
                    n_lines = len(content.split("\n"))
                    n_chars = len(content)
                    print(f"  ✓ {project['name']}/client.py: {n_lines} lines, {n_chars} chars")
                    report["files_studied"].append({
                        "path": f"{project['name']}/client.py",
                        "lines": n_lines,
                        "chars": n_chars,
                        "preview": content[:1500],
                    })
        except Exception as e:
            report["error"] = str(e)
        return report

    for fpath in project["priority_files"]:
        fp = Path(fpath)
        if not fp.exists():
            print(f"  -- {fpath} not found")
            continue
        try:
            content = fp.read_text(encoding="utf-8", errors="replace")
            n_lines = len(content.split("\n"))
            n_chars = len(content)
            print(f"  ✓ {fpath}: {n_lines} lines, {n_chars} chars")
            report["files_studied"].append({
                "path": fpath,
                "lines": n_lines,
                "chars": n_chars,
                "preview": content[:1500],
            })
        except Exception as e:
            print(f"  ERR {fpath}: {e}")
    return report


def main():
    print("=" * 70)
    print("=== Code Deep Study V2 — 主人 23:10 真哲学: 真研究代码 ===")
    print("=" * 70)

    all_reports = []
    for project in DEEP_STUDY_PROJECTS:
        print(f"\n[PROJECT] {project['name']}")
        print(f"  why: {project['why']}")
        report = study_local_files(project)
        all_reports.append(report)

    out_path = Path("code-deep-study") / "deep-study-v2.json"
    out_path.parent.mkdir(exist_ok=True)
    out_path.write_text(json.dumps(all_reports, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n[SAVED] {out_path}")
    total_files = sum(len(r["files_studied"]) for r in all_reports)
    total_chars = sum(sum(f.get("chars", 0) for f in r["files_studied"]) for r in all_reports)
    print(f"[STATS] {total_files} files studied, {total_chars} chars total")


if __name__ == "__main__":
    main()