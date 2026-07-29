# R9 Performance Optimizer Report — V1118

> Task `08e37de5-e292-4d27-95fe-d54e3278e190`
> 主 17:43 实事求是：只报告可复现的 wall-clock/cProfile 数据，不把缓存命中、代码行数或分数波动包装成 ASI 进步。

## 1. Verdict

**PASS** — five real, independently switchable optimizers were implemented. Three-process cold V1074 median fell from **3.252062 s** to **1.018498 s**: **−68.6815%, 3.1930×**, with six of six runs returning `all_ok=true`. This exceeds both stated gates (`<2.5 s`, reduction ≥20%) and the earlier 18% requirement.

Implementation commit:

```text
638e9624d0ed3db9d39860b476a132d7f39018b1
```

## 2. Profile evidence and top-five hot call chain

Required command:

```text
python -m cProfile -o v1074.prof -m apeireth.v1074_asi_production_runner --report --no-write
```

Observed baseline: **718,063 calls in 3.348 s**.

| Rank | Function | Calls | Cumulative |
|---:|---|---:|---:|
| 1 | `StatusSnapshotBuilder.measure_v03` | 1 | 2.999 s |
| 2 | `v1073_run` | 1 | 2.998 s |
| 3 | `ASIIntegrationBridge.run_full_measurement` | 1 | 2.998 s |
| 4 | `End2EndPipeline.run` | 1 | 2.998 s |
| 5 | `V1073Integrator.measure_v02_base` | 1 | 2.979 s |

Actionable leaf evidence beneath that call chain:

- `measure_asi_v02_real`: 2.978 s cumulative;
- `subprocess.run`: 3 calls / 2.720 s cumulative;
- `measure_phi_proxy`: 2.662 s cumulative;
- buffered subprocess reads: 2.654 s cumulative.

The bottleneck was not JSON or Markdown. It was `pytest --collect-only` inside `measure_phi_proxy`, plus duplicated project/git metrics. The optimized profile no longer contains pytest collection in the hot path. Its parent profile measured 1.195 s including process-pool shutdown under cProfile; independent unprofiled median is 1.018498 s.

## 3. Formula-equivalence proof for the fast path

Before implementation, the same repository state produced:

| Metric | V1074 direct count | V1048 score | Formula-equivalent value |
|---|---:|---:|---:|
| modules | 1,127 | 0.751333 | 1,127 / 1,500 = 0.751333 |
| test functions | 5,408 | 1.000000 | clamp(5,408 / 2,000) = 1.0 |
| commits | 471 | 0.942000 | 471 / 500 = 0.942000 |

V1118 passes these values through V1048's existing, public `measure_asi_v02_real(scores=...)` API; the remaining 13 dimensions still execute normally. The `phi_proxy` substitution is allowed only when:

1. `project_dir` resolves to the same directory V1048 would measure (`cwd`), and
2. static test count is ≥2,000, so both formulas are provably clamped to 1.0.

Otherwise it calls the original V1074/V1073 measurement. No score is hard-coded.

## 4. Five implemented optimizers and benchmarks

All microbenchmarks use `time.perf_counter`, real serialization/file enumeration/git/format work, and semantic equality checks. No sleeps or zero-work timing loops were used.

| Optimizer | Before | After | Result | Semantic guard |
|---|---:|---:|---:|---|
| 1. Deferred V1073 import/binding, 5,000 resolutions | 2.6232 ms | 0.3257 ms | **−87.5839%, 8.054×** | identical callable object |
| 2. Compact JSON snapshot, 2,000 serializations | 44.0 µs | 42.4 µs | **−3.6364%** | `json.loads(before) == json.loads(after)` |
| 3. Two-process project dimensions, 15 trials | 62.9878 ms | 39.3860 ms | **−37.4704%, 1.599×** | identical `ProjectMetrics` |
| 4. State-keyed ProjectMetrics LRU, 20 real scans/hits | 61.7611 ms | 9.4411 ms | **−84.7135%, 6.542×** | all 20 cached values equal fresh scans |
| 5. Precompiled Markdown, 5 groups × 5,000 renders | 78.15 µs* | 75.80 µs* | **median −3.0070%** | byte-for-byte equal report |

`*` Representative group whose reduction equals the five-group median. All five groups were positive (2.276%–3.376%) and byte-equal.

Snapshot payload size in the measured sample changed from **8,570 bytes to 6,229 bytes** (**−27.3162%**) by removing redundant JSON whitespace. This is the main benefit of optimizer 2; the smaller CPU gain is reported rather than exaggerated.

### Optimizer 1 — `LazyImporter`

- thread-safe double-check resolution;
- imports V1048/V1073 only on demand;
- binds the resolved module/attribute once;
- exposes resolution count/time and a real benchmark API.

