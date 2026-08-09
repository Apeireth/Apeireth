"""
Round-99: ASI cross-domain research runner (v3 template)
Cron triggered 2026-08-10 02:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-98 done 2026-08-10 00:53 (~1h55min ago, >30min threshold).
Monday 02:48 deep night, isolated cron lane, M3 model.
Decision: RUN round-99 now (12 fresh angles, validated vs r90-r98).
Asking permission: master asleep — isolated cron lane does not interrupt main session.

Theme: 12 angles, 7 cross-domain + 2 GitHub + 2 Gap + 1 supplementary:
  - R1 Functional prion yeast [URE3] [PSI+] self-templating protein substrate
  - R2 Limulus horseshoe crab compound eye lateral inhibition Hartline substrate
  - R3 Cantor set measure zero uncountable fractal topology substrate
  - R4 Active Inference Friston 2017 four-book free-energy variational treatment
  - R5 Octopus chromatophore dynamic skin neural control LeMasters substrate
  - R6 Portia spider web predatory cognition Menninger jumping spider substrate
  - R7 Hyphantria cunea pattern formation Lepidoptera Turing substrate
  - R12 Modern Hopfield Networks Ramsauer 2020 attention equivalence substrate (suppl)

  - GitHub deep: ultralytics/YOLOv9 PGI programmable gradient info source substrate
  - GitHub deep: langchain-ai/textgrad differentiable text agent gradient source substrate

  - Gap R6 reproduction: Dicyemidae mesozoan reductive asexual extreme simplification
  - Gap R11 plasticity: STDP spike-timing-dependent plasticity Hebbian temporal substrate

  Replaced (vs r98): RNA-world/Octopus-9-brains/Bacillus-spore/topological-insulator/
                     constructive-type-theory/phylosymbiosis/predictive-coding/
                     ChatGLM3/tiktoken/DeepSpeed/aphid-cyclical-parthenogenesis/late-LTP-CREB
  Replaced (vs r97): Turritopsis/tardigrade/Ctenophora/PT-symmetric/topos/trophic-rewilding/
                     transgenerational-epi/nanoGPT/crewAI/browser-use/armadillo/astrocyte
  Replaced (vs r96): homing-endonuclease/spinodal/simplicial/corollary/lignin/Casimir/
                     Dictyostelium/skate/Aider/penzai/Aspidoscelis/dendritic-spine
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r99-Q1","domain":"bio-functional-prion-yeast-URE3-PSI-self-templating","gap":"substrate",
     "query":"functional prion yeast URE3 PSI+ self-templating protein conformation inheritance substrate ASI R1 fresh r99","mode":"combined"},
    {"id":"r99-Q2","domain":"bio-Limulus-horseshoe-crab-compound-eye-lateral-inhibition","gap":"substrate",
     "query":"Limulus polyphemus horseshoe crab compound eye lateral inhibition Hartline 1956 Nobel receptive field substrate ASI R2 fresh r99","mode":"combined"},
    {"id":"r99-Q3","domain":"math-Cantor-set-measure-zero-uncountable-fractal","gap":"substrate",
     "query":"Cantor set measure zero uncountable fractal topology dimension substrate ASI R3 fresh r99","mode":"combined"},
    {"id":"r99-Q4","domain":"neuro-Active-Inference-Friston-2017-free-energy-variational","gap":"substrate",
     "query":"Active Inference Friston 2017 free energy principle variational four-book treatment Markov blanket brain substrate ASI R4 fresh r99","mode":"combined"},
    {"id":"r99-Q5","domain":"bio-octopus-chromatophore-dynamic-skin-neural-control","gap":"substrate",
     "query":"octopus chromatophore dynamic skin neural control LeMasters muscular hydrostat camouflage substrate ASI R5 fresh r99","mode":"combined"},
    {"id":"r99-Q6","domain":"bio-Portia-spider-web-predatory-cognition-jumping-spider","gap":"substrate",
     "query":"Portia spider web predatory cognition Menninger jumping spider salticid planning deception substrate ASI R6 fresh r99","mode":"combined"},
    {"id":"r99-Q7","domain":"bio-Hyphantria-cunea-pattern-formation-Lepidoptera-Turing","gap":"substrate",
     "query":"Hyphantria cunea fall webworm pattern formation Lepidoptera Turing instability reaction diffusion substrate ASI R7 fresh r99","mode":"combined"},
    {"id":"r99-Q12","domain":"math-neo-Hopfield-Ramsauer-2020-attention-equivalence","gap":"substrate",
     "query":"Modern Hopfield Networks Ramsauer 2020 attention equivalence meta-learning Hopfield update transformer associative memory substrate ASI R12 fresh r99","mode":"combined"},
    # === 2 GitHub deep (source code, not just README) ===
    {"id":"r99-Q8","domain":"github-ultralytics-YOLOv9-PGI-source","gap":"github",
     "query":"ultralytics YOLOv9 github source PGI programmable gradient information GELAN architecture object detection substrate ASI r99","mode":"combined"},
    {"id":"r99-Q9","domain":"github-langchain-ai-textgrad-differentiable-text-agent","gap":"github",
     "query":"langchain-ai textgrad github source differentiable text agent gradient optimization prompt tuning substrate ASI r99","mode":"combined"},
    # === 2 Gap (reproduction + plasticity MISSING) ===
    {"id":"r99-Q10","domain":"reproduction-gap-Dicyemidae-reductive-asexual-extreme","gap":"reproduction-MISSING",
     "query":"Dicyemidae mesozoan reductive asexual extreme simplification body plan cephalopod renal reproductive substrate ASI Gap fresh r99","mode":"combined"},
    {"id":"r99-Q11","domain":"plasticity-gap-STDP-spike-timing-dependent-Hebbian","gap":"plasticity-MISSING",
     "query":"STDP spike timing dependent plasticity Hebbian temporal asymmetric window Bi Poo Dan synaptic learning substrate ASI Gap fresh r99","mode":"combined"},
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
        with open("research-v7-round-99.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-99 done in", round(total, 1), "sec")
    summary = {
        "round": 99,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-99.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()