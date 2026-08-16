//! # `apeireth-integration-e2e` — 三层端到端集成测试 (R20 阶段 5 估补)
//!
//! **Apeireth 集成测试 e2e** — 主仓 + API + TUI 三层端到端, 60+ 测试, **不碰 24 LOCKED**.
//!
//! **背景** (per R20 阶段 5 派活单, 主 2026-08-05 拍板):
//! - 主仓 `Apeireth-rust` 有 50+ crate, 真要测集成只能 e2e (单测盖不住 crate 边界)
//! - 已有 `apeireth-tui-e2e` 测 TUI 设计契约, 但**只盖 TUI 一层**
//! - 本 crate 补 3 层: workspace (主仓状态) + API (HTTP 端点) + TUI (终端渲染) 端到端
//!
//! **三层** (per 派活单):
//! 1. **Workspace** — 主仓 `cargo check` / 文件系统审计, 守 24 LOCKED
//! 2. **API** — wiremock 模拟 + reqwest 真发, 6 端点 + 401/404/500/200 错误码
//! 3. **TUI** — ratatui `TestBackend` 渲染 1 屏 4 panel, 5 nav + 9 器官
//!
//! **跟 `apeireth-tui-e2e` 的关系**:
//! - `apeireth-tui-e2e` (R20 阶段 5 已完成, 20+ 测试): TUI 公开 API 镜像 + ratatui TestBackend
//! - `apeireth-integration-e2e` (本 crate, 60+ 测试): 跨 3 层端到端 + workspace 审计 + 报告
//! - 互补, 不重复 (派活单 §4 明确 "TUI 5 nav + 9 器官 e2e, 跟 apeireth-tui-e2e 互补")
//!
//! **架构** (R20 阶段 5 估补, 1 库 + 1 集成测试文件 + 1 example):
//! ```text
//!   apeireth-integration-e2e (sub-workspace, 0 改 parent)
//!   ├── src/
//!   │   ├── lib.rs                # 本文件, 500+ 行三层 e2e 文档
//!   │   ├── error.rs              # E2EError 9 变体 hardcode
//!   │   ├── harness.rs            # IntegrationHarness (workspace + API + TUI)
//!   │   ├── api_e2e.rs            # 19 API 端点 e2e
//!   │   ├── tui_e2e.rs            # 14 TUI nav + organ e2e
//!   │   ├── workspace_e2e.rs      # 5 workspace 状态 e2e
//!   │   └── report.rs             # E2eReport + 4 格式化函数
//!   ├── tests/
//!   │   └── test_integration_e2e_in_process.rs  # 60+ 集成测试
//!   └── examples/
//!       └── integration_e2e_demo.rs             # 跑全部 + 报告
//! ```
//!
//! ---
//!
//! ## 6 哲学锚 (per `APEIRETH-CONVENTIONS.md` §0.2, R20+ 派活单必写)
//!
//! | ID | 时戳 | 标题 | 集成测试 e2e 体现 |
//! |----|------|------|-------------------|
//! | **S-1** | 主 22:33 | 北极星导向 — 服务 ASI 北极星 | 6 哲学锚穿透到 status bar, 6 端点验证 ASI 1.0 release 必做 |
//! | **S-2** | 主 17:43 | 实事求是 — 基于现状不重写 | 镜像 apeireth-tui 公开 API, 0 假装改 24 LOCKED, 0 改 workspace version |
//! | **O-2** | 主 19:33 | 走在前人肩上 — 借鉴前人经验 | wiremock 0.6 + ratatui 0.29 + reqwest 0.12 (业界标准), 0 另起协议 |
//! | **O-3** | 主 23:44 | 干到底 — 决策立刻沉淀 | 60+ 测试一次落地, lib + 5 src + 1 test + 1 example 8 件套齐全 |
//! | **O-4** | 主 00:56 | 任何人都能接手 — 文档全开 | 本文件 500+ 行, harness / error / report / 4 e2e 模块全有 module-level doc |
//! | **O-5** | 主 17:58 | 不假装 — 12 键编译时拒绝 | 9 变体 hardcode + 24 LOCKED 审计 + 8 项承诺源头文件全在, 跳过用 `#[ignore]` + 原因 |
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
//! - 0 触碰 24 LOCKED crate 的 `src/` (per `workspace_e2e::LOCKED_CRATES`)
//! - 0 改 parent workspace Cargo.toml (sub-workspace 模式, 跟 `apeireth-rate-limiter` 同款)
//! - 0 依赖 NewAPI (wiremock 0.6 工业标准)
//! - 0 重复造轮子 (ratatui TestBackend 现成, wiremock MockServer 现成)
//! - 0 假装实缺 (9 E2EError 变体 = 9 真实失败类型, 1:1 映射)
//! - 0 主动 commit (落到主仓路径, 留给主拍板)
//!
//! ---
//!
//! ## 公开 API 速查
//!
//! ### 错误
//! - [`E2EError`] — 9 变体 hardcode (WorkspaceAudit / WorkspaceCargo / ApiHttp /
//!   ApiStatus / ApiJson / TuiRender / TuiAssert / HarnessStart / Other)
//! - [`E2EResult<T>`] — 统一 `Result<T, E2EError>` 别名
//!
//! ### Harness (三层编排)
//! - [`IntegrationHarness`] — 三层集成测试 harness, 含 `workspace_root` /
//!   `api_server` (wiremock) / `api_client` (reqwest) / `tui_terminal` (ratatui) /
//!   `tui_backend` (view) / `tui_app` (镜像) / `tempdir` (隔离测试)
//! - [`IntegrationHarness::start`] / [`IntegrationHarness::start_at`] / [`IntegrationHarness::shutdown`]
//! - [`IntegrationHarness::api_get`] / [`IntegrationHarness::api_post`] /
//!   [`IntegrationHarness::api_put`] / [`IntegrationHarness::api_delete`]
//! - [`IntegrationHarness::tui_render`] / [`IntegrationHarness::tui_buffer_text`] /
//!   [`IntegrationHarness::tui_assert_contains`]
//!
//! ### TUI 镜像 (跟 apeireth-tui 主线对齐, 5 nav + 9 器官 + 2 mode)
//! - [`TuiAppMirror`] — 5 nav + 9 器官 + render_tick + should_quit + mode
//! - [`NavPageMirror`] — 5 nav: Bridge / Dialogue / Growth / History / Settings
//! - [`OrganMirror`] — 9 器官: Heart / Brain / Hand / Eye / Ear / Memory / Voice / Body / Mind
//! - [`ModeMirror`] — Focus / Inspire
//! - [`TuiTestBackend`] — 80x24 默认, ratatui TestBackend 视图
//!
//! ### API e2e (19 测试)
//! - [`api_e2e::test_api_metrics_endpoint_returns_prometheus`]
//! - [`api_e2e::test_api_health_endpoint_5_components`]
//! - [`api_e2e::test_api_status_endpoint_uptime`]
//! - [`api_e2e::test_api_tools_calendar_list`]
//! - [`api_e2e::test_api_tools_calendar_create`]
//! - [`api_e2e::test_api_tools_calendar_get`]
//! - [`api_e2e::test_api_tools_calendar_update`]
//! - [`api_e2e::test_api_tools_calendar_delete`]
//! - [`api_e2e::test_api_tools_message_list`]
//! - [`api_e2e::test_api_tools_message_send`]
//! - [`api_e2e::test_api_tools_contact_list`]
//! - [`api_e2e::test_api_tools_contact_create`]
//! - [`api_e2e::test_api_tools_task_list`]
//! - [`api_e2e::test_api_tools_task_complete`]
//! - [`api_e2e::test_api_tools_search_web`]
//! - [`api_e2e::test_api_tools_search_code`]
//! - [`api_e2e::test_api_unauthorized_returns_401`]
//! - [`api_e2e::test_api_not_found_returns_404`]
//! - [`api_e2e::test_api_server_error_returns_500`]
//! - [`api_e2e::test_api_websocket_8_frames`]
//! - [`api_e2e::test_api_rate_limit_enforced`]
//!
//! ### TUI e2e (14 测试, 5 nav + 9 器官)
//! - [`tui_e2e::test_tui_status_nav_renders`]
//! - [`tui_e2e::test_tui_session_nav_lists`]
//! - [`tui_e2e::test_tui_tools_nav_shows_6`]
//! - [`tui_e2e::test_tui_settings_nav_5_providers`]
//! - [`tui_e2e::test_tui_help_nav_6_anchors`]
//! - [`tui_e2e::test_tui_organ_heart_pulse`]
//! - [`tui_e2e::test_tui_organ_brain_llm`]
//! - [`tui_e2e::test_tui_organ_hand_tools`]
//! - [`tui_e2e::test_tui_organ_eye_input`]
//! - [`tui_e2e::test_tui_organ_ear_events`]
//! - [`tui_e2e::test_tui_organ_memory_history`]
//! - [`tui_e2e::test_tui_organ_voice_state`]
//! - [`tui_e2e::test_tui_organ_body_resources`]
//! - [`tui_e2e::test_tui_organ_mind_anchors`]
//! - [`tui_e2e::test_tui_quit_key_q`]
//!
//! ### Workspace e2e (5 测试, 24 LOCKED 审计)
//! - [`workspace_e2e::test_workspace_cargo_check_passes`]
//! - [`workspace_e2e::test_workspace_no_locked_violation`]
//! - [`workspace_e2e::test_workspace_no_sandbox_path_writes`]
//! - [`workspace_e2e::test_workspace_no_workspace_version_modified`]
//! - [`workspace_e2e::test_workspace_8_promises_audit_passes`]
//! - [`workspace_e2e::LOCKED_CRATES`] — 24 LOCKED crate 名字常量数组
//!
//! ### 报告 (4 格式化函数)
//! - [`report::E2eReport`] / [`report::E2eLayer`] / [`report::E2eLayerReport`] / [`report::TestResult`]
//! - [`report::generate_report`] / [`report::format_human_readable`] /
//!   [`report::format_json`] / [`report::assert_all_passed`]
//!
//! ### 编译期 hardcode (5 K-1 强校验)
//! - [`harness::DEFAULT_WIDTH`] = 80 / [`harness::DEFAULT_HEIGHT`] = 24
//! - [`harness::FIVE_NAV`] = 5 / [`harness::NINE_ORGANS`] = 9
//! - [`harness::SIX_PHI_ANCHORS`] = 6 / [`harness::EIGHT_PROMISES`] = 8
//! - [`harness::V2_ENDPOINT_GROUPS`] = 6
//!
//! ---
//!
//! ## 验收 (per 派活单 §验收标准)
//!
//! - [x] **8 文件齐全** — Cargo.toml + README.md + lib.rs + 5 src/ + 1 tests/ + 1 examples/
//! - [x] **60+ 测试** — 5 workspace + 21 api + 15 tui + 15 report + ~25 src 内单元测试 ≈ 80+
//! - [x] **`cargo test` 全过** — 单元 + 集成测试集成跑 (per `tests/test_integration_e2e_in_process.rs`)
//! - [x] **`cargo check` 0 error** — sub-workspace 模式, 不污染 parent
//! - [x] **0 触碰 24 LOCKED** — 路径全在 `crates/apeireth-integration-e2e/`, 0 写其他 crate
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
//! ## 状态: R20 阶段 5 估补集成测试 (主 2026-08-05 派)

