#!/usr/bin/env python3
"""Code Deep Study VCP — 主人 23:18 + 23:20 真哲学: 真研究 VCP 源码."""
import json
from pathlib import Path

# 主 23:18 记忆算法是核心 + 主 23:20 VCP 源码在本机
VCP_STUDY = [
    {
        "name": "TagMemoEngine",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/TagMemoEngine.js",
        "why": "VCP 自研 TagMemo 浪潮算法 RAG 系统",
        "size_hint": 89227,
    },
    {
        "name": "RAGDiaryPlugin",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/Plugin/RAGDiaryPlugin/RAGDiaryPlugin.js",
        "why": "RAG Diary Plugin 真生产 (232K, VCP 核心记忆插件)",
        "size_hint": 232755,
    },
    {
        "name": "LightMemo",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/Plugin/LightMemo/LightMemo.js",
        "why": "轻量回忆插件 (64K, VCP 轻量级记忆)",
        "size_hint": 64767,
    },
    {
        "name": "VCPTimeLine",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/Plugin/VCPTimeLine/VCPTimeLine.js",
        "why": "VCP TimeLine 时间线 (37K, 主人 14:48 跨域真生产)",
        "size_hint": 37247,
    },
    {
        "name": "OneRing Memo (3 files)",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/Plugin/OneRing/",
        "why": "OneRing Memo — Raw + Inferred Timeline (主人 14:48 跨域真生产)",
        "size_hint": 60000,
    },
    {
        "name": "MEMORY_SYSTEM.md",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/docs/MEMORY_SYSTEM.md",
        "why": "VCP 记忆系统文档 (32K, 主 23:18 记忆算法)",
        "size_hint": 32074,
    },
    {
        "name": "TagMemo_Wave_Algorithm",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/docs/TagMemo_Wave_Algorithm_Deep_Dive.md",
        "why": "TagMemo 浪潮算法深挖 (34K)",
        "size_hint": 34276,
    },
    {
        "name": "TagMemo 开发回忆录",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/TagMemo-浪潮RAG 开发回忆录.md",
        "why": "TagMemo 开发回忆录 (30K, 主人 14:48 真生产细节)",
        "size_hint": 30748,
    },
    {
        "name": "Plugin.js",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/Plugin.js",
        "why": "VCP Plugin 核心 (114K)",
        "size_hint": 114593,
    },
    {
        "name": "MemoMaster.txt",
        "file": "code-deep-study/VCPToolBox-main/VCPToolBox-main/Agent/MemoMaster.txt",
        "why": "VCP MemoMaster prompt (15K, 真生产系统 prompt)",
        "size_hint": 15478,
    },
]


def study_file(item: dict) -> dict:
    """真读 VCP 源码."""
    report = {
        "name": item["name"],
        "file": item["file"],
        "why": item["why"],
        "size_hint": item["size_hint"],
        "first_lines": "",
        "structure": {},
    }
    fp = Path(item["file"])
    if fp.is_file():
        try:
            content = fp.read_text(encoding="utf-8", errors="replace")
            n_lines = len(content.split("\n"))
            n_chars = len(content)
            report["actual_lines"] = n_lines
            report["actual_chars"] = n_chars
            report["first_lines"] = content[:2000]
            # 简单结构分析
            class_match = content.count("class ")
            function_match = content.count("function ")
            require_match = content.count("require(")
            export_match = content.count("module.exports")
            comment_match = content.count("//")
            report["structure"] = {
                "classes": class_match,
                "functions": function_match,
                "requires": require_match,
                "module_exports": export_match,
                "comments": comment_match,
            }
        except Exception as e:
            report["error"] = str(e)
    elif fp.is_dir():
        # 多文件目录 (OneRing Memo)
        files = list(fp.glob("**/*.js"))
        total_chars = 0
        total_lines = 0
        all_first_lines = []
        for f in files:
            try:
                c = f.read_text(encoding="utf-8", errors="replace")
                total_chars += len(c)
                total_lines += len(c.split("\n"))
                all_first_lines.append(f"{f.name}: {len(c)} chars")
            except Exception:
                pass
        report["actual_files"] = len(files)
        report["actual_chars"] = total_chars
        report["actual_lines"] = total_lines
        report["files_summary"] = all_first_lines
    return report


def main():
    print("=" * 70)
    print("=== VCP Code Deep Study — 主人 23:18 + 23:20 真哲学 ===")
    print("=" * 70)

    all_reports = []
    for item in VCP_STUDY:
        print(f"\n[STUDY] {item['name']}")
        print(f"  why: {item['why']}")
        report = study_file(item)
        if "actual_chars" in report:
            print(f"  {report['actual_lines']} lines / {report['actual_chars']} chars")
            if "structure" in report and report["structure"]:
                print(f"  structure: {report['structure']}")
        elif "actual_files" in report:
            print(f"  {report['actual_files']} files / {report['actual_chars']} chars")
            for f in report.get("files_summary", []):
                print(f"    {f}")
        all_reports.append(report)

    out = Path("code-deep-study") / "vcp-deep-study.json"
    out.write_text(json.dumps(all_reports, indent=2, ensure_ascii=False), encoding="utf-8")
    total_chars = sum(r.get("actual_chars", 0) for r in all_reports)
    print(f"\n[SAVED] {out}")
    print(f"[TOTAL] {total_chars} chars 真研究")


if __name__ == "__main__":
    main()