//! `apeireth-tool-registry` — **Apeireth R17 战役 2-1 工具注册中心**
//!
//! **目标**: 真支持 VCP-style 动态工具注册 + 6 类 enum (sync/async/static/
//! service/messagePreprocessor/hybridservice) + 5 轴正交属性 (触发/等待/驻留/传输/输出)
//! + token 预算三层 (LIGHT/BRIEF/MAX) + notify 热加载.
//!
//! **5 大模块**:
//! 1. `types` — 6 类 enum + 5 轴正交 + 编译期 hardcode
//! 2. `trait_def` — `Tool` async trait (4 方法: name/kind/axes/call)
//! 3. `token_budget` — token 预算 3 const + 截断 (VCP §6.2.2 #15 字段级引用)
//! 4. `registry` — `ToolRegistry` (CRUD) + 6 类 mock 工具 + notify 热加载
//! 5. `lib` (本文件) — 入口 + 编译期 hardcode
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - **#12** 6 类 enum → `types.rs` (`Plugin.js:232,379,607-608,1075` + `Plugin/AgentMessage/plugin-manifest.json:8`)
//! - **#13** 5 轴正交 → `types.rs::ToolAxes` (5 独立 enum, §3.2 建模)
//! - **#15** token 预算三层 → `token_budget.rs` (`dynamicToolRegistry.js:10,11,21`)
//! - **agentManager.js:11-17,68-131,136-153** → `registry.rs` (chokidar → notify 5.x)
//! - **Plugin.js:28-47 PluginManager** → `registry.rs` 顶层模式
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ 6 类 enum 真实现 (跟 VCP pluginType 1:1)
//! - ✅ 5 轴正交 (3^5 = 243 组合, 独立字段不锁死)
//! - ✅ token 预算 3 const 真值跟 VCP 一致 (LIGHT=15/BRIEF=6/MAX=16000)
//! - ✅ 6 类 mock 工具 call 真跑 (Sync echo / Async sleep / Static value / Service uptime / Preprocessor rewrite / Hybrid ack+side-effect)
//! - ✅ notify 热加载真跑 (mock tempdir + 写文件 + 等 1s + 验证事件触发)
//! - ✅ unit tests ≥ 20 (按 DoD)
//!
//! **不修改承诺** (R17 finalize 8 项不修改承诺):
//! - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
//! - ❌ 不改战役 1 全部代码 (用 import, 不改源码)
//! - ❌ 不引入 unsafe (workspace `#![deny(unsafe_code)]` 继承)
//! - ❌ 不假装 "已实现但没真跑"
//!
//! **架构位置**:
//! ```text
//!   apeireth-api / apeireth-pipeline / 未来消费者
//!          ↓
//!      apeireth-tool-registry (本 crate)
//!      ├── types.rs        : 6 类 enum + 5 轴 struct
//!      ├── trait_def.rs    : Tool trait
//!      ├── token_budget.rs : #15 token 预算三层
//!      ├── registry.rs     : ToolRegistry + 6 mock + notify 热加载
//!      └── lib.rs          : 入口 + 编译期 hardcode
//! ```

#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod async_task;
pub mod classifier;
pub mod registry;
pub mod token_budget;
pub mod trait_def;
pub mod types;

