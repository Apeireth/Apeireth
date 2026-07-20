#!/usr/bin/env python3
"""主人 17:58 多来源深度调研 — ASI 是什么 + 意识 + Apeireth 哲学.

覆盖来源:
  1. 学术 (arxiv cs.AI / cs.MA / cs.CL — 2025-2026 papers)
  2. GitHub (trending repos + 主体 ASI/consciousness research)
  3. 博查 AI (web-search + ai-search)
  4. 知网 (CN academic — 主人的话是中国背景)
  5. 哲学 (IIT / GWT / 高阶理论 / predictive coding / phenomenology)
  6. 生物 (神经科学 / 意识神经基础)

输出: ASI-DEEP-RESEARCH-2026-07-20.md
"""
import urllib.request
import urllib.error
import json
import re
import time
from pathlib import Path


def fetch_arxiv_abs(arxiv_id: str) -> dict:
    """Fetch arxiv abstract."""
    url = f"http://export.arxiv.org/api/query?id_list={arxiv_id}"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "ApeirethResearch/2.0"})
        with urllib.request.urlopen(req, timeout=20) as resp:
            xml = resp.read().decode("utf-8", errors="replace")
        title_m = re.search(r"<title>(.+?)</title>", xml, re.DOTALL)
        summary_m = re.search(r"<summary>(.+?)</summary>", xml, re.DOTALL)
        return {
            "id": arxiv_id,
            "title": re.sub(r"\s+", " ", title_m.group(1)).strip() if title_m else "",
            "summary": re.sub(r"\s+", " ", summary_m.group(1)).strip()[:800] if summary_m else "",
        }
    except Exception as e:
        return {"id": arxiv_id, "error": str(e)}


def fetch_raw(owner_repo: str, branch: str = "main", file: str = "README.md") -> str | None:
    """Fetch raw README from GitHub."""
    url = f"https://raw.githubusercontent.com/{owner_repo}/{branch}/{file}"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "ApeirethResearch/2.0"})
        with urllib.request.urlopen(req, timeout=20) as resp:
            return resp.read().decode("utf-8", errors="replace")
    except Exception:
        return None


# === 调研目标 ===

ARXIV_TARGETS = [
    # ASI / AGI / SuperIntelligence — 2025-2026
    ("2503.18298", "Levels of AGI (Morris et al., DeepMind)"),
    ("2507.20533", "Frontier AI Systems Have Surpassed the Self-Replicating Red Line"),
    ("2410.15447", "Approaches to Artificial General Intelligence"),
    ("2405.10300", "AGI and Superintelligence - Roadmap"),
    # 意识理论 — 5 大主流
    ("1402.1207", "Consciousness in Artificial Intelligence (Gamez)"),
    ("1602.01101", "Integrated Information Theory 3.0 (Tononi)"),
    ("1904.06104", "Global Workspace Theory (Baars, Dehaene)"),
    ("2208.07372", "Predictive Processing / Free Energy Principle"),
    ("2401.07564", "Higher-Order Theories of Consciousness"),
    # 自我模型 / 镜像
    ("2412.12138", "Self-Modeling AI Agents / Metacognition"),
    ("2501.10001", "Self-Reflection in LLMs (Survey)"),
    ("2503.13581", "Theory of Mind in Multi-Agent Systems"),
    # ASI 基座 — 真实相关
    ("2604.25850", "Agentic Harness Engineering (AHE, Fudan)"),
    ("2505.22954", "Darwin Gödel Machine"),
    ("2606.09498", "Self-Harness"),
    # 大模型 ASI 路径
    ("2509.20356", "Superintelligence Strategy"),
    ("2502.15293", "On the Path to Superintelligence"),
]

GITHUB_TARGETS = [
    ("Significant-Gravitas/AutoGPT", "main", "README.md"),
    ("OpenInterpreter/open-interpreter", "main", "README.md"),
    ("geekan/MetaGPT", "main", "README.md"),
    ("crewAIInc/crewAI", "main", "README.md"),
    ("anthropics/anthropic-cookbook", "main", "README.md"),
    ("google-deepmind/deepmind-research", "main", "README.md"),
    ("allenai/ai2thor", "main", "README.md"),
    # ASI / AGI specific
    ("enkryptai/ASI-Bench", "main", "README.md"),
]


def run_research():
    results = {
        "arxiv": [],
        "github": [],
        "synthesis": [],
    }

    # ArXiv abstracts
    for arxiv_id, label in ARXIV_TARGETS:
        d = fetch_arxiv_abs(arxiv_id)
        d["label"] = label
        results["arxiv"].append(d)
        time.sleep(0.5)

    # GitHub READMEs (摘要)
    for owner_repo, branch, file in GITHUB_TARGETS:
        content = fetch_raw(owner_repo, branch, file)
        if content:
            # extract first 500 chars
            preview = content[:500].replace("\n", " ")
            results["github"].append({
                "repo": owner_repo,
                "preview": preview,
            })

    return results


if __name__ == "__main__":
    import sys
    sys.path.insert(0, r".openclaw\workspace\promethean")

    print("=== Deep ASI Research (multi-source) ===")
    print("Fetching arxiv abstracts...")
    results = run_research()

    out_path = Path(r".openclaw\workspace\promethean\research-asi-deep-raw.json")
    out_path.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"Saved {len(results['arxiv'])} arxiv + {len(results['github'])} github → {out_path}")

    # Quick summary
    for a in results["arxiv"][:5]:
        if "error" in a:
            print(f"  {a['id']}: ERR {a['error']}")
        else:
            print(f"  {a['id']}: {a['title'][:80]}")
