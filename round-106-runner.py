"""
Round-106: ASI cross-domain research runner (v3 template, combined mode)
Cron triggered 2026-08-10 16:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-105 done 2026-08-10 02:07 UTC (~6h41min ago, >30min threshold).
Monday 16:48 work-hour, isolated cron lane, M3 model.
Decision: RUN round-106 now (12 fresh angles, validated vs r101-r105).

✅ Bocha web + Bocha AI both 200 (master 14:58 立规: 博查主用, 该用不要吝啬)
   - unified-search.py combined mode: 博查 web + 博查 AI → fallback AnySearch → fallback Brave
   - all 12 queries use combined mode for richest cross-source coverage

Theme: 12 angles, 7 cross-domain (substrate) + 3 GitHub-deep (source code) + 2 Gap (MISSING):
  - R1 Bio-TempComp: circadian/circadian-clock temperature compensation (neutral evolution)
  - R2 Phys-QuantBio: quantum biology (FMO photosynthetic coherence / radical-pair magnetoreception)
  - R3 Cog-Enactivism: enactivism (Varela/Thompson/Noë) embodied cognition substrate
  - R4 Sys-Autopoiesis: Maturana/Varela autopoiesis self-creating system closure
  - R5 Math-TopoDyn: Conley index / Morse decomposition topological dynamics substrate
  - R6 Bio-CRISPR: CRISPR adaptive immunity bacterial defense as substrate
  - R7 Cog-ActiveInf: active inference / Helmholtz machine / free energy principle substrate

  - GitHub deep: ShinkaEvolve (SakanaAI) LLM-driven evolutionary search source code
  - GitHub deep: HarnessAgent agentic harness framework source code
  - GitHub deep: claude-agent-sdk Anthropic agent SDK source code

  - Gap reproduction-MISSING: bacterial conjugation / horizontal gene transfer reproduction substrate
  - Gap consciousness-MISSING: Global Workspace Theory (Baars) consciousness theater substrate

  Replaced (vs r105): circadian-Schumann/FDT/somatic-marker/von-Foerster/GRN-toggle/Thom-catastrophe/BEC
                     + ASI-Arch/openevolve/DGM
                     + plant-tropism/IIT
  Replaced (vs r104): cytochrome-oxidase/Curie-temperature/hunchback/Sperry/miura-ori/amyloid-prion/Tanenbaum-DNA
                     + crewai/camel-ai/langgraph
                     + Hayflick-telomere/axon-guidance
  Replaced (vs r103): prion/octopus/spore/Majorana/HoTT/holobiont/Modern-Hopfield
  Replaced (vs r102): Stentor/Friston/Mandelbrot/BCS/Watts-Strogatz/Chomsky/4E
  Replaced (vs r101): Georgopoulos/Wolfram-rule110/Tononi-IIT/EO-Wilson/Ising/Barabasi/Kauffman

  ASI substrate framing maintained (主 17:43 实事求是 + 主 22:08 中央 AI 是 ASI 位置 + 主 22:46 12 生命特征 = 借鉴工具非伪装).
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id": "r106-Q1", "domain": "bio-temperature-compensation-circadian-clock-cyanobacteria", "gap": "substrate",
     "query": "circadian clock temperature compensation cyanobacteria neutral evolution substrate ASI R1 fresh r106",
     "mode": "combined"},
    {"id": "r106-Q2", "domain": "phys-quantum-biology-FMO-photosynthesis-radical-pair-magnetoreception", "gap": "substrate",
     "query": "quantum biology FMO photosynthetic coherence radical pair magnetoreception substrate ASI R2 fresh r106",
     "mode": "combined"},
    {"id": "r106-Q3", "domain": "cog-enactivism-Varela-Thompson-Noe-embodied-cognition", "gap": "substrate",
     "query": "enactivism Varela Thompson Noe embodied cognition sensorimotor substrate ASI R3 fresh r106",
     "mode": "combined"},
    {"id": "r106-Q4", "domain": "sys-autopoiesis-Maturana-Varela-self-creating-system", "gap": "substrate",
     "query": "autopoiesis Maturana Varela self creating system closure organization substrate ASI R4 fresh r106",
     "mode": "combined"},
    {"id": "r106-Q5", "domain": "math-topological-dynamics-Conley-index-Morse-decomposition", "gap": "substrate",
     "query": "Conley index Morse decomposition topological dynamics attractor chain substrate ASI R5 fresh r106",
     "mode": "combined"},
    {"id": "r106-Q6", "domain": "bio-CRISPR-adaptive-immunity-bacterial-defense", "gap": "substrate",
     "query": "CRISPR adaptive immunity bacterial defense spacer acquisition mechanism substrate ASI R6 fresh r106",
     "mode": "combined"},
    {"id": "r106-Q7", "domain": "cog-active-inference-Helmholtz-machine-free-energy-principle", "gap": "substrate",
     "query": "active inference Helmholtz machine free energy principle variational predictive coding substrate ASI R7 fresh r106",
     "mode": "combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id": "r106-Q8", "domain": "github-ShinkaEvolve-SakanaAI-evolutionary-search-source", "gap": "github",
     "query": "ShinkaEvolve SakanaAI github source code LLM evolutionary search architecture substrate ASI r106",
     "mode": "combined"},
    {"id": "r106-Q9", "domain": "github-HarnessAgent-agentic-harness-framework-source", "gap": "github",
     "query": "HarnessAgent github source code architecture agentic harness framework substrate ASI r106",
     "mode": "combined"},
    {"id": "r106-Q10", "domain": "github-claude-agent-sdk-Anthropic-source-code", "gap": "github",
     "query": "claude-agent-sdk Anthropic github source code architecture agent SDK substrate ASI r106",
     "mode": "combined"},
    # === 2 Gap (reproduction MISSING + consciousness MISSING) ===
    {"id": "r106-Q11", "domain": "reproduction-gap-bacterial-conjugation-horizontal-gene-transfer", "gap": "reproduction-MISSING",
     "query": "bacterial conjugation horizontal gene transfer pilus reproduction mechanism substrate ASI Gap fresh r106",
     "mode": "combined"},
    {"id": "r106-Q12", "domain": "consciousness-gap-Global-Workspace-Theory-Baars-dehaene", "gap": "consciousness-MISSING",
     "query": "global workspace theory Baars Dehaene consciousness theater neural substrate ASI Gap fresh r106",
     "mode": "combined"},
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
            env=env, timeout=150
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
        with open("research-v7-round-106.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i + 1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-106 done in", round(total, 1), "sec")
    summary = {
        "round": 106,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-106.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
