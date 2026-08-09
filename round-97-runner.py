"""
Round-97: ASI cross-domain research runner (v3 template - utf-8 fixed)
Cron triggered 2026-08-09 10:55 Asia/Shanghai (every-2h reminder), executed ~10:57.
Self-decision: round-96 done 2026-08-09 08:52 (~2h3min ago, well past 30-min threshold).
Sunday 10:55 morning, isolated cron lane, M3 model.
Decision: RUN round-97 now (12 TRULY fresh angles, validated vs r85-r96, 0 collisions).
Asking permission: master may be awake — isolated cron lane does not interrupt main session.

Theme: 12 TRULY NEW angles — all scanned clean vs r85-r96:
  - R1 生物 fresh: Turritopsis dohrnii immortal jellyfish transdifferentiation
  - R2 生物 fresh: Tardigrade cryptobiosis anhydrobiosis trehalose vitrification tun state
  - R3 演化 fresh: Ctenophora comb jelly sister-group independent nervous system origin
  - R4 物理 fresh: PT-symmetric quantum mechanics non-Hermitian Bender balanced loss-gain
  - R5 数学 fresh: Elementary topos theory Lawvere Grothendieck sheaf-based logic foundations
  - R6 生态 fresh: Trophic rewilding Pleistocene megafauna Donlan restoration ecosystem engineer
  - R7 系统 fresh: Transgenerational epigenetic inheritance Jablonka Lamarckian Weismann barrier

  - GitHub deep: karpathy/nanoGPT minimal GPT training clean source code
  - GitHub deep: crewAIInc/crewAI multi-agent orchestration role-based collaborative source
  - GitHub deep: browser-use/browser-use LLM browser agent element extraction source

  - Gap R6 繁殖: Armadillo Dasypus novemcinctus obligate polyembryony clonal quadruplets
  - Gap R11 可塑: Astrocyte tripartite synapse gliotransmitter D-serine ATP plasticity

  Replaced (vs r96): homing-endonuclease-meganuclease, spinodal-Cahn-Hilliard, simplicial-set-HoTT,
                     corollary-discharge, lignin-fungal, Casimir-effect, Dictyostelium,
                     skate-regeneration, Aider, penzai, Aspidoscelis-parthenogenesis,
                     dendritic-spine
  Replaced (vs r95): transposon-McClintock, holographic-principle-tHooft, renormalization-group-Wilson,
                     retino-cortical-Shulman, efference-copy-vonHolst, nudibranch-kleptocnida,
                     magnetotactic-bacteria-magnetosome, openai-agents-python, openai-swarm,
                     microsoft-autogen, bdelloid-rotifer, BCM-metaplasticity
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r97-Q1","domain":"bio-Turritopsis-immortal-jellyfish-transdiff","gap":"substrate",
     "query":"Turritopsis dohrnii immortal jellyfish transdifferentiation reversal life cycle ontogeny senescence substrate ASI R1 fresh r97","mode":"combined"},
    {"id":"r97-Q2","domain":"bio-tardigrade-cryptobiosis-trehalose","gap":"substrate",
     "query":"tardigrade cryptobiosis anhydrobiosis trehalose vitrification tun state desiccation tolerance extremophile substrate ASI R2 fresh r97","mode":"combined"},
    {"id":"r97-Q3","domain":"evolution-Ctenophora-comb-jelly-nervous","gap":"substrate",
     "query":"Ctenophora comb jelly sister-group independent nervous system origin basal metazoan neural vs non-neural substrate ASI R3 fresh r97","mode":"combined"},
    {"id":"r97-Q4","domain":"physics-PT-symmetric-non-Hermitian","gap":"substrate",
     "query":"PT-symmetric quantum mechanics non-Hermitian Hamiltonian Bender balanced loss-gain unitarity substrate ASI R4 fresh r97","mode":"combined"},
    {"id":"r97-Q5","domain":"math-topos-theory-Lawvere-Grothendieck","gap":"substrate",
     "query":"elementary topos theory Lawvere Grothendieck sheaf-based logic internal language category foundation substrate ASI R5 fresh r97","mode":"combined"},
    {"id":"r97-Q6","domain":"ecology-trophic-rewilding-Pleistocene","gap":"substrate",
     "query":"trophic rewilding Pleistocene megafauna Donlan restoration ecosystem engineer keystone substrate ASI R6 fresh r97","mode":"combined"},
    {"id":"r97-Q7","domain":"systems-transgenerational-epigenetic-Jablonka","gap":"substrate",
     "query":"transgenerational epigenetic inheritance Jablonka Lamarckian Weismann barrier four dimensions acquired trait substrate ASI R7 fresh r97","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r97-Q8","domain":"github-karpathy-nanoGPT","gap":"github",
     "query":"karpathy nanoGPT github source minimal clean GPT training decoder transformer pytorch substrate ASI r97","mode":"combined"},
    {"id":"r97-Q9","domain":"github-crewAIInc-crewAI","gap":"github",
     "query":"crewAIInc crewAI github source code multi-agent orchestration role-based collaborative crew process substrate ASI r97","mode":"combined"},
    {"id":"r97-Q10","domain":"github-browser-use-browser-use","gap":"github",
     "query":"browser-use browser-use github source LLM browser agent DOM element extraction CDP playwright substrate ASI r97","mode":"combined"},
    # === 2 Gap (reproduction + plasticity MISSING) ===
    {"id":"r97-Q11","domain":"reproduction-gap-armadillo-polyembryony","gap":"reproduction-MISSING",
     "query":"armadillo Dasypus novemcinctus obligate polyembryony clonal quadruplets blastocyst fission mammalian reproduction substrate ASI reproduction Gap fresh r97","mode":"combined"},
    {"id":"r97-Q12","domain":"plasticity-gap-astrocyte-tripartite-synapse","gap":"plasticity-MISSING",
     "query":"astrocyte tripartite synapse gliotransmitter D-serine ATP calcium plasticity non-neuronal substrate ASI plasticity Gap fresh r97","mode":"combined"},
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
        with open("research-v7-round-97.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-97 done in", round(total, 1), "sec")
    summary = {
        "round": 97,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-97.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
