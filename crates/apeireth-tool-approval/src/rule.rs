//! **战役 2-3 / VCP `toolApprovalManager.js` — 5 规则审批规则**
//!
//! **目标**: 5 规则真实现 (Trust / Risk / Frequency / Whitelist / Blacklist),
//! 借鉴 VCP `toolApprovalManager.js:144-225 getApprovalDecision` 字段级 (rule 字符串 +
//! SilentReject + 规则列表优先级).
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - `toolApprovalManager.js:11-19` `config.approvalList + enabled + fuzzyToolMatching` → 我们的 `ApprovalRule` 列表
//! - `toolApprovalManager.js:127-141` `parseApprovalRule` (SilentReject 后缀) → `BlacklistRule` 静默拒绝
//! - `toolApprovalManager.js:144-225` `getApprovalDecision` 三层判断 → 我们的 `ApprovalManager::check`
//! - `toolApprovalManager.js:231-233` `getTimeoutMs` (5 分钟) → `RiskRule` 5min
//!
//! **Apeireth 扩展** (Apeireth 创新, VCP 没有):
//! - 5 规则 (VCP 是 1 个 approvalList, 我们拆 5 维度独立判断)
//! - FrequencyRule 1min/3 次反刷 (VCP 没有)
//! - TrustRule + WhitelistRule 区分 (VCP 是 1 个 list)
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ 5 规则各自真实现, 不只 mock
//! - ✅ FrequencyRule 真用历史 + 时间窗 (1min 滑窗)
//! - ✅ BlacklistRule 静默拒绝 (VCP `::SilentReject` suffix)
//! - ✅ 编译期 hardcode (`APPROVAL_TIMEOUT_MS = 5 * 60 * 1000`)

use std::collections::HashSet;

use parking_lot::RwLock;
use tracing::debug;

use apeireth_tool_runtime::ParsedToolCall;

use crate::decision::ApprovalDecision;
use crate::history::CallRecord;

// ============================================================
// 1. TrustRule — 主人信任列表 (VCP 没有, Apeireth 创新)
// ============================================================

/// **战役 2-3 / 5 规则 #1 — TrustRule (信任规则)**
///
/// 主人 trust 列表内的工具直接过 (Allow). 优先级最高.
///
/// **VCP 借鉴**: VCP 没有 trust 概念, 但 `approveAll` 是全局强制审批.
pub struct TrustRule {
    /// 信任的工具名集合 (case-sensitive)
    trusted: RwLock<HashSet<String>>,
}

impl Default for TrustRule {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustRule {
    /// 新建空 trust rule
    pub fn new() -> Self {
        Self {
            trusted: RwLock::new(HashSet::new()),
        }
    }

    /// 从初始列表构造
    pub fn with_trusted(initial: impl IntoIterator<Item = String>) -> Self {
        Self {
            trusted: RwLock::new(initial.into_iter().collect()),
        }
    }

    /// 添加信任工具
    pub fn trust(&self, tool_name: impl Into<String>) {
        let mut t = self.trusted.write();
        t.insert(tool_name.into());
    }

    /// 移除信任工具
    pub fn untrust(&self, tool_name: &str) -> bool {
        let mut t = self.trusted.write();
        t.remove(tool_name)
    }

    /// 当前信任列表 (克隆)
    pub fn list(&self) -> Vec<String> {
        let t = self.trusted.read();
        let mut v: Vec<String> = t.iter().cloned().collect();
        v.sort();
        v
    }

