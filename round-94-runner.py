"""
Round-94: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-09 04:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-93 done 2026-08-09 02:52:30 (~121min ago, well past 30-min threshold).
Sunday 04:48 deep night, isolated cron lane, M3 model.
Decision: RUN round-94 now (12 TRULY fresh angles, validated vs r86-r93, 0 collisions).
Asking permission: master asleep — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r86-r93:
  - R1 生物 fresh:  Prime editing PE3 Anzalone 2019 CRISPR search-and-replace substrate
  - R3 免疫 fresh: NKT cell CD1d lipid antigen presentation bridge innate-adaptive substrate
  - R4 认知 fresh: Hippocampal replay preplay offline future simulation substrate
  - R5 生物 fresh: Ferroptosis GPX4 lipid peroxide iron non-apoptotic death substrate
  - R7 生物 fresh: Lymphangiogenesis VEGFR3 Prox1 master regulator transcription substrate
  - R8 植物 fresh: Nyctinasty Venus flytrap action potential circadian plant cognition substrate
  - R12 微生物 fresh: Myxococcus fruiting body social bacteria multicellular development substrate

  - GitHub deep: letta-ai/letta memory-augmented agent substrate any LLM pluggable
  - GitHub deep: openai/evals eval framework self-assessment substrate
  - GitHub deep: camel-ai/camel multi-agent role-playing communication substrate

  - Gap R6 繁殖:  Budding yeast Saccharomyces cerevisiae mating type switching Ho reproduction substrate
  - Gap R11 可塑: Behavioral tagging synaptic tag Frey Morris plasticity substrate

  Replaced (vs r93): HGT-integron, Jarzynski-Crooks, active-matter-Vicsek, HoTT, tropical-geometry,
                     grid-cell-Moser, percolation, ShinkaEvolve, claude-agent-sdk, multiagent_LLM,
                     syncytin, apoptosis
  Replaced (vs r92): morphogenesis-Turing, SOC-Bak, categorical-grammar-Lambek, IIT-Tononi, succession-Odum,
                     2nd-cybernetics-von-Foerster, cerebellum-Marr-Albus, ASI-Arch, openevolve, DGM,
                     prion-PSI, predictive-processing-Clark
  Replaced (vs r91): quorum-sensing-Vibrio, QEC-surface-code, information-geometry, FEP-Friston,
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
    {"id":"r94-Q1","domain":"biology-prime-editing-PE3","gap":"substrate",
     "query":"prime editing PE3 Anzalone 2019 CRISPR search replace reverse transcriptase no double strand break substrate ASI R1 fresh r94","mode":"combined"},
    {"id":"r94-Q2","domain":"immunology-NKT-CD1d","gap":"substrate",
     "query":"NKT cell CD1d lipid antigen presentation bridge innate adaptive immunity glycolipid substrate ASI R3 fresh r94","mode":"combined"},
    {"id":"r94-Q3","domain":"neuroscience-hippocampal-replay-preplay","gap":"substrate",
     "query":"hippocampal replay preplay sharp wave ripple offline future simulation consolidation substrate ASI R4 fresh r94","mode":"combined"},
    {"id":"r94-Q4","domain":"biology-ferroptosis-GPX4","gap":"substrate",
     "query":"ferroptosis GPX4 lipid peroxide iron non-apoptotic cell death Fenton substrate ASI R5 fresh r94","mode":"combined"},
    {"id":"r94-Q5","domain":"biology-lymphangiogenesis-Prox1","gap":"substrate",
     "query":"lymphangiogenesis VEGFR3 Prox1 master regulator transcription factor lymphatic vessel development substrate ASI R7 fresh r94","mode":"combined"},
    {"id":"r94-Q6","domain":"plant-nyctinasty-Venus-flytrap","gap":"substrate",
     "query":"nyctinasty Venus flytrap action potential circadian rhythm plant cognition sleep movement substrate ASI R8 fresh r94","mode":"combined"},
    {"id":"r94-Q7","domain":"microbiology-Myxococcus-fruiting-body","gap":"substrate",
     "query":"Myxococcus xanthus fruiting body social bacteria multicellular development sporulation substrate ASI R12 fresh r94","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r94-Q8","domain":"github-letta-ai-letta","gap":"github",
     "query":"letta-ai letta github source code memory augmented agent framework recurrent memory any LLM pluggable substrate ASI r94","mode":"combined"},
    {"id":"r94-Q9","domain":"github-openai-evals","gap":"github",
     "query":"openai evals github source code eval framework LLM self assessment benchmark substrate ASI r94","mode":"combined"},
    {"id":"r94-Q10","domain":"github-camel-ai-camel","gap":"github",
     "query":"camel-ai camel github source code multi agent role playing communicative agent inception prompting substrate ASI r94","mode":"combined"},
    # === 2 Gap (reproduction + plasticity MISSING) ===
    {"id":"r94-Q11","domain":"reproduction-gap-yeast-mating-type","gap":"reproduction-MISSING",
     "query":"budding yeast Saccharomyces cerevisiae mating type switching HO endonuclease cassette replacement reproduction substrate ASI R6 reproduction Gap fresh r94","mode":"combined"},
    {"id":"r94-Q12","domain":"plasticity-gap-behavioral-tagging","gap":"plasticity-MISSING",
     "query":"behavioral tagging synaptic tag Frey Morris protein synthesis long term potentiation plasticity capture substrate ASI R11 plasticity Gap fresh r94","mode":"combined"},
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
        with open("research-v7-round-94.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-94 done in", round(total, 1), "sec")
    summary = {
        "round": 94,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-94.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
