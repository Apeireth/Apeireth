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

    fn silent_on_reject(&self, call: &ParsedToolCall) -> bool {
        // 与 check 同调用语义: 命中黑名单才谈静默
        self.blacklist.read().contains(&call.tool_name) && self.silent
    }
}

// ============================================================
// 6. ApprovalListRule — VCP approvalList 审批清单 (命令级粒度 + 静默后缀)
// ============================================================

/// **从工具参数提取命令列表** (命令级粒度审批的输入)
///
/// **字段级引用 VCP**: `toolApprovalManager.js:93-115 extractCommands` —
/// - `toolArgs.command` (string, trim 非空) → 第一条
/// - `toolArgs.command1 / command2 / ...` (按数字升序) → 依次追加
/// - 非 string / 空白 → 跳过
///
/// 纯函数, 无副作用.
pub fn extract_commands(args: &serde_json::Value) -> Vec<String> {
    let mut commands = Vec::new();
    let Some(obj) = args.as_object() else {
        return commands;
    };
    if let Some(c) = obj.get("command").and_then(|v| v.as_str()) {
        let t = c.trim();
        if !t.is_empty() {
            commands.push(t.to_string());
        }
    }
    // command\d+ 按数字升序 (VCP: Number(a.slice(7)) 排序)
    let mut numbered: Vec<(u64, &str)> = obj
        .iter()
        .filter_map(|(k, v)| {
            let rest = k.strip_prefix("command")?;
            if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let idx: u64 = rest.parse().ok()?;
            let s = v.as_str()?;
            let t = s.trim();
            if t.is_empty() {
                return None;
            }
            Some((idx, t))
        })
        .collect();
    numbered.sort_by_key(|(idx, _)| *idx);
    commands.extend(numbered.into_iter().map(|(_, c)| c.to_string()));
    commands
}

/// **解析后的审批清单条目** (VCP `parseApprovalRule` 对应物)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedApprovalEntry {
    /// 原始条目文本 (含后缀, 供 matchedRule 报告)
    pub raw: String,
    /// 基础规则: `ToolName` (工具级) 或 `ToolName:command` (命令级)
    pub base: String,
    /// 静默拒绝标记 (VCP `::SilentReject` 后缀 → true)
    pub silent: bool,
}

/// VCP 静默拒绝后缀 (字段级引用 `toolApprovalManager.js:127`)
pub const SILENT_REJECT_SUFFIX: &str = "::SilentReject";

/// **解析一条审批清单条目**
///
/// **字段级引用 VCP**: `toolApprovalManager.js:117-142 parseApprovalRule` —
/// - 去 `::SilentReject` 后缀 → `silent = true`
/// - 空串 / 纯后缀 → `None` (无效条目跳过)
///
/// 纯函数.
pub fn parse_approval_entry(entry: &str) -> Option<ParsedApprovalEntry> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }
    let silent = trimmed.ends_with(SILENT_REJECT_SUFFIX);
    let base = if silent {
        trimmed[..trimmed.len() - SILENT_REJECT_SUFFIX.len()].trim()
    } else {
        trimmed
    };
    if base.is_empty() {
        return None;
    }
    Some(ParsedApprovalEntry {
        raw: trimmed.to_string(),
        base: base.to_string(),
        silent,
    })
}