    /// 信任数量
    pub fn len(&self) -> usize {
        self.trusted.read().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.trusted.read().is_empty()
    }
}

impl crate::rule_trait::ApprovalRule for TrustRule {
    fn name(&self) -> &str {
        "trust"
    }
    fn check(&self, call: &ParsedToolCall, _history: &[CallRecord]) -> ApprovalDecision {
        let t = self.trusted.read();
        if t.contains(&call.tool_name) {
            debug!("[TrustRule] 信任工具直接通过: {}", call.tool_name);
            ApprovalDecision::Allow
        } else {
            ApprovalDecision::NoMatch
        }
    }
}

// ============================================================
// 2. RiskRule — 风险规则 (高风险工具要求 5 分钟审批)
// ============================================================

/// **战役 2-3 / 5 规则 #2 — RiskRule (风险规则)**
///
/// 高风险类别工具 (system / network / file) 必须 5 分钟审批 (`APPROVAL_TIMEOUT_MS`).
///
/// **字段级引用 VCP**: `toolApprovalManager.js:231-233 getTimeoutMs` = `5 * 60 * 1000` ms
///
/// **判定方式**: 工具名以高风险类别前缀开头 (e.g. `system_*`, `network_*`, `file_*`).
///   这是工程惯例, 不强制 (VCP 也是用 name pattern 判定).
pub struct RiskRule {
    /// 高风险类别前缀 (e.g. `["system", "network", "file"]`)
    high_risk_categories: Vec<String>,
    /// 审批超时毫秒 (默认 5 * 60 * 1000 = VCP 真值)
    approval_timeout_ms: u64,
}

impl RiskRule {
    /// 默认高风险类别 (VCP 工程惯例, 实战中主人可改)
    pub const DEFAULT_HIGH_RISK_CATEGORIES: &'static [&'static str] =
        &["system", "network", "file"];

    /// 新建 risk rule
    pub fn new(approval_timeout_ms: u64) -> Self {
        Self {
            high_risk_categories: Self::DEFAULT_HIGH_RISK_CATEGORIES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            approval_timeout_ms,
        }
    }

    /// 自定义高风险类别
    pub fn with_categories(
        approval_timeout_ms: u64,
        categories: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            high_risk_categories: categories.into_iter().collect(),
            approval_timeout_ms,
        }
    }

    /// 工具名是否命中高风险类别
    pub fn is_high_risk(&self, tool_name: &str) -> bool {
        let lower = tool_name.to_lowercase();
        self.high_risk_categories
            .iter()
            .any(|cat| lower.starts_with(&cat.to_lowercase()))
    }

    /// 当前高风险类别列表
    pub fn categories(&self) -> Vec<String> {
        self.high_risk_categories.clone()
    }

    /// 审批超时毫秒
    pub fn approval_timeout_ms(&self) -> u64 {
        self.approval_timeout_ms
    }
}

impl crate::rule_trait::ApprovalRule for RiskRule {
    fn name(&self) -> &str {
        "risk"
    }
    fn check(&self, call: &ParsedToolCall, _history: &[CallRecord]) -> ApprovalDecision {
        if self.is_high_risk(&call.tool_name) {
            debug!("[RiskRule] 高风险工具 {} 需 5min 审批", call.tool_name);
            ApprovalDecision::RequireApproval {
                timeout_ms: self.approval_timeout_ms,
            }
        } else {
            ApprovalDecision::NoMatch
        }
    }
}

// ============================================================
// 3. FrequencyRule — 频率规则 (1 分钟同工具 ≥ 3 次自动拒绝, 反刷)
// ============================================================

/// **战役 2-3 / 5 规则 #3 — FrequencyRule (频率规则, 反刷)**
///
/// 1 分钟 (60_000 ms) 窗口内同工具被调 ≥ 3 次 → 自动拒绝 (Deny, 非静默).
///
/// **Apeireth 创新**: VCP `toolApprovalManager.js` 没有此规则, 是我们加的.
///   实战中 LLM 可能陷入死循环反复调同一工具, 频率规则防止无限刷.
pub struct FrequencyRule {
    /// 频率窗口 (毫秒), 默认 60_000 (1 分钟)
    pub window_ms: u64,
    /// 窗口内最大允许调用次数, 默认 3
    pub max_calls: u32,
}

impl Default for FrequencyRule {
    fn default() -> Self {
        Self::new()
    }
}

impl FrequencyRule {
    /// 默认 1 分钟 / 3 次
    pub fn new() -> Self {
        Self {
            window_ms: 60_000,
            max_calls: 3,
        }
    }

    /// 自定义窗口 + 阈值
    pub fn with_limits(window_ms: u64, max_calls: u32) -> Self {
        Self {
            window_ms,
            max_calls,
        }
    }
}

