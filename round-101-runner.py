"""
Round-101: ASI cross-domain research runner (v3 template)
Cron triggered 2026-08-10 06:57 Asia/Shanghai (every-2h reminder).
Self-decision: round-100 done 2026-08-10 05:04 (~1h53min ago, >30min threshold).
Monday 06:57 early morning, isolated cron lane, M3 model.
Decision: RUN round-101 now (12 fresh angles, validated vs r97-r100).

Theme: 12 angles, 7 cross-domain + 3 GitHub-deep + 2 Gap:
  - R1 Neuronal population coding Georgopoulos 1986 motor cortex population vector
  - R2 Wolfram cellular automata rule 110 universal computation emergence (NKS)
  - R3 Tononi Integrated Information Theory IIT Phi consciousness quantification
  - R4 E.O. Wilson eusociality ant colony superorganism pheromone swarm intelligence
  - R5 Ising model critical phase transition universality class renormalization
  - R6 Barabasi scale-free networks preferential attachment hubs robustness
  - R7 Kauffman NK model adjacent possible rugged fitness landscape self-organization

  - GitHub deep: GAIR-NLP ASI-Arch alpha go architecture recursion LLM agent
  - GitHub deep: codelion openevolve evolutionary algorithm MAP-Elites island model
  - GitHub deep: jennyzzt DGM differentiable generative models autonomous improvement

  - Gap reproduction: HGT horizontal gene transfer endosymbiosis eukaryogenesis
  - Gap consciousness: Global Neuronal Workspace GNW Dehaene ignition consciousness

  Replaced (vs r100): Physarum/Wood-Wide-Web/SOC/Myxococcus/planarian/Shannon/Smale/
                     ShinkaEvolve/mem0/langgraph/Volvox/reconsolidation
  Replaced (vs r99): functional-prion/Limulus-eye/Cantor-set/Active-Inference-Friston/
                     octopus-chromatophore/Portia-jumping-spider/Hyphantria-Turing/
                     YOLOv9/textgrad/Dicyemidae/STDP
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r101-Q1","domain":"neuro-population-coding-Georgopoulos-1986-motor-cortex","gap":"substrate",
     "query":"neuronal population coding Georgopoulos 1986 motor cortex population vector tuning curve primate reaching substrate ASI R1 fresh r101","mode":"combined"},
    {"id":"r101-Q2","domain":"math-Wolfram-cellular-automata-rule110-universal-NKS","gap":"substrate",
     "query":"Wolfram cellular automata rule 110 universal computation emergence New Kind of Science substrate ASI R2 fresh r101","mode":"combined"},
    {"id":"r101-Q3","domain":"cog-Tononi-IIT-Integrated-Information-Theory-Phi","gap":"substrate",
     "query":"Tononi Integrated Information Theory IIT Phi consciousness quantification substrate ASI R3 fresh r101","mode":"combined"},
    {"id":"r101-Q4","domain":"bio-eusociality-EO-Wilson-ant-superorganism-pheromone","gap":"substrate",
     "query":"EO Wilson eusociality ant colony superorganism pheromone trail swarm intelligence stigmergy substrate ASI R4 fresh r101","mode":"combined"},
    {"id":"r101-Q5","domain":"physics-Ising-model-critical-phase-transition-universality","gap":"substrate",
     "query":"Ising model critical phase transition universality class renormalization group critical exponents substrate ASI R5 fresh r101","mode":"combined"},
    {"id":"r101-Q6","domain":"complex-Barabasi-scale-free-networks-preferential-attachment","gap":"substrate",
     "query":"Barabasi scale-free networks preferential attachment hubs robustness percolation substrate ASI R6 fresh r101","mode":"combined"},
    {"id":"r101-Q7","domain":"bio-Kauffman-NK-model-adjacent-possible-rugged-fitness","gap":"substrate",
     "query":"Kauffman NK model adjacent possible rugged fitness landscape self-organization criticality substrate ASI R7 fresh r101","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r101-Q8","domain":"github-GAIR-NLP-ASI-Arch-alpha-go-recursion","gap":"github",
     "query":"GAIR-NLP ASI-Arch github source architecture alpha go recursive self-improvement autonomous research agent substrate ASI r101","mode":"combined"},
    {"id":"r101-Q9","domain":"github-codelion-openevolve-evolutionary-MAP-Elites","gap":"github",
     "query":"codelion openevolve github source MAP-Elites island model evolutionary code optimization LLM substrate ASI r101","mode":"combined"},
    {"id":"r101-Q10","domain":"github-jennyzzt-DGM-differentiable-generative-models","gap":"github",
     "query":"jennyzzt DGM github source differentiable generative models autonomous self-improvement commit agent substrate ASI r101","mode":"combined"},
    # === 2 Gap (reproduction MISSING + consciousness MISSING) ===
    {"id":"r101-Q11","domain":"reproduction-gap-HGT-horizontal-gene-transfer-endosymbiosis-eukaryogenesis","gap":"reproduction-MISSING",
     "query":"horizontal gene transfer HGT endosymbiosis mitochondria chloroplast eukaryogenesis symbiogenesis reproduction substrate ASI Gap fresh r101","mode":"combined"},
    {"id":"r101-Q12","domain":"consciousness-gap-GNW-Global-Neuronal-Workspace-Dehaene-ignition","gap":"consciousness-MISSING",
     "query":"Global Neuronal Workspace GNW Dehaene ignition consciousness threshold long-range cortical neurons substrate ASI Gap fresh r101","mode":"combined"},
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
        with open("research-v7-round-101.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-101 done in", round(total, 1), "sec")
    summary = {
        "round": 101,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-101.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()