"""
Round-89: ASI cross-domain research runner (v2 template - utf-8 fixed)
Cron triggered 2026-08-08 18:55 Asia/Shanghai (every-2h reminder).
Self-decision: round-88 done 2026-08-08 16:57 (~118min ago, >30min threshold).
Saturday 18:55 evening, isolated cron lane, M3 model.
Decision: RUN round-89 now (12 TRULY fresh angles, validated vs r8-r88, 0 collisions).

Theme: 12 TRULY NEW angles — all 36 candidate keywords scanned clean vs r8-r88:
  - R1 化学 fresh: SOS DNA damage response LexA RecA lexA repressor
  - R2 群落 fresh: Biofilm c-di-GMP matrix adhesin Pseudomonas pellicle
  - R3 神经 fresh: Predictive coding hierarchical Bayesian cortical top-down precision
  - R5 应激 fresh: Integrated stress response ISR GCN2 eIF2alpha PERK IRE1
  - R7 免疫 fresh: Complement system MAC C3 C5 membrane attack complex opsonization
  - R8 植物 fresh: Circadian rhythm TOC1 LHY CCA1 phytochrome PRR
  - R10 系统论 fresh: Phase transition criticality Ising Hopfield attractor network

  - GitHub deep: Prefect pipeline ETL dataflow artifact scheduling
  - GitHub deep: openai structured-outputs JSON schema function-call strict
  - GitHub deep: vapi-ai voice agent conversational latent-space

  - Gap R6 繁殖:   Volvox germ-soma colony differentiation Carter
  - Gap R11 意识:  Metacognition self-monitoring consciousness Frith Norman

  Replaced from r87: Cas12a/Cas13a, gamma-delta-T, schema-integration, p62/angiopoietin, phyllotaxis,
                     syntrophy, BERTrend, gradio, chainlit, self-incompatibility-SI-RNase,
                     latent-inhibition
  Replaced from r88: HGT/integron, quorum sensing AI-2, hippocampal replay, HSP90/HSP70,
                     common mycorrhizal network, dissipative structure Prigogine, niche construction,
                     letta, openhands, dspy, hydra, IIT Tononi
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    {"id":"r89-Q1","domain":"biology-genetics","gap":"substrate",
     "query":"SOS DNA damage response LexA RecA lexA repressor automutagenesis substrate ASI R1 chemistry fresh r89","mode":"combined"},
    {"id":"r89-Q2","domain":"biology-microbiome","gap":"substrate",
     "query":"biofilm c-di-GMP matrix adhesin Pseudomonas pellicle substrate ASI R2 community fresh r89","mode":"combined"},
    {"id":"r89-Q3","domain":"cognitive-neuroscience","gap":"substrate",
     "query":"predictive coding hierarchical Bayesian cortical top-down precision weighting substrate ASI R3 neural fresh r89","mode":"combined"},
    {"id":"r89-Q4","domain":"biology-stress","gap":"substrate",
     "query":"integrated stress response ISR GCN2 eIF2alpha PERK IRE1 substrate ASI R5 stress fresh r89","mode":"combined"},
    {"id":"r89-Q5","domain":"biology-immunology","gap":"substrate",
     "query":"complement system MAC C3 C5 membrane attack complex opsonization substrate ASI R7 immune fresh r89","mode":"combined"},
    {"id":"r89-Q6","domain":"plant-circadian","gap":"substrate",
     "query":"circadian rhythm TOC1 LHY CCA1 phytochrome PRR substrate ASI R8 plant fresh r89","mode":"combined"},
    {"id":"r89-Q7","domain":"systems-theory","gap":"emergence",
     "query":"phase transition criticality Ising Hopfield attractor neural network substrate ASI R10 system fresh r89","mode":"combined"},
    {"id":"r89-Q8","domain":"github-prefect","gap":"github",
     "query":"prefecthq prefect github source pipeline ETL dataflow artifact scheduling workflow r89","mode":"combined"},
    {"id":"r89-Q9","domain":"github-openai-structured","gap":"github",
     "query":"openai structured-outputs github JSON schema function-call strict mode r89","mode":"combined"},
    {"id":"r89-Q10","domain":"github-vapi","gap":"github",
     "query":"vapi-ai vapi github source voice agent conversational latent-space realtime r89","mode":"combined"},
    {"id":"r89-Q11","domain":"reproduction-gap","gap":"reproduction-MISSING",
     "query":"Volvox germ-soma colony differentiation Carter soma flagellate substrate ASI R6 reproduction Gap fresh r89","mode":"combined"},
    {"id":"r89-Q12","domain":"consciousness-gap","gap":"consciousness-MISSING",
     "query":"metacognition self-monitoring consciousness Frith Norman neurophenomenology substrate ASI R11 consciousness Gap fresh r89","mode":"combined"},
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
        # find first { to skip any [INFO] lines
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
        with open("research-v7-round-89.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-89 done in", round(total, 1), "sec")
    summary = {
        "round": 89,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-89.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
