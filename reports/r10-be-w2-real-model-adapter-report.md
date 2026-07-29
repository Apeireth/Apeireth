# R10-BE-002 — V1128 W2 Real Model Adapter Report

**Task:** `c0ab9024-29a4-4e36-aa85-591c696d86ab`  
**Module:** `apeireth/v1128_real_model_adapter_w2.py`  
**Test:** `tests/test_v1128_real_model_adapter.py`  
**Version:** `0.1.0`

## 1. Result

V1128 adds four real execution paths on top of V1124 without a canned/model-output fallback:

1. **Anthropic** — detects the environment credential, performs a real `/v1/messages` request for deep health, and distinguishes configured, forbidden, unavailable, and healthy states.
2. **Ollama** — probes real `/api/tags`, supports `OLLAMA_BASE_URL`, starts an installed local `ollama serve`, and targets `qwen2.5:1.5b` / `llama3.2:3b`.
3. **Local CLI** — passes prompts as real CLI arguments in an isolated process group.
4. **Executable** — passes prompts over real stdin in an isolated process group and parses bounded stdout.

V1106 `CircuitBreaker` and exponential backoff are reused rather than duplicated. V1128 writes `w2_measurement_started` before a provider call, then appends an explicit success/failure record. A provider crash therefore does not erase or mutate the last durable V1072 identity snapshot.

## 2. Real test data

### Automated suite

Command:

```text
python -m pytest tests/test_v1128_real_model_adapter.py \
  tests/test_v1124_asi_north_star_backend.py \
  tests/test_v1106_engineering_lift.py tests/test_v1072.py -q
```

Result:

```text
collected 284 items
283 passed, 1 skipped in 22.17s
```

- V1128 has **58 collected tests**, exceeding the required 30.
- The one skip is the optional live Anthropic acceptance inside pytest because the repository test harness isolates API-key environment variables.
- The same Anthropic endpoint was probed outside pytest, as recorded below.
- No `unittest.mock`, patching, fake model response, or in-memory provider stub is used.
- Tests use real child processes, real process exit/timeout behavior, real filesystem audit/snapshot state, and real socket probes.

### Live four-path run outside pytest

| Provider path | Live state | Latency | Honest result |
|---|---:|---:|---|
| Anthropic | `forbidden` | 998.46 ms | Real HTTP request reached provider; HTTP 403 `Request not allowed`; **not success** |
| Ollama qwen2.5:1.5b | `unconfigured` | 2013.84 ms | `/api/tags` probe failed and `ollama` executable was absent; **not success** |
| Local CLI contract | `healthy` | 60.77 ms health / 59.03 ms compare | Real isolated Python CLI contract succeeded; **transport evidence only, not LLM evidence** |
| Executable stdin contract | `healthy` | 62.41 ms health / 52.34 ms compare | Real isolated executable contract succeeded; **transport evidence only, not LLM evidence** |

Comparison facts:

```json
{
  "providers_attempted": 4,
  "transport_paths_succeeded": 2,
  "external_or_local_llm_models_succeeded": 0,
  "baseline_proxy": 0.8538,
  "w2_target": 0.90,
  "target_claimed": false
}
```

The successful CLI/executable probes validate process integration, isolation, stdout parsing, hashing, persistence, fallback, and circuit behavior. They are intentionally **not** presented as Qwen/Llama/Claude model inference.

## 3. Reliability evidence

- Circuit opens after the configured number of real process crashes.
- An open circuit rejects without spawning another process.
- After the recovery interval, a real successful process call closes the half-open circuit.
- Retry tests verify the process ran exactly twice through a durable counter file.
- Timeout handling kills the real child process.
- Non-zero exit, empty output, missing executable, and output over 1 MiB are explicit failures.
- Ollama remote URLs are probed but never auto-started locally.
- Raw prompts and raw model output are excluded from comparison/audit output; SHA-256 evidence is retained.
- Failed providers receive no ASI score.
- Restart after a provider crash recovers the identical V1072 identity ID.

## 4. Philosophy gates

- **主 22:33 ASI 北极星:** W2 target remains 0.90, but observed baseline remains 0.8538.
- **主 17:43 实事求是:** 403, missing executable, and unavailable endpoint are recorded with their real state.
- **主 17:58 不假装:** transport execution is not called intelligence; identity continuity is not phenomenal consciousness.
- **主 23:44 干到底:** retry, breaker recovery, process isolation, chaos persistence, and four-path live probing are exercised.
- **主 19:33 走在前人经验上:** V1106's Netflix-style circuit breaker and AWS-style exponential backoff are reused.

## 5. Environment actions required for model acceptance

The implementation is ready, but this machine cannot honestly pass multi-model inference acceptance until at least one of the following is supplied:

- an Anthropic credential permitted to call the configured Anthropic model;
- an installed Ollama runtime with `qwen2.5:1.5b` and/or `llama3.2:3b` pulled;
- a genuine local model CLI/executable configured through `V1128_LOCAL_CLI` or `V1128_EXECUTABLE`.

No dependency or multi-gigabyte model was downloaded automatically. Ollama daemon auto-start is limited to an already-installed local runtime, avoiding unexpected installation and resource consumption.
