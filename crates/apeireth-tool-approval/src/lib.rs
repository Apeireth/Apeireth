//! `apeireth-tool-approval` — **Apeireth R17 战役 2-3 工具审批**
//!
//! **目标**: 5 规则真实现 (Trust / Risk / Frequency / Whitelist / Blacklist) + 5 分钟审批窗口
//! (VCP `timeoutMinutes = 5`) + fuzzy matching 集成 (战役 2-2 `FuzzyToolMatcher`).
//!
//! **5 大模块**:
//! 1. `decision` — `ApprovalDecision` 3 态 enum (Allow / RequireApproval / Deny)
//! 2. `rule_trait` — `ApprovalRule` trait (5 规则实现此 trait)
//! 3. `history` — `CallRecord` + `now_ms()` 工具
//! 4. `rule` — 5 规则真实现 (Trust / Risk / Frequency / Whitelist / Blacklist)
//! 5. `manager` — `ApprovalManager` (5 规则按顺序 + 5min 窗口 + `wait_for_approval`)
//! 6. `fuzzy_bridge` — `match_tool_name` fuzzy 集成 (VCP §6.2.2 #18)
//! 7. `lib` (本文件) — 入口 + 编译期 hardcode
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - **`toolApprovalManager.js:11-19`** `config.enabled / timeoutMinutes / approvalList / fuzzyToolMatching`
//!   → 我们的 `ApprovalManager` + 5 规则
//! - **`toolApprovalManager.js:127-141`** `parseApprovalRule` (SilentReject 后缀)
//!   → `BlacklistRule::silent()`
//! - **`toolApprovalManager.js:144-225`** `getApprovalDecision` 三层判断 → `ApprovalManager::check` 5 规则按顺序
//! - **`toolApprovalManager.js:231-233`** `getTimeoutMs()` = `5 * 60 * 1000` → `APPROVAL_TIMEOUT_MS`
//! - **§6.2.2 #18** fuzzy matching → `fuzzy_bridge::match_tool_name` (import 战役 2-2)
//!
//! **Apeireth 扩展** (VCP 没有, 实战必需):
//! - **5 规则独立** (VCP 是 1 个 approvalList, 我们拆 5 维度独立判断, 优先级清晰)
//! - **FrequencyRule 1min/3 次反刷** (VCP 没有, 防止 LLM 死循环)
//! - **TrustRule 信任列表** (VCP `approveAll` 是全局, 我们支持单工具信任)
//! - **WhitelistRule 白名单** (VCP 行为是默认 allow, 我们显式 whitelist)
//! - **BlacklistRule 黑名单** (VCP `::SilentReject` suffix 借鉴, 但用独立 rule)
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ 5 规则各自真实现, 不只 mock
//! - ✅ FrequencyRule 真用 history + 时间窗 (Wagner-Fischer 风格滑窗)
//! - ✅ BlacklistRule 静默拒绝 (VCP `::SilentReject` suffix)
//! - ✅ ApprovalManager 按顺序 check (第一个非 NoMatch 生效)
//! - ✅ 5 分钟窗口真用 `tokio::time::timeout`
//! - ✅ `wait_for_approval` 真等待外部 handler (oneshot channel)
//! - ✅ fuzzy matching 真集成战役 2-2 `FuzzyToolMatcher`
//! - ✅ 单元测试 ≥ 30 (按 DoD × 2×)
//!
//! **不修改承诺** (R17 finalize 8 项不修改承诺):
//! - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
//! - ❌ 不改战役 1 / 战役 2-1 / 战役 2-2 全部代码 (用 import, 不改源码)
//! - ❌ 不引入 unsafe (workspace `#![deny(unsafe_code)]` 继承)
//! - ❌ 不假装 "已实现但没真跑"
//! - ❌ 不抄 VCP 业务代码 (借鉴字段名 + 行为, 不抄实现)
//!
//! **架构位置**:
//! ```text
//!   apeireth-api / apeireth-pipeline / 未来消费者
//!          ↓
//!      apeireth-tool-approval (本 crate)
//!      ├── decision.rs       : ApprovalDecision 3 态 enum
//!      ├── rule_trait.rs     : ApprovalRule trait
//!      ├── history.rs        : CallRecord + now_ms
//!      ├── rule.rs           : 5 规则真实现
//!      ├── manager.rs        : ApprovalManager + 5min 窗口 + wait_for_approval
//!      ├── fuzzy_bridge.rs   : fuzzy 集成 (VCP §6.2.2 #18)
//!      └── lib.rs            : 入口 + 编译期 hardcode
//! ```