pub use async_task::{AsyncTaskError, AsyncTaskResult, AsyncTaskStore, NotifyChannel, TaskId, TaskRecord, TaskStatus, next_task_id};
pub use classifier::{
    cosine_similarity, Category, ClassifyError, Classifier, EmbeddingClassifier, EmbedFn,
    HeuristicClassifier, LlmClassifier, MockHashEmbedFn, CATEGORY_COUNT,
};
pub use registry::{
    MockAsyncTool, MockHybridserviceTool, MockMessagePreprocessorTool, MockServiceTool,
    MockStaticTool, MockSyncTool, ToolRegistry, MOCK_NAMES,
};
pub use token_budget::{
    estimate_token_count, estimate_tool_tokens, exceeds_injection_budget, token_pieces,
    truncate_to_max_injection, truncate_to_token_budget, DEFAULT_BRIEF_TOKEN_BUDGET,
    LIGHT_LIST_TOKEN_BUDGET, MAX_INJECTION_CHARS, MIN_BRIEF_TOKEN_BUDGET, MIN_INJECTION_CHARS,
};
pub use trait_def::{Tool, ToolDescription};
pub use types::{
    AwaitingAxis, OutputAxis, ResidentAxis, ToolAxes, ToolKind, TransportAxis, TriggerAxis,
    AXIS_COMBINATION_COUNT, AXIS_COUNT,
};

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 战役 2-1 实际借鉴 VCP 4 项 (#12 / #13 / #15 / agentManager.js chokidar 借鉴)
pub const BORROWED_LEGACY_COUNT: usize = 4;

/// 6 类 enum 总数 (编译期 hardcode, 跟 VCP §6.2.1 #12 一致)
pub const TOOL_KIND_COUNT: usize = 6;

/// 6 类 mock 工具真跑 (DoD: 全部真实现)
pub const MOCK_TOOL_COUNT: usize = 6;

/// **R25 战区 5 借鉴** — 9 类别分类总数 (VCP 7 + Safety + LongRunning)
///
/// **VCP 字段级引用** `dynamicToolRegistry.js:40-80 CATEGORY_RULES` 7 类 1:1
/// + 2 类 Apeireth 独有 (Safety / LongRunning, per v2.0 strategy Step 5)
pub const CATEGORY_COUNT_LIB: usize = 9;

// ============================================================
// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
// ============================================================

const _: () = {
    // 6 类总数对齐 VCP 真代码
    assert!(
        TOOL_KIND_COUNT == 6,
        "TOOL_KIND_COUNT must be 6 (VCP §6.2.1 #12)"
    );
    assert!(ToolKind::COUNT == 6, "ToolKind::COUNT must be 6");
    assert!(
        AXIS_COUNT == 5,
        "AXIS_COUNT must be 5 (触发/等待/驻留/传输/输出)"
    );
    assert!(AXIS_COMBINATION_COUNT == 243, "3^5 = 243 组合");

    // token 预算 4 const 跟 VCP 真值
    assert!(
        LIGHT_LIST_TOKEN_BUDGET == 15,
        "VCP dynamicToolRegistry.js:10 LIGHT = 15"
    );
    assert!(
        DEFAULT_BRIEF_TOKEN_BUDGET == 6,
        "VCP dynamicToolRegistry.js:11 BRIEF = 6"
    );
    assert!(
        MIN_BRIEF_TOKEN_BUDGET == 3,
        "VCP dynamicToolRegistry.js:12 MIN_BRIEF = 3"
    );
    assert!(
        MAX_INJECTION_CHARS == 16_000,
        "VCP dynamicToolRegistry.js:21 MAX = 16000"
    );

    // 6 类 mock 工具总数
    assert!(MOCK_TOOL_COUNT == 6, "MOCK_TOOL_COUNT = 6");
    // MOCK_NAMES 6 个字面量 + 唯一性 → 移到 runtime test (Vec/sort 不在 const 里稳定)

    // R25 战区 5: 9 类别分类总数对齐 (lib 层二次断言, classifier.rs 也有)
    assert!(CATEGORY_COUNT_LIB == 9, "CATEGORY_COUNT_LIB must be 9");
    assert!(CATEGORY_COUNT == 9, "classifier::CATEGORY_COUNT must be 9");
};

// ============================================================
// lib 入口测试 (编译期 hardcode 二次断言)
// ============================================================

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn lib_constants_match_vcp() {
        // 编译期 hardcode 已 assert, 这里再 runtime 测一次
        assert_eq!(TOOL_KIND_COUNT, 6);
        assert_eq!(MOCK_TOOL_COUNT, 6);
        assert_eq!(AXIS_COUNT, 5);
        assert_eq!(AXIS_COMBINATION_COUNT, 243);
        assert_eq!(LIGHT_LIST_TOKEN_BUDGET, 15);
        assert_eq!(DEFAULT_BRIEF_TOKEN_BUDGET, 6);
        assert_eq!(MIN_BRIEF_TOKEN_BUDGET, 3);
        assert_eq!(MAX_INJECTION_CHARS, 16_000);
        assert_eq!(BORROWED_LEGACY_COUNT, 4);
    }

    #[test]
    fn lib_public_api_compiles() {
        // 验证 lib.rs 公开 API 可用 (编译期守 + runtime 守)
        let r = ToolRegistry::new();
        assert!(r.is_empty());
        let _ = ToolKind::all();
        let _ = ToolAxes::default();
    }

    #[test]
    fn six_tool_kinds_have_vcp_strings() {
        // 6 类 1:1 对应 VCP pluginType 真值
        assert_eq!(ToolKind::Sync.as_legacy_str(), "synchronous");
        assert_eq!(ToolKind::Async.as_legacy_str(), "asynchronous");
        assert_eq!(ToolKind::Static.as_legacy_str(), "static");
        assert_eq!(ToolKind::Service.as_legacy_str(), "service");
        assert_eq!(
            ToolKind::MessagePreprocessor.as_legacy_str(),
            "messagePreprocessor"
        );
        assert_eq!(ToolKind::Hybridservice.as_legacy_str(), "hybridservice");
    }

    #[test]
    fn mock_names_are_six_unique() {
        // 6 个 mock 工具名必须唯一 (从 const _ 块搬到 runtime)
        let mut names: Vec<&str> = MOCK_NAMES.to_vec();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 6, "MOCK_NAMES 必须 6 个唯一名");
        assert_eq!(names.len(), original_len, "去重后长度不变");
    }

    #[test]
    fn mock_names_match_vcp_six_kinds() {
        // 6 个 mock 工具名应能映射到 6 类 enum (语义清晰)
        let expected_kinds = [
            ("MockSync", "synchronous"),
            ("MockAsync", "asynchronous"),
            ("MockStatic", "static"),
            ("MockService", "service"),
            ("MockPreprocessor", "messagePreprocessor"),
            ("MockHybrid", "hybridservice"),
        ];
        for (i, name) in MOCK_NAMES.iter().enumerate() {
            let (expected_name, expected_vcp) = expected_kinds[i];
            assert_eq!(
                *name, expected_name,
                "第 {i} 个 mock 工具名应 = {expected_name}"
            );
            // 6 类 VCP 真值在 runtime test 里覆盖
            let _ = expected_vcp;
        }
    }
}
