**Source**: https://raw.githubusercontent.com/wilmanrojas/sinqua/main/README.md

---

# agent-runtime-bench

A controlled, apples-to-apples benchmark of **agent runtimes** — the orchestration
layer that drives an LLM through a write → execute → self-correct loop — across
C++, Python, TypeScript, and Rust.

## Why this matters

When people compare "coding agents" they almost always compare the *model*
(pass@1 on HumanEval, SWE-bench, etc.). But in production the model runs behind a
**runtime**: the code that fans out hundreds of agents, streams tokens, spawns
test processes, retries on failure, and tracks state. That runtime — not the
model — decides:

- **Memory footprint** when you run 100+ agents at once,
- **Concurrency ceiling** and tail behavior under load,
- **Overhead** added on top of model latency.

These costs dominate the bill once agents move to scale, yet there is **no
controlled cross-language comparison** of agent runtimes. Published numbers aren't
comparable: different hardware, different model, different framework. This project
fixes the variables — *same tasks, same model, same hardware, same loop logic* —
and changes only the language runtime, so the runtime's cost is isolated and
measurable.

## The workload

[HumanEval](https://github.com/openai/human-eval) (first 100 problems). For each
task the runtime runs a real agentic loop, not one-shot codegen:

```
build prompt (spec + pytest)
  → LLM completion (streamed)
  → extract the Python code block
  → write solution.py into an isolated workspace
  → run `python3 -B -m pytest`
  → pass?  → Done
     fail? → feed the pytest error back into the prompt, retry (max 3)
  → still failing → Failed
```

The agent must *write code, run the tests, read the failure, and fix itself* —
which is what exercises the runtime (concurrency, process spawning, I/O, memory),
not just the model.

## The C++ runtime

| Component | What it does |
|---|---|
| `ThreadPool` | 100 `std::jthread` workers, per-worker work-stealing deques |
| `LLMClient` / `AsyncLLMClient` | libcurl + SSE streaming to any OpenAI-compatible endpoint (sync, and curl_multi async) |
| `ToolDispatcher` | atomic `write_file`; `bash` via fork/exec with separate stdout/stderr, timeout (process-group SIGKILL) and per-call workspace; plus `read_file` / `list_dir` / `search` |
| `AgentLoop` | the write → pytest → retry loop, one isolated workspace per agent |
| `Telemetry` | background RSS sampler (peak), per-task metrics, CSV + summary JSON with p50/p95/p99 |
| Dataset loader + runner | loads `dataset/humaneval_100.json`, fans the tasks across the pool, writes the report |

No heap-heavy framework: just the standard library, libcurl, nlohmann/json and
spdlog. Every component is covered by tests built with `-UNDEBUG` so assertions
stay live even in Release.

## Results — C++ baseline

100 HumanEval tasks, `qwen2.5-coder:7b`, 100-way concurrency, single GPU:

| Metric | Value |
|---|---|
| **Peak RSS (100 concurrent agents)** | **~93 MiB** |
| pass@1 (with up to 3 self-review retries) | 96 % (96/100) |
| first-attempt pass | 87/100 |
| recovered via self-review | 6 |
| failed after 3 retries | 4 |
| avg retries | 0.27 |
| wall time (100 tasks) | 126 s |

**How to read these honestly:**

- **Peak RSS is the runtime number.** ~93 MiB for 100 concurrent agents is the
  headline for the C++ stack — and the metric that will actually differ between
  languages.
- **pass@1 and retries are model properties**, not runtime properties — they will
  be identical across stacks. They're here to prove the harness runs a *real*
  agentic loop (the self-review recovered 6 tasks), not to compare runtimes.
- **Per-task latency is intentionally omitted from the headline.** At 100-way
  concurrency against one GPU, per-task time is dominated by server-side queueing,
  not the runtime. Throughput (`wall time`) is likewise model-bound here. For
  comparable latency, run with bounded concurrency (`BENCH_CONCURRENCY`).

## Running the benchmark (C++ stack)

The agent loop verifies generated code with `python3 -B -m pytest`, so the
`python3` first on your `PATH` must be able to import `pytest`.

```bash
# 1. Install pytest into whichever python3 you use (no virtualenv required)
python3 -m pip install -r runner/requirements.txt

# 2. Verify the environment is ready
runner/run_bench.sh --check

# 3. Build
cmake -B build -G Ninja && cmake --build build

# 4. Run against an OpenAI-compatible endpoint
LLM_BASE_URL="https://your-host/v1" LLM_MODEL="qwen2.5-coder:7b" runner/run_bench.sh
```

`run_bench.sh` picks an interpreter that already has `pytest` and puts it first on
`PATH` — it does not create a virtualenv. Results land in `results/raw/` as a
timestamped `cpp_<ts>.csv` (one row per task) and `cpp_<ts>_summary.json`.
Override `LLM_BASE_URL` / `LLM_MODEL` / `BENCH_DATASET` / `BENCH_CONCURRENCY` via
env.

## Dataset

`dataset/humaneval_100.json` holds the 100 tasks the runner feeds to the
`AgentLoop`. It is an array of `{ "id", "spec", "test" }` objects matching the
runner's `struct Task`:

- `id` — the HumanEval task id (e.g. `HumanEval/0`).
- `spec` — the HumanEval prompt (function signature + docstring) given to the model.
- `test` — a pytest module that imports the entry point from `solution` and runs
  HumanEval's `check()` via `test_humaneval`.

**Source.** OpenAI HumanEval (164 problems), pinned to commit `463c980b` of
[`openai/human-eval`](https://github.com/openai/human-eval) and verified against a
recorded SHA-256.

**Selection criterion.** The first 100 problems by ascending numeric task index
(`HumanEval/0` … `HumanEval/99`). Fully deterministic — no sampling or randomness.

Regenerate (and validate that every canonical solution passes its test under pytest):

```bash
python3 scripts/build_humaneval_dataset.py            # build the JSON
python3 scripts/build_humaneval_dataset.py --validate # build + verify 100/100
```

## Status & roadmap

- **C++ stack — done.** Full runtime + telemetry + 100-task run.
- **Python / TypeScript / Rust — pending.** Each needs a minimal runner with the
  same contract (thread pool / async, OpenAI client, write+bash tools, the loop,
  telemetry). A cross-language comparison is only valid when every stack runs the
  **same tasks, model, hardware and concurrency** — which is the whole point, and
  why external numbers can't simply be cited for the runtime metrics.

The deliverable is the **controlled measurement** (peak RSS + runtime overhead at
fixed concurrency), not the model's pass-rate — that part is already well
documented elsewhere.