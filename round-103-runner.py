"""
Round-103: ASI cross-domain research runner (v3 template)
Cron triggered 2026-08-10 10:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-102 done 2026-08-10 09:00 (~1h48min ago, >30min threshold).
Monday 10:48 morning work-hour, isolated cron lane, M3 model.
Decision: RUN round-103 now (12 fresh angles, validated vs r100-r102).
Master 14:58 Bocha-primary rule still active.

Theme: 12 angles, 7 cross-domain + 3 GitHub-deep + 2 Gap:
  - R1 Bio Prion feedback loop self-propagating misfolded protein templating
  - R2 Bio-Cog Octopus distributed cognition arm-local decision decentralized NS
  - R3 Bio Spore DNA damage SOS response LexA RecA error-prone polymerase
  - R4 Phys Majorana zero mode topological anyonic braiding fault-tolerant qubit
  - R5 Math Constructive type theory HoTT univalent foundations Voevodsky
  - R6 Eco Holobiont hologenome host-microbiome metaorganism Rosenberg Zilber
  - R7 Phys-Cog Modern Hopfield Ramsauer effect quantum tunneling dense associative

  - GitHub deep: openai/swarm multi-agent handoff orchestration source
  - GitHub deep: letta-ai/letta agent memory layered recall source
  - GitHub deep: deepmind/acme distributed RL actor-learner source

  - Gap reproduction: tardigrade cryptobiosis anhydrobiosis trehalose desiccation
  - Gap plasticity: epigenome transgenerational inheritance Lamarckian-ish Waddington

  Replaced (vs r102): Stentor/Friston/Mandelbrot/BCS/Watts-Strogatz/Chomsky/4E
                     + anthropics-claude-code/microsoft-autogen/deepagents-hub
                     + chemoton-Gánti-protocell/Turrigiano-homeostatic-synaptic
  Replaced (vs r101): Georgopoulos/Wolfram-rule110/Tononi-IIT/EO-Wilson/Ising/Barabasi/Kauffman
                     + ASI-Arch/openevolve/DGM-jennyzzt + HGT-eukaryogenesis/GNW-ignition
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
    {"id": "r103-Q1", "domain": "bio-prion-feedback-loop-misfolded-protein-templating", "gap": "substrate",
     "query": "prion feedback loop misfolded protein templating autocatalytic self-propagation PSI+ yeast substrate ASI R1 fresh r103",
     "mode": "combined"},
    {"id": "r103-Q2", "domain": "bio-cog-octopus-distributed-cognition-arm-local-decision", "gap": "substrate",
     "query": "octopus distributed cognition arm local decision decentralized nervous system cephalopod intelligence Godfrey-Smith substrate ASI R2 fresh r103",
     "mode": "combined"},
    {"id": "r103-Q3", "domain": "bio-spore-DNA-repair-SOS-response-LexA-RecA", "gap": "substrate",
     "query": "spore DNA damage SOS response LexA RecA error prone polymerase translesion synthesis bacterial substrate ASI R3 fresh r103",
     "mode": "combined"},
    {"id": "r103-Q4", "domain": "phys-Majorana-zero-mode-topological-anyon-braiding", "gap": "substrate",
     "query": "Majorana zero mode topological anyonic braiding fault tolerant qubit non-Abelian substrate ASI R4 fresh r103",
     "mode": "combined"},
    {"id": "r103-Q5", "domain": "math-constructive-type-theory-HoTT-univalent-Voevodsky", "gap": "substrate",
     "query": "constructive type theory homotopy type theory HoTT univalent foundations Voevodsky BISHOP substrate ASI R5 fresh r103",
     "mode": "combined"},
    {"id": "r103-Q6", "domain": "eco-holobiont-hologenome-metaorganism-host-microbiome", "gap": "substrate",
     "query": "holobiont hologenome metaorganism host microbiome Rosenberg Zilber-Rosenberg evolutionary unit substrate ASI R6 fresh r103",
     "mode": "combined"},
    {"id": "r103-Q7", "domain": "phys-cog-Modern-Hopfield-Ramsauer-effect-quantum-tunneling", "gap": "substrate",
     "query": "modern Hopfield network Ramsauer effect quantum tunneling dense associative memory attention retrieval substrate ASI R7 fresh r103",
     "mode": "combined"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id": "r103-Q8", "domain": "github-openai-swarm-multi-agent-handoff-orchestration", "gap": "github",
     "query": "openai swarm github source code multi agent handoff orchestration routine transfer substrate ASI r103",
     "mode": "combined"},
    {"id": "r103-Q9", "domain": "github-letta-ai-letta-agent-memory-layered-recall", "gap": "github",
     "query": "letta-ai letta github source code agent memory layered recall archival context management substrate ASI r103",
     "mode": "combined"},
    {"id": "r103-Q10", "domain": "github-deepmind-acme-distributed-RL-actor-learner", "gap": "github",
     "query": "deepmind acme github source distributed reinforcement learning actor learner loop reverb substrate ASI r103",
     "mode": "combined"},
    # === 2 Gap (reproduction MISSING + plasticity MISSING) ===
    {"id": "r103-Q11", "domain": "reproduction-gap-tardigrade-cryptobiosis-anhydrobiosis-trehalose", "gap": "reproduction-MISSING",
     "query": "tardigrade cryptobiosis anhydrobiosis trehalose vitrification desiccation revival water bear reproduction substrate ASI Gap fresh r103",
     "mode": "combined"},
    {"id": "r103-Q12", "domain": "plasticity-gap-epigenome-transgenerational-inheritance-Waddington", "gap": "plasticity-MISSING",
     "query": "epigenome transgenerational inheritance Waddington epigenetic landscape Lamarckian-ish inheritance plasticity substrate ASI Gap fresh r103",
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
        with open("research-v7-round-103.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i + 1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-103 done in", round(total, 1), "sec")
    summary = {
        "round": 103,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-103.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()