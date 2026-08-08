"""
Round-88: ASI cross-domain research runner (v2 - utf-8 fixed)
"""
import json, subprocess, sys, time, os

WORKDIR = r".openclaw\workspace\promethean"
SCRIPT_DIR = r".openclaw\scripts"
PY = sys.executable

QUERIES = [
    {"id":"r88-Q1","domain":"biology-genetics","gap":"substrate",
     "query":"horizontal gene transfer mobile genetic elements integron transposon ICE plasmid substrate ASI R1 fresh r88","mode":"combined"},
    {"id":"r88-Q2","domain":"biology-microbiome","gap":"substrate",
     "query":"quorum sensing autoinducer AI-2 LuxR LuxS Vibrio fischeri bioluminescence collective decision substrate ASI R2 fresh r88","mode":"combined"},
    {"id":"r88-Q3","domain":"cognitive-neuroscience","gap":"substrate",
     "query":"hippocampal replay preplay theta sharp-wave ripple predictive internal simulation successor representation substrate ASI R3 fresh r88","mode":"combined"},
    {"id":"r88-Q4","domain":"biology-stress","gap":"substrate",
     "query":"heat shock protein HSP90 HSP70 chaperone protein homeostasis stress response substrate ASI R5 fresh r88","mode":"combined"},
    {"id":"r88-Q5","domain":"ecology-mycorrhiza","gap":"substrate",
     "query":"common mycorrhizal network wood wide web carbon nitrogen nutrient transfer interplant signaling substrate ASI R12 fresh r88","mode":"combined"},
    {"id":"r88-Q6","domain":"physics-nonequilibrium","gap":"substrate",
     "query":"dissipative structure Prigogine non-equilibrium self-organization entropy export autopoiesis substrate ASI R10 fresh r88","mode":"combined"},
    {"id":"r88-Q7","domain":"ecology-niche","gap":"substrate",
     "query":"niche construction ecosystem engineering beaver dam organism-environment feedback loop substrate ASI R9 fresh r88","mode":"combined"},
    {"id":"r88-Q8","domain":"github-letta","gap":"github",
     "query":"letta-ai letta github source memgpt memory block agent core architecture sleep-time archival r88","mode":"combined"},
    {"id":"r88-Q9","domain":"github-openhands","gap":"github",
     "query":"all-hands-ai openhands github source code agent runtime sandbox event stream action r88","mode":"combined"},
    {"id":"r88-Q10","domain":"github-dspy","gap":"github",
     "query":"stanfordnlp dspy github source signature module teleprompter optimizer bootstrap few-shot r88","mode":"combined"},
    {"id":"r88-Q11","domain":"reproduction-gap","gap":"reproduction-MISSING",
     "query":"hydra asexual reproduction interstitial stem cell piwi vasa nanos totipotency asexual substrate ASI R6 reproduction Gap fresh r88","mode":"combined"},
    {"id":"r88-Q12","domain":"consciousness-gap","gap":"consciousness-MISSING",
     "query":"integrated information theory Tononi Phi consciousness substrate neural complexity IIT 3.0 substrate ASI R11 consciousness Gap fresh r88","mode":"combined"},
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
        with open("research-v7-round-88.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        ok = res.get("ok", False)
        print("  ...saved partial ({}/12) ok={}".format(i+1, ok))
        time.sleep(1)
    total = time.time() - t0
    print("\nRound-88 done in", round(total, 1), "sec")
    summary = {
        "round": 88,
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "queries_count": len(QUERIES),
        "ok_count": sum(1 for r in results if r.get("ok")),
        "total_sec": round(total, 1),
        "queries": results,
    }
    with open("research-v7-round-88.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print("Final written. ok_count =", summary["ok_count"])


if __name__ == "__main__":
    main()