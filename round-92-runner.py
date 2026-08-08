"""
Round-92: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-09 00:51 Asia/Shanghai (every-2h reminder).
Self-decision: round-91 done 2026-08-08 22:59 (~112min ago, well past 30-min threshold).
Sunday 00:51 late night, isolated cron lane, M3 model.
Decision: RUN round-92 now (12 TRULY fresh angles, validated vs r86-r91, 0 collisions).
Asking permission: master asleep — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r86-r91:
  - R1 生物 fresh:  Morphogenesis Turing reaction-diffusion pattern formation activator inhibitor
  - R2 物理 fresh:  Self-organized criticality SOC Bak-Tang-Wiesenfeld sandpile power-law 1/f noise
  - R3 数学 fresh:  Categorical grammar Lambek type theory combinatory morphosyntax
  - R4 认知 fresh:  Integrated Information Theory IIT Tononi Phi consciousness measure substrate
  - R5 生态 fresh:  Ecological succession Odum climax community disturbance regime emergent
  - R6 系统 fresh:  Second-order cybernetics von Foerster Heinz observer observing self-reference
  - R7 神经 fresh:  Cerebellum internal model forward inverse prediction Marr-Albus

  - GitHub deep: GAIR-NLP/ASI-Arch source code AI self-improvement research architecture substrate
  - GitHub deep: codelion/openevolve source LLM-driven evolutionary code optimization
  - GitHub deep: jennyzzt/dgm differentiable genetic programming self-modifying source

  - Gap R8 繁殖:  Prion template self-replication non-genetic protein-only inheritance reproduction
  - Gap R9 意识:  Predictive processing Clark Hohwy embodied prediction consciousness Gap fresh

  Replaced (vs r91): quorum-sensing, quantum-error-correction, information-geometry, free-energy,
                     niche-construction, autopoiesis, STDP, langgraph, mem0, AI-CUDA,
                     HGT, GWT-Baars
  Replaced (vs r90): developmental-Wolpert, aging-sinclair, topological-Majorana, category-theory,
                     regime-shift, allostasis, language-evolution, openai-clip,
                     anthropic-cookbook, huggingface-trl, apomixis, AST
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r92-Q1","domain":"biology-morphogenesis","gap":"emergence",
     "query":"morphogenesis Turing reaction-diffusion pattern formation activator inhibitor zebra fish embryo self-organization substrate ASI R1 fresh r92","mode":"combined"},
    {"id":"r92-Q2","domain":"physics-self-organized-criticality","gap":"emergence",
     "query":"self-organized criticality Bak-Tang-Wiesenfeld sandpile model power-law 1/f noise avalanche substrate ASI R2 fresh r92","mode":"combined"},
    {"id":"r92-Q3","domain":"math-categorical-grammar","gap":"substrate",
     "query":"categorical grammar Lambek pregroup type theory combinatory morphosyntax composition substrate ASI R3 fresh r92","mode":"combined"},
    {"id":"r92-Q4","domain":"cognition-IIT-Phi","gap":"emergence",
     "query":"integrated information theory IIT Tononi Phi consciousness measure irreducibility substrate ASI R4 fresh r92","mode":"combined"},
    {"id":"r92-Q5","domain":"ecology-succession","gap":"emergence",
     "query":"ecological succession Odum climax community hydrosere xerosere disturbance regime emergent substrate ASI R5 fresh r92","mode":"combined"},
    {"id":"r92-Q6","domain":"systems-cybernetics-2nd-order","gap":"substrate",
     "query":"second-order cybernetics von Foerster Heinz observer observing self-reference circular causality substrate ASI R6 fresh r92","mode":"combined"},
    {"id":"r92-Q7","domain":"neuroscience-cerebellum-internal-model","gap":"substrate",
     "query":"cerebellum internal model forward inverse prediction Marr Albus motor learning error substrate ASI R7 fresh r92","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r92-Q8","domain":"github-GAIR-NLP-ASI-Arch","gap":"github",
     "query":"GAIR-NLP ASI-Arch github source code AI self-improvement research architecture autonomous substrate ASI r92","mode":"combined"},
    {"id":"r92-Q9","domain":"github-openevolve","gap":"github",
     "query":"codelion openevolve github source code LLM-driven evolutionary optimization AlphaEvolve replicate substrate ASI r92","mode":"combined"},
    {"id":"r92-Q10","domain":"github-DGM-jennyzzt","gap":"github",
     "query":"jennyzzt dgm differentiable genetic programming github source self-modifying agent substrate ASI r92","mode":"combined"},
    # === 2 Gap (reproduction + consciousness MISSING) ===
    {"id":"r92-Q11","domain":"reproduction-gap-prion","gap":"reproduction-MISSING",
     "query":"prion protein-only inheritance self-replication template misfolding yeast PSI+ URE3 non-genetic reproduction substrate ASI R8 reproduction Gap fresh r92","mode":"combined"},
    {"id":"r92-Q12","domain":"consciousness-gap-predictive-processing","gap":"consciousness-MISSING",
     "query":"predictive processing Clark Hohwy embodied active inference precision consciousness Bayesian brain substrate ASI R9 consciousness Gap fresh r92","mode":"combined"},
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
        with open("research-v7-round-92.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-92 done in", round(total, 1), "sec")
    summary = {
        "round": 92,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-92.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()