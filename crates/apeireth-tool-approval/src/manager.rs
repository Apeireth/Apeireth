//! **战役 2-3 / VCP `toolApprovalManager.js` — ApprovalManager 主体**
//!
//! **目标**: 5 规则按顺序检查 + 5 分钟审批窗口 + wait_for_approval 真实现.
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - `toolApprovalManager.js:144-225 getApprovalDecision` 三层判断 → 我们的 5 规则按顺序 check
//! - `toolApprovalManager.js:231-233 getTimeoutMs` (5min) → `APPROVAL_TIMEOUT_MS = 5 * 60 * 1000`
//!
//! **5 规则按顺序** (VCP 借鉴 + Apeireth 创新):
//! 1. **BlacklistRule** — 最严, 最高优先级
//! 2. **TrustRule** — 信任放行
//! 3. **RiskRule** — 高风险要求 5min 审批
//! 4. **FrequencyRule** — 反刷自动拒绝
//! 5. **WhitelistRule** — 白名单放行 (兜底)
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ 5 规则真按顺序 check (第一个非 NoMatch 生效)
//! - ✅ 5 分钟窗口真用 tokio::time::timeout
//! - ✅ wait_for_approval 真等待外部 handler 响应
//! - ✅ 编译期 hardcode (`APPROVAL_TIMEOUT_MS = 5 * 60 * 1000`)

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{debug, warn};

use apeireth_tool_runtime::ParsedToolCall;
use serde::Serialize;

use crate::decision::{ApprovalDecision, ApprovalOutcome, CheckDetail, RejectErrorType, Rejection};
use crate::history::CallRecord;
use crate::rule_trait::ApprovalRule;

/// **战役 2-3 / VCP 真值 — 5 分钟审批超时 (毫秒)**
///
/// **字段级引用 VCP**: `toolApprovalManager.js:11 config.timeoutMinutes` 默认 5
/// + `toolApprovalManager.js:231-233 getTimeoutMs` = `5 * 60 * 1000`
pub const APPROVAL_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// **战役 2-3 — 审批请求处理器 (外部系统用)**
///
/// **VCP 借鉴**: VCP 是把审批请求发到 Node.js event loop, 主人通过 web UI 响应.
///   我们用 tokio oneshot channel + handler trait, 实战中 Tauri / SSE handler 注册.
///
/// **默认行为**: 没注册 handler 时 `wait_for_approval` 超时返 false (主人没响应 = 拒绝).
#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// 处理一个审批请求, 返 true = 批准, false = 拒绝
    ///
    /// **超时**: 由 `ApprovalManager::wait_for_approval` 控制在 `timeout_ms` 内
    async fn handle(&self, call: &ParsedToolCall) -> bool;

    /// 处理一个审批请求, 附带主人填写的审核理由 (可选)
    ///
    /// **字段级引用 VCP**: `TOOL_APPROVAL_REASON_PROTOCOL.md` — 审批响应可选
    /// `reason` 字段. 默认实现委托 `handle` 且无理由 (旧 handler 零改动兼容).
    ///
    /// **静默约束**: 若命中的规则是静默拒绝 (`::SilentReject`), 即使主人在此
    /// 填了理由, 上层也**不得**把理由回传给 AI (只留痕审计).
    async fn handle_with_reason(&self, call: &ParsedToolCall) -> (bool, Option<String>) {
        (self.handle(call).await, None)
    }
}

/// **默认审批 handler — 自动拒绝 (无注册 handler 时用)**
pub struct DefaultDenyHandler;

#[async_trait]
impl ApprovalHandler for DefaultDenyHandler {
    async fn handle(&self, _call: &ParsedToolCall) -> bool {
        warn!("[ApprovalManager] 无 handler 注册, 默认拒绝 (主人未响应)");
        false
    }
}

/// **自动批准 handler (供测试 / 受信任环境用)**
pub struct AutoApproveHandler;

#[async_trait]
impl ApprovalHandler for AutoApproveHandler {
    async fn handle(&self, _call: &ParsedToolCall) -> bool {
        true
    }
}

/// 审计台账最大长度 (防御性裁剪, 同 history 思路)
pub const MAX_AUDIT_LEN: usize = 1_000;

/// **审批通道结果审计条目** (静默拒绝"只留痕"的载体)
///
/// 每次 `wait_for_approval_outcome` 产生终态 (批准/拒绝) 都追加一条,
/// 无论是否静默 — 静默拒绝不打扰 AI, 但审计永远可查.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApprovalAuditEntry {
    /// 条目 ID (audit- 前缀)
    pub id: String,
    /// 工具名
    pub tool_name: String,
    /// 最终是否批准
    pub approved: bool,
    /// 是否主人亲自拒绝 (仅拒绝时有意义)
    pub rejected_by_user: bool,
    /// 结构化错误码 (批准时 None)
    pub error_type: Option<RejectErrorType>,
    /// 静默标记 (true = 该拒绝不通知 AI)
    pub silent: bool,
    /// 命中的规则名
    pub matched_rule: Option<String>,
    /// 命中的命令级键 (命令级粒度)
    pub matched_command: Option<String>,
    /// 时间戳 (unix ms)
    pub timestamp_ms: i64,
}