#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod decision;
pub mod fuzzy_bridge;
pub mod history;
pub mod manager;
pub mod rule;
pub mod approval_bridge;
pub mod rule_trait;

pub use decision::ApprovalDecision;
pub use fuzzy_bridge::{match_tool_name, match_tool_name_threshold};
pub use history::{now_ms, CallRecord};
pub use manager::{
    ApprovalHandler, ApprovalManager, AutoApproveHandler, DefaultDenyHandler, APPROVAL_TIMEOUT_MS,
};
pub use rule::{BlacklistRule, FrequencyRule, RiskRule, TrustRule, WhitelistRule};
pub use rule_trait::ApprovalRule;

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 战役 2-3 实际借鉴 VCP 5 个真字段
/// (`enabled` / `timeoutMinutes` / `approveAll` / `approvalList` / `fuzzyToolMatching` / `SilentReject`)
pub const BORROWED_VCP_FIELDS: usize = 6;

/// 5 规则 (Trust / Risk / Frequency / Whitelist / Blacklist) — 编译期 hardcode
pub const RULE_COUNT: usize = 5;

/// 5 分钟审批窗口毫秒 (VCP `getTimeoutMs` 真值)
pub const APPROVAL_TIMEOUT_MS_CONST: u64 = 5 * 60 * 1000;

/// 频率规则默认窗口 (1 分钟 = 60_000 ms)
pub const FREQUENCY_WINDOW_MS: u64 = 60_000;

/// 频率规则默认阈值 (1 分钟内 3 次 = 反刷)
pub const FREQUENCY_MAX_CALLS: u32 = 3;

/// Fuzzy matching 距离阈值 (战役 2-2 借鉴 VCP §6.2.2 #18)
pub const FUZZY_MAX_DISTANCE: usize = 2;

/// 默认高风险类别前缀 (VCP 工程惯例, 实战中主人可改)
pub const DEFAULT_HIGH_RISK_CATEGORIES: [&str; 3] = ["system", "network", "file"];

/// 历史最大长度 (防御性裁剪, 防长会话内存增长)
pub const MAX_HISTORY_LEN: usize = 10_000;

// ============================================================
// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
// ============================================================

const _: () = {
    // 5 规则
    assert!(
        RULE_COUNT == 5,
        "RULE_COUNT = 5 (Trust / Risk / Frequency / Whitelist / Blacklist)"
    );
    assert!(
        BORROWED_VCP_FIELDS == 6,
        "BORROWED_VCP_FIELDS = 6 (VCP toolApprovalManager 字段)"
    );

    // 5min 窗口 = VCP 真值
    assert!(
        APPROVAL_TIMEOUT_MS_CONST == 300_000,
        "5min = 300_000 ms (VCP getTimeoutMs)"
    );
    assert!(
        APPROVAL_TIMEOUT_MS == 300_000,
        "manager APPROVAL_TIMEOUT_MS = 300_000"
    );

    // 频率 1min/3
    assert!(
        FREQUENCY_WINDOW_MS == 60_000,
        "FREQUENCY_WINDOW_MS = 60_000 (1min)"
    );
    assert!(FREQUENCY_MAX_CALLS == 3, "FREQUENCY_MAX_CALLS = 3");

    // Fuzzy 距离 ≤ 2
    assert!(
        FUZZY_MAX_DISTANCE == 2,
        "FUZZY_MAX_DISTANCE = 2 (VCP §6.2.2 #18)"
    );

    // 默认高风险 3 类
    assert!(DEFAULT_HIGH_RISK_CATEGORIES.len() == 3, "3 默认高风险类别");

    // 历史 10_000
    assert!(MAX_HISTORY_LEN == 10_000, "MAX_HISTORY_LEN = 10_000");
};

// ============================================================
// lib 入口测试 (编译期 hardcode 二次断言 + 5 规则可达性)
// ============================================================

#[cfg(test)]
mod lib_tests {
    use super::*;
    use apeireth_tool_runtime::ParsedToolCall;
    use serde_json::json;

