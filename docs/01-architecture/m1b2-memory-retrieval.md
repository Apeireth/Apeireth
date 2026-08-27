# M1B2 — Canonical Memory Retrieval Semantics

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。

## Status

Implemented on `reconstruct_v2`. No schema change in this phase.

## Donor source

- `origin/master:reconstruction_v2/crates/apeireth-storage/src/memory_v2.rs`
  (`calculate_act_r_activation`, `MemoryStore::query`)

## Reused

- ACT-R-inspired activation formula:
  `sum = Σ max(current_time - t_j, 1)^(-decay)`, `activation = ln(sum) + beta`
  when `sum > 0`, otherwise `beta`.
- Donor query score: `activation + importance * 2.0`.
- Donor default parameters: `decay = 0.5`, `beta = 0.0`, importance weight `2.0`.
- Future access timestamps clipped to a one-second difference.

## Adapted

- The retrieval entry point is a pure function `retrieve` over the canonical
  `MemoryRepository` rather than a method on a combined store.
- `QueryMode` was not ported as a public enum; temporal eligibility is the
  repository's `MemoryFilter` (`as_of`), and tombstone inclusion is explicit.
- Access metadata is not mutated by retrieval. Reads are pure; access tracking
  remains an explicit repository-level operation for a later phase.

## Rejected

- Jaccard similarity and greedy clustering are not part of M1B2.
- No vector, embedding, or LLM ranking.
- No hidden wall clock; `as_of` is always explicit.

## API

- `act_r_activation(access_times, as_of, decay, beta) -> Result<f64, MemoryError>`
- `RetrievalOptions` (as_of, include_tombstones, limit, minimum_importance,
  decay, beta, importance_weight)
- `MemoryHit { item, score }`
- `retrieve(repo, options) -> Result<Vec<MemoryHit>, MemoryError>`

## Determinism

- Invalid numeric parameters (`decay <= 0`, non-finite values) are rejected.
- Ordering: score descending, then `created_at` ascending, then `MemoryId`
  ascending. No `HashMap` iteration order is used.
- Empty access history returns `beta`.

## Tests

Golden activation values, future timestamp clipping, invalid parameters,
temporal eligibility, tombstone inclusion, importance filtering, recency and
access-count ranking, zero-access behavior, stable tie ordering, and
deterministic limit.
