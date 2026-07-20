#!/usr/bin/env python3
"""主人 17:29 第二轮深度调研 — 高 leverage 项目深读.

执行方式: 直接 raw.githubusercontent.com GET, 绕过 GitHub API 限流.
"""
import urllib.request
import urllib.error
import json
import os
import sys
import time
from pathlib import Path

OUT = Path(r".openclaw\workspace\promethean\research-master-list-2026")
OUT.mkdir(parents=True, exist_ok=True)

# 主人清单 + 真名/候选名
TARGETS = [
    # === A. 认知/记忆 (L3 Memory) — 高 leverage ===
    ("MemPalace", ["MemPalace/mempalace", "memPalace/mempalace"]),
    # === B. Agent 框架 (L4-L5 Central AI / Self-organizing) ===
    ("simular-ai/Agent-S", ["simular-ai/Agent-S", "simularai/Agent-S", "simular-ai/agent-s"]),
    # === C. 待找真名的项目 ===
    ("Self-herness", ["self-herness/self-herness", "Self-Herness/self-herness", "herness-ai/herness"]),
    ("Openhuman", ["openhumanai/openhuman", "OpenHuman/openhuman", "openhuman-ai/openhuman"]),
    ("Dexter-AI", ["Dexter-AI/Dexter", "dexter-ai/dexter", "DexterAI/dexter"]),
    ("Odysseus", ["OdysseusAI/odysseus", "odysseus-ai/odysseus", "Odysseus-LLM/Odysseus"]),
    # === D. 技能 / Skills / Karpathy ===
    ("multice-ai/andrej-karpathy-skills", ["multice-ai/andrej-karpathy-skills"]),
    # === E. 工具链: OCR / 浏览器 / 时间序列 ===
    ("hyOCR1.5", ["hyocr1.5/hyocr", "HyOCR/HyOCR", "yuyz0112/hyocr", "hy-ocr/hyocr"]),
    ("Unlimited-OCR", ["unlimited-ocr/unlimited-ocr", "UnlimitedOCR/unlimited-ocr"]),
    ("camofox-browser", ["camofox-ai/camofox", "Camofox/camofox", "camofox-browser/camofox-browser"]),
    ("TimesFM", ["google-research/timesfm", "TimesFM/TimesFM"]),
    # === F. 模型 / 部署 ===
    ("lyogavin/airllm", ["lyogavin/airllm"]),
    ("avaiga/taipy", ["avaiga/taipy"]),
    # === G. Content extraction ===
    ("wechat-article-exporter", ["liaojieGitHub/wechat-article-exporter", "wechat-article-exporter/wechat-article-exporter"]),
    # === H. Misc ===
    ("pi-mono-badlogic", ["badlogic/pi-mono"]),  # already exists, re-check
    ("LangChain-openwiki", ["langchain-ai/openwiki", "langchain-ai/langgraph"]),
    ("m_flow", ["m-flow/m_flow", "mflow-ai/m_flow", "0xHoneyJar/m_flow"]),
    ("feynman", ["getcompanion-ai/feynman", "feynman-ai/feynman"]),
    ("getcompanion-ai/feynman", ["getcompanion-ai/feynman"]),
    # === I. Trading reference (主人 background) ===
    ("openbyteinc/quantdinger", ["OpenByteInc/QuantDinger", "openbyteinc/QuantDinger"]),
]


def fetch_raw(owner_repo: str, branch: str = "main") -> str | None:
    """Fetch README.md from raw.githubusercontent.com — no rate limit."""
    url = f"https://raw.githubusercontent.com/{owner_repo}/{branch}/README.md"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "ApeirethResearch/2.0"})
        with urllib.request.urlopen(req, timeout=20) as resp:
            data = resp.read()
            return data.decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        if e.code == 404:
            # try master branch
            if branch == "main":
                return fetch_raw(owner_repo, branch="master")
            return None
        return None
    except Exception as e:
        print(f"  ERR {owner_repo}: {e}")
        return None


def main():
    results = []
    not_found = []
    for label, candidates in TARGETS:
        print(f"\n=== {label} ===")
        got = None
        for owner in candidates:
            print(f"  trying {owner}...")
            got = fetch_raw(owner)
            if got:
                # save
                safe_name = label.replace("/", "_").replace(" ", "_")
                out_path = OUT / f"{safe_name}_README.md"
                out_path.write_text(got, encoding="utf-8")
                results.append({
                    "label": label,
                    "owner_repo": owner,
                    "path": str(out_path),
                    "len": len(got),
                })
                print(f"  OK {owner} -> {len(got)} chars -> {out_path.name}")
                break
        if not got:
            print(f"  NOT FOUND any candidate")
            not_found.append({"label": label, "tried": candidates})

    # save summary
    summary = {"found": results, "not_found": not_found}
    summary_path = OUT / "deep_list_results.json"
    summary_path.write_text(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"\n=== summary saved to {summary_path} ===")
    print(f"  found: {len(results)}/{len(TARGETS)}")
    print(f"  not_found: {len(not_found)}")
    for nf in not_found:
        print(f"    - {nf['label']}: tried {nf['tried']}")


if __name__ == "__main__":
    main()