impl crate::rule_trait::ApprovalRule for FrequencyRule {
    fn name(&self) -> &str {
        "frequency"
    }
    fn check(&self, call: &ParsedToolCall, history: &[CallRecord]) -> ApprovalDecision {
        let now_ms = crate::history::now_ms();
        // 统计窗口内 (now - window_ms, now] 同工具调用次数 (不含当前)
        let count = history
            .iter()
            .filter(|r| {
                r.tool_name == call.tool_name
                    && r.timestamp_ms >= now_ms - self.window_ms as i64
                    && r.timestamp_ms <= now_ms
            })
            .count() as u32;
        // 这次调用是第 (count+1) 次, 若 count+1 >= max_calls 触发反刷拒绝
        if count + 1 >= self.max_calls {
            debug!(
                "[FrequencyRule] 工具 {} 1min 内第 {} 次调用 (≥ {} 阈值), 触发反刷拒绝",
                call.tool_name,
                count + 1,
                self.max_calls
            );
            ApprovalDecision::Deny {
                reason: format!(
                    "频率超限: 1min 内第 {} 次调用 (阈值 {})",
                    count + 1,
                    self.max_calls
                ),
                silent: false,
            }
        } else {
            ApprovalDecision::NoMatch
        }
    }
}

// ============================================================
// 4. WhitelistRule — 白名单规则 (低风险工具直接过)
// ============================================================

/// **战役 2-3 / 5 规则 #4 — WhitelistRule (白名单规则)**
///
/// 白名单内 (low risk) 工具直接过 (Allow). 优先级低于 Trust (黑名单优先于白名单).
///
/// **VCP 借鉴**: VCP `approvalList` 是反向 (在列表里 = 需要审批), 我们白名单是正向 (在列表里 = 直接过).
pub struct WhitelistRule {
    /// 白名单工具名集合
    whitelist: RwLock<HashSet<String>>,
}

impl Default for WhitelistRule {
    fn default() -> Self {
        Self::new()
    }
}

impl WhitelistRule {
    /// 新建空 whitelist
    pub fn new() -> Self {
        Self {
            whitelist: RwLock::new(HashSet::new()),
        }
    }

    /// 从初始列表构造
    pub fn with_whitelist(initial: impl IntoIterator<Item = String>) -> Self {
        Self {
            whitelist: RwLock::new(initial.into_iter().collect()),
        }
    }

    /// 加入白名单
    pub fn allow(&self, tool_name: impl Into<String>) {
        let mut w = self.whitelist.write();
        w.insert(tool_name.into());
    }

    /// 移出白名单
    pub fn disallow(&self, tool_name: &str) -> bool {
        let mut w = self.whitelist.write();
        w.remove(tool_name)
    }

    /// 当前白名单 (克隆)
    pub fn list(&self) -> Vec<String> {
        let w = self.whitelist.read();
        let mut v: Vec<String> = w.iter().cloned().collect();
        v.sort();
        v
    }

    /// 白名单大小
    pub fn len(&self) -> usize {
        self.whitelist.read().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.whitelist.read().is_empty()
    }
}

impl crate::rule_trait::ApprovalRule for WhitelistRule {
    fn name(&self) -> &str {
        "whitelist"
    }
    fn check(&self, call: &ParsedToolCall, _history: &[CallRecord]) -> ApprovalDecision {
        let w = self.whitelist.read();
        if w.contains(&call.tool_name) {
            debug!("[WhitelistRule] 白名单工具直接过: {}", call.tool_name);
            ApprovalDecision::Allow
        } else {
            ApprovalDecision::NoMatch
        }
    }
}

// ============================================================
// 5. BlacklistRule — 黑名单规则 (已知危险直接拒绝, 支持静默)
// ============================================================

/// **战役 2-3 / 5 规则 #5 — BlacklistRule (黑名单规则)**
///
/// 黑名单内工具直接拒绝. 优先级最高 (黑名单永远最严).
///
/// **字段级引用 VCP**: `toolApprovalManager.js:127-141 parseApprovalRule` `SilentReject` 后缀
/// → 我们的 `silent: bool` 字段
pub struct BlacklistRule {
    /// 黑名单 (tool_name → 静默标记)
    blacklist: RwLock<HashSet<String>>,
    /// 静默模式 (true = 拒绝时不通知 AI, VCP `::SilentReject`)
    silent: bool,
    /// 黑名单 reason 文案
    reason_template: String,
}

impl Default for BlacklistRule {
    fn default() -> Self {
        Self::new()
    }
}

impl BlacklistRule {
    /// 新建黑名单 (默认非静默)
    pub fn new() -> Self {
        Self {
            blacklist: RwLock::new(HashSet::new()),
            silent: false,
            reason_template: "黑名单拒绝".to_string(),
        }
    }

