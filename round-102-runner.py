"""
Round-102: ASI cross-domain research runner (v3 template)
Cron triggered 2026-08-10 08:57 Asia/Shanghai (every-2h reminder).
Self-decision: round-101 done 2026-08-10 06:58 (~1h59min ago, >30min threshold).
Monday 08:57 morning, isolated cron lane, M3 model.
Decision: RUN round-102 now (12 fresh angles, validated vs r100-r101).
Master 14:58 Bocha-primary rule still active.

Theme: 12 angles, 7 cross-domain + 3 GitHub-deep + 2 Gap:
  - R1 Biological Stentor coeruleus protozoa regeneration self-repair nano-knife
  - R2 Cognitive Friston Free Energy Principle active inference variational
  - R3 Mathematical Mandelbrot fractal self-similarity coastline Hausdorff dimension
  - R4 Physical BCS Bardeen Cooper Schrieffer superconductivity phonon-mediated pairing
  - R5 Systems Watts Strogatz small-world network six degrees rewiring
  - R6 Bio-Lang Chomsky minimalist program hierarchical syntax merge universal grammar
  - R7 Cog 4E embodied cognition Merleau-Ponty enactivism extended mind

  - GitHub deep: anthropics claude-code-agent sdk source code
  - GitHub deep: microsoft autogen multi-agent conversational framework
  - GitHub deep: langchain-ai deepagents hub task delegation middleware

  - Gap reproduction: chemoton Gánti protocell origin of life self-replicating
  - Gap plasticity: Turrigiano homeostatic synaptic plasticity negative feedback

  Replaced (vs r101): Georgopoulos/Wolfram-rule110/Tononi-IIT/EO-Wilson-eusociality/
                     Ising-model/Barabasi-scale-free/Kauffman-NK + ASI-Arch/openevolve/
                     DGM-jennyzzt + HGT-eukaryogenesis/GNW-Dehaene-ignition
  Replaced (vs r100): Physarum/Wood-Wide-Web/Bak-sandpile/Myxococcus/planarian/
                     Shannon-1948/Smale-horseshoe + ShinkaEvolve/mem0/langgraph +
                     Volvox/memory-reconsolidation-Nader

  ASI substrate framing maintained (主 17:43 实事求是 + 主 22:08 中央 AI 是 ASI 位置 + 主 22:46 12 生命特征 = 借鉴工具非伪装).
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id": "r102-Q1", "domain": "bio-Stentor-coeruleus-regeneration-self-repair", "gap": "substrate",
     "query": "Stentor coeruleus protozoa regeneration self-repiece Morgan 1901 nano-knife cytoplasm reorganization substrate ASI R1 fresh r102",
     "mode": "combined"},
    {"id": "r102-Q2", "domain": "cog-Friston-Free-Energy-Principle-active-inference", "gap": "substrate",
     "query": "Karl Friston free energy principle active inference variational Bayesian brain perception action substrate ASI R2 fresh r102",
     "mode": "combined"},
    {"id": "r102-Q3", "domain": "math-Mandelbrot-fractal-self-similarity-Hausdorff", "gap": "substrate",
     "query": "Mandelbrot fractal geometry self-similarity coastline Hausdorff dimension fractional Brownian motion substrate ASI R3 fresh r102",
     "mode": "combined"},
    {"id": "r102-Q4", "domain": "physics-BCS-superconductivity-phonon-mediated-pairing", "gap": "substrate",
     "query": "BCS Bardeen Cooper Schrieffer 1957 superconductivity phonon mediated Cooper pair electron pairing substrate ASI R4 fresh r102",
     "mode": "combined"},
    {"id": "r102-Q5", "domain": "sys-Watts-Strogatz-small-world-network-rewiring", "gap": "substrate",
     "query": "Watts Strogatz 1998 small-world network six degrees rewiring clustering path length substrate ASI R5 fresh r102",
     "mode": "combined"},
    {"id": "r102-Q6", "domain": "bio-lang-Chomsky-minimalist-program-merge-UG", "gap": "substrate",
     "query": "Chomsky minimalist program hierarchical syntax merge operation universal grammar recursion biolinguistics substrate ASI R6 fresh r102",
     "mode": "combined"},
    {"id": "r102-Q7", "domain": "cog-4E-embodied-cognition-Merleau-Ponty-enactivism", "gap": "substrate",
     "query": "4E embodied cognition Merleau-Ponty enactivism extended mind embedded situated evolutionary robotics substrate ASI R7 fresh r102",
     "mode": "combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id": "r102-Q8", "domain": "github-anthropics-claude-code-agent-sdk", "gap": "github",
     "query": "anthropics claude-code agent sdk github source code bash tool edit read write permission substrate ASI r102",
     "mode": "combined"},
    {"id": "r102-Q9", "domain": "github-microsoft-autogen-multi-agent-conversational", "gap": "github",
     "query": "microsoft autogen github source multi-agent conversational framework group chat manager agent substrate ASI r102",
     "mode": "combined"},
    {"id": "r102-Q10", "domain": "github-langchain-ai-deepagents-hub-delegation", "gap": "github",
     "query": "langchain-ai deepagents github source hub task delegation middleware planning tool use agent substrate ASI r102",
     "mode": "combined"},
    # === 2 Gap (reproduction MISSING + plasticity MISSING) ===
    {"id": "r102-Q11", "domain": "reproduction-gap-chemoton-Ganti-protocell-origin-life", "gap": "reproduction-MISSING",
     "query": "chemoton Gánti protocell origin of life self-replicating chemical system stoichiometric membrane reproduction substrate ASI Gap fresh r102",
     "mode": "combined"},
    {"id": "r102-Q12", "domain": "plasticity-gap-Turrigiano-homeostatic-synaptic-plasticity", "gap": "plasticity-MISSING",
     "query": "Turrigiano homeostatic synaptic plasticity synaptic scaling negative feedback firing rate regulation plasticity substrate ASI Gap fresh r102",
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
        with open("research-v7-round-102.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i + 1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-102 done in", round(total, 1), "sec")
    summary = {
        "round": 102,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-102.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