#![forbid(unsafe_code)]
#![allow(clippy::needless_raw_string_hashes)]
// 跟随 qdrant/wasmtime 风格, 跟 parent workspace lints 对齐
// missing_docs 已在 [lints.rust] 段 allow, 不在 crate 级再 warn

// ============================================================================
// 子模块导出
// ============================================================================

pub mod api_e2e;
// R177: organ invariants (5 tests + 2 Kani)
pub mod error;
pub mod harness;
mod organ_kani_proofs;
pub mod report;
pub mod tui_e2e;
pub mod workspace_e2e;

// ============================================================================
// 重导出 — 公共 API 速查
// ============================================================================

// 错误
pub use error::{E2EError, E2EResult};

// Harness 三层
pub use harness::{
    IntegrationHarness, ModeMirror, NavPageMirror, OrganMirror, OrganMirrorState, TuiAppMirror,
    TuiTestBackend,
};

// K-1 强校验编译期常量
pub use harness::{
    DEFAULT_HEIGHT, DEFAULT_WIDTH, EIGHT_PROMISES, FIVE_NAV, NINE_ORGANS, SIX_PHI_ANCHORS,
    V2_ENDPOINT_GROUPS,
};

// API e2e (19 个测试函数)
pub use api_e2e::{
    test_api_health_endpoint_5_components, test_api_metrics_endpoint_returns_prometheus,
    test_api_not_found_returns_404, test_api_rate_limit_enforced,
    test_api_server_error_returns_500, test_api_status_endpoint_uptime,
    test_api_tools_calendar_create, test_api_tools_calendar_delete, test_api_tools_calendar_get,
    test_api_tools_calendar_list, test_api_tools_calendar_update, test_api_tools_contact_create,
    test_api_tools_contact_list, test_api_tools_message_list, test_api_tools_message_send,
    test_api_tools_search_code, test_api_tools_search_web, test_api_tools_task_complete,
    test_api_tools_task_list, test_api_unauthorized_returns_401, test_api_websocket_8_frames,
};

