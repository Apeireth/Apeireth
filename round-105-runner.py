"""
Round-105: ASI cross-domain research runner (v3 template, combined mode)
Cron triggered 2026-08-10 15:03 Asia/Shanghai (every-2h reminder).
Self-decision: round-104 done 2026-08-10 13:07 (~1h56min ago, >30min threshold).
Monday 15:03 work-hour, isolated cron lane, M3 model.
Decision: RUN round-105 now (12 fresh angles, validated vs r101-r104).

✅ Bocha web + Bocha AI both 200 (master 14:58 立规: 博查主用, 该用不要吝啬)
   - unified-search.py combined mode: 博查 web + 博查 AI → fallback AnySearch → fallback Brave
   - all 12 queries use combined mode for richest cross-source coverage

Theme: 12 angles, 7 cross-domain (substrate) + 3 GitHub-deep (source code) + 2 Gap (MISSING):
  - R1 Bio-Rhythm: circadian / ultradian / Schumann biological timing as ASI substrate
  - R2 Phys-FDT: fluctuation-dissipation theorem (noise vs info balance) as ASI substrate
  - R3 Cog-Somatic: Damasio somatic-marker hypothesis (body-grounded cognition) substrate
  - R4 Sys-Cyber: Heinz von Foerster second-order cybernetics (observer in system) substrate
  - R5 Bio-GRN: gene regulatory network toggle switch / bistability / cellular decision substrate
  - R6 Math-Catastrophe: René Thom catastrophe theory (morphology of sudden change) substrate
  - R7 Phys-BEC: Bose-Einstein condensate (macroscopic quantum state) substrate

  - GitHub deep: ASI-Arch (GAIR-NLP) alphaevolve source code
  - GitHub deep: openevolve source code architecture
  - GitHub deep: DGM (jennyzzt) differentiable generative model source code

  - Gap irritability-MISSING: plant tropism / chemotaxis as fundamental irritability (non-neural)
  - Gap consciousness-MISSING: IIT Tononi integrated information Φ phenomenal substrate

  Replaced (vs r104): cytochrome-oxidase/Curie-temperature/hunchback/Sperry/miura-ori/
                     amyloid-prion/Tanenbaum-DNA + crewai/camel-ai/langgraph +
                     Hayflick-telomere/axon-guidance
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
    {"id": "r105-Q1", "domain": "bio-rhythm-circadian-ultadian-Schumann-biological-timing", "gap": "substrate",
     "query": "circadian ultradian rhythm Schumann resonance biological timing substrate ASI R1 fresh r105",
     "mode": "combined"},
    {"id": "r105-Q2", "domain": "phys-fluctuation-dissipation-theorem-noise-information-balance", "gap": "substrate",
     "query": "fluctuation dissipation theorem noise information balance equilibrium substrate ASI R2 fresh r105",
     "mode": "combined"},
    {"id": "r105-Q3", "domain": "cog-somatic-marker-Damasio-body-grounded-cognition", "gap": "substrate",
     "query": "Damasio somatic marker hypothesis body grounded cognition feeling substrate ASI R3 fresh r105",
     "mode": "combined"},
    {"id": "r105-Q4", "domain": "sys-cybernetics-second-order-von-Foerster-observer-system", "gap": "substrate",
     "query": "Heinz von Foerster second order cybernetics observer included system substrate ASI R4 fresh r105",
     "mode": "combined"},
    {"id": "r105-Q5", "domain": "bio-GRN-toggle-switch-bistability-cellular-decision", "gap": "substrate",
     "query": "gene regulatory network toggle switch bistability cellular decision making substrate ASI R5 fresh r105",
     "mode": "combined"},
    {"id": "r105-Q6", "domain": "math-catastrophe-Thom-morphology-sudden-change", "gap": "substrate",
     "query": "René Thom catastrophe theory morphology sudden change bifurcation substrate ASI R6 fresh r105",
     "mode": "combined"},
    {"id": "r105-Q7", "domain": "phys-BEC-Bose-Einstein-condensate-macroscopic-quantum-state", "gap": "substrate",
     "query": "Bose Einstein condensate macroscopic quantum state coherence substrate ASI R7 fresh r105",
     "mode": "combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id": "r105-Q8", "domain": "github-ASI-Arch-GAIR-NLP-alphaevolve-source-code", "gap": "github",
     "query": "ASI-Arch GAIR-NLP alphaevolve github source code architecture automated research substrate ASI r105",
     "mode": "combined"},
    {"id": "r105-Q9", "domain": "github-openevolve-source-code-architecture", "gap": "github",
     "query": "openevolve github source code architecture evolutionary algorithm LLM substrate ASI r105",
     "mode": "combined"},
    {"id": "r105-Q10", "domain": "github-DGM-jennyzzt-differentiable-generative-model-source", "gap": "github",
     "query": "DGM differentiable generative model jennyzzt github source code architecture substrate ASI r105",
     "mode": "combined"},
    # === 2 Gap (irritability MISSING + consciousness MISSING) ===
    {"id": "r105-Q11", "domain": "irritability-gap-plant-tropism-chemotaxis-stimulus-response", "gap": "irritability-MISSING",
     "query": "plant tropism chemotaxis stimulus response non-neural irritability fundamental ASI Gap fresh r105",
     "mode": "combined"},
    {"id": "r105-Q12", "domain": "consciousness-gap-IIT-Tononi-integrated-information-phi-phenomenal", "gap": "consciousness-MISSING",
     "query": "integrated information theory Tononi Phi consciousness substrate ASI phenomenal Gap fresh r105",
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
        with open("research-v7-round-105.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i + 1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-105 done in", round(total, 1), "sec")
    summary = {
        "round": 105,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-105.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()