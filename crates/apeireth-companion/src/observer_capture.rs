//! `apeireth-companion::observer_capture` — TP22: 工具执行结果即时沉淀候选.
//!
//! 主人洞察 (2026-08-18, W5): 「工具执行成功/失败本身就是最强记忆信号: 这个方法有效
//! 不该等反思周期, 执行完即沉淀。」
//!
//! 设计:
//! - 工具执行完成 (成功/失败两条路径) → `PostExecuteHook` 链触发一次 `ObserverCaptureHook`
//! - 候选条目 = `ExperienceCandidate { tool, args_hash, outcome, ts_ms, source }`,
//!   **不直接 put_episode**, 仅入候选队列 (`ExperienceQueue`).
//! - 后续 reflection / 对账周期再 promote 到正式记忆库 (与 E1 反思期闭环衔接).
//! - 去重: 同 `(tool, args_hash)` 在 24h 内不重复沉淀;
//!   in-memory LRU (O(1) 命中) + sqlite 持久化 (跨重启仍去重).
//!
//! 0 假装:
//! - LRU 容量 1024, 超出会淘汰最旧条目 (sqlite 仍存; 重启后 sqlite 查询兜底).
//! - args_hash 用 `serde_json::to_string` + sha256 (JSON 字段顺序在 `Map` 默认按 key 字母,
//!   但允许调用方先转 `Value` 再 to_string, 此处假定上游 `args: Value` 已是稳定形态).
//! - SQLite 表 schema 不增列, 复用 `episodes` 表 (id 前缀 `expc-`, content=JSON).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::executor::ExecutionResult;
use apeireth_tool_runtime::parser::ParsedToolCall;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::tool_bridge::PostExecuteHook;

/// 候选经验来源 (未来可扩: Dialog / Reflection / ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceSource {
    /// 工具执行 hook 捕获.
    ToolExecution,
}

/// 工具执行结果摘要 (产物 → 沉淀信号).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Outcome {
    /// 成功 + 产物摘要 (限长 200 字, 防止超大输出污染候选队列).
    Success { summary: Option<String> },
    /// 失败 + 错误摘要 (限长 200 字).
    Failure { error: Option<String> },
}

impl Outcome {
    /// 从 `ExecutionResult` 派生 (供 ObserverCaptureHook 使用).
    /// ponytail: 摘要从 output/error 截前 200 字; 既不假装全量, 也不丢信号.
    pub fn from_result(r: &ExecutionResult) -> Self {
        const MAX_SUMMARY: usize = 200;
        let truncate = |s: &str| -> String {
            let mut out: String = s.chars().take(MAX_SUMMARY).collect();
            if s.chars().count() > MAX_SUMMARY {
                out.push('…');
            }
            out
        };
        if r.success {
            Outcome::Success {
                summary: Some(match &r.output {
                    Value::Null => "ok (null output)".to_string(),
                    Value::String(s) => truncate(s),
                    other => truncate(&other.to_string()),
                }),
            }
        } else {
            Outcome::Failure {
                error: Some(truncate(r.error.as_deref().unwrap_or("unknown error"))),
            }
        }
    }

    /// 三态语义标签 (与 `experience::Experience.outcome` 对齐: success/failure/partial).
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Success { .. } => "success",
            Outcome::Failure { .. } => "failure",
        }
    }
}

/// 一条候选经验 (待 reflection 周期 promote).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceCandidate {
    pub tool: String,
    /// args 规范化 hash (sha256 hex, 16 字符 — 折中: 64 浪费/16 碰撞概率 2^-64 仍极低).
    pub args_hash: String,
    pub outcome: Outcome,
    pub ts_ms: i64,
    pub source: ExperienceSource,
}

impl ExperienceCandidate {
    fn id() -> String {
        format!("expc-{}", uuid::Uuid::new_v4())
    }
}

/// 候选条目 id 前缀 (复用 episodes 表, 无新迁移).
pub const CANDIDATE_ID_PREFIX: &str = "expc-";