/// **战役 2-3 增强 / 规则 #6 — ApprovalListRule (VCP approvalList 语义)**
///
/// **字段级引用 VCP**: `toolApprovalManager.js:144-225 getApprovalDecision` —
/// 清单内条目命中 = **需要主人审批** (`RequireApproval`), 高危操作走
/// "AI 请求 → 主人批准" 通道 (洋葱安全红线不破).
///
/// **命令级粒度** (VCP 新版吸收):
/// - 条目 `Tool` → 工具级 (specificity 1)
/// - 条目 `Tool:command` → 命令级 (specificity 2), command 从 `call.args`
///   的 `command` / `command1..N` 键提取 (`extract_commands`)
/// - 命令级优先于工具级; 同级并列时静默条目优先 (VCP `considerMatch`)
///
/// **静默拒绝** (VCP 新版吸收): 条目带 `::SilentReject` 后缀 →
/// 命中后若被拒绝, 不通知 AI, 仅留痕审计 (`silent_on_reject` 覆写).
pub struct ApprovalListRule {
    /// 解析后的清单条目
    entries: RwLock<Vec<ParsedApprovalEntry>>,
    /// 审批超时毫秒 (默认 `APPROVAL_TIMEOUT_MS` = VCP 5min 真值)
    timeout_ms: u64,
}

impl Default for ApprovalListRule {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalListRule {
    /// 新建空审批清单 (5min 默认窗口)
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            timeout_ms: crate::manager::APPROVAL_TIMEOUT_MS,
        }
    }

    /// 从字符串清单构造 (无效条目自动跳过, 同 VCP `parseApprovalRule` 行为)
    pub fn with_entries(entries: impl IntoIterator<Item = String>, timeout_ms: u64) -> Self {
        let parsed: Vec<ParsedApprovalEntry> = entries
            .into_iter()
            .filter_map(|e| parse_approval_entry(&e))
            .collect();
        Self {
            entries: RwLock::new(parsed),
            timeout_ms,
        }
    }

    /// 自定义超时 (审批窗口毫秒)
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// 追加一条清单条目; 无效条目返 false
    pub fn add_entry(&self, entry: &str) -> bool {
        match parse_approval_entry(entry) {
            Some(p) => {
                self.entries.write().push(p);
                true
            }
            None => false,
        }
    }

    /// 移除与 raw 完全一致的条目; 返是否移除了至少一条
    pub fn remove_entry(&self, raw: &str) -> bool {
        let mut e = self.entries.write();
        let before = e.len();
        e.retain(|p| p.raw != raw.trim());
        e.len() < before
    }

    /// 当前清单 (raw 文本克隆, 排序)
    pub fn list(&self) -> Vec<String> {
        let e = self.entries.read();
        let mut v: Vec<String> = e.iter().map(|p| p.raw.clone()).collect();
        v.sort();
        v
    }

    /// 条目数
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// 审批窗口毫秒
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// **考虑一个匹配, 择优更新 best** (VCP `considerMatch` 语义)
    ///
    /// specificity 高者胜; 同级并列时静默条目优先.
    fn consider<'e>(
        best: &mut Option<(&'e ParsedApprovalEntry, u8, Option<String>)>,
        entry: &'e ParsedApprovalEntry,
        specificity: u8,
        matched_command: Option<String>,
    ) {
        let take = match best {
            None => true,
            Some((_, s, _)) if specificity > *s => true,
            Some((best_entry, s, _))
                if specificity == *s && entry.silent && !best_entry.silent =>
            {
                true
            }
            _ => false,
        };
        if take {
            *best = Some((entry, specificity, matched_command));
        }
    }

    /// **最优匹配** (VCP `considerMatch` 语义, 纯函数)
    ///
    /// 返 `(条目, 命中的命令 or None)`; specificity 2 (命令级) > 1 (工具级),
    /// 同级静默优先.
    fn best_match<'e>(
        entries: &'e [ParsedApprovalEntry],
        tool_name: &str,
        commands: &[String],
    ) -> Option<(&'e ParsedApprovalEntry, Option<String>)> {
        let mut best: Option<(&'e ParsedApprovalEntry, u8, Option<String>)> = None;
        for entry in entries {
            if entry.base == tool_name {
                Self::consider(&mut best, entry, 1, None);
            }
            for command in commands {
                if entry.base == format!("{tool_name}:{command}") {
                    Self::consider(&mut best, entry, 2, Some(command.clone()));
                }
            }
        }
        best.map(|(e, _, c)| (e, c))
    }
}