/// **战役 2-3 — 审批管理器**
///
/// **核心字段**:
/// - `rules: Vec<Box<dyn ApprovalRule>>` — 5 规则列表 (按顺序 check)
/// - `approval_timeout_ms: u64` — 5min 窗口 (VCP 真值)
/// - `history: Mutex<VecDeque<CallRecord>>` — 审批历史 (FrequencyRule 用)
/// - `handler: Arc<Mutex<Option<Arc<dyn ApprovalHandler>>>>` — 外部审批 handler
pub struct ApprovalManager {
    /// 规则链 (按顺序 check, 第一个非 NoMatch 生效; 核心 5 规则 + 可选 ApprovalListRule)
    rules: Vec<Box<dyn ApprovalRule>>,
    /// 审批超时毫秒 (默认 `APPROVAL_TIMEOUT_MS = 5 * 60 * 1000`)
    approval_timeout_ms: u64,
    /// 审批历史 (滑动窗口, 给 FrequencyRule 用)
    history: Mutex<VecDeque<CallRecord>>,
    /// 外部审批 handler (无注册 = 通道不可用, 结构化拒绝 ChannelUnavailable)
    handler: Arc<Mutex<Option<Arc<dyn ApprovalHandler>>>>,
    /// 审批通道结果审计台账 (静默拒绝只留痕的载体, 上限 `MAX_AUDIT_LEN`)
    audit: Mutex<VecDeque<ApprovalAuditEntry>>,
}