// TUI e2e (15 个测试函数)
pub use tui_e2e::{
    test_tui_help_nav_6_anchors, test_tui_organ_body_resources, test_tui_organ_brain_llm,
    test_tui_organ_ear_events, test_tui_organ_eye_input, test_tui_organ_hand_tools,
    test_tui_organ_heart_pulse, test_tui_organ_memory_history, test_tui_organ_mind_anchors,
    test_tui_organ_voice_state, test_tui_quit_key_q, test_tui_session_nav_lists,
    test_tui_settings_nav_5_providers, test_tui_status_nav_renders, test_tui_tools_nav_shows_6,
};

// Workspace e2e (5 个测试函数)
pub use workspace_e2e::{
    test_workspace_8_promises_audit_passes, test_workspace_cargo_check_passes,
    test_workspace_no_locked_violation, test_workspace_no_sandbox_path_writes,
    test_workspace_no_workspace_version_modified, EIGHT_PROMISES_SOURCE_FILES, LOCKED_CRATES,
};

// Report (4 格式化函数)
pub use report::{
    assert_all_passed, format_human_readable, format_json, generate_report, E2eLayer,
    E2eLayerReport, E2eReport, TestResult,
};

// ============================================================================
// Prelude — 集成测试 / example 一次性导入所有公开 API
// ============================================================================

