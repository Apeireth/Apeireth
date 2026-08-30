//! `durable::history` — 追加式活动事件日志 (EventHistory 语义).
//!
//! **语义来源**: donor/apeireth-workflow `Event`/`EventKind` (typed events, 单调
//! event_id, input/output/error/timestamp) + donor/apeireth-supervisor `Journal`
//! (单调 seq 重分配, JSONL 行序列化, filter 查询) + donor/apeireth-bus `EventLog`
//! (since / last_n replay 查询语义)。
//!
//! **不变量**:
//! 1. `seq` 在单条 history 内严格单调 (1-based), 由 [`DurableHistory::record`] 重分配,
//!    外部传入值被覆盖 (supervisor Journal 模式)。
//! 2. `events()[i].seq == i + 1` — 索引即 seq-1, [`from_jsonl`](DurableHistory::from_jsonl)
//!    会校验该不变量, 损坏即 fail-closed。
//! 3. 时间戳由调用方注入 ([`now_epoch_ms`] 只是便捷注入源), 测试可注入固定值保证确定性。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::DurableError;

/// Run 级事件 (RunStarted/RunCompleted/RunFailed) 在 `activity_id` 字段上的占位标识。
pub const RUN_EVENT_ACTOR: &str = "run";

/// 活动事件类型 (donor workflow `EventKind` 语义, 命名对齐 v2: SubLoop/step 而非 Workflow)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventKind {
    /// 一次持久 run 开始 (输入已记录; donor `WorkflowStarted`)。
    RunStarted,
    /// 活动已调度 (输入 + 尝试号已记录; donor `ActivityScheduled`)。
    ActivityScheduled,
    /// 活动成功完成 (输出已记录; donor `ActivityCompleted`)。
    ActivityCompleted,
    /// 活动失败 (错误 + 尝试号已记录; donor `ActivityFailed`)。
    ActivityFailed,
    /// 一次持久 run 成功结束 (输出已记录; donor `WorkflowCompleted`)。
    RunCompleted,
    /// 一次持久 run 失败结束 (错误已记录; donor `WorkflowFailed`)。
    RunFailed,
}

impl ActivityEventKind {
    /// snake_case 字符串 (与 serde 表示一致)。
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RunStarted => "run_started",
            Self::ActivityScheduled => "activity_scheduled",
            Self::ActivityCompleted => "activity_completed",
            Self::ActivityFailed => "activity_failed",
            Self::RunCompleted => "run_completed",
            Self::RunFailed => "run_failed",
        }
    }
}

/// 单条持久事件 (EventHistory entry)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// 单调序号 (1-based, 由 `record` 重分配)。
    pub seq: u64,
    /// 事件类型
    pub kind: ActivityEventKind,
    /// 活动 ID (Run 级事件为 [`RUN_EVENT_ACTOR`])
    pub activity_id: String,
    /// 输入 (Scheduled/RunStarted 记录)
    pub input: Option<Value>,
    /// 输出 (Completed/RunCompleted 记录)
    pub output: Option<Value>,
    /// 错误 (Failed/RunFailed 记录)
    pub error: Option<String>,
    /// 尝试号 (1-based; failure/retry 元数据, Run 级事件为 0)
    pub attempt: u32,
    /// 注入时间戳 (Unix epoch 毫秒)
    pub timestamp_ms: i64,
}

/// 当前 Unix epoch 毫秒 (便捷时间注入源; 测试请注入固定值)。
pub fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 追加式持久事件日志。
///
/// 纯数据结构 + 查询/序列化; 不拥有执行权, 不持久化到磁盘 (持久化由调用方经
/// `ContinuationStore` 或任意存储组合)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DurableHistory {
    events: Vec<ActivityEvent>,
}

impl DurableHistory {
    /// 空日志。
    pub fn new() -> Self {
        Self::default()
    }

    /// 事件数。
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 全部事件 (追加序)。
    pub fn events(&self) -> &[ActivityEvent] {
        &self.events
    }

    /// 追加一条事件; seq 由日志重分配为单调 `len + 1`, 返回该 seq。
    pub fn record(
        &mut self,
        kind: ActivityEventKind,
        activity_id: impl Into<String>,
        input: Option<Value>,
        output: Option<Value>,
        error: Option<String>,
        attempt: u32,
        timestamp_ms: i64,
    ) -> u64 {
        let seq = self.events.len() as u64 + 1;
        self.events.push(ActivityEvent {
            seq,
            kind,
            activity_id: activity_id.into(),
            input,
            output,
            error,
            attempt,
            timestamp_ms,
        });
        seq
    }