impl ApprovalManager {
    /// 新建空 manager (无规则), 默认 5min 窗口
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            approval_timeout_ms: APPROVAL_TIMEOUT_MS,
            history: Mutex::new(VecDeque::new()),
            handler: Arc::new(Mutex::new(None)),
            audit: Mutex::new(VecDeque::new()),
        }
    }

    /// 用规则链构造
    pub fn with_rules(rules: Vec<Box<dyn ApprovalRule>>) -> Self {
        Self {
            rules,
            approval_timeout_ms: APPROVAL_TIMEOUT_MS,
            history: Mutex::new(VecDeque::new()),
            handler: Arc::new(Mutex::new(None)),
            audit: Mutex::new(VecDeque::new()),
        }
    }

    /// 自定义超时
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.approval_timeout_ms = timeout_ms;
        self
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: Box<dyn ApprovalRule>) {
        self.rules.push(rule);
    }

    /// 注册外部审批 handler
    pub fn set_handler(&self, handler: Arc<dyn ApprovalHandler>) {
        let mut h = self.handler.lock();
        *h = Some(handler);
    }

    /// 清除审批 handler (恢复 DefaultDenyHandler)
    pub fn clear_handler(&self) {
        let mut h = self.handler.lock();
        *h = None;
    }

    /// 规则数量
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 审批超时毫秒
    pub fn approval_timeout_ms(&self) -> u64 {
        self.approval_timeout_ms
    }

    /// 当前历史长度
    pub fn history_len(&self) -> usize {
        self.history.lock().len()
    }

    /// 取历史 (克隆, 清空)
    pub fn take_history(&self) -> Vec<CallRecord> {
        let mut h = self.history.lock();
        h.drain(..).collect()
    }

    /// 浅克隆历史 (供单次 check 用, 不清空)
    pub fn snapshot_history(&self) -> Vec<CallRecord> {
        self.history.lock().iter().cloned().collect()
    }

    /// **核心方法 — 检查一个调用**
    ///
    /// **规则链按顺序**:
    /// 1. 遍历 rules
    /// 2. 调 `rule.check(call, history_snapshot)`
    /// 3. 第一个非 `NoMatch` 的决策生效
    /// 4. 全 `NoMatch` → 默认 `Allow` (VCP `defaultDecision.requiresApproval = false`)
    ///
    /// **副作用**: 不论结果都记录到 history (供 FrequencyRule 下次判断)
    ///
    /// **VCP 借鉴**: `toolApprovalManager.js:144-225 getApprovalDecision` 三层判断
    ///
    /// 需要匹配明细 (命令级键 / 静默标记) 时用 `check_detailed`.
    pub fn check(&self, call: &ParsedToolCall) -> ApprovalDecision {
        self.check_detailed(call).0
    }

    /// **核心方法 — 检查一个调用, 附带匹配明细** (命令级粒度 + 静默标记)
    ///
    /// **字段级引用 VCP**: `getApprovalDecision` 返
    /// `{requiresApproval, notifyAiOnReject, matchedRule, matchedCommand}` →
    /// 我们返 `(ApprovalDecision, CheckDetail)` (决策器纯函数, 与审批通道解耦).
    ///
    /// `CheckDetail` 由命中规则的 `silent_on_reject` / `matched_command` 填充
    /// (trait 默认实现保证工具级规则返 false / None).
    pub fn check_detailed(&self, call: &ParsedToolCall) -> (ApprovalDecision, CheckDetail) {
        let history_snapshot = self.snapshot_history();
        let mut final_decision = ApprovalDecision::Allow; // 默认 Allow (VCP 行为)
        let mut detail = CheckDetail::default();

        for rule in &self.rules {
            let d = rule.check(call, &history_snapshot);
            if d.is_no_match() {
                continue;
            }
            final_decision = d;
            detail.matched_rule = Some(rule.name().to_string());
            detail.silent_on_reject = rule.silent_on_reject(call);
            detail.matched_command = rule.matched_command(call);
            break;
        }

        // 记录到 history (不论结果, 供 FrequencyRule 累计 + 审计留痕)
        let mut record = CallRecord::new(call, final_decision.clone(), detail.matched_rule.clone());
        record.matched_command = detail.matched_command.clone();
        record.silent_on_reject = detail.silent_on_reject;
        self.push_history(record);

        debug!(
            "[ApprovalManager] tool={}, decision={:?}, matched_rule={:?}, matched_command={:?}, silent={}",
            call.tool_name, final_decision, detail.matched_rule, detail.matched_command,
            detail.silent_on_reject
        );
        (final_decision, detail)
    }

    /// **核心方法 — 等主人审批 (5 分钟窗口), 返回结构化结果**
    ///
    /// **VCP 借鉴**: `toolApprovalManager.js getTimeoutMs` (5min) +
    /// `TOOL_APPROVAL_REASON_PROTOCOL.md` 结构化拒绝 `{rejected_by_user, error_type}`.
    ///
    /// **行为**:
    /// 1. `check_detailed` 拿决策 + 明细
    /// 2. `Allow` → `Approved`
    /// 3. `Deny` → `Rejected { rejected_by_user: false, error_type: PolicyDeny, silent }`
    /// 4. `RequireApproval` → 走审批通道 (高危操作"AI 请求 → 主人批准", 洋葱安全):
    ///    - 无 handler → `Rejected { ChannelUnavailable }`
    ///    - handler 批准 → `Approved`
    ///    - handler 拒绝 → `Rejected { rejected_by_user: true, reason: 主人理由 }`
    ///    - 超时 → `Rejected { ApprovalTimeout }` (VCP: 超时 = 拒绝)
    ///    - channel 取消 → `Rejected { ChannelUnavailable }`
    ///
    /// **静默拒绝**: 命中静默规则 (`::SilentReject`) 被拒时 `silent = true` —
    /// 不打扰 AI (上层不得把 `reason` 回传 AI), 但照常写入审计台账.
    ///
    /// **副作用**: 每个终态追加一条 `ApprovalAuditEntry` (留痕审计).
    pub async fn wait_for_approval_outcome(&self, call: &ParsedToolCall) -> ApprovalOutcome {
        let (decision, detail) = self.check_detailed(call);
        let outcome = match decision {
            ApprovalDecision::Allow => ApprovalOutcome::Approved {
                matched_rule: detail.matched_rule.clone(),
                matched_command: detail.matched_command.clone(),
            },
            ApprovalDecision::Deny { reason, silent } => {
                warn!(
                    "[ApprovalManager] 规则直接拒绝 tool={}, reason={}, silent={}",
                    call.tool_name, reason, silent
                );
                ApprovalOutcome::Rejected(Rejection {
                    rejected_by_user: false,
                    error_type: RejectErrorType::PolicyDeny,
                    silent,
                    reason: Some(reason),
                    matched_rule: detail.matched_rule.clone(),
                    matched_command: detail.matched_command.clone(),
                })
            }
            ApprovalDecision::RequireApproval { timeout_ms } => {
                let handler = match self.handler.lock().clone() {
                    Some(h) => h,
                    None => {
                        warn!(
                            "[ApprovalManager] 审批通道未注册 (无 handler), 拒绝: {}",
                            call.tool_name
                        );
                        let outcome = ApprovalOutcome::Rejected(Rejection {
                            rejected_by_user: false,
                            error_type: RejectErrorType::ChannelUnavailable,
                            silent: detail.silent_on_reject,
                            reason: None,
                            matched_rule: detail.matched_rule.clone(),
                            matched_command: detail.matched_command.clone(),
                        });
                        self.push_audit(call, &outcome);
                        return outcome;
                    }
                };
                // 用 oneshot 防 handler 内部死锁
                let (tx, rx) = oneshot::channel::<(bool, Option<String>)>();
                let call_clone = call.clone();
                tokio::spawn(async move {
                    let result = handler.handle_with_reason(&call_clone).await;
                    let _ = tx.send(result);
                });
                match timeout(Duration::from_millis(timeout_ms), rx).await {
                    Ok(Ok((approved, user_reason))) => {
                        if approved {
                            debug!("[ApprovalManager] 主人批准: {}", call.tool_name);
                            ApprovalOutcome::Approved {
                                matched_rule: detail.matched_rule.clone(),
                                matched_command: detail.matched_command.clone(),
                            }
                        } else {
                            warn!(
                                "[ApprovalManager] 主人拒绝: {} (silent={})",
                                call.tool_name, detail.silent_on_reject
                            );
                            ApprovalOutcome::Rejected(Rejection {
                                rejected_by_user: true,
                                error_type: RejectErrorType::RejectedByUser,
                                silent: detail.silent_on_reject,
                                // 静默时上层不得把理由回传 AI (此处仅承载供审计)
                                reason: user_reason,
                                matched_rule: detail.matched_rule.clone(),
                                matched_command: detail.matched_command.clone(),
                            })
                        }
                    }
                    Ok(Err(_canceled)) => {
                        warn!(
                            "[ApprovalManager] handler channel 取消, 拒绝: {}",
                            call.tool_name
                        );
                        ApprovalOutcome::Rejected(Rejection {
                            rejected_by_user: false,
                            error_type: RejectErrorType::ChannelUnavailable,
                            silent: detail.silent_on_reject,
                            reason: None,
                            matched_rule: detail.matched_rule.clone(),
                            matched_command: detail.matched_command.clone(),
                        })
                    }
                    Err(_elapsed) => {
                        warn!(
                            "[ApprovalManager] 审批超时 ({} ms), 拒绝: {}",
                            timeout_ms, call.tool_name
                        );
                        ApprovalOutcome::Rejected(Rejection {
                            rejected_by_user: false,
                            error_type: RejectErrorType::ApprovalTimeout,
                            silent: detail.silent_on_reject,
                            reason: None,
                            matched_rule: detail.matched_rule.clone(),
                            matched_command: detail.matched_command.clone(),
                        })
                    }
                }
            }
            ApprovalDecision::NoMatch => {
                // NoMatch 不应在 check 后出现, 但防御性处理 (与旧行为一致: Allow)
                warn!("[ApprovalManager] 收到 NoMatch (内部态泄漏), 默认 Allow");
                ApprovalOutcome::Approved {
                    matched_rule: None,
                    matched_command: None,
                }
            }
        };

        self.push_audit(call, &outcome);
        outcome
    }

    /// **核心方法 — 等主人审批 (5 分钟窗口), 布尔简化版**
    ///
    /// **VCP 借鉴**: `toolApprovalManager.js:231-233 getTimeoutMs()` = 5min
    ///
    /// 行为与旧版完全一致 (Allow→true / Deny→false / 超时→false / 无 handler→false),
    /// 内部委托 `wait_for_approval_outcome`. 需要结构化错误码
    /// (`{rejected_by_user, error_type}`) 时改用 `wait_for_approval_outcome`.
    ///
    /// **生产集成**: 实战中 Tauri 主进程 / SSE handler 注册 `ApprovalHandler`,
    /// 把请求 push 到前端, 主人点批准/拒绝, handler 返 bool.
    pub async fn wait_for_approval(&self, call: &ParsedToolCall) -> Result<bool, String> {
        Ok(self.wait_for_approval_outcome(call).await.is_approved())
    }

    /// 当前审计台账长度
    pub fn audit_len(&self) -> usize {
        self.audit.lock().len()
    }

    /// 审计台账快照 (克隆)
    pub fn audit_snapshot(&self) -> Vec<ApprovalAuditEntry> {
        self.audit.lock().iter().cloned().collect()
    }

    /// **静默拒绝审计视图** — 所有 `silent == true` 的拒绝条目
    ///
    /// 静默拒绝不打扰 AI / 主人, 但此视图保证"只留痕审计"可查.
    pub fn silent_rejection_audit(&self) -> Vec<ApprovalAuditEntry> {
        self.audit
            .lock()
            .iter()
            .filter(|e| !e.approved && e.silent)
            .cloned()
            .collect()
    }

    /// 追加一条审计记录 (内部, 自动裁剪到 `MAX_AUDIT_LEN`)
    fn push_audit(&self, call: &ParsedToolCall, outcome: &ApprovalOutcome) {
        let entry = match outcome {
            ApprovalOutcome::Approved {
                matched_rule,
                matched_command,
            } => ApprovalAuditEntry {
                id: format!("audit-{}", uuid::Uuid::new_v4()),
                tool_name: call.tool_name.clone(),
                approved: true,
                rejected_by_user: false,
                error_type: None,
                silent: false,
                matched_rule: matched_rule.clone(),
                matched_command: matched_command.clone(),
                timestamp_ms: crate::history::now_ms(),
            },
            ApprovalOutcome::Rejected(r) => ApprovalAuditEntry {
                id: format!("audit-{}", uuid::Uuid::new_v4()),
                tool_name: call.tool_name.clone(),
                approved: false,
                rejected_by_user: r.rejected_by_user,
                error_type: Some(r.error_type),
                silent: r.silent,
                matched_rule: r.matched_rule.clone(),
                matched_command: r.matched_command.clone(),
                timestamp_ms: crate::history::now_ms(),
            },
        };
        let mut a = self.audit.lock();
        a.push_back(entry);
        while a.len() > MAX_AUDIT_LEN {
            a.pop_front();
        }
    }

    /// 推一条历史 (内部, 自动按窗口大小裁剪)
    fn push_history(&self, record: CallRecord) {
        let mut h = self.history.lock();
        h.push_back(record);
        // 防御性裁剪: 历史最多 10_000 条 (实战中 LLM 长会话保护)
        const MAX_HISTORY: usize = 10_000;
        while h.len() > MAX_HISTORY {
            h.pop_front();
        }
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 单元测试 (ApprovalManager 5 规则组合 + 5min 窗口)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{BlacklistRule, FrequencyRule, RiskRule, TrustRule, WhitelistRule};
    use apeireth_tool_runtime::ParsedToolCall;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn make_call(tool: &str) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.to_string(),
            args: json!({}),
            raw_marker: format!("tool_name:<<<{tool}>>>"),
            archery: false,
            archery_no_reply: false,
        }
    }

    /// 测试用 approval handler: 按 counter 返 true (前 N 次) / false (之后)
    #[allow(dead_code)]
    struct ScriptedHandler {
        approved: AtomicU32,
    }
    #[allow(dead_code)]
    #[async_trait]
    impl ApprovalHandler for ScriptedHandler {
        async fn handle(&self, _call: &ParsedToolCall) -> bool {
            self.approved.fetch_sub(1, Ordering::SeqCst) > 0
        }
    }

    /// 测试用 delay handler: 延迟 N ms 后返 bool
    struct DelayedHandler {
        delay_ms: u64,
        approve: bool,
    }
    #[async_trait]
    impl ApprovalHandler for DelayedHandler {
        async fn handle(&self, _call: &ParsedToolCall) -> bool {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            self.approve
        }
    }

    // ====== ApprovalManager 5 规则组合 ======

    #[test]
    fn manager_with_5_rules_priority_blacklist_first() {
        // Blacklist > Trust > Risk > Frequency > Whitelist
        // 工具 "X" 同时在黑名单 + 白名单 + 信任 → Blacklist 胜 (最高优先级)
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(BlacklistRule::with_blacklist(
            ["X".to_string()],
            false,
        )));
        mgr.add_rule(Box::new(TrustRule::with_trusted(["X".to_string()])));
        mgr.add_rule(Box::new(RiskRule::new(300_000)));
        mgr.add_rule(Box::new(FrequencyRule::new()));
        mgr.add_rule(Box::new(WhitelistRule::with_whitelist(["X".to_string()])));

        let d = mgr.check(&make_call("X"));
        assert!(d.is_deny(), "黑名单应胜, 实际: {d:?}");
    }

    #[test]
    fn manager_trust_beats_whitelist() {
        // Trust 在 whitelist 前, 工具只 whitelist 也应 allow (whitelist 兜底)
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(TrustRule::new())); // 无信任
        mgr.add_rule(Box::new(WhitelistRule::with_whitelist([
            "Greeting".to_string()
        ])));

        let d = mgr.check(&make_call("Greeting"));
        assert!(d.is_allow(), "白名单工具应 allow, 实际: {d:?}");
    }

    #[test]
    fn manager_no_rules_default_allow() {
        // VCP 行为: 无 enabled config → 不需要审核 → allow
        let mgr = ApprovalManager::new();
        let d = mgr.check(&make_call("Anything"));
        assert!(d.is_allow(), "无规则默认 Allow, 实际: {d:?}");
    }

    #[test]
    fn manager_risk_rule_requires_5min_approval() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(RiskRule::new(APPROVAL_TIMEOUT_MS)));
        let d = mgr.check(&make_call("system.exec"));
        match d {
            ApprovalDecision::RequireApproval { timeout_ms } => {
                assert_eq!(timeout_ms, 300_000, "VCP 5min = 300_000 ms");
            }
            _ => panic!("应 RequireApproval, 实际: {d:?}"),
        }
    }

    #[test]
    fn manager_default_timeout_is_5min() {
        let mgr = ApprovalManager::new();
        assert_eq!(mgr.approval_timeout_ms(), 300_000);
        assert_eq!(APPROVAL_TIMEOUT_MS, 300_000, "VCP 5min 编译期守");
    }

    #[test]
    fn manager_custom_timeout() {
        let mgr = ApprovalManager::new().with_timeout(60_000);
        assert_eq!(mgr.approval_timeout_ms(), 60_000);
    }

    #[test]
    fn manager_history_records_every_call() {
        let mgr = ApprovalManager::new();
        assert_eq!(mgr.history_len(), 0);
        let _ = mgr.check(&make_call("A"));
        assert_eq!(mgr.history_len(), 1);
        let _ = mgr.check(&make_call("B"));
        assert_eq!(mgr.history_len(), 2);
    }

    // ====== 5 分钟审批窗口 (timeout) ======

    #[tokio::test]
    async fn manager_wait_for_approval_allow_skips_handler() {
        let mgr = ApprovalManager::new();
        // 无 RequireApproval 决策, wait_for_approval 应直接返 Ok(true)
        let r = mgr.wait_for_approval(&make_call("Anything")).await;
        assert_eq!(r, Ok(true), "Allow 决策应直接返 true, 不等 handler");
    }

    #[tokio::test]
    async fn manager_wait_for_approval_deny_skips_handler() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(BlacklistRule::with_blacklist(
            ["Bad".to_string()],
            false,
        )));
        // 即使 handler 是 AutoApprove, Deny 决策应直接返 Ok(false)
        mgr.set_handler(Arc::new(AutoApproveHandler));
        let r = mgr.wait_for_approval(&make_call("Bad")).await;
        assert_eq!(r, Ok(false), "Deny 决策应直接返 false, 不问 handler");
    }

    #[tokio::test]
    async fn manager_wait_for_approval_approved() {
        // RequireApproval + AutoApprove handler → true
        let mut mgr = ApprovalManager::new().with_timeout(2_000);
        mgr.add_rule(Box::new(RiskRule::new(2_000)));
        mgr.set_handler(Arc::new(AutoApproveHandler));
        let r = mgr.wait_for_approval(&make_call("system.exec")).await;
        assert_eq!(r, Ok(true));
    }

    #[tokio::test]
    async fn manager_wait_for_approval_timeout_returns_false() {
        // RequireApproval + handler 慢 (1s) + timeout 100ms → Ok(false)
        let mut mgr = ApprovalManager::new().with_timeout(100);
        mgr.add_rule(Box::new(RiskRule::new(100)));
        mgr.set_handler(Arc::new(DelayedHandler {
            delay_ms: 1_000,
            approve: true,
        }));
        let r = mgr.wait_for_approval(&make_call("system.exec")).await;
        assert_eq!(r, Ok(false), "超时应返 false (VCP 行为)");
    }

    #[tokio::test]
    async fn manager_no_handler_uses_default_deny() {
        // RequireApproval + 无 handler → DefaultDenyHandler → false
        let mut mgr = ApprovalManager::new().with_timeout(2_000);
        mgr.add_rule(Box::new(RiskRule::new(2_000)));
        // 不调 set_handler
        let r = mgr.wait_for_approval(&make_call("system.exec")).await;
        assert_eq!(r, Ok(false), "无 handler 应默认拒绝");
    }

    // ====== FrequencyRule 1min/3 边界 (ApprovalManager 集成) ======

    #[test]
    fn manager_frequency_triggers_after_3_calls() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(FrequencyRule::new())); // 1min/3
                                                      // 前 2 次应 NoMatch (Allow)
        let d1 = mgr.check(&make_call("Spam"));
        let d2 = mgr.check(&make_call("Spam"));
        assert!(d1.is_allow(), "第 1 次 Allow, 实际: {d1:?}");
        assert!(d2.is_allow(), "第 2 次 Allow, 实际: {d2:?}");
        // 第 3 次应触发 FrequencyRule (因为 history 已有 2 条)
        let d3 = mgr.check(&make_call("Spam"));
        assert!(d3.is_deny(), "第 3 次应 Deny (1min/3 反刷), 实际: {d3:?}");
    }

    #[test]
    fn manager_history_max_10k() {
        let mgr = ApprovalManager::new();
        for i in 0..10_005 {
            let _ = mgr.check(&make_call(&format!("T{i}")));
        }
        // 历史最多 10_000 条 (防无限增长)
        assert!(
            mgr.history_len() <= 10_000,
            "历史裁剪到 10_000, 实际: {}",
            mgr.history_len()
        );
    }

    #[test]
    fn manager_rule_count() {
        let mut mgr = ApprovalManager::new();
        assert_eq!(mgr.rule_count(), 0);
        mgr.add_rule(Box::new(TrustRule::new()));
        mgr.add_rule(Box::new(RiskRule::new(300_000)));
        mgr.add_rule(Box::new(FrequencyRule::new()));
        mgr.add_rule(Box::new(WhitelistRule::new()));
        mgr.add_rule(Box::new(BlacklistRule::new()));
        assert_eq!(mgr.rule_count(), 5);
    }

    #[test]
    fn manager_take_history_drains() {
        let mgr = ApprovalManager::new();
        let _ = mgr.check(&make_call("X"));
        let _ = mgr.check(&make_call("Y"));
        assert_eq!(mgr.history_len(), 2);
        let drained = mgr.take_history();
        assert_eq!(drained.len(), 2);
        assert_eq!(mgr.history_len(), 0, "take_history 应清空");
    }

    #[test]
    fn manager_clear_handler() {
        let mgr = ApprovalManager::new();
        mgr.set_handler(Arc::new(AutoApproveHandler));
        mgr.clear_handler();
        // 之后 wait_for_approval_outcome 返 ChannelUnavailable (无通道 = 拒绝)
        // 我们不直接测, 但验证 clear_handler 不 panic
    }

    // ====== 结构化结果 (toolApprovalManager 增强 P1) ======

    fn make_call_with_args(tool: &str, args: serde_json::Value) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.to_string(),
            args,
            raw_marker: format!("tool_name:<<<{tool}>>>"),
            archery: false,
            archery_no_reply: false,
        }
    }

    /// 拒绝 + 带主人理由的 handler (VCP reason 协议)
    struct RejectWithReasonHandler(String);
    #[async_trait]
    impl ApprovalHandler for RejectWithReasonHandler {
        async fn handle(&self, _call: &ParsedToolCall) -> bool {
            false
        }
        async fn handle_with_reason(&self, _call: &ParsedToolCall) -> (bool, Option<String>) {
            (false, Some(self.0.clone()))
        }
    }

    #[tokio::test]
    async fn outcome_approved_carries_matched_rule() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(TrustRule::with_trusted(["X".to_string()])));
        mgr.set_handler(Arc::new(AutoApproveHandler));
        let o = mgr.wait_for_approval_outcome(&make_call("X")).await;
        assert!(o.is_approved());
        if let ApprovalOutcome::Approved {
            matched_rule,
            matched_command,
        } = o
        {
            assert_eq!(matched_rule.as_deref(), Some("trust"));
            assert_eq!(matched_command, None);
        }
    }

    #[tokio::test]
    async fn outcome_user_rejection_structured_with_reason() {
        // 结构化拒绝: {rejected_by_user: true, error_type: rejected_by_user, reason}
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(RiskRule::new(300_000)));
        mgr.set_handler(Arc::new(RejectWithReasonHandler(
            "风险太高, 先列影响范围".to_string(),
        )));
        let o = mgr
            .wait_for_approval_outcome(&make_call("system.exec"))
            .await;
        let r = o.rejection().expect("应拒绝");
        assert!(r.rejected_by_user);
        assert_eq!(r.error_type, RejectErrorType::RejectedByUser);
        assert!(!r.silent);
        assert_eq!(r.reason.as_deref(), Some("风险太高, 先列影响范围"));
        assert_eq!(r.matched_rule.as_deref(), Some("risk"));
    }

    #[tokio::test]
    async fn outcome_policy_deny_from_blacklist() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(BlacklistRule::with_blacklist(
            ["Bad".to_string()],
            false,
        )));
        mgr.set_handler(Arc::new(AutoApproveHandler)); // 不应走到通道
        let o = mgr.wait_for_approval_outcome(&make_call("Bad")).await;
        let r = o.rejection().expect("黑名单应拒");
        assert!(!r.rejected_by_user, "规则拒绝不是主人拒绝");
        assert_eq!(r.error_type, RejectErrorType::PolicyDeny);
        assert!(!r.silent);
        assert!(r.reason.as_ref().unwrap().contains("Bad"));

        // 静默黑名单 → PolicyDeny + silent=true
        let mut mgr2 = ApprovalManager::new();
        mgr2.add_rule(Box::new(BlacklistRule::with_blacklist(
            ["Bad".to_string()],
            true,
        )));
        let o2 = mgr2.wait_for_approval_outcome(&make_call("Bad")).await;
        let r2 = o2.rejection().expect("静默黑名单应拒");
        assert!(r2.silent);
        assert_eq!(r2.error_type, RejectErrorType::PolicyDeny);
    }

    #[tokio::test]
    async fn outcome_timeout_is_structured() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(RiskRule::new(20))); // 20ms 极短窗口
        mgr.set_handler(Arc::new(DelayedHandler {
            delay_ms: 200,
            approve: true,
        }));
        let o = mgr
            .wait_for_approval_outcome(&make_call("system.exec"))
            .await;
        let r = o.rejection().expect("超时应拒");
        assert!(!r.rejected_by_user, "超时不是主人拒绝");
        assert_eq!(r.error_type, RejectErrorType::ApprovalTimeout);
    }

    #[tokio::test]
    async fn outcome_no_handler_is_channel_unavailable() {
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(RiskRule::new(300_000)));
        // 不注册 handler
        let o = mgr
            .wait_for_approval_outcome(&make_call("system.exec"))
            .await;
        let r = o.rejection().expect("无通道应拒");
        assert!(!r.rejected_by_user);
        assert_eq!(r.error_type, RejectErrorType::ChannelUnavailable);
        // 旧布尔接口行为不变: 无 handler = Ok(false)
        let b = mgr.wait_for_approval(&make_call("system.exec")).await;
        assert_eq!(b, Ok(false));
    }

    #[tokio::test]
    async fn outcome_silent_rejection_via_command_level_rule() {
        // 命令级静默条目: 主人拒绝 → silent=true (不打扰 AI), 审计留痕
        use crate::rule::ApprovalListRule;
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(ApprovalListRule::with_entries(
            ["Shell:reboot::SilentReject".to_string()],
            300_000,
        )));
        mgr.set_handler(Arc::new(RejectWithReasonHandler("不批".to_string())));

        let call = make_call_with_args("Shell", json!({"command": "reboot"}));
        let o = mgr.wait_for_approval_outcome(&call).await;
        assert!(o.is_silent_rejection());
        let r = o.rejection().unwrap();
        assert!(r.rejected_by_user);
        assert!(r.silent, "::SilentReject 条目 → 静默拒绝");
        assert_eq!(r.matched_command.as_deref(), Some("reboot"));
        assert_eq!(r.matched_rule.as_deref(), Some("approval_list"));
        assert_eq!(r.error_type, RejectErrorType::RejectedByUser);

        // 审计视图能查到这条静默拒绝 (留痕)
        let silent_audit = mgr.silent_rejection_audit();
        assert_eq!(silent_audit.len(), 1);
        assert_eq!(silent_audit[0].tool_name, "Shell");
        assert_eq!(silent_audit[0].matched_command.as_deref(), Some("reboot"));
        assert!(silent_audit[0].rejected_by_user);
    }

    #[tokio::test]
    async fn outcome_non_silent_command_level_rejection_not_in_silent_audit() {
        use crate::rule::ApprovalListRule;
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(ApprovalListRule::with_entries(
            ["Shell:reboot".to_string()],
            300_000,
        )));
        mgr.set_handler(Arc::new(RejectWithReasonHandler("不批".to_string())));
        let call = make_call_with_args("Shell", json!({"command": "reboot"}));
        let o = mgr.wait_for_approval_outcome(&call).await;
        assert!(o.is_rejected());
        assert!(!o.is_silent_rejection());
        assert!(mgr.silent_rejection_audit().is_empty());
        // 但普通审计台账有记录
        assert_eq!(mgr.audit_len(), 1);
    }

    #[tokio::test]
    async fn check_detailed_returns_command_and_silent() {
        use crate::rule::ApprovalListRule;
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(ApprovalListRule::with_entries(
            ["Shell:reboot::SilentReject".to_string()],
            300_000,
        )));
        let call = make_call_with_args("Shell", json!({"command": "reboot"}));
        let (decision, detail) = mgr.check_detailed(&call);
        assert!(decision.is_require_approval());
        assert_eq!(detail.matched_rule.as_deref(), Some("approval_list"));
        assert_eq!(detail.matched_command.as_deref(), Some("reboot"));
        assert!(detail.silent_on_reject);

        // history 记录也带上命令级明细 (留痕)
        let h = mgr.snapshot_history();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].matched_command.as_deref(), Some("reboot"));
        assert!(h[0].silent_on_reject);

        // 普通 check 签名不变 (向后兼容)
        let d2 = mgr.check(&make_call("NoRule"));
        assert!(d2.is_allow(), "无命中默认 Allow (VCP 行为)");
    }

    #[tokio::test]
    async fn wait_for_approval_bool_backcompat() {
        // 旧接口逐路径回归: Allow→true / Deny→false / 批准→true / 拒绝→false
        let mut mgr = ApprovalManager::new();
        mgr.add_rule(Box::new(BlacklistRule::with_blacklist(
            ["Bad".to_string()],
            false,
        )));
        mgr.add_rule(Box::new(RiskRule::new(300_000)));
        mgr.set_handler(Arc::new(AutoApproveHandler));

        assert_eq!(mgr.wait_for_approval(&make_call("Safe")).await, Ok(true));
        assert_eq!(mgr.wait_for_approval(&make_call("Bad")).await, Ok(false));
        assert_eq!(
            mgr.wait_for_approval(&make_call("system.exec")).await,
            Ok(true)
        );
    }

    #[tokio::test]
    async fn audit_capped_at_max_len() {
        let mut mgr = ApprovalManager::new();
        mgr.set_handler(Arc::new(AutoApproveHandler));
        for i in 0..(MAX_AUDIT_LEN + 5) {
            let _ = mgr
                .wait_for_approval_outcome(&make_call(&format!("T{i}")))
                .await;
        }
        assert!(
            mgr.audit_len() <= MAX_AUDIT_LEN,
            "审计裁剪到 {MAX_AUDIT_LEN}, 实际: {}",
            mgr.audit_len()
        );
    }
}
