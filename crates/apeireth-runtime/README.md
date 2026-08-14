# `apeireth-runtime` — R147 end-to-end orchestration

**R147 milestone**: 7 modules wired into a single living runtime. Before R147 each
module had rich standalone semantics but no end-to-end driver. This crate is the
concrete driver that proves the modules cooperate correctly.

## 7 modules orchestrated

| # | Module | Source crate | Role |
|---|--------|--------------|------|
| 1 | `HeartbeatScheduler` | `apeireth-supervisor` | Time-driven tick source |
| 2 | `AsyncTaskStore`     | `apeireth-tool-registry` | Pending -> running -> completed |
| 3 | `ChanneledBus`       | `apeireth-bus` | 3-channel fan-out (Ai / Human / Both) |
| 4 | `ArbitrationLog`     | `apeireth-arbitration` | HASH-SQL append-only canonical timeline |
| 5 | `SearchEngine`       | `apeireth-tool-search` | Inverted-index full-text recall |
| 6 | `GroupChat`          | `apeireth-council` | Multi-agent room |
| 7 | `EmotionEngine`      | `apeireth-consciousness` | PAD emotional resonance |

## End-to-end loop

```text
HeartbeatScheduler (Time tick)
  -> dispatch_async_task("classify", payload)
     -> AsyncTaskStore.register(Pending)
     -> worker (spawn) -> mark_running -> complete(result)
     -> ChanneledBus.publish_multi(ChannelSet::BOTH, "async_result", msg)
  -> ArbitrationLog.append(EventSource::AgentComm, "runtime", "task_complete", payload)
  -> SearchEngine.index(doc)
  -> GroupChat.post(ChatMessage)
  -> EmotionEngine.apply(EmotionEvent::TaskSuccess)
```

## Usage

```rust
use apeireth_runtime::{Runtime, RuntimeConfig, EmotionEvent};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeConfig {
        tick_interval: Duration::from_secs(60),
        ..Default::default()
    };
    let rt = Arc::new(Runtime::with_config(config));

    // Stage 1: bootstrap default group chat room + 3 participants
    let room_id = rt.bootstrap()?;

    // Stage 2: run a single end-to-end orchestration cycle
    let report = rt.run_one_cycle().await?;
    println!("cycle done: task_id={} arb_seq={} elapsed={}ms",
        report.task_id, report.arbitration_seq, report.elapsed_ms);

    // Stage 3: register heartbeat + auto-drive
    rt.clone().start().await?;

    Ok(())
}
```

## Architecture decisions

- **`Runtime` is the unified facade**: 7 `pub` fields (`scheduler`, `task_store`, `bus`,
  `arbitration`, `search`, `group_chat`, `emotion`) — each is the real type from the
  underlying crate. No wrapper classes.
- **`SearchEngine` is wrapped in `Arc<SearchEngine>`**: enables `Arc::clone` inside
  spawned worker tasks without copying the index.
- **`GroupChat` got `#[derive(Clone)]` + 2 helper methods (`add_participant_public`,
  `post_message_public`)**: non-breaking additions so the runtime can mutate state
  from outside (the existing `GroupRoomRef` is immutable-only).
- **`EmotionEngine` got `set_baseline()`**: runtime baseline overrides require a
  mutable setter (the original `with_baseline()` is a builder).
- **`apeireth-consciousness::emotion::*` was promoted to lib-level re-exports**:
  `BaseEmotion`, `EmotionEngine`, `EmotionEvent`, `EmotionSnapshot`, `Pad`,
  `ResponseStyle`, `EmError`, `EmResult` are now accessible via
  `apeireth_consciousness::*`.

## Test & demo

```bash
cargo test -p apeireth-runtime           # 10 unit tests covering each module
cargo run -p apeireth-runtime --example r147_full_chain  # 8-stage end-to-end demo
```

The demo runs in <1s, exercises all 7 modules, and prints:

- 3 manual orchestration cycles (trace_id, task_id, arb_seq, search_doc, emotion)
- Search engine recall hits
- Arbitration canonical order (HASH-SQL timeline)
- Group chat message + participant counts
- Emotion PAD snapshot + dominant emotion
- Scheduler infrastructure summary

## R267: Real LLM dispatch via `Runtime::dispatch_llm_task`

