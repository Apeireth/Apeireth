//! R20 阶段 4 集成测试顶层 wrapper
//!
//! 主人 2026-08-05 21:35 拍板"版本号不要出的那么快, 有意义的是效果" + 21:35 "现在在干的活儿不要停, 继续推进"
//!
//! 本文件聚合 6 个 cross-crate 集成测试, 覆盖 14 new crate 协同 (5 P0 MCP + 3 估缺核心 + 2 工具 + 2 基础设施 + 2 SDK stub) + 3 估补 (i18n + observability + sdk):
//!
//! | 子文件 | 覆盖 | 估测试数 |
//! |--------|------|----------|
//! | `integration/test_e2e_tools.rs` | apeireth-sdk 6 工具 + D-02 子路径 + Auth 5 组件 | 6-8 |
//! | `integration/test_5_provider_stub.rs` | apeireth-team-lead 14 fn + 4 Provider fallback | 5-6 |
//! | `integration/test_observability_bus.rs` | apeireth-observability 3 端点 + PII 脱敏 + trace_id | 5-6 |
//! | `integration/test_i18n_runtime.rs` | apeireth-i18n 5 语言 + fallback + 模板变量 | 5-6 |
//! | `integration/test_m3_defense.rs` | 14 crate TOOL_WHITELIST 跨 crate 守门 | 8-10 |
//! | `integration/test_71gb_incident.rs` | apeireth-rollback 4 重防御 + 6 策略 | 6-8 |
//!
//! 估 30-50 集成测试, 跟 5 P0 / 9 skeleton crate 1:1 翻译 v0.9.21 商业版.
//!
//! ## 设计原则 (Ponytail)
//!
//! - 不创建新 crate; 14 new crate 已是 workspace member
//! - 跟 `workspace-integration-v2.rs` 同模式: 不动 workspace 编译, 文件存在即验收
//! - 0 改 24 LOCKED crate mtime
//! - 0 改 workspace version (1.0.0)
//! - 0 引 NewAPI
//! - 0 重复造轮子 (复用 14 crate 现有 API)
//!
//! ## 跑法
//!
//! ```bash
//! cargo test --test integration-r20-stage4
//! ```
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md`

#![allow(dead_code, unused_imports, clippy::needless_return, unused_variables)]

// ============================================================
// Sub-file 1: E2E 6 工具 (apeireth-sdk)
// ============================================================
#[path = "integration/test_e2e_tools.rs"]
mod e2e_tools;

// ============================================================
// Sub-file 2: 5 Provider fallback (apeireth-team-lead)
// ============================================================
#[path = "integration/test_5_provider_stub.rs"]
mod provider_stub;

// ============================================================
// Sub-file 3: Observability bus (apeireth-observability)
// ============================================================
#[path = "integration/test_observability_bus.rs"]
mod observability_bus;

// ============================================================
// Sub-file 4: i18n 5 语言运行时 (apeireth-i18n)
// ============================================================
#[path = "integration/test_i18n_runtime.rs"]
mod i18n_runtime;

// ============================================================
// Sub-file 5: m3 hallucination 5 道防御 (14 crate 跨守门)
// ============================================================
#[path = "integration/test_m3_defense.rs"]
mod m3_defense;

// ============================================================
// Sub-file 6: 71GB rollback 4 重防御 (apeireth-rollback)
// ============================================================
#[path = "integration/test_71gb_incident.rs"]
mod incident_71gb;
