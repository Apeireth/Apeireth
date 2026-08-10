# Apeireth — AGI 操作系统 (Rust 重写)

> **R119-5 Mavis 收尾 (2026-08-10)**: 顶层 README 从 48KB 缩到 ~4KB。codex R114-R118 动态运营层(源仓 4921 passed / 88 suites / 0 failed,workspace.version 1.1.0)以状态行保留,历史技术报告下沉到 `docs/release/`,根目录 100+ 临时文件 + 6.8GB src-tauri + 277GB target 全清。

```
[Document-Meta]
Document: README.md
Version: 1.1.0-R114 (顶层)
R-Cycle: R119-5 (10 commit 收尾, R114-R118 之上重建)
Commit: 5c546a84 (R114-R118 动态运营层基线) + R119-1..R119-5
Last-Modified: 2026-08-10
Status: 🟢 活跃 (R114-R118 动态运营层已真接, R119 文档体系推倒重建 + 根目录清)
```

[![CI](https://github.com/apeireth/apeireth-rust/actions/workflows/rust-ci.yml/badge.svg)]()
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%20stable-orange.svg)](rust-toolchain.toml)
[![Version](https://img.shields.io/badge/version-1.1.0-blue.svg)](Cargo.toml)
[![Eval-LIVE](https://img.shields.io/badge/eval--live-MiniMax-blueviolet.svg)](.github/workflows/eval-live.yml)
[![R-Measure](https://img.shields.io/badge/R--Measure-0.92-success.svg)](reports/)

**Apeireth** = VCP 的全栈 Rust 重写 + 独家的形式化安全(Self-Disable) + 双洋葱架构 + 编译期保证(12 键 hardcode)。
5 战区(终端 Coding Agent / LLM 网关 / Multi-Agent / 长期记忆 / 工具协议)同时打,终极前端 Tauri,TUI 是"集成测试床"。

---

## 🚀 快速开始

跳 [`docs/installation/01-quick-start.md`](docs/installation/01-quick-start.md) — 5 分钟跑通。

## 🏛️ 规范系统

跳 [`docs/conventions/README.md`](docs/conventions/README.md) — 12 子规范系统(命名空间 / 路径 / ADR / 报告 / Commit / 状态标记 / 哲学锚穿透等)。

## 🏗️ 架构

跳 [`docs/v2-strategy/00-VISION.md`](docs/v2-strategy/00-VISION.md) — 5 战区战略 + 核心护城河(v2 / v4 / v4.1 哲学层)。

## 🛣️ 路线图

跳 [`docs/roadmap/README.md`](docs/roadmap/README.md) — R 周期 / 1.0 / 1.1 / 1.2 / 1.2 patch LIVE 时间线。

## 📚 文档

跳 [`docs/README.md`](docs/README.md) — 完整文档索引(按子目录分类)。

## 🤝 贡献

跳 [`CONTRIBUTING.md`](CONTRIBUTING.md) — 贡献流程 / commit 规范 / 8 项不修改承诺(形式撤销, 原意保留)。

---

## 📊 状态(2026-08-10)

| 指标 | 值 |
|---|---|
| **源仓测试** | **4921 passed / 88 suites / 0 failed** |
| **Desktop 同步** | 4489 passed / 88 suites / 0 failed |
| **workspace.version** | 1.1.0 (semver 严守) |
| **R 周期** | R114-R118 动态运营层已真接 |
| **24 LOCKED crate** | 0 触 (mtime baseline 16:34 之前) |
| **R11 baseline** | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 严守 |
| **R-Method** | 0.92 badge |

**R114-R118 动态运营层**(2026-08-10, codex 5c546a84):
- R114: `EvalToolServer` MCP bridge(`crates/apeireth-eval/src/mcp_bridge.rs`)
- R115: Council MCP bridge(`crates/apeireth-council/src/mcp_bridge.rs`)
- R116: CLI command families(`crates/apeireth-cli/src/commands/`)
- R117: TUI cognition live(`crates/apeireth-tui/src/cognition_live.rs` 接 main.rs:259)
- R118: Protocol transport bridges(`crates/apeireth-protocol/src/bridge_ext.rs:43`)

详见 [`reports/r114-r118-batch-final-2026-08-10.md`](reports/r114-r118-batch-final-2026-08-10.md)。

---

## 📜 引用

- 借鉴:LangGraph / AutoGen / MCP 2025-03-26 / RFC 8628 / VCP vcptoolbox / LSP / semver
- 哲学:6 哲学锚(S-1 / S-2 / O-2 / O-3 / O-4 / O-5) + 12 键编译期 hardcode
- 6 哲学锚来源:主人 2026-07-30 ~ 2026-08-04 关键决策(详见 `docs/conventions/09-anchor.md`)

---

_本 README 由 Mavis R119-2 重写,原 48KB README 已下沉到 `docs/release/1.1/`(R38 1.1 RC 9 B-stage 详单)和 `docs/release/1.2-patch-live/`(R70-R72 1.2 patch LIVE)。codex R114-R118 状态行(2026-08-10)在状态表中保留,其他历史技术报告按"思想历史 + 最新"原则筛选保留。_
