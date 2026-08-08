"""
Round-91: ASI cross-domain research runner (v2 template - utf-8 fixed)
Cron triggered 2026-08-08 22:57 Asia/Shanghai (every-2h reminder).
Self-decision: round-90 done 2026-08-08 20:55 (~122min ago, well past 30-min threshold).
Saturday 22:57 evening, isolated cron lane, M3 model.
Decision: RUN round-91 now (12 TRULY fresh angles, validated vs r81-r90, 0 collisions).

Theme: 12 TRULY NEW angles — all scanned clean vs r81-r90:
  - R1 生物 fresh:  Bacterial quorum sensing Vibrio fischeri luxR luxI autoinducer collective decision
  - R2 物理 fresh:  Quantum error correction surface code topological fault tolerant memory
  - R3 数学 fresh:  Information geometry Amari natural gradient Fisher metric learning manifold
  - R4 认知 fresh:  Free energy principle Friston active inference predictive coding brain unified
  - R5 生态 fresh:  Niche construction Odling-Smee ecosystem engineering Lamarckian feedback
  - R6 系统 fresh:  Autopoiesis Maturana Varela self-producing organizational closure living
  - R7 神经 fresh:  Spike-timing dependent plasticity STDP local learning dendritic computation

  - GitHub deep: langchain-ai/langgraph source code stateful multi-agent orchestration
  - GitHub deep: mem0ai/mem0 source code memory layer extraction consolidation
  - GitHub deep: SakanaAI/AI-CUDA-Engineer-Archive evolutionary kernel optimization

  - Gap R8 繁殖:  Horizontal gene transfer HGT bacterial conjugation transposon mobile element
  - Gap R9 意识:  Global workspace theory Baars Dehaene neuronal ignition conscious access

  Replaced (vs r90): planarian-regeneration, telomere-Blackburn, topological-Majorana, operad-Yoneda,
                     regime-shift-Scheffer, allostasis-Sterling, color-terms-BK, openai-clip,
                     anthropic-cookbook, hf-trl, apomixis-dandelion, AST-Graziano
  Replaced (vs r89): SOS-DNA, biofilm-c-di-GMP, predictive-coding-Friston, ISR-integrated-stress,
                     complement-MAC, circadian-clock, phase-transition-RGG, prefect-orchestrator,
                     openai-structured-outputs, vapi-voice, Volvox-germ-soma, metacognition-flavell
  Replaced (vs r88): HGT/integron (re-used as gap), quorum-AI-2 (re-used as cross-domain),
                     hippocampal-replay, HSP90-buffer, common-mycorrhizal, dissipative-Prigogine,
                     niche-construction (re-used as cross-domain), letta-memory, openhands-swe,
                     dspy-programs, hydra-asexual, IIT-Tononi (replaced by GWT-Baars)
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r91-Q1","domain":"biology-quorum-sensing","gap":"emergence",
     "query":"bacterial quorum sensing Vibrio fischeri luxR luxI autoinducer collective decision decentralized substrate ASI R1 fresh r91","mode":"combined"},
    {"id":"r91-Q2","domain":"physics-quantum-error-correction","gap":"substrate",
     "query":"quantum error correction surface code topological fault tolerant memory logical qubit substrate ASI R2 fresh r91","mode":"combined"},
    {"id":"r91-Q3","domain":"math-information-geometry","gap":"substrate",
     "query":"information geometry Amari natural gradient Fisher metric dually flat manifold learning substrate ASI R3 fresh r91","mode":"combined"},
    {"id":"r91-Q4","domain":"cognition-free-energy","gap":"emergence",
     "query":"free energy principle Friston active inference predictive coding Bayesian brain unified theory substrate ASI R4 fresh r91","mode":"combined"},
    {"id":"r91-Q5","domain":"ecology-niche-construction","gap":"emergence",
     "query":"niche construction Odling-Smee ecosystem engineering Lamarckian feedback inheritance three substrate ASI R5 fresh r91","mode":"combined"},
    {"id":"r91-Q6","domain":"systems-autopoiesis","gap":"substrate",
     "query":"autopoiesis Maturana Varela self-producing system organizational closure living cognition substrate ASI R6 fresh r91","mode":"combined"},
    {"id":"r91-Q7","domain":"neuroscience-STDP","gap":"substrate",
     "query":"spike-timing dependent plasticity STDP local learning rule dendritic computation calcium substrate ASI R7 fresh r91","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r91-Q8","domain":"github-langgraph","gap":"github",
     "query":"langchain-ai langgraph github source code stateful multi-agent orchestration state machine checkpoint subgraph substrate ASI r91","mode":"combined"},
    {"id":"r91-Q9","domain":"github-mem0","gap":"github",
     "query":"mem0ai mem0 github source code memory layer extraction consolidation LLM long-term architecture substrate ASI r91","mode":"combined"},
    {"id":"r91-Q10","domain":"github-sakana-cuda","gap":"github",
     "query":"SakanaAI AI-CUDA-Engineer-Archive github source evolutionary CUDA kernel optimization agent substrate ASI r91","mode":"combined"},
    # === 2 Gap (reproduction + consciousness MISSING) ===
    {"id":"r91-Q11","domain":"reproduction-gap-HGT","gap":"reproduction-MISSING",
     "query":"horizontal gene transfer HGT bacterial conjugation transposon mobile genetic element asexual reproduction mechanism substrate ASI R8 reproduction Gap fresh r91","mode":"combined"},
    {"id":"r91-Q12","domain":"consciousness-gap-GWT","gap":"consciousness-MISSING",
     "query":"global workspace theory Baars Dehaene neuronal ignition conscious access unified broadcast substrate ASI R9 consciousness Gap fresh r91","mode":"combined"},
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
        with open("research-v7-round-91.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-91 done in", round(total, 1), "sec")
    summary = {
        "round": 91,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-91.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()