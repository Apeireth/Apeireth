"""
Round-90: ASI cross-domain research runner (v2 template - utf-8 fixed)
Cron triggered 2026-08-08 20:52 Asia/Shanghai (every-2h reminder).
Self-decision: round-89 done 2026-08-08 19:00 (~112min ago, well past 30-min threshold).
Saturday 20:52 evening, isolated cron lane, M3 model.
Decision: RUN round-90 now (12 TRULY fresh angles, validated vs r80-r89, 0 collisions).

Theme: 12 TRULY NEW angles — all scanned clean vs r80-r89:
  - R1 发育 fresh:  Planarian regeneration Wnt polarity neoblast pluripotent
  - R2 衰老 fresh:  Telomere attrition Blackburn telomerase Hayflick senescence replicative
  - R3 拓扑 fresh:  Topological order anyon Majorana zero mode topologically protected
  - R4 数学 fresh:  Operad higher category Yoneda compositional structure
  - R5 生态 fresh:  Catastrophic regime shift Scheffer alternative stable state early warning
  - R6 生理 fresh:  Allostasis Sterling Schulkin anticipatory regulation predictive homeostasis
  - R7 演化 fresh:  Color terms Berlin Kay 1969 linguistic relativity universality

  - GitHub deep: openai CLIP multimodal alignment contrastive vision language
  - GitHub deep: anthropic-cookbook Claude skills tool use agentic patterns
  - GitHub deep: huggingface trl DPO PPO GRPO RLHF preference optimization

  - Gap R8 繁殖:  Apomixis asexual seed Taraxacum dandelion gametophytic apospory polyembryony
  - Gap R9 意识:  Attention schema theory Graziano AST self-model awareness

  Replaced (vs r89): SOS-DNA, biofilm-c-di-GMP, predictive-coding, ISR, complement-MAC, circadian,
                     phase-transition, prefect, openai-structured, vapi, Volvox, metacognition
  Replaced (vs r88): HGT/integron, quorum-AI-2, hippocampal-replay, HSP90/HSP70, common-mycorrhizal,
                     dissipative-Prigogine, niche-construction, letta, openhands, dspy, hydra-asexual, IIT-Tononi
  Replaced (vs r87): Cas12a, gamma-delta-T, schema-integration, p62/angiopoietin, phyllotaxis,
                     syntrophy, BERTrend, gradio, chainlit, SI-RNase, latent-inhibition
  Replaced (vs r86): Taq, Treg, presynaptic-plasticity, TMAO, cancer-dormancy, thigmotropism,
                     fungal-mycelium, dottxt, triton, firecrawl, Plasmodium, trained-immunity
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    # === 7 Cross-domain (ASI substrate) ===
    {"id":"r90-Q1","domain":"biology-developmental","gap":"substrate",
     "query":"planarian regeneration Wnt polarity neoblast pluripotent stem cell substrate ASI R1 fresh r90","mode":"combined"},
    {"id":"r90-Q2","domain":"biology-aging","gap":"substrate",
     "query":"telomere attrition Blackburn telomerase Hayflick limit replicative senescence SASP aging substrate ASI R2 fresh r90","mode":"combined"},
    {"id":"r90-Q3","domain":"physics-topological","gap":"substrate",
     "query":"topological order anyon Majorana zero mode topologically protected state fault tolerance substrate ASI R3 fresh r90","mode":"combined"},
    {"id":"r90-Q4","domain":"math-category-theory","gap":"emergence",
     "query":"operad higher category Yoneda lemma compositional structure mathematics substrate ASI R4 fresh r90","mode":"combined"},
    {"id":"r90-Q5","domain":"ecology-regime-shift","gap":"substrate",
     "query":"catastrophic regime shift Scheffer alternative stable state early warning indicator critical slowing substrate ASI R5 fresh r90","mode":"combined"},
    {"id":"r90-Q6","domain":"biology-allostasis","gap":"substrate",
     "query":"allostasis Sterling Schulkin anticipatory regulation predictive homeostasis beyond reactive homeo substrate ASI R6 fresh r90","mode":"combined"},
    {"id":"r90-Q7","domain":"linguistics-evolution","gap":"emergence",
     "query":"color terms Berlin Kay 1969 linguistic relativity universality evolution cognition substrate ASI R7 fresh r90","mode":"combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id":"r90-Q8","domain":"github-openai-clip","gap":"github",
     "query":"openai CLIP github source multimodal alignment contrastive vision language ViT image text substrate ASI r90","mode":"combined"},
    {"id":"r90-Q9","domain":"github-anthropic-cookbook","gap":"github",
     "query":"anthropics anthropic-cookbook github source Claude skills tool use agentic patterns substrate ASI r90","mode":"combined"},
    {"id":"r90-Q10","domain":"github-huggingface-trl","gap":"github",
     "query":"huggingface trl github source DPO PPO GRPO RLHF preference optimization trainer substrate ASI r90","mode":"combined"},
    # === 2 Gap (reproduction + consciousness MISSING) ===
    {"id":"r90-Q11","domain":"reproduction-gap","gap":"reproduction-MISSING",
     "query":"apomixis asexual seed Taraxacum dandelion gametophytic apospory polyembryony armadillo substrate ASI R8 reproduction Gap fresh r90","mode":"combined"},
    {"id":"r90-Q12","domain":"consciousness-gap","gap":"consciousness-MISSING",
     "query":"attention schema theory Graziano AST self-model awareness consciousness brain substrate ASI R9 consciousness Gap fresh r90","mode":"combined"},
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
        # find first { to skip any [INFO] lines
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
        # save partial after each
        with open("research-v7-round-90.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-90 done in", round(total, 1), "sec")
    summary = {
        "round": 90,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-90.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()