/// 默认去重窗口 (24 小时, ms).
pub const DEFAULT_DEDUP_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

/// 默认 LRU 容量 (in-memory 索引上限, 防止无界增长).
pub const DEFAULT_LRU_CAP: usize = 1024;

/// 候选队列: in-memory LRU + 可选 sqlite 持久化.
///
/// 线程安全 (`Mutex` 内层; 内部用 `parking_lot`-like std 即可).
/// ponytail: 用 std `Mutex` (与现有 experience/store 同风格), 不引第三方.
pub struct ExperienceQueue {
    inner: Mutex<Inner>,
    store: Option<Arc<SqliteMemoryStore>>,
    window_ms: i64,
}

/// 候选队列配置 (builder 模式注入; 默认值见 `DEFAULT_*`).
#[derive(Debug, Clone)]
pub struct ExperienceQueueConfig {
    pub window_ms: i64,
    pub lru_cap: usize,
}

impl Default for ExperienceQueueConfig {
    fn default() -> Self {
        Self {
            window_ms: DEFAULT_DEDUP_WINDOW_MS,
            lru_cap: DEFAULT_LRU_CAP,
        }
    }
}

#[derive(Debug)]
struct Inner {
    /// (tool, args_hash) → 最近 push 时间 (ms). O(1) 命中.
    lru: HashMap<(String, String), i64>,
    /// 等待 reflection 周期消费的候选 (FIFO).
    pending: Vec<ExperienceCandidate>,
    /// 插入顺序 (用于 LRU 淘汰).
    order: VecDeque<(String, String)>,
    lru_cap: usize,
}

impl ExperienceQueue {
    /// 新建空队列 (无 sqlite 持久化, 进程内去重).
    pub fn new() -> Self {
        Self::with_config(None, ExperienceQueueConfig::default())
    }

    /// 新建队列 + sqlite 持久化 (跨重启仍 24h 去重).
    pub fn with_store(store: Arc<SqliteMemoryStore>) -> Self {
        Self::with_config(Some(store), ExperienceQueueConfig::default())
    }

    /// 全配置入口.
    pub fn with_config(store: Option<Arc<SqliteMemoryStore>>, cfg: ExperienceQueueConfig) -> Self {
        let mut s = Self {
            inner: Mutex::new(Inner {
                lru: HashMap::new(),
                pending: Vec::new(),
                order: VecDeque::new(),
                lru_cap: cfg.lru_cap,
            }),
            store,
            window_ms: cfg.window_ms,
        };
        // 启动时: 从 sqlite 复活窗口内的 LRU 索引 + 候选池 (跨重启去重).
        if let Err(e) = s.rehydrate() {
            eprintln!("[observer_capture] rehydrate 失败 (不阻断): {e}");
        }
        s
    }

    /// 推一条候选. 返回 `true` = 入队, `false` = 24h 内重复 (去重命中).
    pub fn push(&self, candidate: ExperienceCandidate) -> bool {
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.push_at(candidate, now_ms)
    }

    /// 时间注入版 (测试用). 同 `push` 语义, 但 `now_ms` 由调用方提供.
    pub fn push_at(&self, candidate: ExperienceCandidate, now_ms: i64) -> bool {
        let key = (candidate.tool.clone(), candidate.args_hash.clone());

        // 1) in-memory LRU 命中检查 (O(1))
        {
            let inner = self.inner.lock().expect("queue mutex poisoned");
            if let Some(&prev_ts) = inner.lru.get(&key) {
                if now_ms - prev_ts < self.window_ms {
                    return false;
                }
            }
        }

        // 2) sqlite 兜底: LRU 漏掉 (被淘汰或首次启动) → 查 sqlite 验证
        if let Some(store) = &self.store {
            if self.sqlite_recent_within_window(store, &key.0, &key.1, now_ms) {
                // 同步回 LRU
                self.update_lru(key.clone(), now_ms);
                return false;
            }
        }

        // 3) 入队 + 更新 LRU + 写 sqlite
        {
            let mut inner = self.inner.lock().expect("queue mutex poisoned");
            inner.pending.push(candidate.clone());
            // LRU 容量上限: 淘汰最旧
            if inner.order.len() >= inner.lru_cap {
                if let Some(evicted) = inner.order.pop_front() {
                    inner.lru.remove(&evicted);
                }
            }
            inner.order.push_back(key.clone());
            inner.lru.insert(key, now_ms);
        }
        if let Some(store) = &self.store {
            if let Err(e) = self.sqlite_insert(store, &candidate) {
                eprintln!("[observer_capture] sqlite 写入失败 (不阻断): {e}");
            }
        }
        true
    }

