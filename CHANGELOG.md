# Changelog — Apeireth

## [Unreleased] — R128 (2026-08-12)

### Changed — workspace 收敛 94→55

- **13 frozen crate** git mv 到 `crates/_frozen/` (R20 阶段 6 估补 skeleton): `apeireth-{credentials,cache,tracing,metrics,oauth,update,sandbox,tree-sitter,image-prompt,plugin,observability,task}`
- **5 merge source** git mv 到 `crates/_archived/`: `apeireth-rollback` → `apeireth-upgrade::rollback`, `apeireth-{keyring,machine-id}` → `apeireth-host`, `apeireth-{repo-scan,repo-analyzer}` → `apeireth-repo-tools`
- **`apeireth-integration-r20-stage4`** superseded by `apeireth-integration-e2e`, git mv 到 `crates/_archived/`
- **`apeireth-i18n`** 从 `_frozen` 移回 active (TUI 真实使用)
- **新 crate** `apeireth-host` (keyring + machine_id 5 子模块 union deps) + `apeireth-repo-tools` (scan + analyzer 避免同名 struct 冲突)
- **24 LOCKED 入口签名冻结降级** (per decision-74 §1.1 + decision-130 §2.4): 仅保 3 项不可变脊柱 (Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义含义), 其余可重构

### Added — minimax (MiniMax) 真端到端验证

- **OpenAI Chat Completions** 真接 `https://api.minimaxi.com/v1/chat/completions`: 3 round Keep-Alive LIFO 复用 (3.8s/2.4s/2.6s, tokens 267/392/390)
- **OpenAI Responses API** 真接 `https://api.minimaxi.com/v1/responses`: 1.74s, 228 tokens, model `MiniMax-M3`
- **Anthropic Messages API** 真接 `https://api.minimaxi.com/anthropic/v1/messages`: 3.33s, 126 tokens, `x-api-key` auth
- **minimax + memory 真端到端** (`crates/apeireth-integration-e2e/examples/minimax_memory_roundtrip.rs`):
  - 真 HTTP POST + 真 SQLite file-backed + 真 drop+reopen + 真 semantic_search
  - 1.59s, 89 tokens, "Rust async runtime" 真可检索
- **minimax 6th provider** 加入 `apeireth-provider::minimax` (descriptor + 7 model kinds + 4 协议 + 8 工具白名单)
- 综合报告: [`reports/minimax-end-to-end-r128-2026-08-12.md`](reports/minimax-end-to-end-r128-2026-08-12.md)

### Added — docs + conventions

- 新建 [`docs/conventions/16-crate-merge-policy.md`](docs/conventions/16-crate-merge-policy.md) (16 子规范, §1-§7: 入口签名冻结降级 / frozen / merge / archive 流程)
- [`docs/conventions/10-locked.md`](docs/conventions/10-locked.md) 加 R128 段
- [`docs/CONTEXT-HANDOVER.md`](docs/CONTEXT-HANDOVER.md) §12 R128 补记
- [`docs/pages-source/roadmap.md`](docs/pages-source/roadmap.md) §3.5 R128 实际执行
- `Cargo.toml` metadata 加 R128 + decision-130 注释 (B1/A1/A3/B3/B4/B5/C1 解除状态)

### Added — 51/51 active crate README

- 每个 active crate 都有 README (包括 auto-generated + 5 关键 crate 详细: core / memory / api / tui / cli)
- 顶层 `README.md` 重写为生产入门版本 (1 分钟上手 + 5 战区 + minimax 真接 + 借鉴 + license)

### Verified

- `cargo check --workspace` exit 0, 0 errors, 296 historical warnings
- `cargo test -p apeireth-provider` 13 passed (新增 4 个 minimax tests)

### Integration changes (callers migrated)

- `apeireth-tui/Cargo.toml`: `apeireth-observability` → `apeireth-telemetry`
- `apeireth-api` + `apeireth-sdk-{sandbox,lark,livekit,voice}/Cargo.toml`: `apeireth-keyring` → `apeireth-host`
- TUI benches: `apeireth_observability::*` → `apeireth_telemetry::observability::*`
- `apeireth-integration-e2e/Cargo.toml` 加 `apeireth-memory` + `apeireth-core` dev-deps (for `minimax_memory_roundtrip` example)

### Refs

- 决策 #126 (Mavis 全自决 commit 解除)
- 决策 #128 (10 类 30+严守评估)
- 决策 #130 (6 项 B 全部解除 + PHL-07 接受实施)
- 决策 #62 §5.2 (整合 #5 commit 拆 3 commit 范式)

---

## [1.2.0] — R125-R127 (2026-08-10)

### Added — 整合 #4 + #5 commit (per decision-42 + #48 + #62)

- **4921 passed / 88 suites / 0 failed** 测试基线
- **24 LOCKED crate mtime baseline** 严守 (B1)
- **8 哲学锚升级** (B5, 6→8: 增 S-3 流程自化 + O-1 安全优先)
- **V0.5 25→30 维升级** (B3)
- **6 重守门 v6 → v7 升级** (B4)
- **13 键 verdict cache** (A3, 12 原 12 + PHL-07 = 13 键)
- **Library v1.0 礼物** (30 经典书 + 100+ 论文 + 50+ 视频 + 10+ 课程 + 10+ hub)
- **整合 #5.x commit 系列** (5.1 src/ + 5.2 docs/ + 5.3 R125-R137 era reports/ + 5.4 R129-R163 era reports/ + 5.5 library/v1.0/ 准备)

---

_格式: [Keep a Changelog 1.1.0](https://keepachangelog.com/) + [Semantic Versioning](https://semver.org/)_