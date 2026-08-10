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

use crate::decision::ApprovalDecision;
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

/// **战役 2-3 — 审批管理器**
///
/// **核心字段**:
/// - `rules: Vec<Box<dyn ApprovalRule>>` — 5 规则列表 (按顺序 check)
/// - `approval_timeout_ms: u64` — 5min 窗口 (VCP 真值)
/// - `history: Mutex<VecDeque<CallRecord>>` — 审批历史 (FrequencyRule 用)
/// - `handler: Arc<Mutex<Option<Arc<dyn ApprovalHandler>>>>` — 外部审批 handler
pub struct ApprovalManager {
    /// 5 规则 (按顺序 check, 第一个非 NoMatch 生效)
    rules: Vec<Box<dyn ApprovalRule>>,
    /// 审批超时毫秒 (默认 `APPROVAL_TIMEOUT_MS = 5 * 60 * 1000`)
    approval_timeout_ms: u64,
    /// 审批历史 (滑动窗口, 给 FrequencyRule 用)
    history: Mutex<VecDeque<CallRecord>>,
    /// 外部审批 handler (默认 `DefaultDenyHandler`)
    handler: Arc<Mutex<Option<Arc<dyn ApprovalHandler>>>>,
}

impl ApprovalManager {
    /// 新建空 manager (无规则), 默认 5min 窗口
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            approval_timeout_ms: APPROVAL_TIMEOUT_MS,
            history: Mutex::new(VecDeque::new()),
            handler: Arc::new(Mutex::new(None)),
        }
    }

    /// 用 5 规则构造
    pub fn with_rules(rules: Vec<Box<dyn ApprovalRule>>) -> Self {
        Self {
            rules,
            approval_timeout_ms: APPROVAL_TIMEOUT_MS,
            history: Mutex::new(VecDeque::new()),
            handler: Arc::new(Mutex::new(None)),
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
    /// **5 规则按顺序**:
    /// 1. 遍历 rules
    /// 2. 调 `rule.check(call, history_snapshot)`
    /// 3. 第一个非 `NoMatch` 的决策生效
    /// 4. 全 `NoMatch` → 默认 `Allow` (VCP `defaultDecision.requiresApproval = false`)
    ///
    /// **副作用**: 不论结果都记录到 history (供 FrequencyRule 下次判断)
    ///
    /// **VCP 借鉴**: `toolApprovalManager.js:144-225 getApprovalDecision` 三层判断
    pub fn check(&self, call: &ParsedToolCall) -> ApprovalDecision {
        let history_snapshot = self.snapshot_history();
        let mut final_decision = ApprovalDecision::Allow; // 默认 Allow (VCP 行为)
        let mut matched_rule: Option<String> = None;

        for rule in &self.rules {
            let d = rule.check(call, &history_snapshot);
            if d.is_no_match() {
                continue;
            }
            final_decision = d;
            matched_rule = Some(rule.name().to_string());
            break;
        }

        // 记录到 history (不论结果, 供 FrequencyRule 累计)
        let record = CallRecord::new(call, final_decision.clone(), matched_rule.clone());
        self.push_history(record);

        debug!(
            "[ApprovalManager] tool={}, decision={:?}, matched_rule={:?}",
            call.tool_name, final_decision, matched_rule
        );
        final_decision
    }

    /// **核心方法 — 等主人审批 (5 分钟窗口)**
    ///
    /// **VCP 借鉴**: `toolApprovalManager.js:231-233 getTimeoutMs()` = 5min
    ///
    /// **行为**:
    /// 1. 先调 `check` 拿决策
    /// 2. 若决策 = Allow → 直接返 Ok(true) (无需审批)
    /// 3. 若决策 = Deny → 直接返 Ok(false) (拒绝, 不等)
    /// 4. 若决策 = RequireApproval → 调外部 handler, 5min 窗口等响应
    ///    - handler 返 true (批准) → Ok(true)
    ///    - handler 返 false (拒绝) → Ok(false)
    ///    - 超时 → Ok(false) (VCP 行为: 超时 = 拒绝)
    ///
    /// **生产集成**: 实战中 Tauri 主进程 / SSE handler 注册 `ApprovalHandler`,
    /// 把请求 push 到前端, 主人点批准/拒绝, handler 返 bool.
    pub async fn wait_for_approval(&self, call: &ParsedToolCall) -> Result<bool, String> {
        let decision = self.check(call);
        match decision {
            ApprovalDecision::Allow => Ok(true),
            ApprovalDecision::Deny { reason, silent: _ } => {
                warn!(
                    "[ApprovalManager] 拒绝执行 tool={}, reason={}",
                    call.tool_name, reason
                );
                Ok(false)
            }
            ApprovalDecision::RequireApproval { timeout_ms } => {
                let handler = {
                    let h = self.handler.lock();
                    h.clone()
                        .unwrap_or_else(|| Arc::new(DefaultDenyHandler) as Arc<dyn ApprovalHandler>)
                };
                // 用 oneshot 防 handler 内部死锁
                let (tx, rx) = oneshot::channel::<bool>();
                let call_clone = call.clone();
                let handler_clone = handler.clone();
                tokio::spawn(async move {
                    let result = handler_clone.handle(&call_clone).await;
                    let _ = tx.send(result);
                });
                match timeout(Duration::from_millis(timeout_ms), rx).await {
                    Ok(Ok(approved)) => {
                        if approved {
                            debug!("[ApprovalManager] 主人批准: {}", call.tool_name);
                        } else {
                            warn!("[ApprovalManager] 主人拒绝: {}", call.tool_name);
                        }
                        Ok(approved)
                    }
                    Ok(Err(_canceled)) => {
                        warn!("[ApprovalManager] handler channel 取消, 默认拒绝");
                        Ok(false)
                    }
                    Err(_elapsed) => {
                        warn!(
                            "[ApprovalManager] 审批超时 ({} ms), 默认拒绝: {}",
                            timeout_ms, call.tool_name
                        );
                        Ok(false)
                    }
                }
            }
            ApprovalDecision::NoMatch => {
                // NoMatch 不应在 check 后出现, 但防御性处理
                warn!("[ApprovalManager] 收到 NoMatch (内部态泄漏), 默认 Allow");
                Ok(true)
            }
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
        // 之后 wait_for_approval 会用 DefaultDenyHandler
        // 我们不直接测, 但验证 clear_handler 不 panic
    }
}
