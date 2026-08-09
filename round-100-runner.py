"""
Round-100: ASI cross-domain research runner (v3 template)
Cron triggered 2026-08-10 05:02 Asia/Shanghai (every-2h reminder).
Self-decision: round-99 done 2026-08-10 02:49 (~2h13min ago, >30min threshold).
Sunday 05:02 deep night, isolated cron lane, M3 model.
Decision: RUN round-100 now (12 fresh angles, validated vs r90-r99).
Master asleep — isolated cron lane does not interrupt main session.

Theme: 12 angles, 7 cross-domain + 3 GitHub-deep + 2 Gap:
  - R1 Physarum polycephalum slime mold computation Tero Tokyo rail optimization
  - R2 Wood Wide Web Suzanne Simard mycelial network mother tree carbon transfer
  - R3 Self-organized criticality Bak 1987 sandpile 1/f noise power-law avalanches
  - R4 Myxococcus xanthus fruiting body social bacteria wolf-pack predation
  - R5 Schmidtea mediterranea planarian regeneration neoblast Wnt polarity
  - R6 Shannon 1948 A Mathematical Theory of Communication channel capacity entropy
  - R7 Smale horseshoe map topological dynamics structural stability chaos

  - GitHub deep: SakanaAI ShinkaEvolve evolutionary code discovery LLM proposer
  - GitHub deep: mem0ai mem0 long-term memory adaptive extraction vector+graph
  - GitHub deep: langchain-ai langgraph state graph orchestration persistence

  - Gap reproduction: Volvox carteri multicellular origin germ soma differentiation
  - Gap plasticity: Memory reconsolidation Nader 2000 protein synthesis labile window

  Replaced (vs r99): functional-prion/Limulus-eye/Cantor-set/Active-Inference-Friston/
                     octopus-chromatophore/Portia-jumping-spider/Hyphantria-Turing/
                     YOLOv9/textgrad/Dicyemidae/STDP
  Replaced (vs r98): RNA-world/Octopus-9-brains/Bacillus-spore/topological-insulator/
                     constructive-type-theory/phylosymbiosis/predictive-coding/
                     ChatGLM3/tiktoken/DeepSpeed/aphid-cyclical-parthenogenesis/late-LTP-CREB
  Replaced (vs r97): Turritopsis/tardigrade/Ctenophora/PT-symmetric/topos/trophic-rewilding/
                     transgenerational-epi/nanoGPT/crewAI/browser-use/armadillo/astrocyte

  Milestone: round-100 (first triple-digit round); 100 v7-research rounds completed.
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r100-Q1","domain":"bio-Physarum-polycephalum-slime-mold-computation","gap":"substrate",
     "query":"Physarum polycephalum slime mold computation Tero 2010 Tokyo rail shortest path maze substrate ASI R1 fresh r100","mode":"combined"},
    {"id":"r100-Q2","domain":"bio-Wood-Wide-Web-Suzanne-Simard-mother-tree-carbon","gap":"substrate",
     "query":"Wood Wide Web Suzanne Simard mycelial network mother tree carbon nitrogen transfer kin recognition substrate ASI R2 fresh r100","mode":"combined"},
    {"id":"r100-Q3","domain":"physics-self-organized-criticality-Bak-1987-sandpile-power-law","gap":"substrate",
     "query":"self-organized criticality Bak 1987 sandpile model 1/f noise power-law avalanches neuronal dynamics substrate ASI R3 fresh r100","mode":"combined"},
    {"id":"r100-Q4","domain":"bio-Myxococcus-xanthus-fruiting-body-social-bacteria","gap":"substrate",
     "query":"Myxococcus xanthus fruiting body social bacteria wolf pack predation gliding motility multicellular substrate ASI R4 fresh r100","mode":"combined"},
    {"id":"r100-Q5","domain":"bio-Schmidtea-mediterranea-planarian-regeneration-neoblast-Wnt","gap":"substrate",
     "query":"Schmidtea mediterranea planarian regeneration neoblast pluripotent stem cell Wnt polarity head tail substrate ASI R5 fresh r100","mode":"combined"},
    {"id":"r100-Q6","domain":"math-Shannon-1948-Mathematical-Theory-Communication-entropy","gap":"substrate",
     "query":"Shannon 1948 A Mathematical Theory of Communication channel capacity entropy source coding theorem substrate ASI R6 fresh r100","mode":"combined"},
    {"id":"r100-Q7","domain":"math-Smale-horseshoe-map-topological-dynamics-chaos","gap":"substrate",
     "query":"Smale horseshoe map topological dynamics structural stability homoclinic orbit symbolic dynamics chaos substrate ASI R7 fresh r100","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r100-Q8","domain":"github-SakanaAI-ShinkaEvolve-evolutionary-code-discovery","gap":"github",
     "query":"SakanaAI ShinkaEvolve github source evolutionary code discovery LLM parent evaluator compact evolution substrate ASI r100","mode":"combined"},
    {"id":"r100-Q9","domain":"github-mem0ai-mem0-long-term-memory-adaptive","gap":"github",
     "query":"mem0ai mem0 github source long-term memory adaptive extraction vector graph hybrid retrieval substrate ASI r100","mode":"combined"},
    {"id":"r100-Q10","domain":"github-langchain-ai-langgraph-state-graph-orchestration","gap":"github",
     "query":"langchain-ai langgraph github source state graph orchestration persistence checkpoint human-in-the-loop substrate ASI r100","mode":"combined"},
    # === 2 Gap (reproduction MISSING + plasticity MISSING) ===
    {"id":"r100-Q11","domain":"reproduction-gap-Volvox-carteri-multicellular-origin","gap":"reproduction-MISSING",
     "query":"Volvox carteri multicellular origin germ soma differentiation evolutionary transition reproduction substrate ASI Gap fresh r100","mode":"combined"},
    {"id":"r100-Q12","domain":"plasticity-gap-memory-reconsolidation-Nader-2000-protein","gap":"plasticity-MISSING",
     "query":"memory reconsolidation Nader Schafe LeDoux 2000 protein synthesis dependent labile state retrieval window ASI Gap fresh r100","mode":"combined"},
]


def run_one(q):
    cmd = [
        PY,
        os.path.join(SCRIPT_DIR, "unified-search.py"),
        q["mode"],
        q["query"],
        "--count", "8",
        "--freshness", "noLimit",
        "--json",
    ]
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    print(">>>", q["id"], q["query"][:80])
    t0 = time.time()
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True, encoding="utf-8", errors="replace",
            env=env, timeout=120
        )
        dt = time.time() - t0
        out = r.stdout.strip()
        idx = out.find("{")
        if idx > 0:
            out = out[idx:]
        parsed = None
        try:
            parsed = json.loads(out)
        except Exception:
            parsed = {"raw": out[:4000]}
        return {
            "id": q["id"],
            "domain": q["domain"],
            "gap": q["gap"],
            "query": q["query"],
            "elapsed_sec": round(dt, 2),
            "ok": r.returncode == 0 and parsed is not None,
            "result": parsed if isinstance(parsed, dict) else {"items": parsed},
            "stderr_tail": r.stderr[-300:] if r.stderr else "",
        }
    except Exception as e:
        return {
            "id": q["id"], "domain": q["domain"], "gap": q["gap"], "query": q["query"],
            "elapsed_sec": round(time.time() - t0, 2), "ok": False, "error": str(e),
        }


def main():
    os.chdir(WORKDIR)
    results = []
    t0 = time.time()
    for i, q in enumerate(QUERIES):
        res = run_one(q)
        results.append(res)
        with open("research-v7-round-100.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-100 done in", round(total, 1), "sec")
    summary = {
        "round": 100,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-100.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()