    /// 当前候选池长度.
    pub fn len(&self) -> usize {
        self.inner.lock().expect("queue mutex poisoned").pending.len()
    }

    /// 队列是否为空.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 排空候选 (reflection 周期消费).
    pub fn drain_pending(&self) -> Vec<ExperienceCandidate> {
        let mut inner = self.inner.lock().expect("queue mutex poisoned");
        std::mem::take(&mut inner.pending)
    }

    /// 窥视当前候选数 (不入队/不移除).
    pub fn peek_len(&self) -> usize {
        self.len()
    }

    /// 在 sqlite 上检查 `expc-{tool}-hash` 是否在窗口内有记录.
    /// ponytail: 用 `recent_episodes` 拉一批 + 过滤, 避免再加专用索引 (队列规模小, N≤1024/window).
    fn sqlite_recent_within_window(
        &self,
        store: &SqliteMemoryStore,
        tool: &str,
        args_hash: &str,
        now_ms: i64,
    ) -> bool {
        let since_ms = now_ms - self.window_ms;
        let eps = match store.recent_episodes("me", 4096) {
            Ok(eps) => eps,
            Err(_) => return false,
        };
        eps.iter()
            .filter(|e| e.id.starts_with(CANDIDATE_ID_PREFIX))
            .filter(|e| e.timestamp >= since_ms)
            .any(|e| match serde_json::from_str::<ExperienceCandidate>(&e.content) {
                Ok(c) => c.tool == tool && c.args_hash == args_hash,
                Err(_) => false,
            })
    }

    /// 把候选写入 sqlite (content=JSON, 复用 episodes 表).
    fn sqlite_insert(
        &self,
        store: &SqliteMemoryStore,
        candidate: &ExperienceCandidate,
    ) -> Result<(), String> {
        let content = serde_json::to_string(candidate).map_err(|e| format!("序列化候选失败: {e}"))?;
        let ep = CoreEpisode {
            id: Self::candidate_id(),
            timestamp: candidate.ts_ms / 1000, // CoreEpisode 用秒
            role: "assistant".into(),
            content,
            session_id: "me".into(),
        };
        store.put_episode(&ep).map_err(|e| e.to_string())
    }

    fn candidate_id() -> String {
        ExperienceCandidate::id()
    }

    /// 启动时: 从 sqlite 复活窗口内的 LRU 索引 + 候选池.
    fn rehydrate(&self) -> Result<(), String> {
        let store = match &self.store {
            Some(s) => s.clone(),
            None => return Ok(()),
        };
        let now_ms = chrono::Utc::now().timestamp_millis();
        let since_ms = now_ms - self.window_ms;
        let eps = store
            .recent_episodes("me", 8192)
            .map_err(|e| format!("rehydrate recent: {e}"))?;
        let mut inner = self.inner.lock().expect("queue mutex poisoned");
        let mut restored: Vec<ExperienceCandidate> = Vec::new();
        for e in eps
            .iter()
            .filter(|e| e.id.starts_with(CANDIDATE_ID_PREFIX))
            .filter(|e| e.timestamp * 1000 >= since_ms)
        {
            if let Ok(c) = serde_json::from_str::<ExperienceCandidate>(&e.content) {
                let key = (c.tool.clone(), c.args_hash.clone());
                // LRU 复活 (按 ts_ms 取最新)
                let ts_ms = c.ts_ms;
                match inner.lru.get(&key) {
                    Some(prev) if *prev >= ts_ms => {}
                    _ => {
                        inner.lru.insert(key.clone(), ts_ms);
                        inner.order.push_back(key);
                        if inner.order.len() > inner.lru_cap {
                            if let Some(ev) = inner.order.pop_front() {
                                inner.lru.remove(&ev);
                            }
                        }
                        restored.push(c);
                    }
                }
            }
        }
        inner.pending.extend(restored);
        Ok(())
    }

