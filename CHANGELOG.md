# Changelog — Apeireth

> **R119-5 Mavis 收尾 (2026-08-10)**: 顶层 CHANGELOG 从 76KB 缩到 ~3KB。完整 release notes 下沉到 `docs/release/<version>/`。

```
[Document-Meta]
Document: CHANGELOG.md
Version: 1.1.0-R114
R-Cycle: R119-5
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

格式: [Keep a Changelog](https://keepachangelog.com/) + Semantic Versioning

## Release 索引

| 版本 | 日期 | 主题 | 详细 |
|---|---|---|---|
| **v1.0.0** | 2026-08-05 | R20 阶段 1-6 1.0 release 收口, 12 项 checklist 100% PASS | [`docs/release/1.0.0/CHANGELOG.md`](docs/release/1.0.0/CHANGELOG.md) |
| **v1.1.0** | 2026-08-09 | R38 1.1 RC 9 B-stage 一气呵成 (telemetry 4→1 + provider 5→1 真合并) | [`docs/release/1.1.0/CHANGELOG.md`](docs/release/1.1.0/CHANGELOG.md) |
| **v1.1.1** | 2026-08-09 | R46-R53 follow-up (5 R: mini-redis + cognition_graph + docs index + CI summary + README badges) | [`docs/release/1.1.1/CHANGELOG.md`](docs/release/1.1.1/CHANGELOG.md) |
| **v1.1.2** | 2026-08-09 | R54 B8 续升级 (backend wire-up + cognition_graph 真接 TUI memory + render 0 假装小修) | [`docs/release/1.1.2/CHANGELOG.md`](docs/release/1.1.2/CHANGELOG.md) |
| **v1.1.2-followup-2** | 2026-08-09 | R57-R62 1.1.2 follow-up-2 (Cargo audit + RUSTSEC 续) | [`docs/release/1.1.2-followup-2/CHANGELOG.md`](docs/release/1.1.2-followup-2/CHANGELOG.md) |
| **v1.2-candidate** | 2026-08-09 | R63-R68 1.2 candidate (5 既有 crate 加 submodule) | [`docs/release/1.2-candidate/CHANGELOG.md`](docs/release/1.2-candidate/CHANGELOG.md) |
| **v1.2-patch-LIVE** | 2026-08-09 | R70-R72 1.2 patch LIVE (LIVE MiniMax 7 model + MCP subscribe push) | [`docs/release/1.2-patch-live/CHANGELOG.md`](docs/release/1.2-patch-live/CHANGELOG.md) |
| **v1.2-patch-LIVE-续** | 2026-08-10 | R78-R113 1.2 patch LIVE 续 (11 R + 1 LIVE: skills / graph / MCP 真接) | [`docs/release/1.2-patch-live-followup/CHANGELOG.md`](docs/release/1.2-patch-live-followup/CHANGELOG.md) |
| **v1.2-R114-118** | 2026-08-10 | R114-R118 动态运营层 (codex 5c546a84, 源仓 4921 / 88 suites / 0 failed) | [`docs/release/1.2-r114-r118/CHANGELOG.md`](docs/release/1.2-r114-r118/CHANGELOG.md) |

## R 周期报告(思想历史)

R 周期报告(R17 → R38 → R54 → R70-R72 → R78-R113 → R114-R118)按"思想历史 + 最新"原则筛选保留在 [`docs/release/`](docs/release/)。

## 历史归档

R11 末 / R12 接手 / R13 MVP / R14 Rust 重写 / R17 战役 0-4 / R23 baseline 24 LOCKED 等历史阶段归档在 [`docs/release/archive/`](docs/release/archive/)。

---

_本 CHANGELOG 由 Mavis R119-2 重写,原 76KB 详单下沉到 `docs/release/<version>/CHANGELOG.md`。codex 5c546a84 R114-R118 状态保留在 v1.2-R114-118 索引行。_
