# R10-BE-003 — V1130 ASI 北极星 backend v2 Report

**Task:** `88c94996-004a-4e14-8602-6e4eb2ef6a50`  
**Module:** `apeireth/v1130_asi_north_star_backend_v2.py`  
**Test:** `tests/test_v1130_asi_north_star_backend_v2.py`  
**Version:** `0.1.0`

## 1. Result

V1130 is the R10 W3 reinforced backend that reuses V1124 (`ASINorthStarBackend`),
V1128 (real provider routing), V1125 (`compute_v05_score`), V1118 (process pool
& LRU pattern) and V1130 (`AlertSink`, `_safe_subprocess_call`).  It exposes:

- Four real provider paths evaluated in parallel via `ThreadPoolExecutor`.
- V0.5 18-dim formula `v05 = v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05`
  reused from `apeireth.v1125_r10_integration_protocol.V05Score`.
- V1074 runtime sampler with baseline `3.05s`, target `<2.5s`.
- fail-soft wrappers `fail_soft(...)` and `run_subprocess_with_fail_soft(...)`.
- chaos: `w3_plan_started/succeeded/failed` audit events plus `AlertSink` alerts.
- HTTP endpoints `GET /asi/v1130/{level,runtime,alerts}` and
  `POST /asi/v1130/evaluate`, falling through to V1124 for other paths.

## 2. Real test data

### Automated suite

```text
python -m pytest tests/test_v1130_asi_north_star_backend_v2.py \
  tests/test_v1128_real_model_adapter.py \
  tests/test_v1124_asi_north_star_backend.py \
  tests/test_v1106_engineering_lift.py tests/test_v1072.py -q
```

```text
collected 332 items
331 passed, 1 skipped in 24.72s
```

- V1130 has **48 collected tests**, exceeding the required 30.
- No mock API (`unittest.mock`, `MagicMock`, `patch`) is used.
- Tests run real subprocesses, real `ThreadingHTTPServer` (403 Anthropic),
  real file audit/snapshot state and real concurrency.
- The single skip is the live Anthropic acceptance test inside pytest because
  the harness isolates API-key environment variables.

### Live cross-provider + V1074 runtime sample (outside pytest)

| Provider path | State | Latency | Honest result |
|---|---:|---:|---|
| Anthropic | `forbidden` | 1006.9 ms | real `/v1/messages` returned HTTP 403 |
| Ollama qwen2.5:1.5b | `unavailable` | 2045.2 ms | no `ollama` executable, no remote URL |
| local_cli contract | `healthy` | 49.92 ms | real isolated Python CLI contract |
| executable stdin contract | `healthy` | 49.76 ms | real isolated executable contract |

Aggregated facts:

```text
providers_attempted   = 4
providers_succeeded   = 2     (transport contracts only, NOT LLM models)
providers_forbidden   = 1     (Anthropic HTTP 403)
providers_unavailable = 1     (Ollama daemon missing)
v04_score             = 0.8538
v05_score             = 0.8532
passes_r10_start      = False
passes_r10_ultimate   = False
identity_preserved    = True
parallel_wall_seconds = 2.047  (target <= 2.5)
alerts                = 1 YELLOW (v05 < R10_START)
```

V1074 runtime sampler (warm workload):

```text
mean_seconds   = 0.00016
median_seconds = 0.00013
max_seconds    = 0.00024
target_seconds = 2.5
passes_target  = True
savings_pct    = 99.99 (vs 3.05 baseline)
```

The two `healthy` paths are real transport contracts (isolated Python CLI /
executable subprocess), explicitly labelled and never counted as Qwen / Llama
/ Claude model inference.

## 3. Chaos and recovery evidence

- Cross-provider chaos: two `crasher` providers → `w3_plan_failed` audit
  recorded, `identity_preserved=True`, `AlertSink` emits `RED` and never
  fabricates success.
- Identity durability: reloading the backend after provider crashes reuses
  the same `identity_id` (V1124 durable store + V1072 `IdentityManifest`).
- Sink write failure: `AlertSink(persist_path=missing-dir/...)` still keeps
  the alert in memory and never throws.
- Runtime regression: a slow runner (`time.sleep(0.4)`) yields
  `passes_target=False`, recorded with `fail_soft`/`AlertSink` warnings.

## 4. Provider latency comparison (parallel wall ≈ max latency)

| Provider | Wall seconds (sample) | Transport | Process | Process group |
|---|---:|---|---|---|
| Anthropic | 1.007 | HTTP POST | urllib.request | n/a |
| Ollama | 2.045 | HTTP GET + attempt | subprocess.Popen (ollama serve) | detached |
| Local CLI | 0.050 | exec via args | subprocess.Popen | detached |
| Executable stdin | 0.050 | exec via stdin | subprocess.Popen | detached |

Anthropic and Ollama are network-bound; the local contracts are
process-bound and complete in tens of milliseconds.

## 5. Philosophy gates

- **主 22:33 ASI 北极星:** baseline `0.8538`, v05 `0.8532`.  `passes_r10_ultimate=False`.
- **主 17:43 实事求是:** Anthropic HTTP 403 and missing Ollama daemon are reported truthfully.
- **主 17:58 不假装:** local CLI / executable success is recorded as
  transport contract only; the comparison row says "2/4 transport, 0 LLM".
- **主 23:44 干到底:** chaos test asserts identity survives and alerts are
  emitted; runtime sampler fails loud when above target.
- **主 19:33 走在前人经验上:** V1124/V1125/V1128/V1118/V1130 are reused,
  no new framework invented.
- **主 12:14 中央 AI 是永恒身份:** durable snapshot + audit chain protect
  measurement identity across provider failures.

## 6. Environment actions required for full LLM acceptance

The implementation supports real LLM access but this machine cannot honestly
assert ≥3 real model successes until one of:

- an Anthropic credential permitted to call the configured Anthropic model,
- an installed Ollama runtime with `qwen2.5:1.5b` / `llama3.2:3b` pulled,
- a genuine local model CLI/executable configured through `V1128_LOCAL_CLI`
  or `V1128_EXECUTABLE`,

is supplied.  The cross-provider shell never auto-installs a model daemon or
downloads weights.