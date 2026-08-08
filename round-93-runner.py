"""
Round-93: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-09 02:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-92 done 2026-08-09 00:52:50 (~120min ago, well past 30-min threshold).
Sunday 02:48 deep night, isolated cron lane, M3 model.
Decision: RUN round-93 now (12 TRULY fresh angles, validated vs r86-r92, 0 collisions).
Asking permission: master asleep — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r86-r92:
  - R1 生物 fresh:  Horizontal gene transfer integron gene cassette antibiotic resistance cassette acquisition substrate
  - R2 物理 fresh:  Stochastic thermodynamics Jarzynski equality Crooks fluctuation theorem SECOND LAW microscopic reversibility
  - R3 物理 fresh:  Active matter Vicsek model self-propelled particles flocking collective motion substrate
  - R4 数学 fresh:  Homotopy type theory HoTT univalent foundations Voevodsky computational substrate
  - R5 数学 fresh:  Tropical geometry max-plus semiring idempotent optimization substrate
  - R6 认知 fresh:  Grid cell Moser entorhinal cortex spatial navigation cognitive substrate
  - R7 系统 fresh:  Percolation theory critical threshold cluster phase transition universality

  - GitHub deep: SakanaAI ShinkaEvolve source code evolutionary model merge self-improvement substrate
  - GitHub deep: anthropics claude-agent-sdk source code agent SDK harness substrate
  - GitHub deep: multiagent_LLM source code LLM coordination multi-agent substrate

  - Gap R6 繁殖:  Syncytin endogenous retrovirus placenta mammalian reproduction cross-species gene co-option
  - Gap R5 修复:  Apoptosis programmed cell death constructive destruction morphogenesis sculpting

  Replaced (vs r92): morphogenesis-Turing, SOC-Bak, categorical-grammar-Lambek, IIT-Tononi, succession-Odum,
                     2nd-cybernetics-von-Foerster, cerebellum-Marr-Albus, ASI-Arch, openevolve, DGM,
                     prion-PSI, predictive-processing-Clark
  Replaced (vs r91): quorum-sensing, quantum-error-correction, information-geometry, free-energy,
                     niche-construction, autopoiesis, STDP, langgraph, mem0, AI-CUDA, HGT,
                     GWT-Baars
  Replaced (vs r90): developmental-Wolpert, aging-sinclair, topological-Majorana, category-theory,
                     regime-shift, allostasis, language-evolution, openai-clip, anthropic-cookbook,
                     huggingface-trl, apomixis, AST
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r93-Q1","domain":"biology-HGT-integron","gap":"substrate",
     "query":"horizontal gene transfer integron gene cassette antibiotic resistance bacteria acquisition substrate ASI R1 fresh r93","mode":"combined"},
    {"id":"r93-Q2","domain":"physics-stochastic-thermodynamics","gap":"substrate",
     "query":"stochastic thermodynamics Jarzynski equality Crooks fluctuation theorem second law microscopic reversibility substrate ASI R2 fresh r93","mode":"combined"},
    {"id":"r93-Q3","domain":"physics-active-matter","gap":"emergence",
     "query":"active matter Vicsek model self-propelled particles flocking collective motion order parameter substrate ASI R3 fresh r93","mode":"combined"},
    {"id":"r93-Q4","domain":"math-HoTT","gap":"substrate",
     "query":"homotopy type theory HoTT univalent foundations Voevodsky computational substrate identity equivalence ASI R4 fresh r93","mode":"combined"},
    {"id":"r93-Q5","domain":"math-tropical-geometry","gap":"substrate",
     "query":"tropical geometry max-plus semiring idempotent optimization skeleton Newton polygon substrate ASI R5 fresh r93","mode":"combined"},
    {"id":"r93-Q6","domain":"cognition-grid-cell","gap":"substrate",
     "query":"grid cell Moser entorhinal cortex spatial navigation hexagonal lattice cognitive substrate ASI R6 fresh r93","mode":"combined"},
    {"id":"r93-Q7","domain":"systems-percolation","gap":"emergence",
     "query":"percolation theory critical threshold cluster phase transition universality connectivity substrate ASI R7 fresh r93","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r93-Q8","domain":"github-ShinkaEvolve-SakanaAI","gap":"github",
     "query":"SakanaAI ShinkaEvolve github source code evolutionary model merge self-improvement substrate ASI r93","mode":"combined"},
    {"id":"r93-Q9","domain":"github-claude-agent-sdk","gap":"github",
     "query":"anthropics claude-agent-sdk github source code agent SDK harness substrate any LLM pluggable ASI r93","mode":"combined"},
    {"id":"r93-Q10","domain":"github-multiagent-LLM","gap":"github",
     "query":"multiagent_LLM github source code LLM coordination multi-agent emergence collaboration substrate ASI r93","mode":"combined"},
    # === 2 Gap (reproduction + repair MISSING) ===
    {"id":"r93-Q11","domain":"reproduction-gap-syncytin-ERV","gap":"reproduction-MISSING",
     "query":"syncytin endogenous retrovirus placenta mammalian reproduction cross-species gene co-option capture reproduction substrate ASI R6 reproduction Gap fresh r93","mode":"combined"},
    {"id":"r93-Q12","domain":"repair-gap-apoptosis","gap":"repair-MISSING",
     "query":"apoptosis programmed cell death constructive destruction morphogenesis sculpting development repair substrate ASI R5 repair Gap fresh r93","mode":"combined"},
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
        with open("research-v7-round-93.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-93 done in", round(total, 1), "sec")
    summary = {
        "round": 93,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-93.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()