The runtime ships a built-in `LlmWorker` that dispatches to a real LLM HTTP endpoint
(default: `https://api.minimaxi.com/v1/chat/completions`, OpenAI-compatible Chat
Completions protocol). It is wired through the full `AsyncTaskStore` -> worker ->
arbitration -> bus -> emotion pipeline.

**API key resolution** (R257 + R26-3-fixes Bearer auth fix):

| Source              | Priority | Notes |
|---------------------|----------|-------|
| `APEIRETH_API_KEY` env var | 1 (highest) | Override for dev/CI |
| `.openclaw\apikey.txt` (Windows) | 2 (default) | Local default |
| `~/.openclaw/apikey.txt` (Unix)                 | 2           | Local default |

The key file is trimmed; empty files fail with a clear error.

**Usage** (gated by `APEIRETH_MINIMAX_LIVE_TEST=1` for CI safety):

```rust
let rt = Arc::new(Runtime::with_config(RuntimeConfig::default()));
rt.register_worker("llm", Arc::new(LlmWorker::new("llm", api_key.clone())));
let params = serde_json::json!({"prompt": "Reply: hi", "system": "terse"});
let task_id = rt.dispatch_async_task("llm", &params.to_string()).await;
let rec = rt.task_store.wait_for_completion(task_id, Duration::from_secs(30)).await?;
assert_eq!(rec.status, TaskStatus::Completed);
```

Or use the all-in-one helper:

```rust
let task_id = rt.dispatch_llm_task("Reply: hi", Some("terse"), None, None, &api_key).await;
```

End-to-end smoke (no test gate):

```bash
cargo run --example r257_minimax_real_api -p apeireth-runtime
```

A successful run prints:

```
[1] API key loaded: sk-cp-...Yb5Wbk (len=125)
[4] Task status: Completed
[4] Task result_json: Some("{\"model\":\"MiniMax-M3\",\"result\":\"hello from MiniMax\",\"task_id\":1}")
SUCCESS: real MiniMax API responded.
```

The example reads the key from disk, builds a `Runtime` + `LlmWorker`,
dispatches a real prompt, waits for completion, and prints runtime metrics
(cycle counters, latency histograms, arbitration event count).

**Key flow**:

```
LlmWorker (HTTP client)
  -> POST {base_url}/v1/chat/completions (Bearer auth, model, messages, temperature, max_tokens)
  -> response.choices[0].message.content -> result_json
  -> AsyncTaskStore.complete(task_id, Ok(result_json))
  -> metrics: llm_latency_seconds histogram + llm_total counter
```

**Honest** (per O-5 不假装):
- Real HTTP call via reqwest; no caching of responses (each dispatch is fresh).
- Bearer auth header always included (R26-3-fixes fix).
- 4xx/5xx errors return `Err(...)` to `AsyncTaskStore.complete(task_id, Err(...))`.
- Network errors return `Err(network: ...)`.
- 30s default timeout per call.

## Compile-time guards

- `MODULES_ORCHESTRATED = 7` — bumping this requires updating `Runtime`'s field count
- `CHANNEL_COUNT = 3`, `NOTIFY_CHANNEL_COUNT = 3`, `WAKEUP_SOURCE_COUNT = 5`,
  `EVENT_SOURCE_COUNT = 6`, `EMOTION_EVENT_COUNT = 12`, `BASE_EMOTION_COUNT = 6`,
  `TURN_POLICY_COUNT = 3`, `PARTICIPANT_ROLE_COUNT = 3` — all hardcoded in their
  respective modules

## Honest stub notes (per O-5 不假装)

- The "worker" inside `dispatch_async_task` is a deterministic simulator (`SimulatedWorker`),
  not a real LLM call. Real workers plug in via the `AsyncWorker` trait.
- Arbitration log uses in-memory SQLite by default; persistence is opt-in via
  `RuntimeConfig.arbitration_path`.
- Group chat participants are seeded with fixed IDs; persistent identity is out of scope.

## Related documents

- `docs/architecture-v4-2-r145-modules/README.md` — 7 module philosophical basis
- `docs/conventions/16-crate-merge-policy.md` §6 — R146 module consolidation
- `docs/conventions/09-anchor.md` — 8 anchor mapping (S-1..S-3 + O-1..O-5)