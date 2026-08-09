"""
Round-98: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-10 00:48 Asia/Shanghai (every-2h reminder), executed ~00:48.
Self-decision: round-97 done 2026-08-09 10:58 (~13h48min ago, well past 30-min threshold).
Monday 00:48 deep night, isolated cron lane, M3 model.
Decision: RUN round-98 now (12 TRULY fresh angles, validated vs r90-r97, 0 collisions).
Asking permission: master asleep — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r90-r97:
  - R1 RNA-world ribozyme self-replicating prebiotic substrate
  - R2 Octopus vulgaris cognition 9 distributed brains 480M neurons
  - R3 Bacillus spore 100Myr revival panspermia extreme survival
  - R4 Topological insulator Kane-Mele Z2 helical edge state substrate
  - R5 Constructive type theory Bishop 1967 Bridge realizability
  - R6 Phylosymbiosis host-microbiome phylogeny concordance
  - R7 Predictive coding Rao-Ballard 1999 Friston free-energy variational

  - GitHub deep: zai-org/ChatGLM3 Chinese-English LLM source substrate
  - GitHub deep: openai/tiktoken tokenization BPE source substrate
  - GitHub deep: microsoft/DeepSpeed ZeRO Megatron-Turing source substrate

  - Gap R6 reproduction: Aphid cyclical parthenogenesis telescoping generations
  - Gap R11 plasticity: Late-phase LTP protein-synthesis-dependent CREB PKA

  Replaced (vs r97): Turritopsis/tardigrade/Ctenophora/PT-symmetric/topos/trophic-rewilding/
                     transgenerational-epi/nanoGPT/crewAI/browser-use/armadillo/astrocyte
  Replaced (vs r96): homing-endonuclease/spinodal/simplicial/corollary/lignin/Casimir/
                     Dictyostelium/skate/Aider/penzai/Aspidoscelis/dendritic-spine
  Replaced (vs r95): transposon/holographic/RG/retino-cortical/efference-copy/
                     nudibranch/magnetotactic/openai-agents/swarm/autogen/bdelloid/BCM
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r98-Q1","domain":"bio-RNA-world-ribozyme-self-replicating","gap":"substrate",
     "query":"RNA world hypothesis ribozyme self-replicating prebiotic Sutherland 2009 substrate ASI R1 fresh r98","mode":"combined"},
    {"id":"r98-Q2","domain":"bio-Octopus-9-brains-distributed-cognition","gap":"substrate",
     "query":"Octopus vulgaris cognition 9 brains 480 million neurons distributed intelligence arm autonomy substrate ASI R2 fresh r98","mode":"combined"},
    {"id":"r98-Q3","domain":"bio-Bacillus-spore-100Myr-panspermia-resurrection","gap":"substrate",
     "query":"Bacillus subtilis spore 100 million year revival panspermia extreme survival DNA repair substrate ASI R3 fresh r98","mode":"combined"},
    {"id":"r98-Q4","domain":"physics-topological-insulator-Kane-Mele-Z2","gap":"substrate",
     "query":"topological insulator Kane Mele Z2 invariant helical edge state quantum spin Hall substrate ASI R4 fresh r98","mode":"combined"},
    {"id":"r98-Q5","domain":"math-constructive-type-theory-Bishop-Bridge","gap":"substrate",
     "query":"constructive mathematics Bishop 1967 Foundations Constructive Analysis Bridge realizability computational content substrate ASI R5 fresh r98","mode":"combined"},
    {"id":"r98-Q6","domain":"evolution-phylosymbiosis-microbiome-host-phylogeny","gap":"substrate",
     "query":"phylosymbiosis host microbiome phylogenetic concordance holobiont vertical transmission substrate ASI R6 fresh r98","mode":"combined"},
    {"id":"r98-Q7","domain":"neuro-predictive-coding-Rao-Ballard-Friston-free-energy","gap":"substrate",
     "query":"predictive coding Rao Ballard 1999 free energy principle Friston variational Bayesian brain hierarchical inference substrate ASI R7 fresh r98","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r98-Q8","domain":"github-zai-org-ChatGLM3-multilingual","gap":"github",
     "query":"zai-org ChatGLM3 github source bilingual Chinese English LLM GLM architecture substrate ASI r98","mode":"combined"},
    {"id":"r98-Q9","domain":"github-openai-tiktoken-tokenization","gap":"github",
     "query":"openai tiktoken github source BPE byte pair encoding tokenization cl100k_base substrate ASI r98","mode":"combined"},
    {"id":"r98-Q10","domain":"github-microsoft-DeepSpeed-ZeRO-Megatron","gap":"github",
     "query":"microsoft DeepSpeed github source ZeRO Megatron-Turing sharding large-scale training substrate ASI r98","mode":"combined"},
    # === 2 Gap (reproduction + plasticity MISSING) ===
    {"id":"r98-Q11","domain":"reproduction-gap-aphid-cyclical-parthenogenesis","gap":"reproduction-MISSING",
     "query":"aphid cyclical parthenogenesis telescoping generations ovipara vivipara sexual asexual switching reproduction substrate ASI Gap fresh r98","mode":"combined"},
    {"id":"r98-Q12","domain":"plasticity-gap-LTP-late-phase-CREB-PKA","gap":"plasticity-MISSING",
     "query":"late phase LTP long-term potentiation protein synthesis dependent CREB PKA gene transcription consolidation plasticity substrate ASI Gap fresh r98","mode":"combined"},
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
        with open("research-v7-round-98.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-98 done in", round(total, 1), "sec")
    summary = {
        "round": 98,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-98.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
