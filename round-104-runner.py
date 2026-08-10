"""
Round-104: ASI cross-domain research runner (v3 template, web-mode fallback)
Cron triggered 2026-08-10 13:05 Asia/Shanghai (every-2h reminder).
Self-decision: round-103 done 2026-08-10 10:50 (~2h14min ago, >30min threshold).
Monday 13:05 work-hour, isolated cron lane, M3 model.
Decision: RUN round-104 now (12 fresh angles, validated vs r100-r103).

⚠️ Bocha AI quota EXHAUSTED (ba=403 verified at 13:05, log_id=b1d1dd57)
   - master 14:58 rule: 博查主用 (still active for web; AI temporarily down)
   - web endpoint (bw) still 200 OK
   - unified-search.py web mode: 博查 web → fallback AnySearch → fallback Brave
   - runner switches to mode="web" (not "combined") for all 12 queries
   - prior 12:59 r104 attempt logged running but produced no file (likely AI 403 mid-assembly)

Theme: 12 angles, 7 cross-domain + 3 GitHub-deep + 2 Gap (uses r103's next_round_hint):
  - R1 Bio-Energetics: mitochondrial cytochrome-c oxidase electron tunneling quantum biology
  - R2 Phys-Critical: ferromagnet Curie temperature critical fluctuation phase transition
  - R3 Bio-Dev: drosophila hunchback morphogen gradient Bicoid anterior-posterior
  - R4 Cog-Split: Sperry split brain corpus callosum hemispheric specialization
  - R5 Eng-Geom: origami miura-ori tensegrity fold programmable structure
  - R6 Bio-Cross: amyloid fibril prion as architecture self-templating functional
  - R7 Bio-Sensor: Tanenbaum DNA biosensor nucleic acid computation cellular logic

  - GitHub deep: crewai multi-agent source crew orchestration role task
  - GitHub deep: camel-ai role-playing source communicative agents communicative role
  - GitHub deep: langgraph langchain graph state machine source code

  - Gap reproduction: cellular senescence Hayflick limit telomere aging reproduction
  - Gap plasticity: axon guidance molecular cues netrin semaphorin plasticity growth

  Replaced (vs r103): prion/octopus/spore/Majorana/HoTT/holobiont/Modern-Hopfield
                     + openai-swarm/letta-ai/acme-deepmind
                     + tardigrade/epigenome-Waddington
  Replaced (vs r102): Stentor/Friston/Mandelbrot/BCS/Watts-Strogatz/Chomsky/4E
  Replaced (vs r101): Georgopoulos/Wolfram-rule110/Tononi-IIT/EO-Wilson/Ising/Barabasi/Kauffman
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
    {"id": "r104-Q1", "domain": "bio-energetics-mitochondrial-cytochrome-oxidase-electron-tunneling", "gap": "substrate",
     "query": "mitochondrial cytochrome c oxidase electron tunneling quantum biology respiration substrate ASI R1 fresh r104",
     "mode": "web"},
    {"id": "r104-Q2", "domain": "phys-critical-ferromagnet-Curie-temperature-critical-fluctuation-phase", "gap": "substrate",
     "query": "ferromagnet Curie temperature critical fluctuation phase transition universality substrate ASI R2 fresh r104",
     "mode": "web"},
    {"id": "r104-Q3", "domain": "bio-dev-drosophila-hunchback-morphogen-gradient-Bicoid-anterior-posterior", "gap": "substrate",
     "query": "drosophila hunchback morphogen gradient Bicoid anterior posterior pattern formation substrate ASI R3 fresh r104",
     "mode": "web"},
    {"id": "r104-Q4", "domain": "cog-split-Sperry-split-brain-corpus-callosum-hemispheric-specialization", "gap": "substrate",
     "query": "Sperry split brain corpus callosum hemispheric specialization lateralization dual consciousness substrate ASI R4 fresh r104",
     "mode": "web"},
    {"id": "r104-Q5", "domain": "eng-geom-origami-miura-tensegrity-fold-programmable-structure", "gap": "substrate",
     "query": "origami miura ori tensegrity fold programmable structure deployable engineering substrate ASI R5 fresh r104",
     "mode": "web"},
    {"id": "r104-Q6", "domain": "bio-cross-amyloid-fibril-prion-as-architecture-self-templating-functional", "gap": "substrate",
     "query": "amyloid fibril prion as architecture self templating functional functional amyloid substrate ASI R6 fresh r104",
     "mode": "web"},
    {"id": "r104-Q7", "domain": "bio-sensor-Tanenbaum-DNA-biosensor-nucleic-acid-computation-cellular-logic", "gap": "substrate",
     "query": "Tanenbaum DNA biosensor nucleic acid computation cellular logic synthetic biology substrate ASI R7 fresh r104",
     "mode": "web"},
    # === 3 GitHub deep (source code, not just README) ===
    {"id": "r104-Q8", "domain": "github-crewai-multi-agent-crew-orchestration-role-task", "gap": "github",
     "query": "crewai github source code multi agent crew orchestration role task delegation substrate ASI r104",
     "mode": "web"},
    {"id": "r104-Q9", "domain": "github-camel-ai-role-playing-communicative-agents-source", "gap": "github",
     "query": "camel ai role playing github source communicative agents inception prompting substrate ASI r104",
     "mode": "web"},
    {"id": "r104-Q10", "domain": "github-langgraph-langchain-graph-state-machine-source-code", "gap": "github",
     "query": "langgraph langchain github source graph state machine checkpoint workflow substrate ASI r104",
     "mode": "web"},
    # === 2 Gap (reproduction MISSING + plasticity MISSING) ===
    {"id": "r104-Q11", "domain": "reproduction-gap-cellular-senescence-Hayflick-limit-telomere-aging", "gap": "reproduction-MISSING",
     "query": "cellular senescence Hayflick limit telomere shortening aging reproduction decline substrate ASI Gap fresh r104",
     "mode": "web"},
    {"id": "r104-Q12", "domain": "plasticity-gap-axon-guidance-molecular-cues-netrin-semaphorin-plasticity", "gap": "plasticity-MISSING",
     "query": "axon guidance molecular cues netrin semaphorin ephrin plasticity growth cone substrate ASI Gap fresh r104",
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
        with open("research-v7-round-104.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i + 1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-104 done in", round(total, 1), "sec")
    summary = {
        "round": 104,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-104.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()