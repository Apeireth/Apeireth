//! # `apeireth-integration-r20-stage4` — R20 阶段 4 集成测试 wrapper
//!
//! **背景** (per R20 阶段 4 派活单, 主 2026-08-05 21:35 拍板):
//! - 14 new crate 估补 (5 P0 MCP + 3 估缺核心 + 2 工具 + 2 基础设施 + 2 SDK stub)
//! - 30+ 集成测试覆盖跨 crate 协同 (SDK 6 工具 / 5 Provider fallback / observability bus / i18n runtime / m3 hallucination 5 道防御 / 71GB rollback)
//!
//! **架构** (R20 阶段 4 估补, 1 库 + 1 集成测试文件 + 6 子文件):
//! ```text
//!   apeireth-integration-r20-stage4 (sub-workspace, 0 改 parent)
//!   ├── src/
//!   │   └── lib.rs                # 本文件, 模块文档
//!   ├── tests/
//!   │   ├── r20_stage4_integration_14crates.rs  # 6 子文件 mod wrapper
//!   │   └── integration/
//!   │       ├── test_e2e_tools.rs           # 6 工具 e2e (SDK)
//!   │       ├── test_5_provider_stub.rs     # 5 Provider fallback (team-lead)
//!   │       ├── test_observability_bus.rs   # observability 3 端点
//!   │       ├── test_i18n_runtime.rs        # i18n 5 语言
//!   │       ├── test_m3_defense.rs          # 14 crate 跨守门
//!   │       └── test_71gb_incident.rs       # rollback 4 重防御
//! ```
//!
//! ---
//!
//! ## 6 哲学锚 (per `APEIRETH-CONVENTIONS.md` §0.2, R20+ 派活单必写)
//!
//! | ID | 时戳 | 标题 | 集成测试 e2e 体现 |
//! |----|------|------|-------------------|
//! | **S-1** | 主 22:33 | 北极星导向 — 服务 ASI 北极星 | 14 crate 跨守门 + 5 Provider fallback 守 1 通道 |
//! | **S-2** | 主 17:43 | 实事求是 — 基于现状不重写 | 镜像 14 crate 公开 API, 0 假装改 24 LOCKED, 0 改 workspace version |
//! | **O-2** | 主 19:33 | 走在前人肩上 — 借鉴前人经验 | 1:1 翻译 v0.9.21 商业版, 用现成 path-dep (0 另起独立 sub-crate) |
//! | **O-3** | 主 23:44 | 干到底 — 决策立刻沉淀 | 30+ 测试一次落地, 6 子文件齐全 |
//! | **O-4** | 主 00:56 | 任何人都能接手 — 文档全开 | 本文件 100+ 行, 6 子文件全有 module-level doc |
//! | **O-5** | 主 17:58 | 不假装 — 12 键编译时拒绝 | 14 crate TOOL_WHITELIST 跨守门 (m3_defense), stub 显式标 NotImplemented |
//!
//! ---
//!
//! ## 8 项不修改承诺 (per `docs/stage4/8-locked-unified-2026-08-05.md` §2)
//!
//! 1. **0 改阶段 1+2+3 LOCKED 文档** — `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 0 行改动
//! 2. **0 改 v2 / v4 / v4.1 LOCKED** — `APEIRETH-CONVENTIONS.md` §0.2 0 行改动
//! 3. **0 改阶段 4 核心文档** — `docs/stage4/8-locked-unified-2026-08-05.md` 0 行改动
//! 4. **0 改阶段 5 施工文档** — `docs/stage5/stage5-construction-document.md` 0 行改动
//! 5. **0 改 v6 基础架构** — 4 重守门 + 权限发放 + E 层修改路径 0 行改动
//! 6. **0 改 R11 baseline 三值** — V1141=0.8682 / V1131=0.8532 / V1136=0.9063 0 行改动
//! 7. **0 改顶层 3 规范** — `APEIRETH-CONVENTIONS.md` / `APEIRETH-VERSIONING.md` / `GLOSSARY.md` 0 行改动
//! 8. **0 改 workspace version** — workspace Cargo.toml `[workspace.package] version = "1.0.0"` 0 行改动
//!
//! **本 crate 守的子承诺**:
//! - 0 触碰 24 LOCKED crate 的 `src/` (per `tests/integration/test_m3_defense.rs` 14 crate 跨守门)
//! - 0 改 parent workspace Cargo.toml (sub-workspace 模式, 跟 `apeireth-integration-e2e` + `apeireth-rate-limiter` 同款)
//! - 0 依赖 NewAPI (1:1 翻译 v0.9.21 商业版, 自建 stub)
//! - 0 重复造轮子 (复用 14 crate 现有 TOOL_WHITELIST 常量)
//! - 0 假装实缺 (5 Provider stub 显式标 NotImplemented, 1:1 映射)
//! - 0 主动 commit (落到主仓路径, 留给主拍板)
//!
//! ---
//!
//! ## 验收 (per 派活单 §验收标准)
//!
//! - [x] **8 文件齐全** — Cargo.toml + README.md + lib.rs + 1 tests/ + 6 tests/integration/* = 9 文件
//! - [x] **30+ 测试** — 6 子文件各 5-10 测试 ≈ 30-50 总
//! - [x] **`cargo test` 全过** — 集成测试集成跑 (per `tests/r20_stage4_integration_14crates.rs` 6 mod 包含)
//! - [x] **`cargo check` 0 error** — sub-workspace 模式, 不污染 parent
//! - [x] **0 触碰 24 LOCKED** — 路径全在 `crates/apeireth-integration-r20-stage4/`, 0 写其他 crate src
//! - [x] **6 哲学锚 / 8 项承诺** — 本文件 header 段已写齐
//! - [x] **不主动 commit** — 落到主仓路径, 等主拍板
//!
//! ---
//!
//! ## 边界 (per 派活单 §12)
//!
//! - ❌ **不**改 24 LOCKED crate 的任何 `src/` 或 `Cargo.toml`
//! - ❌ **不**改 parent workspace Cargo.toml (sub-workspace 模式)
//! - ❌ **不**改任何已有 crate (本 crate 仅新增文件)
//! - ❌ **不**写 workspace version (本 crate 自己的 `version = "1.0.0"` 跟 parent 一致, 不修改 parent)
//! - ❌ **不**写到 sandbox 错路径 `.minimax-agent-cn\projects\apeireth-debug\Apeireth-rust\`
//! - ❌ **不**干 Tauri 2.0 / 前端活儿 (主 2026-08-05 22:13 拍"只干 TUI")
//!
//! ---
//!
//! ## 状态: R20 阶段 4 估补集成测试 (主 2026-08-05 拍, 1.0 release #2 test 100% 收尾时搬运)
#![forbid(unsafe_code)]
#![allow(clippy::needless_raw_string_hashes)]
// 跟随 qdrant/wasmtime 风格, 跟 parent workspace lints 对齐
// missing_docs 已在 [lints.rust] 段 allow, 不在 crate 级再 warn

// 本 lib 仅承载模块文档; 测试代码全在 tests/ 下 (sub-workspace 模式,
// 顶层 tests/ 在 main workspace 不被 cargo 自动 pick up, 这是
// R20 阶段 4 sub-agent 设计的架构错位, R20 阶段 5 拍板决策 5
// 走"搬新 member crate"路径, 落到本 crate)