impl crate::rule_trait::ApprovalRule for ApprovalListRule {
    fn name(&self) -> &str {
        "approval_list"
    }

    fn check(&self, call: &ParsedToolCall, _history: &[CallRecord]) -> ApprovalDecision {
        let entries = self.entries.read();
        if entries.is_empty() {
            return ApprovalDecision::NoMatch;
        }
        let commands = extract_commands(&call.args);
        match Self::best_match(&entries, &call.tool_name, &commands) {
            Some((entry, matched_command)) => {
                let scope = if matched_command.is_some() { "命令级" } else { "工具级" };
                debug!(
                    "[ApprovalListRule] 命中{}审批规则 [{}] tool={}{} → 需主人审批",
                    scope,
                    entry.raw,
                    call.tool_name,
                    if entry.silent { " (拒绝时静默)" } else { "" }
                );
                ApprovalDecision::RequireApproval {
                    timeout_ms: self.timeout_ms,
                }
            }
            None => ApprovalDecision::NoMatch,
        }
    }

    fn silent_on_reject(&self, call: &ParsedToolCall) -> bool {
        let entries = self.entries.read();
        let commands = extract_commands(&call.args);
        Self::best_match(&entries, &call.tool_name, &commands)
            .map(|(e, _)| e.silent)
            .unwrap_or(false)
    }

    fn matched_command(&self, call: &ParsedToolCall) -> Option<String> {
        let entries = self.entries.read();
        let commands = extract_commands(&call.args);
        Self::best_match(&entries, &call.tool_name, &commands).and_then(|(_, c)| c)
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
                matched_command: None,
                silent_on_reject: false,
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
                matched_command: None,
                silent_on_reject: false,
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
                matched_command: None,
                silent_on_reject: false,
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
                matched_command: None,
                silent_on_reject: false,
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
                matched_command: None,
                silent_on_reject: false,
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
        // 6 个不同 rule 类型塞进 Vec<Box<dyn ApprovalRule>>
        let mut rules: Vec<Box<dyn ApprovalRule>> = Vec::new();
        rules.push(Box::new(TrustRule::new()));
        rules.push(Box::new(RiskRule::new(300_000)));
        rules.push(Box::new(FrequencyRule::new()));
        rules.push(Box::new(WhitelistRule::new()));
        rules.push(Box::new(BlacklistRule::new()));
        rules.push(Box::new(ApprovalListRule::new()));
        assert_eq!(rules.len(), 6);

        // 每条规则都能调 check (不 panic)
        for r in &rules {
            let _ = r.check(&make_call("test"), &[]);
        }
    }

    // ====== extract_commands (VCP extractCommands 字段级) ======

    #[test]
    fn extract_commands_from_command_key() {
        let args = json!({"command": "ls -la"});
        assert_eq!(extract_commands(&args), vec!["ls -la".to_string()]);
    }

    #[test]
    fn extract_commands_numbered_sorted_and_trimmed() {
        // command2 写在 command1 前, 仍按数字升序输出 (VCP 行为)
        let args = json!({"command2": " b ", "command1": "a", "command10": "c10"});
        assert_eq!(
            extract_commands(&args),
            vec!["a".to_string(), "b".to_string(), "c10".to_string()]
        );
    }

    #[test]
    fn extract_commands_mixed_plain_and_numbered() {
        let args = json!({"command": "first", "command1": "second"});
        assert_eq!(
            extract_commands(&args),
            vec!["first".to_string(), "second".to_string()]
        );
    }

    #[test]
    fn extract_commands_skips_non_string_empty_non_object() {
        assert_eq!(extract_commands(&json!({"command": 42})), Vec::<String>::new());
        assert_eq!(extract_commands(&json!({"command": "   "})), Vec::<String>::new());
        assert_eq!(extract_commands(&json!(["command"])), Vec::<String>::new());
        assert_eq!(extract_commands(&json!(null)), Vec::<String>::new());
        // commandX (非数字后缀) 不算
        assert_eq!(
            extract_commands(&json!({"commandX": "nope"})),
            Vec::<String>::new()
        );
    }