    fn update_lru(&self, key: (String, String), now_ms: i64) {
        let mut inner = self.inner.lock().expect("queue mutex poisoned");
        inner.lru.insert(key.clone(), now_ms);
        inner.order.push_back(key);
        if inner.order.len() > inner.lru_cap {
            if let Some(ev) = inner.order.pop_front() {
                inner.lru.remove(&ev);
            }
        }
    }
}

impl Default for ExperienceQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 计算 args 的稳定 hash (sha256 hex 前 16 字符).
pub fn args_hash(args: &Value) -> String {
    let canonical = serde_json::to_string(args).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(canonical.as_bytes());
    let digest = h.finalize();
    let hex = digest
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    hex
}

/// Observer 捕获 hook: 工具执行完即推一条候选.
///
/// 不修改 `ExecutionResult` (纯旁路, 失败仅 eprintln 不抛错).
/// ponytail: 用 `parking_lot`-like std Mutex (与队列同风格), eprintln 不阻断主线.
pub struct ObserverCaptureHook {
    queue: Arc<ExperienceQueue>,
}

impl ObserverCaptureHook {
    pub fn new(queue: Arc<ExperienceQueue>) -> Self {
        Self { queue }
    }
}

impl PostExecuteHook for ObserverCaptureHook {
    fn apply(&self, call: &ParsedToolCall, result: &ExecutionResult) -> ExecutionResult {
        let candidate = ExperienceCandidate {
            tool: call.tool_name.clone(),
            args_hash: args_hash(&call.args),
            outcome: Outcome::from_result(result),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            source: ExperienceSource::ToolExecution,
        };
        // 旁路推: 失败/重复均不阻断工具结果.
        let _ = self.queue.push(candidate);
        result.clone()
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_runtime::executor::ExecutionResult;
    use apeireth_tool_runtime::parser::ParsedToolCall;
    use serde_json::json;

    fn mk_call(tool: &str, args: Value) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.into(),
            args,
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        }
    }

    fn mk_ok(tool: &str, output: Value) -> ExecutionResult {
        ExecutionResult {
            tool_name: tool.into(),
            success: true,
            output,
            error: None,
            duration_ms: 1,
            ..Default::default()
        }
    }

    fn mk_err(tool: &str, msg: &str) -> ExecutionResult {
        ExecutionResult {
            tool_name: tool.into(),
            success: false,
            output: json!(null),
            error: Some(msg.into()),
            duration_ms: 1,
            ..Default::default()
        }
    }

    #[test]
    fn args_hash_is_stable_and_distinguishes_args() {
        // 同 args → 同 hash (跨调用稳定)
        assert_eq!(args_hash(&json!({"a": 1, "b": 2})), args_hash(&json!({"a": 1, "b": 2})));
        // 不同 args → 不同 hash
        assert_ne!(args_hash(&json!({"a": 1})), args_hash(&json!({"a": 2})));
        // hash 长度 = 16 字符
        assert_eq!(args_hash(&json!({})).len(), 16);
    }

    #[test]
    fn outcome_from_result_truncates_long_strings() {
        let big = "x".repeat(1000);
        let r = mk_ok("t", Value::String(big.clone()));
        let o = Outcome::from_result(&r);
        match o {
            Outcome::Success { summary } => {
                let s = summary.unwrap();
                assert!(s.chars().count() <= 201, "应 ≤ 200 字 + 省略号: {}", s.chars().count());
                assert!(s.ends_with('…'));
            }
            _ => panic!("期望 Success"),
        }
        // 失败路径
        let r2 = mk_err("t", &big);
        match Outcome::from_result(&r2) {
            Outcome::Failure { error } => {
                let e = error.unwrap();
                assert!(e.ends_with('…'));
            }
            _ => panic!("期望 Failure"),
        }
    }

    #[test]
    fn hook_fires_on_success_path() {
        let q = Arc::new(ExperienceQueue::new());
        let hook = ObserverCaptureHook::new(q.clone());
        let call = mk_call("recall_memory", json!({"query": "考试"}));
        let r = mk_ok("recall_memory", json!({"found": 3}));
        // hook 不改 result (旁路)
        let out = hook.apply(&call, &r);
        assert_eq!(out.success, true);
        assert_eq!(out.output["found"], json!(3));
        assert_eq!(q.len(), 1, "候选应入队");
        let drained = q.drain_pending();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].tool, "recall_memory");
        assert_eq!(drained[0].source, ExperienceSource::ToolExecution);
        match &drained[0].outcome {
            Outcome::Success { summary } => assert!(summary.is_some()),
            _ => panic!("期望 Success"),
        }
    }

    #[test]
    fn hook_fires_on_failure_path() {
        let q = Arc::new(ExperienceQueue::new());
        let hook = ObserverCaptureHook::new(q.clone());
        let call = mk_call("FileOperator", json!({"op": "read", "path": "/x"}));
        let r = mk_err("FileOperator", "权限拒绝");
        let out = hook.apply(&call, &r);
        assert!(!out.success, "hook 不应改 success");
        assert_eq!(out.error.as_deref(), Some("权限拒绝"));
        assert_eq!(q.len(), 1);
        let drained = q.drain_pending();
        match &drained[0].outcome {
            Outcome::Failure { error } => assert_eq!(error.as_deref(), Some("权限拒绝")),
            _ => panic!("期望 Failure"),
        }
    }

    #[test]
    fn dedup_within_window_suppresses_duplicate() {
        let q = Arc::new(ExperienceQueue::new());
        let hook = ObserverCaptureHook::new(q.clone());
        let call = mk_call("t", json!({"x": 1}));
        let r = mk_ok("t", json!({}));
        // 1) 首次入队
        assert_eq!(q.len(), 0);
        hook.apply(&call, &r);
        assert_eq!(q.len(), 1, "首次入队");
        // 2) 同 (tool, args_hash) 24h 内重复 → 被去重
        hook.apply(&call, &r);
        assert_eq!(q.len(), 1, "同 hash 重复应去重, 队列仍 1 条");
    }

    #[test]
    fn dedup_allows_after_window_expires() {
        // 直接用 push_at 注入时间
        let q = Arc::new(ExperienceQueue::new());
        let c1 = ExperienceCandidate {
            tool: "t".into(),
            args_hash: "h".into(),
            outcome: Outcome::Success { summary: Some("a".into()) },
            ts_ms: 1_000_000,
            source: ExperienceSource::ToolExecution,
        };
        let c2 = c1.clone();
        assert!(q.push_at(c1, 1_000_000), "首次入队");
        // 24h 内重复 → 去重
        let c3 = ExperienceCandidate {
            tool: "t".into(),
            args_hash: "h".into(),
            outcome: Outcome::Success { summary: Some("b".into()) },
            ts_ms: 1_000_000 + 1000,
            source: ExperienceSource::ToolExecution,
        };
        assert!(!q.push_at(c3, 1_000_000 + 1000), "24h 内重复");
        // 24h 后 → 允许再次入队
        let c4 = ExperienceCandidate {
            tool: "t".into(),
            args_hash: "h".into(),
            outcome: Outcome::Success { summary: Some("c".into()) },
            ts_ms: 1_000_000 + 24 * 60 * 60 * 1000 + 1,
            source: ExperienceSource::ToolExecution,
        };
        assert!(
            q.push_at(c4, 1_000_000 + 24 * 60 * 60 * 1000 + 1),
            "24h 后应允许"
        );
        let _ = c2; // 抑制未用警告
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn different_args_hash_not_deduped() {
        let q = Arc::new(ExperienceQueue::new());
        let hook = ObserverCaptureHook::new(q.clone());
        let r = mk_ok("t", json!({}));
        hook.apply(&mk_call("t", json!({"x": 1})), &r);
        hook.apply(&mk_call("t", json!({"x": 2})), &r);
        assert_eq!(q.len(), 2, "不同 args 应分别入队");
    }

    #[test]
    fn sqlite_persistence_survives_rehydrate() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let q1 = Arc::new(ExperienceQueue::with_store(Arc::clone(&store)));
        let hook = ObserverCaptureHook::new(q1.clone());
        hook.apply(
            &mk_call("t", json!({"x": 1})),
            &mk_ok("t", json!({"v": 1})),
        );
        hook.apply(
            &mk_call("t", json!({"x": 2})),
            &mk_ok("t", json!({"v": 2})),
        );
        assert_eq!(q1.len(), 2);
        // 模拟进程重启: 新建 q2 指向同一 sqlite, rehydrate 应复活
        let q2 = Arc::new(ExperienceQueue::with_store(Arc::clone(&store)));
        assert_eq!(q2.len(), 2, "重启后应复活 2 条候选");
        // 重启后再次执行同 (tool, args_hash) → 应被 sqlite 去重
        let hook2 = ObserverCaptureHook::new(q2.clone());
        hook2.apply(
            &mk_call("t", json!({"x": 1})),
            &mk_ok("t", json!({"v": 999})),
        );
        assert_eq!(
            q2.len(),
            2,
            "重启后同 hash 重复应被 sqlite 兜底去重 (不增加)"
        );
    }

    #[test]
    fn drain_pending_clears_queue() {
        let q = Arc::new(ExperienceQueue::new());
        let hook = ObserverCaptureHook::new(q.clone());
        for i in 0..3 {
            hook.apply(&mk_call("t", json!({"i": i})), &mk_ok("t", json!({})));
        }
        assert_eq!(q.len(), 3);
        let drained = q.drain_pending();
        assert_eq!(drained.len(), 3);
        assert!(q.is_empty(), "drain 后应清空");
    }

    #[test]
    fn lru_cap_evicts_oldest() {
        let cfg = ExperienceQueueConfig {
            window_ms: 1_000_000, // 长窗口, 不触发时间去重
            lru_cap: 3,
        };
        let q = Arc::new(ExperienceQueue::with_config(None, cfg));
        for i in 0..5 {
            let c = ExperienceCandidate {
                tool: "t".into(),
                args_hash: format!("h{i}"),
                outcome: Outcome::Success { summary: None },
                ts_ms: 0,
                source: ExperienceSource::ToolExecution,
            };
            q.push_at(c, 0);
        }
        // 内存 LRU 容量 3, 但 sqlite 无 → 候选应全部入队 (LRU 只影响 hash 索引, 不影响 pending)
        assert_eq!(q.len(), 5, "LRU 容量仅限 hash 索引, 不限候选池");
    }

    #[test]
    fn tool_bridge_backward_compat_post_hook_chain_still_works() {
        // 不挂 ObserverCaptureHook 时, 现有 with_post_hook / execute_if_allowed 流程不变
        use crate::tool_bridge::{PostExecuteHook, ToolBridge};
        use apeireth_memory::SqliteMemoryStore;
        use std::sync::Arc;

        struct NoopHook;
        impl PostExecuteHook for NoopHook {
            fn apply(&self, _c: &ParsedToolCall, r: &ExecutionResult) -> ExecutionResult {
                r.clone()
            }
        }

        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store).with_post_hook(Arc::new(NoopHook));
        // 旧 API 仍可构造, 不要求 ObserverCaptureHook 存在
        assert_eq!(bridge.post_hooks_len(), 1);
    }
}
