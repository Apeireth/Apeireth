# apeireth-core

> Apeireth 主路径核心类型 (Episode/Note/Session/IdentityCard) — R14 Phase 1 入口

Apeireth **v2.0.0-rc.4** 工作区 crate（30 members 之一）。`src` 现有 **17 个公开模块**：

`clock` / `cost_router` / `deadline` / `eight_anchors` / `gate` / `kernel` / `lifecycle` /
`model_profile` / `onion` / `onion_gate` / `philosophy` / `release_stats` / `resource_lease` /
`statechart` / `storage_atomic` / `stored_doc` / `user_domain`

> **哲学锚口径（0 装订正，2026-10-25）**：`eight_anchors` 模块里的类型名 `PhilosophicalAnchor8`
> 是**历史命名**，现行锚数是 **9**：`ALL_NINE_ANCHORS: [PhilosophicalAnchor8; 9]` /
> `ALL_NINE_ANCHOR_CODES: [&str; 9]`（另有 `LEGACY_SIX_ANCHOR_CODES: [&str; 6]` 供历史互转）。
> 本行此前写「8 锚 / `ALL_EIGHT_ANCHORS`」是过期口径，已订正；**锚本体属 LOCKED 5 项，
> 正文历史值不改写，口径以 `crates/foundation/core/src/eight_anchors.rs` 与 `AGENTS.md` 为准**。
> 模块清单以 `src/lib.rs` 的 `pub mod` 为唯一真源（此前列的 `memory` / `organ_kani_proofs` 已不在该 crate）。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