    /// 按 activity_id 过滤 (supervisor `Journal::filter_child` 语义)。
    pub fn filter_activity<'a>(&'a self, activity_id: &str) -> impl Iterator<Item = &'a ActivityEvent> + 'a {
        let needle = activity_id.to_string();
        self.events
            .iter()
            .filter(move |e| e.activity_id == needle)
    }

    /// 按类型过滤 (supervisor `Journal::filter_kind` 语义)。
    pub fn filter_kind(&self, kind: ActivityEventKind) -> impl Iterator<Item = &ActivityEvent> {
        self.events.iter().filter(move |e| e.kind == kind)
    }

    /// 时间过滤重放: `timestamp_ms >= since_ms` 的全部事件 (bus `EventLog::replay_since`)。
    pub fn since_ms(&self, since_ms: i64) -> Vec<&ActivityEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp_ms >= since_ms)
            .collect()
    }

    /// 最新 N 条 (新→旧; bus `EventLog::last_n` 语义)。
    pub fn last_n(&self, n: usize) -> Vec<&ActivityEvent> {
        let start = self.events.len().saturating_sub(n);
        self.events[start..].iter().rev().collect()
    }

    /// 某活动已记录的成功完成数。
    pub fn completed_count(&self, activity_id: &str) -> usize {
        self.filter_activity(activity_id)
            .filter(|e| e.kind == ActivityEventKind::ActivityCompleted)
            .count()
    }

    /// 序列化为 JSONL (每行一条事件, 独立可解析; supervisor journal JSONL 兼容模式)。
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for e in &self.events {
            if let Ok(line) = serde_json::to_string(e) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }

    /// 从 JSONL 反序列化; 校验每行可独立解析且 seq 严格单调 1..=n, 损坏即 fail-closed。
    pub fn from_jsonl(jsonl: &str) -> Result<Self, DurableError> {
        let mut history = Self::new();
        for (idx, line) in jsonl.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let event: ActivityEvent = serde_json::from_str(line).map_err(|e| {
                DurableError::HistoryCorrupted {
                    reason: format!("JSONL line {} parse failed: {e}", idx + 1),
                }
            })?;
            let expected_seq = history.events.len() as u64 + 1;
            if event.seq != expected_seq {
                return Err(DurableError::HistoryCorrupted {
                    reason: format!(
                        "JSONL line {} has seq {}, expected monotonic {expected_seq}",
                        idx + 1,
                        event.seq
                    ),
                });
            }
            history.events.push(event);
        }
        Ok(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ====== donor workflow lib.rs 移植测试 ======

    /// donor `runner_event_ids_monotonic`: seq 严格单调 1..=n。
    #[test]
    fn seq_is_strictly_monotonic() {
        let mut h = DurableHistory::new();
        for i in 0..6 {
            let seq = h.record(
                ActivityEventKind::ActivityScheduled,
                "a",
                Some(json!(i)),
                None,
                None,
                1,
                1000 + i,
            );
            assert_eq!(seq, (i + 1) as u64);
        }
        for (i, e) in h.events().iter().enumerate() {
            assert_eq!(e.seq, (i + 1) as u64);
        }
    }

    /// donor `event_kind_serialization`: snake_case serde 表示。
    #[test]
    fn event_kind_serialization_is_snake_case() {
        assert_eq!(
            serde_json::to_value(ActivityEventKind::RunStarted).unwrap(),
            json!("run_started")
        );
        assert_eq!(
            serde_json::to_value(ActivityEventKind::ActivityScheduled).unwrap(),
            json!("activity_scheduled")
        );
        assert_eq!(
            serde_json::to_value(ActivityEventKind::ActivityCompleted).unwrap(),
            json!("activity_completed")
        );
        assert_eq!(
            serde_json::to_value(ActivityEventKind::ActivityFailed).unwrap(),
            json!("activity_failed")
        );
        assert_eq!(
            serde_json::to_value(ActivityEventKind::RunCompleted).unwrap(),
            json!("run_completed")
        );
        assert_eq!(
            serde_json::to_value(ActivityEventKind::RunFailed).unwrap(),
            json!("run_failed")
        );
        assert_eq!(ActivityEventKind::RunStarted.as_str(), "run_started");
    }

    /// donor `event_serialization_round_trip`: 事件 serde 往返无损。
    #[test]
    fn event_serialization_round_trip() {
        let e = ActivityEvent {
            seq: 42,
            kind: ActivityEventKind::ActivityCompleted,
            activity_id: "send_email".into(),
            input: Some(json!({"to": "alice@example.com"})),
            output: Some(json!({"status": "sent"})),
            error: None,
            attempt: 1,
            timestamp_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&e).unwrap();
        let parsed: ActivityEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, e);
    }

    // ====== supervisor journal_entry.rs 移植测试 ======

    /// donor `journal_append_assigns_monotonic_seq`: 外部 seq 被重分配。
    #[test]
    fn record_reassigns_external_seq() {
        let mut h = DurableHistory::new();
        h.record(ActivityEventKind::RunStarted, RUN_EVENT_ACTOR, None, None, None, 0, 1);
        // 故意传入错误 attempt/时间, seq 仍单调
        let s = h.record(
            ActivityEventKind::ActivityScheduled,
            "a",
            Some(json!(null)),
            None,
            None,
            999,
            2,
        );
        assert_eq!(s, 2);
    }

    /// donor `journal_filter_kind_isolates` + `journal_filter_child_isolates`。
    #[test]
    fn filter_kind_and_activity_isolate() {
        let mut h = DurableHistory::new();
        h.record(ActivityEventKind::ActivityScheduled, "a", None, None, None, 1, 1);
        h.record(ActivityEventKind::ActivityFailed, "b", None, None, Some("x".into()), 1, 2);
        h.record(ActivityEventKind::ActivityCompleted, "a", None, Some(json!(1)), None, 1, 3);

        let scheduled: Vec<_> = h.filter_kind(ActivityEventKind::ActivityScheduled).collect();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].activity_id, "a");

        let a_events: Vec<_> = h.filter_activity("a").collect();
        assert_eq!(a_events.len(), 2);
        let b_failures: Vec<_> = h.filter_activity("b").collect();
        assert_eq!(b_failures.len(), 1);
        assert_eq!(b_failures[0].error.as_deref(), Some("x"));
    }

    /// donor `journal_entry_serde_jsonl_compat`: 每行独立可解析 (JSONL)。
    #[test]
    fn jsonl_lines_are_independently_parseable() {
        let mut h = DurableHistory::new();
        h.record(ActivityEventKind::RunStarted, RUN_EVENT_ACTOR, Some(json!({"k":1})), None, None, 0, 1);
        // 不用 json!(null): serde 把 JSON null 解成 Option::None, 与 Some(Null) 无法往返。
        h.record(ActivityEventKind::ActivityScheduled, "a", Some(json!({"i": 0})), None, None, 1, 2);
        let jsonl = h.to_jsonl();
        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let _: ActivityEvent = serde_json::from_str(line).expect("JSONL line parse");
        }
        let back = DurableHistory::from_jsonl(&jsonl).unwrap();
        assert_eq!(back, h);
    }

    /// fail-closed: seq 不单调的 JSONL 显式拒绝。
    #[test]
    fn jsonl_non_monotonic_seq_is_rejected() {
        let e1 = ActivityEvent {
            seq: 1,
            kind: ActivityEventKind::RunStarted,
            activity_id: "run".into(),
            input: None,
            output: None,
            error: None,
            attempt: 0,
            timestamp_ms: 1,
        };
        let mut e2 = e1.clone();
        e2.seq = 5; // 跳号
        let jsonl = format!(
            "{}\n{}\n",
            serde_json::to_string(&e1).unwrap(),
            serde_json::to_string(&e2).unwrap()
        );
        assert!(matches!(
            DurableHistory::from_jsonl(&jsonl),
            Err(DurableError::HistoryCorrupted { .. })
        ));
        // 坏行也拒绝
        assert!(matches!(
            DurableHistory::from_jsonl("not json"),
            Err(DurableError::HistoryCorrupted { .. })
        ));
    }

    // ====== bus event_log.rs 查询语义移植 ======

    /// donor `replay_since_filters_by_timestamp` + `last_n_reverses_order`。
    #[test]
    fn since_and_last_n_query_semantics() {
        let mut h = DurableHistory::new();
        h.record(ActivityEventKind::ActivityScheduled, "a", None, None, None, 1, 100);
        h.record(ActivityEventKind::ActivityScheduled, "b", None, None, None, 1, 200);
        h.record(ActivityEventKind::ActivityCompleted, "a", None, Some(json!(1)), None, 1, 300);

        let since = h.since_ms(150);
        assert_eq!(since.len(), 2);
        assert_eq!(since[0].timestamp_ms, 200);
        assert_eq!(since[1].timestamp_ms, 300);

        let last2 = h.last_n(2);
        assert_eq!(last2.len(), 2);
        assert_eq!(last2[0].activity_id, "a"); // 最新在前
        assert_eq!(last2[0].kind, ActivityEventKind::ActivityCompleted);
        assert_eq!(last2[1].activity_id, "b");

        assert_eq!(h.completed_count("a"), 1);
        assert_eq!(h.completed_count("b"), 0);
    }
}
