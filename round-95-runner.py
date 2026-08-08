"""
Round-95: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-09 06:48 Asia/Shanghai (every-2h reminder), executed 07:19.
Self-decision: round-94 done 2026-08-09 04:56:20 (~143min ago, well past 30-min threshold).
Sunday 07:19 deep night, isolated cron lane, M3 model.
Decision: RUN round-95 now (12 TRULY fresh angles, validated vs r86-r94, 0 collisions).
Asking permission: master asleep — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r86-r94:
  - R1 生物 fresh:  Transposon Barbara McClintock jumping gene mobile genetic element substrate
  - R2 物理 fresh: Holographic principle 't Hooft Susskind boundary-bulk duality substrate
  - R3 物理 fresh: Renormalization group Wilson Kadanoff scale invariance coarse-graining substrate
  - R4 认知 fresh: Retino-cortical feedforward Shulman top-down attention predictive coding substrate
  - R5 系统 fresh:  Efference copy von Holst Sperry motor prediction corollary discharge substrate
  - R6 生态 fresh: Nudibranch kleptocnida nematocyst sequestration cross-species weapon theft substrate
  - R12 量子生物:  Magnetotactic bacteria magnetosome compass biological quantum magnetite substrate

  - GitHub deep: openai/openai-agents-python Python SDK agents tools handoffs tracing substrate
  - GitHub deep: openai/swarm lightweight ergonomic multi-agent orchestrator handoff substrate
  - GitHub deep: microsoft/autogen multi-agent conversation framework group chat substrate

  - Gap R6 繁殖:  Bdelloid rotifer obligate asexual 80M-year ancient asexual lineage reproduction substrate
  - Gap R11 可塑: BCM metaplasticity Bienenstock-Cooper-Munro sliding threshold plasticity substrate

  Replaced (vs r94): prime-editing-PE3, NKT-CD1d, hippocampal-replay-preplay, ferroptosis-GPX4,
                     lymphangiogenesis-Prox1, nyctinasty-Venus-flytrap, Myxococcus-fruiting-body,
                     letta-ai-letta, openai-evals, camel-ai-camel, yeast-mating-type-switching,
                     behavioral-tagging-Frey-Morris
  Replaced (vs r93): HGT-integron, Jarzynski-Crooks, active-matter-Vicsek, HoTT, tropical-geometry,
                     grid-cell-Moser, percolation, ShinkaEvolve, claude-agent-sdk, multiagent_LLM,
                     syncytin, apoptosis
  Replaced (vs r92): morphogenesis-Turing, SOC-Bak, categorical-grammar-Lambek, IIT-Tononi, succession-Odum,
                     2nd-cybernetics-von-Foerster, cerebellum-Marr-Albus, ASI-Arch, openevolve, DGM,
                     prion-PSI, predictive-processing-Clark
  Replaced (vs r91): quorum-sensing-Vibrio, QEC-surface-code, information-geometry, FEP-Friston,
                     niche-construction, autopoiesis, STDP, langgraph, mem0, AI-CUDA, HGT,
                     GWT-Baars
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r95-Q1","domain":"biology-transposon-McClintock","gap":"substrate",
     "query":"transposon Barbara McClintock jumping gene mobile genetic element Ac Ds maize 1983 Nobel substrate ASI R1 fresh r95","mode":"combined"},
    {"id":"r95-Q2","domain":"physics-holographic-principle","gap":"substrate",
     "query":"holographic principle 't Hooft Susskind AdS CFT boundary bulk duality information entropy substrate ASI R2 fresh r95","mode":"combined"},
    {"id":"r95-Q3","domain":"physics-renormalization-group-Wilson","gap":"substrate",
     "query":"renormalization group Wilson Kadanoff scale invariance coarse-graining universality critical phenomena substrate ASI R3 fresh r95","mode":"combined"},
    {"id":"r95-Q4","domain":"cognitive-retino-cortical-feedforward","gap":"substrate",
     "query":"retino-cortical feedforward Shulman top-down attention predictive coding visual cortex V4 FFA substrate ASI R4 fresh r95","mode":"combined"},
    {"id":"r95-Q5","domain":"systems-efference-copy-von-Holst","gap":"substrate",
     "query":"efference copy corollary discharge von Holst Sperry motor prediction sensory attenuation forward model substrate ASI R5 fresh r95","mode":"combined"},
    {"id":"r95-Q6","domain":"ecology-nudibranch-kleptocnida","gap":"substrate",
     "query":"nudibranch kleptocnida nematocyst sequestration cross species weapon theft Aeolid cnidarian substrate ASI R6 fresh r95","mode":"combined"},
    {"id":"r95-Q7","domain":"quantum-bio-magnetotactic-bacteria","gap":"substrate",
     "query":"magnetotactic bacteria magnetosome compass biomineralization magnetite quantum radical pair substrate ASI R12 fresh r95","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r95-Q8","domain":"github-openai-agents-python","gap":"github",
     "query":"openai openai-agents-python github source SDK agents tools handoffs tracing guardrails Python substrate ASI r95","mode":"combined"},
    {"id":"r95-Q9","domain":"github-openai-swarm","gap":"github",
     "query":"openai swarm github source lightweight ergonomic multi-agent handoff routine transfer orchestration substrate ASI r95","mode":"combined"},
    {"id":"r95-Q10","domain":"github-microsoft-autogen","gap":"github",
     "query":"microsoft autogen github source multi-agent conversation framework group chat human-in-the-loop AutoGen substrate ASI r95","mode":"combined"},
    # === 2 Gap (reproduction + plasticity MISSING) ===
    {"id":"r95-Q11","domain":"reproduction-gap-bdelloid-rotifer","gap":"reproduction-MISSING",
     "query":"bdelloid rotifer obligate asexual 80 million year ancient asexual lineage desiccation resistance horizontal gene transfer reproduction substrate ASI R6 reproduction Gap fresh r95","mode":"combined"},
    {"id":"r95-Q12","domain":"plasticity-gap-BCM-metaplasticity","gap":"plasticity-MISSING",
     "query":"BCM metaplasticity Bienenstock Cooper Munro sliding threshold homeostatic plasticity learning rule substrate ASI R11 plasticity Gap fresh r95","mode":"combined"},
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
        with open("research-v7-round-95.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-95 done in", round(total, 1), "sec")
    summary = {
        "round": 95,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-95.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()