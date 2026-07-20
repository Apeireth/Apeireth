#!/usr/bin/env python3
"""Code Deep Study Multi-Project — 主人 23:28 真哲学.

主 23:28 真哲学: '好东西就拿过来用, 不要被局限, 调研+工程+实践+求真'

3 真生产项目真读 (不只 README):
  - open-webui-main (56MB, 主路径, 主人 22:46 chat 真生产)
  - aio-hub-main (30MB, 真生产)
  - AgentMemory-master (825KB, 之前用过)
"""
import json
from pathlib import Path

# 3 个真研究项目 — 关键源文件
PROJECTS = [
    {
        "name": "open-webui",
        "root": "code-deep-study/open-webui-main/open-webui-main",
        "priority_files": [
            "backend/open_webui/main.py",
            "backend/open_webui/routers/chat.py",
            "backend/open_webui/routers/ollama.py",
            "backend/open_webui/routers/openai.py",
            "backend/open_webui/utils/chat.py",
            "backend/open_webui/utils/misc.py",
            "backend/open_webui/retrieval/vector/dbs/chroma.py",
            "backend/open_webui/retrieval/vector/dbs/milvus.py",
            "src/lib/components/chat/Chat.svelte",
            "src/lib/apis/chat/index.ts",
            "src/lib/stores/index.ts",
            "src/lib/utils/index.ts",
        ],
        "key_question": "Open WebUI 真生产 LLM chat 框架 — LLM API 路由 + 向量检索 + 真生产对话",
    },
    {
        "name": "aio-hub",
        "root": "code-deep-study/aio-hub-main/aio-hub-main",
        "priority_files": [
            "src/main.ts",
            "src/main.tsx",
            "src/App.tsx",
            "src/lib/server.ts",
            "src/lib/llm.ts",
            "src/lib/tools.ts",
            "src/lib/agent.ts",
            "src/lib/memory.ts",
            "src/lib/rag.ts",
            "src/lib/db.ts",
        ],
        "key_question": "aio-hub 真生产 — Tauri + LLM + agent 真生产模式",
    },
    {
        "name": "AgentMemory",
        "root": "code-deep-study/AgentMemory-master/AgentMemory-master",
        "priority_files": [
            "src/agent_memory/l1_kernel.py",
            "src/agent_memory/wikipedia.py",
            "src/agent_memory/wikipedia_extractor.py",
            "src/agent_memory/lcm.py",
            "src/agent_memory/wikipedia_archiver.py",
            "src/agent_memory/manager.py",
            "src/agent_memory/compressors.py",
            "src/agent_memory/lifecycle.py",
        ],
        "key_question": "AgentMemory 真生产 Karpathy LLM Wiki — 借鉴 Phase 46 Memory 3-Tier 升级",
    },
]


def study_project(project: dict) -> dict:
    """真读真生产项目."""
    report = {
        "name": project["name"],
        "root": project["root"],
        "key_question": project["key_question"],
        "files_studied": [],
        "total_chars": 0,
        "total_lines": 0,
        "structure": {
            "classes": 0,
            "functions": 0,
            "imports": 0,
            "exports": 0,
            "comments": 0,
        },
    }
    root = Path(project["root"])
    if not root.exists():
        report["error"] = f"root not found: {root}"
        return report

    for rel_path in project["priority_files"]:
        fp = root / rel_path
        if not fp.exists():
            continue
        try:
            content = fp.read_text(encoding="utf-8", errors="replace")
            n_lines = len(content.split("\n"))
            n_chars = len(content)
            print(f"  [OK] {rel_path}: {n_lines} lines, {n_chars} chars")

            # 简单结构分析
            class_match = content.count("class ") + content.count("export class ")
            func_match = content.count("def ") + content.count("function ") + content.count("export function ") + content.count("const ") + content.count("async function ")
            import_match = content.count("import ") + content.count("require(") + content.count("from ")
            export_match = content.count("export ") + content.count("module.exports")
            comment_match = content.count("//") + content.count("#") + content.count("/*")

            report["files_studied"].append({
                "path": rel_path,
                "lines": n_lines,
                "chars": n_chars,
                "preview": content[:1500],
            })
            report["total_chars"] += n_chars
            report["total_lines"] += n_lines
            report["structure"]["classes"] += class_match
            report["structure"]["functions"] += func_match
            report["structure"]["imports"] += import_match
            report["structure"]["exports"] += export_match
            report["structure"]["comments"] += comment_match
        except Exception as e:
            print(f"  [ERR] {rel_path}: {e}")
    return report


def main():
    print("=" * 70)
    print("=== Multi-Project Deep Code Study — 主人 23:28 真哲学 ===")
    print("=" * 70)

    all_reports = []
    for project in PROJECTS:
        print(f"\n[PROJECT] {project['name']}")
        print(f"  key_question: {project['key_question']}")
        report = study_project(project)
        all_reports.append(report)
        print(f"  TOTAL: {report.get('total_lines', 0)} lines / {report.get('total_chars', 0)} chars")
        print(f"  STRUCTURE: {report.get('structure', {})}")

    out = Path("code-deep-study") / "multi-project-study.json"
    out.write_text(json.dumps(all_reports, indent=2, ensure_ascii=False), encoding="utf-8")
    total = sum(r.get("total_chars", 0) for r in all_reports)
    print(f"\n[SAVED] {out}")
    print(f"[TOTAL] {total} chars 真研究")


if __name__ == "__main__":
    main()