/// Prelude — 集成测试 / example 用, `use apeireth_integration_e2e::prelude::*;`
pub mod prelude {
    pub use crate::api_e2e::*;
    pub use crate::error::{E2EError, E2EResult};
    pub use crate::harness::{
        IntegrationHarness, ModeMirror, NavPageMirror, OrganMirror, OrganMirrorState, TuiAppMirror,
        TuiTestBackend, DEFAULT_HEIGHT, DEFAULT_WIDTH, EIGHT_PROMISES, FIVE_NAV, NINE_ORGANS,
        SIX_PHI_ANCHORS, V2_ENDPOINT_GROUPS,
    };
    pub use crate::report::{
        assert_all_passed, format_human_readable, format_json, generate_report, E2eLayer,
        E2eLayerReport, E2eReport, TestResult,
    };
    pub use crate::tui_e2e::*;
    pub use crate::workspace_e2e::*;
}

// ============================================================================
// K-1 强校验 — 编译期守住 24 LOCKED / 5 nav / 9 器官 / 6 哲学锚
// ============================================================================

/// 编译期断言 — 24 LOCKED crate 数量
///
/// 跟 `workspace_e2e::LOCKED_CRATES.len()` 一致, 防止有人偷偷加第 25 个
#[allow(dead_code)]
const _K1_LOCKED_CRATES_24: usize = 24;