### Optimizer 2 — `SnapshotCompressor`

- one compact `json.dumps(..., separators=(",", ":"))` call in production;
- UTF-8 and `default=str` semantics retained;
- no previous draft's double serialization merely to calculate stats;
- artifact test reads the compact file back and checks the score.

### Optimizer 3 — `ParallelDimensionEvaluator`

- real `ProcessPoolExecutor`, explicit Windows-safe `spawn` context;
- exactly **2 workers**;
- picklable top-level jobs for tests/modules/commits;
- longest jobs submitted first so test scan and git overlap;
- persistent pool within an orchestrator; safe serial fallback if processes are forbidden;
- cold process startup remains included in the primary independent-process benchmark.

### Optimizer 4 — `SubmoduleResultCache`

- true `OrderedDict` LRU: reads update recency and `popitem(last=False)` evicts;
- hard maximum 32 entries;
- thread-safe;
- cache scope is deliberately **only ProjectMetrics**, not dynamic V1073/V1072 scores;
- invalidation token hashes exactly the metric inputs: `apeireth/v*.py` names, `tests/test_v*.py` name/size/mtime, and Git HEAD;
- state-token median: about 9.3–10.0 ms, down from the rejected over-broad 82.6 ms prototype.

### Optimizer 5 — `MarkdownTemplateCompiler`

- one precompiled report format;
- static guard/header/footer reused;
- reference block cached with O(1) identity hits and content-key fallback;
- output is byte-for-byte equal with and without history.

## 5. Orchestrator and safety properties

`V1118Optimizers` provides `enable`, `disable`, `enable_all`, `disable_all`, `is_enabled`, `wrap`, `unwrap`, `stats`, `close`, and `bench`.

Important safety differences from the rejected draft:

- no global `subprocess.run` monkey-patch;
- no duplicate serial-then-parallel work in a production evaluation;
- no FIFO masquerading as LRU;
- no zeroing of the 17-dimension snapshot;
- no truncated Markdown report;
- no baseline factory pre-wrapped by accident;
- no dynamic V1073 result caching;
- `unwrap` restores every patched instance method.

The compatibility module `apeireth/v1118_performance_optimization.py` re-exports the canonical implementation because the assigned specification names both module paths. There is one implementation source of truth.

## 6. V1074 before/after evidence

Primary independent-process result (3 trials each):

| Side | Runs | Median |
|---|---|---:|
| before | 3.163009 / 3.252062 / 3.259237 s | **3.252062 s** |
| after | 1.004566 / 1.018498 / 1.073251 s | **1.018498 s** |

- saved: 2.233565 s;
- reduction: 68.6815%;
- speedup: 3.1930×;
- vs injected 3.05 s baseline: 66.6066% reduction;
- all return codes 0 and all `all_ok=true`.

Raw focused evidence: [`reports/v1074_perf_before_after.md`](v1074_perf_before_after.md).

## 7. Tests and guards

Final command:

```text
python -m pytest tests/test_v1118_perf_optimizer.py tests/test_v1074.py -q
```

Result:

```text
139 passed in 44.82s
```

Breakdown:

- 61 new V1118 tests (requirement ≥30);
- 78 existing V1074 regressions;
- Windows 2-worker spawn tested with no fallback;
- compact JSON round-trip/file write;
- true LRU recency/eviction/None values/clear;
- dynamic measurement is re-executed on cache hits;
- state token invalidates test changes but ignores irrelevant module-body changes;
- Markdown byte equality with/without history;
- real V1074 `<2.5 s`, `all_ok`, score `<1.0`, and cache-hit guards.

## 8. Files

- `apeireth/v1118_perf_optimizer_v01.py` — canonical implementation (1,126 lines)
- `apeireth/v1118_performance_optimization.py` — compatibility CLI/re-export (12 lines)
- `tests/test_v1118_perf_optimizer.py` — 61 tests (636 lines)
- `reports/v1074_perf_before_after.md` — focused raw before/after evidence
- `reports/r9-performance-optimizer-report.md` — this report
- `reports/r9-performance-optimization-report.md` — required-name compatibility index

## 9. Honest limits / upgrade triggers

- Parallelism is beneficial for current project size, but its pool has cold-start cost. Keep the independent-process benchmark as the release gate.
- Test count substitution is intentionally disabled below 2,000 tests; a future exact fast collector would be needed there.
- File metadata is an invalidation signal, not a content-addressed build system. If timestamps can be rewritten without size/mtime change, upgrade the test part of the token to content hashes.
- Markdown rendering is already a microsecond-scale non-bottleneck; its ~3% local gain must not be presented as the cause of the 68.7% end-to-end improvement.
- Optimizer code is tooling, not ASI; V1074 score remains a proxy, not truth.