    // ====== parse_approval_entry (VCP parseApprovalRule 字段级) ======

    #[test]
    fn parse_entry_plain_tool() {
        let p = parse_approval_entry("PowerShellExecutor").unwrap();
        assert_eq!(p.base, "PowerShellExecutor");
        assert!(!p.silent);
        assert_eq!(p.raw, "PowerShellExecutor");
    }

    #[test]
    fn parse_entry_command_level_with_silent_suffix() {
        let p = parse_approval_entry("FileOperator:delete ::SilentReject").unwrap();
        assert_eq!(p.base, "FileOperator:delete");
        assert!(p.silent);
        assert_eq!(p.raw, "FileOperator:delete ::SilentReject");
    }

    #[test]
    fn parse_entry_rejects_empty_and_bare_suffix() {
        assert!(parse_approval_entry("").is_none());
        assert!(parse_approval_entry("   ").is_none());
        assert!(parse_approval_entry("::SilentReject").is_none());
        assert!(parse_approval_entry("  ::SilentReject  ").is_none());
    }

    // ====== ApprovalListRule (命令级粒度 + 静默拒绝) ======

    fn make_call_with_args(tool: &str, args: serde_json::Value) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.to_string(),
            args,
            raw_marker: format!("tool_name:<<<{tool}>>>"),
            archery: false,
            archery_no_reply: false,
        }
    }

    #[test]
    fn approval_list_tool_level_requires_approval() {
        let rule = ApprovalListRule::with_entries(["PowerShellExecutor".to_string()], 300_000);
        let d = rule.check(&make_call("PowerShellExecutor"), &[]);
        assert!(d.is_require_approval(), "工具级命中需审批, 实际: {d:?}");
        assert!(!rule.silent_on_reject(&make_call("PowerShellExecutor")));
        assert_eq!(rule.matched_command(&make_call("PowerShellExecutor")), None);
    }

    #[test]
    fn approval_list_command_level_requires_approval() {
        let rule = ApprovalListRule::with_entries(
            ["FileOperator:delete".to_string()],
            300_000,
        );
        let call = make_call_with_args("FileOperator", json!({"command": "delete"}));
        let d = rule.check(&call, &[]);
        assert!(d.is_require_approval(), "命令级命中需审批, 实际: {d:?}");
        assert_eq!(rule.matched_command(&call), Some("delete".to_string()));
    }

    #[test]
    fn approval_list_command_must_match_exactly() {
        let rule = ApprovalListRule::with_entries(
            ["FileOperator:delete".to_string()],
            300_000,
        );
        // 命令不同 → NoMatch (不误伤其他命令)
        let call = make_call_with_args("FileOperator", json!({"command": "read"}));
        assert!(rule.check(&call, &[]).is_no_match());
        // 工具不同 → NoMatch
        let call2 = make_call_with_args("OtherTool", json!({"command": "delete"}));
        assert!(rule.check(&call2, &[]).is_no_match());
    }

    #[test]
    fn approval_list_command_beats_tool_specificity() {
        // 同清单既有工具级 (非静默) 又有命令级 (静默) → 命令级胜 (specificity 2 > 1)
        let rule = ApprovalListRule::with_entries(
            [
                "Shell".to_string(),
                "Shell:reboot::SilentReject".to_string(),
            ],
            300_000,
        );
        let call = make_call_with_args("Shell", json!({"command": "reboot"}));
        assert!(rule.check(&call, &[]).is_require_approval());
        assert!(rule.silent_on_reject(&call), "命令级静默条目应胜出");
        assert_eq!(rule.matched_command(&call), Some("reboot".to_string()));

        // 无命令参数 → 落回工具级 (非静默)
        let call2 = make_call("Shell");
        assert!(rule.check(&call2, &[]).is_require_approval());
        assert!(!rule.silent_on_reject(&call2));
    }

    #[test]
    fn approval_list_same_specificity_silent_wins() {
        // VCP considerMatch: specificity 并列时静默条目优先
        let rule = ApprovalListRule::with_entries(
            ["Shell".to_string(), "Shell::SilentReject".to_string()],
            300_000,
        );
        let call = make_call("Shell");
        assert!(rule.check(&call, &[]).is_require_approval());
        assert!(rule.silent_on_reject(&call), "同级并列静默优先");
    }

    #[test]
    fn approval_list_numbered_command_args() {
        let rule = ApprovalListRule::with_entries(
            ["Shell:shutdown".to_string()],
            300_000,
        );
        // command2 命中 (批量命令场景, VCP extractCommands 行为)
        let call = make_call_with_args(
            "Shell",
            json!({"command1": "echo hi", "command2": "shutdown"}),
        );
        assert!(rule.check(&call, &[]).is_require_approval());
        assert_eq!(rule.matched_command(&call), Some("shutdown".to_string()));
    }

    #[test]
    fn approval_list_no_match_when_empty_or_unlisted() {
        let rule = ApprovalListRule::new();
        assert!(rule.is_empty());
        assert!(rule.check(&make_call("X"), &[]).is_no_match());

        let rule2 = ApprovalListRule::with_entries(["A".to_string()], 300_000);
        assert!(rule2.check(&make_call("B"), &[]).is_no_match());
    }

    #[test]
    fn approval_list_invalid_entries_skipped() {
        // VCP parseApprovalRule: 无效条目 (空/纯后缀) 跳过不炸
        let rule = ApprovalListRule::with_entries(
            [
                "".to_string(),
                "::SilentReject".to_string(),
                "Good".to_string(),
            ],
            300_000,
        );
        assert_eq!(rule.len(), 1);
        assert!(rule.check(&make_call("Good"), &[]).is_require_approval());
    }

    #[test]
    fn approval_list_add_remove_entry() {
        let rule = ApprovalListRule::new();
        assert!(rule.add_entry("Shell:reboot::SilentReject"));
        assert!(!rule.add_entry("  "));
        assert_eq!(rule.len(), 1);
        assert!(rule.remove_entry("Shell:reboot::SilentReject"));
        assert!(!rule.remove_entry("nope"));
        assert!(rule.is_empty());
    }

    #[test]
    fn approval_list_timeout_is_vcp_5min_by_default() {
        let rule = ApprovalListRule::new();
        assert_eq!(rule.timeout_ms(), crate::manager::APPROVAL_TIMEOUT_MS);
        // 自定义超时真传到 RequireApproval
        let rule2 = ApprovalListRule::with_entries(["X".to_string()], 1234);
        match rule2.check(&make_call("X"), &[]) {
            ApprovalDecision::RequireApproval { timeout_ms } => assert_eq!(timeout_ms, 1234),
            other => panic!("清单命中应 RequireApproval, 实际: {other:?}"),
        }
    }

    #[test]
    fn blacklist_rule_silent_on_reject_override() {
        let silent = BlacklistRule::with_blacklist(["Bad".to_string()], true);
        assert!(silent.silent_on_reject(&make_call("Bad")));
        assert!(!silent.silent_on_reject(&make_call("Good")));

        let loud = BlacklistRule::with_blacklist(["Bad".to_string()], false);
        assert!(!loud.silent_on_reject(&make_call("Bad")));
    }

    // 测试 Arc 包装 (ApprovalManager 内部用 Arc<dyn ApprovalRule>)
    #[allow(dead_code)]
    fn _arc_works() {
        let _arc: std::sync::Arc<dyn ApprovalRule> = std::sync::Arc::new(TrustRule::new());
    }
}