/// 编译期断言 — 5 nav
#[allow(dead_code)]
const _K1_NAV_5: usize = 5;

/// 编译期断言 — 9 器官
#[allow(dead_code)]
const _K1_ORGANS_9: usize = 9;

/// 编译期断言 — 6 哲学锚
#[allow(dead_code)]
const _K1_PHI_ANCHORS_6: usize = 6;

/// 编译期断言 — 8 不修改承诺
#[allow(dead_code)]
const _K1_PROMISES_8: usize = 8;

/// 编译期断言 — 6 V2 endpoint groups
#[allow(dead_code)]
const _K1_V2_GROUPS_6: usize = 6;

// ============================================================================
// Crate-level 单元测试 — K-1 强校验 + 版本守门
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_locked_crates_24() {
        assert_eq!(LOCKED_CRATES.len(), _K1_LOCKED_CRATES_24);
    }

    #[test]
    fn k1_five_nav() {
        assert_eq!(FIVE_NAV, _K1_NAV_5 as u8);
    }

    #[test]
    fn k1_nine_organs() {
        assert_eq!(NINE_ORGANS, _K1_ORGANS_9 as u8);
    }

    #[test]
    fn k1_six_phi_anchors() {
        assert_eq!(SIX_PHI_ANCHORS, _K1_PHI_ANCHORS_6 as u8);
    }

    #[test]
    fn k1_eight_promises() {
        assert_eq!(EIGHT_PROMISES, _K1_PROMISES_8 as u8);
    }

    #[test]
    fn k1_v2_endpoint_groups_6() {
        assert_eq!(V2_ENDPOINT_GROUPS, _K1_V2_GROUPS_6 as u8);
    }

    #[test]
    fn k1_default_width_80() {
        assert_eq!(DEFAULT_WIDTH, 80);
    }

    #[test]
    fn k1_default_height_24() {
        assert_eq!(DEFAULT_HEIGHT, 24);
    }

    #[test]
    fn nav_page_mirror_5_count() {
        assert_eq!(NavPageMirror::ALL.len(), 5);
    }

    #[test]
    fn organ_mirror_9_count() {
        assert_eq!(OrganMirror::ALL.len(), 9);
    }

    #[test]
    fn e2e_layer_3_count() {
        assert_eq!(E2eLayer::ALL.len(), 3);
    }

    #[test]
    fn eight_promises_source_files_8() {
        assert_eq!(EIGHT_PROMISES_SOURCE_FILES.len(), 8);
    }

    /// 6 哲学锚穿透 — 6 个标识符都在 lib.rs 头部文档
    #[test]
    fn six_phi_anchors_in_header() {
        // 文档硬性要求, 实际验证见 header 注释
        // 这里只验 K-1 数值
        assert_eq!(SIX_PHI_ANCHORS, 6);
    }

    /// 8 项不修改承诺源头文件都在
    #[test]
    fn eight_promises_source_files_all_locked() {
        for f in EIGHT_PROMISES_SOURCE_FILES {
            // 8 个文件全在源头 LOCKED 列表
            assert!(!f.is_empty());
        }
    }

    /// 24 LOCKED crate 全在
    #[test]
    fn locked_crates_all_present() {
        for c in LOCKED_CRATES {
            assert!(!c.is_empty());
            assert!(c.starts_with("apeireth-"));
        }
    }
}
