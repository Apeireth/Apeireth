"""
Round-107: ASI cross-domain research runner (v3 template, web mode)
Cron triggered 2026-08-10 18:52 Asia/Shanghai (every-2h reminder).
Self-decision: round-106 done 2026-08-10 16:52:41 (~2h ago, >30min threshold met).
Monday 18:52 evening, isolated cron lane, M3 model.
Decision: RUN round-107 now (12 fresh angles, validated vs r100-r106).

Bocha web=200 OK, Bocha AI=403 (quota still exhausted, master 14:58 rule still applies
to web but AI is dead, so we use web-only unified-search.py mode for speed).

Theme: 12 angles, 7 cross-domain (substrate) + 3 GitHub-deep (source code) + 2 Gap (MISSING):
  - R1 Bio-Morphogenesis: Turing pattern / reaction-diffusion (chemical morphogen substrate)
  - R2 Bio-Swarm: Myxobacteria fruiting body / Bonabeau swarm intelligence substrate
  - R3 Cog-MorphComp: Morphological computation (Pfeifer/Bongard) embodied cognition substrate
  - R4 Cog-Glia: Astrocyte / glial cell / tripartite synapse (non-neuronal cognition substrate)
  - R5 Bio-QuorumSens: Quorum sensing Vibrio fischeri bioluminescence cell-cell comm substrate
  - R6 Cog-CogMap: Cognitive map O'Keefe place cells / Moser grid cells (spatial memory substrate)
  - R7 Math-Hyperbolic: Hyperbolic embedding Poincare disk (hierarchical representation substrate)

  - GitHub deep: openhands (formerly open-devin) AI software engineer architecture source
  - GitHub deep: mlflow model lifecycle / experiment tracking source code
  - GitHub deep: pexpect / PTY process control architecture (AI agent terminal substrate)

  - Gap reproduction-MISSING: parthenogenesis / aphid asexual reproduction substrate
  - Gap consciousness-MISSING: Attention Schema Theory (Graziano) consciousness as self-model

  Replaced (vs r106): circadian-temperature-compensation/quantum-bio/enactivism/autopoiesis/Conley-index/CRISPR/active-inference
                     + ShinkaEvolve/HarnessAgent/claude-agent-sdk
                     + bacterial-conjugation/global-workspace-Baars
  Replaced (vs r105): Ciona-GRN/KPZ-stochastic/sheaf-cohomology/Bayesian-predictive/lambda-phage-CI-Cro/auxin-PIN/aeolian-sand
                     + vllm-PagedAttention/Prefect-workflow/MCP-server
                     + Apis-mellifera-capensis/engram-cells-Tonegawa
  Replaced (vs r104): cytochrome/Curie/hunchback/Sperry/miura/amyloid/Tanenbaum-DNA
                     + crewai/camel-ai/langgraph
                     + Hayflick-telomere/axon-guidance
  Replaced (vs r103): prion/octopus/spore/Majorana/HoTT/holobiont/Modern-Hopfield
                     + ASI-Arch/openevolve/DGM/letta/acme
                     + tardigrade/epigenome
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
    {"id": "r107-Q1", "domain": "bio-turing-pattern-reaction-diffusion-morphogenesis", "gap": "substrate",
     "query": "Turing pattern reaction diffusion morphogenesis activator inhibitor substrate ASI R1 fresh r107",
     "mode": "web"},
    {"id": "r107-Q2", "domain": "bio-myxobacteria-fruiting-body-swarm-intelligence-Bonabeau", "gap": "substrate",
     "query": "myxobacteria fruiting body swarm intelligence Bonabeau self-organization substrate ASI R2 fresh r107",
     "mode": "web"},
    {"id": "r107-Q3", "domain": "cog-morphological-computation-Pfeifer-Bongard-embodiment", "gap": "substrate",
     "query": "morphological computation Pfeifer Bongard embodiment passive dynamics substrate ASI R3 fresh r107",
     "mode": "web"},
    {"id": "r107-Q4", "domain": "cog-astrocyte-glial-cell-tripartite-synapse", "gap": "substrate",
     "query": "astrocyte glial cell tripartite synapse non-neuronal cognition substrate ASI R4 fresh r107",
     "mode": "web"},
    {"id": "r107-Q5", "domain": "bio-quorum-sensing-Vibrio-fischeri-bioluminescence", "gap": "substrate",
     "query": "quorum sensing Vibrio fischeri bioluminescence cell-cell communication substrate ASI R5 fresh r107",
     "mode": "web"},
    {"id": "r107-Q6", "domain": "cog-cognitive-map-place-cell-grid-cell-O-Keefe-Moser", "gap": "substrate",
     "query": "cognitive map place cell grid cell O'Keefe Moser entorhinal spatial memory substrate ASI R6 fresh r107",
     "mode": "web"},
    {"id": "r107-Q7", "domain": "math-hyperbolic-embedding-Poincare-disk-hierarchy", "gap": "substrate",
     "query": "hyperbolic embedding Poincare disk hierarchical representation Nickel Kiela substrate ASI R7 fresh r107",
     "mode": "web"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id": "r107-Q8", "domain": "github-openhands-ai-software-engineer-architecture", "gap": "github",
     "query": "openhands open-devin github source code AI software engineer architecture substrate ASI r107",
     "mode": "web"},
    {"id": "r107-Q9", "domain": "github-mlflow-model-lifecycle-experiment-tracking-source", "gap": "github",
     "query": "mlflow github source code model lifecycle experiment tracking architecture substrate ASI r107",
     "mode": "web"},
    {"id": "r107-Q10", "domain": "github-pexpect-PTY-process-control-architecture", "gap": "github",
     "query": "pexpect PTY process control github source code architecture AI agent terminal substrate ASI r107",
     "mode": "web"},
    # === 2 Gap (reproduction MISSING + consciousness MISSING) ===
    {"id": "r107-Q11", "domain": "reproduction-gap-parthenogenesis-aphid-asexual", "gap": "reproduction-MISSING",
     "query": "parthenogenesis aphid asexual reproduction mechanism telitoky arrhenotoky substrate ASI Gap fresh r107",
     "mode": "web"},
    {"id": "r107-Q12", "domain": "consciousness-gap-attention-schema-theory-Graziano", "gap": "consciousness-MISSING",
     "query": "attention schema theory Graziano consciousness self-model predictive brain substrate ASI Gap fresh r107",
     "mode": "web"},
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
        with open("research-v7-round-107.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i + 1, ok))
        time.sleep(0.8)
    total = time.time() - t0
    print("\nRound-107 done in", round(total, 1), "sec")
    summary = {
        "round": 107,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-107.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()