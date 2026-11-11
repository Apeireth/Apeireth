# apeireth-orchestration

> Foundation layer: the multi-advisor council and orchestration traits that the runtime drives — this crate owns no provider client and no LLM dependency.

`apeireth-orchestration` sits in the **foundation** layer of the Apeireth engine
workspace. It provides the typed advisor boundary, the bounded deterministic evaluation
path, and the scheduling/queueing primitives that the runtime layer composes on top.

## The core surface

- `Council` — `Council::decide(proposal) -> CouncilVerdict`: the compatibility path for
  local advisors (multi-opinion weighted evaluation with a hold/defer exit).
- `Council::decide_with_invoker` — runs bounded advisor side-calls with per-advisor and
  overall timeouts; **the runtime supplies the invoker**.
- `Orchestrator` (trait) — the dispatch boundary for subagents.
- `SubagentSpec` / `SubagentRole` — role and isolation contract for subagents
  (each subagent is expected to run on its own LLM factory instance).

## What is inside

- `llm.rs`, `subagent_llm.rs` — the mirrored LLM traits this crate consumes
- `call_scheduler.rs` — bounded rolling pool, model-order commit, cancel-and-compensate
- `task_queue.rs`, `job_board.rs`, `job_ring.rs` — durable work surfaces
- `context_budget.rs`, `context_fold/`, `context_rot.rs`, `context_overflow.rs`,
  `compaction_checkpoint.rs`, `token_meter.rs` — context engineering
- `cron.rs`, `durable_inbox.rs`, `durable_schedule.rs`, `continuation.rs`,
  `lineage_spawning.rs` — scheduling and continuation
- `quorum.rs`, `capacity.rs`, `cognitive_quota_scheduler.rs`, `research_cost_ledger.rs`
- `self_tuning.rs` — tunable parameter authority for memory/cognitive decay constants
- `untrusted_envelope.rs` — shared reference envelope for untrusted content disclosure
- `nociception.rs`, `repetition_advisory.rs`, `speech_arbiter.rs`, `work_consumption.rs`,
  `ambient_context.rs`, `async_context.rs` — cognitive-loop support surfaces

## Honest limits

- This crate **never owns a provider client**. Provider calls are injected by the
  runtime through a small adapter.
- The `Orchestrator` trait is the boundary only: actually dispatching to a subagent
  requires runtime involvement, because a subagent is a separate LLM factory instance
  and is therefore out of this crate's scope.
- `SelfAssessmentCache` lives in `apeireth-plugin::self_assessment` — this crate
  re-exports it rather than defining a second copy.

## Install

```toml
[dependencies]
apeireth-orchestration = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/foundation/orchestration>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/foundation/orchestration/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
