"""主人 00:29 真务实修调研型任务 — 不调 LLM 跑 research, 直接用 background Python 跑.

主 00:29 真修:
- v2/v3 调 LLM 跑研究 = 4-5 分钟 timeout
- 主人真务实: 直接调 Python 不经 LLM, 让 research 真的跑通
- 主 22:40 自决 + 主 22:52 调研不停
"""
import sys
import json
import time
from pathlib import Path

# 加 apeireth
sys.path.insert(0, str(Path(__file__).parent.parent))
from deep_research_dual import dual_research


def run_research_round(round_n: int, queries: list, top_k: int = 3) -> dict:
    """真生产调研一轮 — 不调 LLM,直接调 Bocha + AnySearch."""
    results = []
    started = time.time()
    for q in queries:
        try:
            r = dual_research(q, top_k=top_k)
            results.append(r)
            print(f"  [OK] {q[:60]}")
        except Exception as e:
            print(f"  [ERR] {q[:60]}: {e}")
            results.append({"query": q, "error": str(e)})
    duration = time.time() - started

    out_path = Path(f"research-v7-round-{round_n}.json")
    out_path.write_text(
        json.dumps(results, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    print(f"\n  [SAVED] {out_path}  ({out_path.stat().st_size} bytes, {duration:.1f}s)")
    return {
        "round": round_n,
        "n_queries": len(queries),
        "duration_s": duration,
        "n_saved": len(results),
        "out": str(out_path),
    }


NIGHT_QUERIES = [
    # 主 22:33 ASI 北极星 + 主 23:50 抓紧干 + 主 22:52 调研不停
    "ASI architecture real production 2026",
    "self-improving AI agent recursive 2026",
    "Apeireth ASI base agent foundation 2026",
    "multi-agent real world deployment 2026",
    "memory system production grade 2026",
    "tool use LLM function calling 2026",
    "context window million token 2026",
    "long horizon agentic reasoning 2026",
]


if __name__ == "__main__":
    import os
    round_n = int(os.environ.get("ROUND_N", "12"))
    print(f"=" * 60)
    print(f"主人 00:29 真务实: 调研型任务 round-{round_n} (不调 LLM, 直接调 Python)")
    print(f"=" * 60)
    result = run_research_round(round_n, NIGHT_QUERIES, top_k=3)
    print(f"\n  [DONE] round-{round_n}: {result}")