    #[test]
    fn lib_constants_match_vcp() {
        // 编译期 hardcode 已 assert, 这里再 runtime 测一次
        assert_eq!(RULE_COUNT, 5);
        assert_eq!(BORROWED_VCP_FIELDS, 6);
        assert_eq!(APPROVAL_TIMEOUT_MS_CONST, 300_000);
        assert_eq!(APPROVAL_TIMEOUT_MS, 300_000);
        assert_eq!(FREQUENCY_WINDOW_MS, 60_000);
        assert_eq!(FREQUENCY_MAX_CALLS, 3);
        assert_eq!(FUZZY_MAX_DISTANCE, 2);
        assert_eq!(DEFAULT_HIGH_RISK_CATEGORIES.len(), 3);
        assert_eq!(MAX_HISTORY_LEN, 10_000);
    }

    #[test]
    fn lib_public_api_compiles() {
        // 验证 lib.rs 公开 API 全部可见
        let _decision = ApprovalDecision::Allow;
        let _record = CallRecord::new(
            &ParsedToolCall {
                tool_name: "X".to_string(),
                args: json!({}),
                raw_marker: "tool_name:<<<X>>>".to_string(),
                archery: false,
                archery_no_reply: false,
            },
            ApprovalDecision::Allow,
            None,
        );
        let _mgr = ApprovalManager::new();
        let _trust = TrustRule::new();
        let _risk = RiskRule::new(APPROVAL_TIMEOUT_MS);
        let _freq = FrequencyRule::new();
        let _white = WhitelistRule::new();
        let _black = BlacklistRule::new();
    }

    #[test]
    fn lib_five_rules_unique_names() {
        // 5 规则 name() 唯一
        let rules: Vec<Box<dyn ApprovalRule>> = vec![
            Box::new(TrustRule::new()),
            Box::new(RiskRule::new(300_000)),
            Box::new(FrequencyRule::new()),
            Box::new(WhitelistRule::new()),
            Box::new(BlacklistRule::new()),
        ];
        assert_eq!(rules.len(), RULE_COUNT);
        let names: Vec<&str> = rules.iter().map(|r| r.name()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 5, "5 规则 name() 必须唯一, 实际: {names:?}");
    }

    #[tokio::test]
    async fn lib_end_to_end_5_rules() {
        // 端到端: 5 规则组合 + 5min 窗口
        // 工具 "X" 同时: 黑名单 + 信任 + 白名单 + 系统风险 + 多次调用
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(BlacklistRule::with_blacklist(
            ["X".to_string()],
            false,
        )));
        mgr.add_rule(Box::new(TrustRule::with_trusted(["X".to_string()])));
        mgr.add_rule(Box::new(RiskRule::new(APPROVAL_TIMEOUT_MS)));
        mgr.add_rule(Box::new(FrequencyRule::new()));
        mgr.add_rule(Box::new(WhitelistRule::with_whitelist(["X".to_string()])));

        // 1. Blacklist 胜 (最高优先级) → Deny
        let d = mgr.check(&make_call("X"));
        assert!(
            d.is_deny(),
            "黑名单 + 信任 + 风险: Blacklist 胜, 实际: {d:?}"
        );

        // 2. 没有 blacklisted 的工具 + Trust → Allow
        let d2 = mgr.check(&make_call("Safe"));
        assert!(d2.is_allow(), "无规则命中应 Allow, 实际: {d2:?}");

        // 3. 高风险 + 无 handler → 5min 窗口内默认拒绝
        let r3 = mgr.wait_for_approval(&make_call("system.exec")).await;
        assert_eq!(r3, Ok(false), "无 handler 应默认拒绝");
    }

    #[test]
    fn lib_default_risk_categories_match_vcp() {
        // VCP 工程惯例: 3 个高风险类别 system / network / file
        assert_eq!(DEFAULT_HIGH_RISK_CATEGORIES.len(), 3);
        let c = &DEFAULT_HIGH_RISK_CATEGORIES;
        assert!(c.contains(&"system"));
        assert!(c.contains(&"network"));
        assert!(c.contains(&"file"));
    }

    #[test]
    fn lib_high_risk_categories_compile_time() {
        // 编译期 hardcode 3 个: 验证 RiskRule 用同一组
        let rule = RiskRule::new(APPROVAL_TIMEOUT_MS);
        let cats = rule.categories();
        assert_eq!(cats.len(), 3, "RiskRule 默认 3 类");
        for c in DEFAULT_HIGH_RISK_CATEGORIES.iter() {
            assert!(cats.contains(&(*c).to_string()), "应含 {c}");
        }
    }

    // helper
    fn make_call(tool: &str) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.to_string(),
            args: json!({}),
            raw_marker: format!("tool_name:<<<{tool}>>>"),
            archery: false,
            archery_no_reply: false,
        }
    }
}