    /// 静默黑名单 (VCP `::SilentReject` 风格)
    pub fn silent() -> Self {
        Self {
            blacklist: RwLock::new(HashSet::new()),
            silent: true,
            reason_template: "黑名单静默拒绝".to_string(),
        }
    }

    /// 从初始列表构造
    pub fn with_blacklist(initial: impl IntoIterator<Item = String>, silent: bool) -> Self {
        let s = if silent { Self::silent() } else { Self::new() };
        {
            let mut b = s.blacklist.write();
            for n in initial {
                b.insert(n);
            }
        }
        s
    }

    /// 加入黑名单
    pub fn deny(&self, tool_name: impl Into<String>) {
        let mut b = self.blacklist.write();
        b.insert(tool_name.into());
    }

    /// 移出黑名单
    pub fn undeny(&self, tool_name: &str) -> bool {
        let mut b = self.blacklist.write();
        b.remove(tool_name)
    }

    /// 当前黑名单 (克隆)
    pub fn list(&self) -> Vec<String> {
        let b = self.blacklist.read();
        let mut v: Vec<String> = b.iter().cloned().collect();
        v.sort();
        v
    }

    /// 黑名单大小
    pub fn len(&self) -> usize {
        self.blacklist.read().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.blacklist.read().is_empty()
    }

    /// 是否静默模式
    pub fn is_silent(&self) -> bool {
        self.silent
    }
}

impl crate::rule_trait::ApprovalRule for BlacklistRule {
    fn name(&self) -> &str {
        "blacklist"
    }
    fn check(&self, call: &ParsedToolCall, _history: &[CallRecord]) -> ApprovalDecision {
        let b = self.blacklist.read();
        if b.contains(&call.tool_name) {
            debug!(
                "[BlacklistRule] 黑名单工具 {} 直接拒绝 (silent={})",
                call.tool_name, self.silent
            );
            ApprovalDecision::Deny {
                reason: format!("{}: {}", self.reason_template, call.tool_name),
                silent: self.silent,
            }
        } else {
            ApprovalDecision::NoMatch
        }
    }
}

