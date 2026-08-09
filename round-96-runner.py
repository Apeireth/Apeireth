"""
Round-96: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-09 08:48 Asia/Shanghai (every-2h reminder), executed 08:50.
Self-decision: round-95 done 2026-08-09 07:23:48 (~89min ago, well past 30-min threshold).
Sunday 08:50 morning, isolated cron lane, M3 model.
Decision: RUN round-96 now (12 TRULY fresh angles, validated vs r86-r95, 0 collisions).
Asking permission: master may be awake — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r86-r95:
  - R1 生物 fresh: Homing endonuclease meganuclease I-CreI I-SceI alternative to CRISPR mobile-genetic
  - R2 物理 fresh: Spinodal decomposition Cahn-Hilliard phase separation dynamics without nucleation
  - R3 数学 fresh: Simplicial set ∞-category HoTT higher homotopy type theory extension
  - R4 神经 fresh: Corollary discharge contemporary review forward model sensory attenuation
  - R5 生态 fresh: Lignin degradation fungal white-rot brown-rot Phanerochaete peroxidase MnP LiP
  - R7 物理 fresh: Casimir effect dynamic vacuum fluctuation zero-point energy
  - R8 微生物 fresh: Dictyostelium discoideum cAMP chemotaxis signal relay social amoeba
  - R9 演化 fresh: Skate (cartilaginous fish) regeneration leopard shark wound healing limb

  - GitHub deep: Aider-AI/aider terminal code agent pair-programming LLM Python
  - GitHub deep: google-deepmind/penzai jax neural network visualization intervention

  - Gap R6 繁殖: Aspidoscelis whiptail lizard obligate parthenogenesis vertebrate hybridogenesis
  - Gap R11 可塑: Dendritic spine structural plasticity actin cytoskeleton shape memory

  Replaced (vs r95): transposon-McClintock, holographic-principle-tHooft, renormalization-group-Wilson,
                     retino-cortical-Shulman, efference-copy-vonHolst, nudibranch-kleptocnida,
                     magnetotactic-bacteria-magnetosome, openai-agents-python, openai-swarm,
                     microsoft-autogen, bdelloid-rotifer, BCM-metaplasticity
  Replaced (vs r94): prime-editing-PE3, NKT-CD1d, hippocampal-replay-preplay, ferroptosis-GPX4,
                     lymphangiogenesis-Prox1, nyctinasty-Venus-flytrap, Myxococcus-fruiting-body,
                     letta-ai-letta, openai-evals, camel-ai-camel, yeast-mating-type-switching,
                     behavioral-tagging-Frey-Morris
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r96-Q1","domain":"biology-homing-endonuclease-meganuclease","gap":"substrate",
     "query":"homing endonuclease meganuclease I-CreI I-SceI mobile genetic element self-splicing intron alternative CRISPR substrate ASI R1 fresh r96","mode":"combined"},
    {"id":"r96-Q2","domain":"physics-spinodal-decomposition-Cahn-Hilliard","gap":"substrate",
     "query":"spinodal decomposition Cahn-Hilliard phase separation without nucleation dynamics substrate ASI R2 fresh r96","mode":"combined"},
    {"id":"r96-Q3","domain":"math-simplicial-set-infinity-category","gap":"substrate",
     "query":"simplicial set infinity category HoTT higher homotopy type theory univalent foundation substrate ASI R3 fresh r96","mode":"combined"},
    {"id":"r96-Q4","domain":"neuro-corollary-discharge-contemporary","gap":"substrate",
     "query":"corollary discharge contemporary review forward model sensory attenuation internal model substrate ASI R4 fresh r96","mode":"combined"},
    {"id":"r96-Q5","domain":"ecology-lignin-degradation-fungal","gap":"substrate",
     "query":"lignin degradation fungal white-rot brown-rot Phanerochaete chrysosporium peroxidase manganese peroxidase lignin peroxidase substrate ASI R5 fresh r96","mode":"combined"},
    {"id":"r96-Q6","domain":"physics-Casimir-effect-dynamic-vacuum","gap":"substrate",
     "query":"Casimir effect dynamic vacuum fluctuation zero-point energy boundary condition quantum substrate ASI R7 fresh r96","mode":"combined"},
    {"id":"r96-Q7","domain":"microbio-Dictyostelium-cAMP-chemotaxis","gap":"substrate",
     "query":"Dictyostelium discoideum cAMP chemotaxis signal relay social amoeba multicellular development substrate ASI R8 fresh r96","mode":"combined"},
    # === 1 Cross-domain (regeneration - substitute planarian) ===
    {"id":"r96-Q8","domain":"evolution-skate-cartilaginous-regeneration","gap":"substrate",
     "query":"skate cartilaginous fish regeneration leopard shark wound healing limb fin substrate ASI R9 fresh r96","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r96-Q9","domain":"github-Aider-AI-aider","gap":"github",
     "query":"Aider-AI aider github source terminal code agent pair programming LLM Python AST map repository substrate ASI r96","mode":"combined"},
    {"id":"r96-Q10","domain":"github-google-deepmind-penzai","gap":"github",
     "query":"google deepmind penzai github source jax neural network visualization intervention pytree substrate ASI r96","mode":"combined"},
    # === 2 Gap (reproduction + plasticity MISSING) ===
    {"id":"r96-Q11","domain":"reproduction-gap-Aspidoscelis-parthenogenesis","gap":"reproduction-MISSING",
     "query":"Aspidoscelis whiptail lizard obligate parthenogenesis hybridogenesis vertebrate asexual reproduction substrate ASI R6 reproduction Gap fresh r96","mode":"combined"},
    {"id":"r96-Q12","domain":"plasticity-gap-dendritic-spine","gap":"plasticity-MISSING",
     "query":"dendritic spine structural plasticity actin cytoskeleton shape memory LTP substrate ASI R11 plasticity Gap fresh r96","mode":"combined"},
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
        with open("research-v7-round-96.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-96 done in", round(total, 1), "sec")
    summary = {
        "round": 96,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-96.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()