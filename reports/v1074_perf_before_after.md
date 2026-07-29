# V1074 Performance Before/After — R9-PO-002 / V1118

- Task: `08e37de5-e292-4d27-95fe-d54e3278e190`
- Implementation commit: `638e9624d0ed3db9d39860b476a132d7f39018b1`
- Platform: Windows 11, CPython 3.13.14
- Timer: parent-process `time.perf_counter()` around a fresh child process
- Trials: 3 independent processes per command; median is the decision statistic
- Artifact writes: disabled for both sides

## Commands

```text
Before: python -m apeireth.v1074_asi_production_runner --report --no-write --print-json
After:  python -m apeireth.v1118_performance_optimization --run --print-json
```

The optimized command starts a new interpreter and a new two-worker process pool on every trial. Therefore this comparison includes cold interpreter/import/process-start overhead and does not count same-process LRU warm hits as cold-start savings.

## Raw runs

| Side | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|
| V1074 before | 3.163009 s | 3.252062 s | 3.259237 s | **3.252062 s** |
| V1118 after | 1.004566 s | 1.018498 s | 1.073251 s | **1.018498 s** |

## Result

| Metric | Result | Gate |
|---|---:|---:|
| Wall time saved | 2.233565 s | — |
| Reduction vs same-machine measured baseline | **68.6815%** | ≥20% ✅ |
| Speedup | **3.1930×** | — |
| Optimized median | **1.018498 s** | <2.5 s ✅ |
| Reduction vs injected V1110 W1 3.05 s baseline | **66.6066%** | ≥18% / ≥20% ✅ |

All six processes returned code 0 and `all_ok=true`.

## Score truth guard

| Side | Scores |
|---|---|
| Before | 0.8922, 0.8926, 0.8921 |
| After | 0.8933, 0.8923, 0.8931 |

V1072/V1073 includes dynamic session state, so exact cross-run equality is neither expected nor claimed. V1118 deliberately does **not** cache the dynamic V1073 result; every run recomputes it. The ranges overlap, all scores remain below 1.0, and all V3/V1074 gates pass.

## Same-process operational benchmark (secondary)

`python -m apeireth.v1118_performance_optimization --bench --n-trials 3 --print-json`:

- baseline: 2.963844 / 3.330682 / 3.060616 s; median 3.060616 s
- optimized: 0.591498 / 0.225692 / 0.189326 s; median 0.225692 s
- reduction: 92.6259%
- cache: 1 miss + 2 state-keyed ProjectMetrics hits
- dynamic fast path executions: 3 (not cached)

This secondary number represents repeated gates in one process. The independent-process 68.6815% result above is the primary acceptance evidence.

## Reproduction

```text
python -m cProfile -o v1074.prof -m apeireth.v1074_asi_production_runner --report --no-write
python -m apeireth.v1118_performance_optimization --bench --n-trials 3 --print-json
python -m pytest tests/test_v1118_perf_optimizer.py tests/test_v1074.py -q
```

Measured repository counts continued changing during parallel R9 integration. After implementation commit `638e962`, V1074 observed 1,139 modules, 5,844 test functions, and 487 commits. V1118 derives these values at runtime; no report value is hard-coded into production code.