// ============================================================
// 单元测试 (5 规则 2+ test, 字段级引用 VCP + 实战边界)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_trait::ApprovalRule;
    use apeireth_tool_runtime::ParsedToolCall;
    use serde_json::json;

    fn make_call(tool: &str) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.to_string(),
            args: json!({}),
            raw_marker: format!("tool_name:<<<{}>>>", tool),
            archery: false,
            archery_no_reply: false,
        }
    }

    // ====== TrustRule (5 规则 #1) ======

    #[test]
    fn trust_rule_allows_trusted() {
        let rule = TrustRule::with_trusted(["SafeTool".to_string(), "ReadOnly".to_string()]);
        let d = rule.check(&make_call("SafeTool"), &[]);
        assert!(d.is_allow(), "信任工具应 Allow, 实际: {d:?}");
    }

    #[test]
    fn trust_rule_skips_untrusted() {
        let rule = TrustRule::with_trusted(["SafeTool".to_string()]);
        let d = rule.check(&make_call("Dangerous"), &[]);
        assert!(d.is_no_match(), "未信任应 NoMatch (交给下条规则)");
    }

    #[test]
    fn trust_rule_add_remove() {
        let rule = TrustRule::new();
        rule.trust("X");
        assert_eq!(rule.len(), 1);
        assert!(rule.untrust("X"));
        assert_eq!(rule.len(), 0);
        assert!(!rule.untrust("X"));
    }

    // ====== RiskRule (5 规则 #2) ======

    #[test]
    fn risk_rule_requires_5min_for_system() {
        let rule = RiskRule::new(5 * 60 * 1000);
        let d = rule.check(&make_call("system.exec"), &[]);
        match d {
            ApprovalDecision::RequireApproval { timeout_ms } => {
                assert_eq!(timeout_ms, 300_000, "5min = 300_000 ms");
            }
            _ => panic!("高风险系统工具应 RequireApproval(5min), 实际: {d:?}"),
        }
    }

    #[test]
    fn risk_rule_skips_low_risk() {
        let rule = RiskRule::new(5 * 60 * 1000);
        let d = rule.check(&make_call("Hello"), &[]);
        assert!(d.is_no_match(), "低风险应 NoMatch, 实际: {d:?}");
    }

    #[test]
    fn risk_rule_custom_categories() {
        let rule =
            RiskRule::with_categories(60_000, vec!["admin".to_string(), "secret".to_string()]);
        assert!(rule.is_high_risk("admin.delete"));
        assert!(rule.is_high_risk("SecretRead"));
        assert!(!rule.is_high_risk("PublicRead"));
    }

    // ====== FrequencyRule (5 规则 #3) ======
    //
    // **语义**: 第 N 次调用时, count = 历史中 1min 内的同工具次数. 第 N = count+1 次
    //   若 N >= max_calls → Deny. 即"1min 内 ≥ 3 次"在第 3 次调用时触发反刷拒绝.
    //
    // 测试约定: history 提供前 N-1 次, 模拟"当前是第 N 次"调用

    #[test]
    fn frequency_rule_denies_at_3rd_call() {
        // 第 3 次: 2 in history (前 2 次) + 1 (本次) = 3 ≥ 3 → Deny
        let rule = FrequencyRule::new(); // 1min/3
        let now = crate::history::now_ms();
        let history: Vec<CallRecord> = (0..2)
            .map(|i| CallRecord {
                id: format!("r{i}"),
                tool_name: "Spammy".to_string(),
                args: json!({}),
                timestamp_ms: now - 100 + i as i64, // 100ms 内 2 次
                decision: ApprovalDecision::Allow,
                matched_rule: None,
            })
            .collect();
        let d = rule.check(&make_call("Spammy"), &history);
        assert!(d.is_deny(), "1min 内第 3 次应 Deny, 实际: {d:?}");
    }

    #[test]
    fn frequency_rule_allows_2nd_call() {
        // 第 2 次: 1 in history + 1 = 2 < 3 → NoMatch (Allow)
        let rule = FrequencyRule::new();
        let now = crate::history::now_ms();
        let history: Vec<CallRecord> = (0..1)
            .map(|i| CallRecord {
                id: format!("r{i}"),
                tool_name: "Normal".to_string(),
                args: json!({}),
                timestamp_ms: now - 100 + i as i64,
                decision: ApprovalDecision::Allow,
                matched_rule: None,
            })
            .collect();
        let d = rule.check(&make_call("Normal"), &history);
        assert!(d.is_no_match(), "第 2 次 (< 3) 应 NoMatch, 实际: {d:?}");
    }

    #[test]
    fn frequency_rule_window_resets() {
        // 2 次在 2 分钟前, 不在窗口内 → count=0, 第 3 次允许
        let rule = FrequencyRule::new();
        let now = crate::history::now_ms();
        let history: Vec<CallRecord> = (0..2)
            .map(|i| CallRecord {
                id: format!("r{i}"),
                tool_name: "OldSpam".to_string(),
                args: json!({}),
                timestamp_ms: now - 120_000 + i as i64, // 2min 前
                decision: ApprovalDecision::Allow,
                matched_rule: None,
            })
            .collect();
        let d = rule.check(&make_call("OldSpam"), &history);
        assert!(d.is_no_match(), "2min 前的 2 次不在窗口内, 应 NoMatch");
    }

    #[test]
    fn frequency_rule_custom_threshold_allows_4th() {
        // threshold=5: 第 4 次 (3 in history + 1 = 4) < 5 → NoMatch
        let rule = FrequencyRule::with_limits(60_000, 5);
        let now = crate::history::now_ms();
        let history: Vec<CallRecord> = (0..3)
            .map(|i| CallRecord {
                id: format!("r{i}"),
                tool_name: "X".to_string(),
                args: json!({}),
                timestamp_ms: now - 100 + i as i64,
                decision: ApprovalDecision::Allow,
                matched_rule: None,
            })
            .collect();
        let d = rule.check(&make_call("X"), &history);
        assert!(
            d.is_no_match(),
            "第 4 次 (threshold 5) 应 NoMatch, 实际: {d:?}"
        );
    }

    #[test]
    fn frequency_rule_custom_threshold_denies_5th() {
        // threshold=5: 第 5 次 (4 in history + 1 = 5) >= 5 → Deny
        let rule = FrequencyRule::with_limits(60_000, 5);
        let now = crate::history::now_ms();
        let history: Vec<CallRecord> = (0..4)
            .map(|i| CallRecord {
                id: format!("r{i}"),
                tool_name: "X".to_string(),
                args: json!({}),
                timestamp_ms: now - 100 + i as i64,
                decision: ApprovalDecision::Allow,
                matched_rule: None,
            })
            .collect();
        let d = rule.check(&make_call("X"), &history);
        assert!(d.is_deny(), "第 5 次 (threshold 5) 应 Deny, 实际: {d:?}");
    }

    // ====== WhitelistRule (5 规则 #4) ======

    #[test]
    fn whitelist_rule_allows_listed() {
        let rule = WhitelistRule::with_whitelist(["Greeting".to_string(), "Help".to_string()]);
        let d = rule.check(&make_call("Greeting"), &[]);
        assert!(d.is_allow(), "白名单工具应 Allow");
    }

    #[test]
    fn whitelist_rule_skips_unlisted() {
        let rule = WhitelistRule::with_whitelist(["Greeting".to_string()]);
        let d = rule.check(&make_call("RiskTool"), &[]);
        assert!(d.is_no_match(), "非白名单应 NoMatch");
    }

    // ====== BlacklistRule (5 规则 #5) ======

    #[test]
    fn blacklist_rule_denies_listed_not_silent() {
        let rule = BlacklistRule::with_blacklist(["DangerousTool".to_string()], false);
        let d = rule.check(&make_call("DangerousTool"), &[]);
        match d {
            ApprovalDecision::Deny { silent, .. } => {
                assert!(!silent, "默认非静默");
            }
            _ => panic!("黑名单应 Deny, 实际: {d:?}"),
        }
    }

    #[test]
    fn blacklist_rule_silent_mode() {
        let rule = BlacklistRule::with_blacklist(["D".to_string()], true);
        let d = rule.check(&make_call("D"), &[]);
        match d {
            ApprovalDecision::Deny { silent, .. } => {
                assert!(silent, "VCP `::SilentReject` 模式: 拒绝时不通知 AI");
            }
            _ => panic!("应 Deny(silent=true)"),
        }
    }

    #[test]
    fn blacklist_rule_skips_unlisted() {
        let rule = BlacklistRule::with_blacklist(["D".to_string()], false);
        let d = rule.check(&make_call("OK"), &[]);
        assert!(d.is_no_match(), "非黑名单应 NoMatch");
    }

    #[test]
    fn rule_names_match_vcp_5() {
        // 5 规则 name() 必须唯一 + 字段名跟 VCP 一致
        let trust: Box<dyn ApprovalRule> = Box::new(TrustRule::new());
        let risk: Box<dyn ApprovalRule> = Box::new(RiskRule::new(300_000));
        let freq: Box<dyn ApprovalRule> = Box::new(FrequencyRule::new());
        let white: Box<dyn ApprovalRule> = Box::new(WhitelistRule::new());
        let black: Box<dyn ApprovalRule> = Box::new(BlacklistRule::new());
        let names: Vec<&str> = vec![
            trust.name(),
            risk.name(),
            freq.name(),
            white.name(),
            black.name(),
        ];
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 5, "5 规则 name() 必须唯一, 实际: {names:?}");
    }

    #[test]
    fn rule_trait_is_object_safe() {
        // 编译期守: ApprovalRule trait 是 dyn-compatible (对象安全)
        // 5 个不同 rule 类型塞进 Vec<Box<dyn ApprovalRule>>
        let mut rules: Vec<Box<dyn ApprovalRule>> = Vec::new();
        rules.push(Box::new(TrustRule::new()));
        rules.push(Box::new(RiskRule::new(300_000)));
        rules.push(Box::new(FrequencyRule::new()));
        rules.push(Box::new(WhitelistRule::new()));
        rules.push(Box::new(BlacklistRule::new()));
        assert_eq!(rules.len(), 5);

        // 每条规则都能调 check (不 panic)
        for r in &rules {
            let _ = r.check(&make_call("test"), &[]);
        }
    }

    // 测试 Arc 包装 (ApprovalManager 内部用 Arc<dyn ApprovalRule>)
    #[allow(dead_code)]
    fn _arc_works() {
        let _arc: std::sync::Arc<dyn ApprovalRule> = std::sync::Arc::new(TrustRule::new());
    }
}
