//! 任务面板组数据模型 (驾驶舱面板 7): 任务列表 / 六态分组计数 / 选中详情
//! (payload 摘要 + 时间线)。纯状态 + 纯渲染助手, 无 I/O —— 数据来源见后端端口
//! [`crate::backend::CockpitBackend::fetch_tasks`] 与人判动作
//! [`crate::backend::CockpitBackend::human_judge_retry`]。
//!
//! **面板边界 (G 组核心: failed 只能人判)**: 本面板只有「读」+「人判重试」两个
//! 动作; 任务的增/删/改派/完成走后端工具面 (人工审批档), 不进本面板。
//!
//! 数据诚实纪律: 没数据 = 显式 "—" (`HONEST_PLACEHOLDER`), 未知状态不猜分组
//! (原样上屏 + 不着色), 绝不编数。

use serde_json::Value;

/// 诚实占位: 数据缺位时显式 "—", 不猜不装 (与遥测面板组同语法)。
pub const HONEST_PLACEHOLDER: &str = "—";

/// 状态六态 (冻结词表, 与队列状态机同名同序)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStateView {
    /// 入队待派。
    Pending,
    /// 已指派并已唤醒。
    Assigned,
    /// 执行段在跑。
    Running,
    /// 完成 (终态)。
    Done,
    /// 失败 (只能人判; 重试只能人判)。
    Failed,
    /// 外部等待 / 重投递熔断。
    Blocked,
}

/// 六态全集 (分组计数的固定顺序)。
pub const ALL_STATES: [TaskStateView; 6] = [
    TaskStateView::Pending,
    TaskStateView::Assigned,
    TaskStateView::Running,
    TaskStateView::Done,
    TaskStateView::Failed,
    TaskStateView::Blocked,
];

impl TaskStateView {
    /// 从合同标签解析 (六态冻结词表); 未知标签不猜 —— 回 None。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "pending" => Some(Self::Pending),
            "assigned" => Some(Self::Assigned),
            "running" => Some(Self::Running),
            "done" => Some(Self::Done),
            "failed" => Some(Self::Failed),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }

    /// 上屏标签 (无障碍语义, 不只靠颜色区分)。
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "待派",
            Self::Assigned => "已派",
            Self::Running => "执行",
            Self::Done => "完成",
            Self::Failed => "失败",
            Self::Blocked => "阻塞",
        }
    }

    /// 合同标签 (与后端契约同词)。
    pub fn wire(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Assigned => "assigned",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }
}

/// 时间线一行 (事件账按任务的只读投影)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTimelineEntry {
    /// 事件种类 (合同冻结词表)。
    pub kind: String,
    /// 事件时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 事件细节 (紧凑 JSON 文本; 无 = None)。
    pub detail: Option<String>,
}

/// 一条任务面板行 (列表行与选中详情同源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRow {
    /// 任务 id (列表上屏取 [`short_hash`])。
    pub id: String,
    /// 任务标题。
    pub title: String,
    /// 状态 (None = 合同外标签: 不猜分组, 原样上屏)。
    pub state: Option<TaskStateView>,
    /// 状态原样标签 (合同外标签如实显示用)。
    pub state_label: String,
    /// 尝试数 (派发执行次数)。
    pub attempts: u32,
    /// 负责人 (可空)。
    pub assignee: Option<String>,
    /// payload 摘要 (单行截断; 空载荷 = "—")。
    pub payload_summary: String,
    /// 时间线 (时间升序)。
    pub timeline: Vec<TaskTimelineEntry>,
}

impl TaskRow {
    /// 是否可人判重试 (G 组核心: failed 只能人判)。
    pub fn judge_retryable(&self) -> bool {
        self.state == Some(TaskStateView::Failed)
    }
}

/// 六态分组计数 (面板计数条的真值)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskCounts {
    /// pending 计数。
    pub pending: u64,
    /// assigned 计数。
    pub assigned: u64,
    /// running 计数。
    pub running: u64,
    /// done 计数。
    pub done: u64,
    /// failed 计数。
    pub failed: u64,
    /// blocked 计数。
    pub blocked: u64,
    /// 合计。
    pub total: u64,
}

impl TaskCounts {
    /// (状态, 计数) 全集, 顺序即上屏顺序。
    pub fn groups(&self) -> [(TaskStateView, u64); 6] {
        [
            (TaskStateView::Pending, self.pending),
            (TaskStateView::Assigned, self.assigned),
            (TaskStateView::Running, self.running),
            (TaskStateView::Done, self.done),
            (TaskStateView::Failed, self.failed),
            (TaskStateView::Blocked, self.blocked),
        ]
    }
}

/// 一帧任务面板快照 (后端一次快照读回灌)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskBoardSnapshot {
    /// 任务行 (后端稳定排序: 入队时刻, 同刻按 id)。
    pub rows: Vec<TaskRow>,
    /// 六态分组计数。
    pub counts: TaskCounts,
}

/// 任务面板的应用状态 (渲染层只读)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskBoardModel {
    /// 最近一次快照 (None = 未回灌: 上屏显式 "—", 不编数)。
    pub snapshot: Option<TaskBoardSnapshot>,
    /// 选中行下标。
    pub selected: usize,
    /// 来源备注 (端点失败 / 未接线, 如实上屏)。
    pub notes: Vec<String>,
}

impl TaskBoardModel {
    /// 回灌一帧快照 (选中行跟随任务 id 锚定, 列表变化不跳行)。
    pub fn absorb(&mut self, snapshot: TaskBoardSnapshot) {
        let anchored = self.selected_id().map(str::to_string);
        self.snapshot = Some(snapshot);
        let len = self.len();
        self.selected = anchored
            .and_then(|id| {
                self.snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.rows.iter().position(|row| row.id == id))
            })
            .unwrap_or(0)
            .min(len.saturating_sub(1));
    }

    /// 行数 (无快照 = 0)。
    pub fn len(&self) -> usize {
        self.snapshot
            .as_ref()
            .map(|snapshot| snapshot.rows.len())
            .unwrap_or(0)
    }

    /// 是否空 (无快照或零行)。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 当前选中行 (无 = None)。
    pub fn selected_row(&self) -> Option<&TaskRow> {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.rows.get(self.selected))
    }

    /// 当前选中任务 id (无 = None)。
    pub fn selected_id(&self) -> Option<&str> {
        self.selected_row().map(|row| row.id.as_str())
    }

    /// 选择上一行 (顶行不动)。
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// 选择下一行 (底行不动)。
    pub fn select_next(&mut self) {
        if self.len() > 0 {
            self.selected = (self.selected + 1).min(self.len() - 1);
        }
    }
}

/// 任务 id 短哈希 (FNV-1a 64 → 8 位十六进制, 前缀 `#`): 列表身份列。
pub fn short_hash(id: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("#{:08x}", (hash >> 32) as u32)
}

/// payload 摘要: 紧凑 JSON 单行截断到 `max_chars` (超出以 `…` 标截断);
/// 空载荷 (`null` / `{}` / `[]`) = 显式 "—"。
pub fn payload_summary(payload: &Value, max_chars: usize) -> String {
    let compact = match payload {
        Value::Null => String::new(),
        Value::Object(map) if map.is_empty() => String::new(),
        Value::Array(list) if list.is_empty() => String::new(),
        other => other.to_string(),
    };
    if compact.is_empty() {
        return HONEST_PLACEHOLDER.to_string();
    }
    let max_chars = max_chars.max(1);
    if compact.chars().count() <= max_chars {
        return compact;
    }
    let truncated: String = compact.chars().take(max_chars).collect();
    format!("{truncated}…")
}

/// 六态分组计数行: `待派 1 · 已派 0 · 执行 2 · 完成 3 · 失败 1 · 阻塞 0 (合计 7)`;
/// 无快照 = 显式 "—" (不编数)。
pub fn counts_line(counts: Option<TaskCounts>) -> String {
    let Some(counts) = counts else {
        return HONEST_PLACEHOLDER.to_string();
    };
    let groups: Vec<String> = counts
        .groups()
        .iter()
        .map(|(state, count)| format!("{} {count}", state.label()))
        .collect();
    format!("{} (合计 {})", groups.join(" · "), counts.total)
}

/// 从合同 JSON 行解析面板行 (缺字段如实留空, 不猜)。
pub fn row_from_json(value: &Value) -> TaskRow {
    let state_label = value
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    TaskRow {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        state: TaskStateView::parse(&state_label),
        state_label,
        attempts: value
            .get("attempts")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        assignee: value
            .get("assignee")
            .and_then(Value::as_str)
            .map(str::to_string),
        payload_summary: payload_summary(
            value.get("payload").unwrap_or(&Value::Null),
            PAYLOAD_SUMMARY_CHARS,
        ),
        timeline: value
            .get("timeline")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| TaskTimelineEntry {
                        kind: item
                            .get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        at_ms: item.get("at_ms").and_then(Value::as_i64).unwrap_or(0),
                        detail: item
                            .get("detail")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// payload 摘要的截断宽度 (面板详情单行)。
pub const PAYLOAD_SUMMARY_CHARS: usize = 72;

/// 从合同 JSON 快照解析面板快照 (计数缺字段 = 0 如实, 不猜)。
pub fn snapshot_from_json(value: &Value) -> TaskBoardSnapshot {
    let rows = value
        .get("tasks")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(row_from_json).collect())
        .unwrap_or_default();
    let counts = value.get("counts");
    let count = |name: &str| {
        counts
            .and_then(|counts| counts.get(name))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    TaskBoardSnapshot {
        rows,
        counts: TaskCounts {
            pending: count("pending"),
            assigned: count("assigned"),
            running: count("running"),
            done: count("done"),
            failed: count("failed"),
            blocked: count("blocked"),
            total: count("total"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed_row(id: &str) -> TaskRow {
        TaskRow {
            id: id.to_string(),
            title: "校对发布清单".to_string(),
            state: Some(TaskStateView::Failed),
            state_label: "failed".to_string(),
            attempts: 2,
            assignee: Some("worker-a".to_string()),
            payload_summary: "{\"step\":3}".to_string(),
            timeline: vec![TaskTimelineEntry {
                kind: "failure_judged".to_string(),
                at_ms: 1_700_000_000_500,
                detail: Some(r#"{"judge":"owner"}"#.to_string()),
            }],
        }
    }

    /// 六态解析: 冻结词表全中, 未知标签不猜 (None)。
    #[test]
    fn task_states_parse_the_closed_six_word_vocabulary() {
        for state in ALL_STATES {
            assert_eq!(TaskStateView::parse(state.wire()), Some(state));
            assert!(!state.label().is_empty());
        }
        assert_eq!(TaskStateView::parse("maybe"), None, "合同外标签不猜分组");
    }

    /// id 短哈希: 确定性 (同 id 同哈希), 8 位十六进制, 不同 id 不撞面。
    #[test]
    fn short_hash_is_deterministic_and_sized() {
        assert_eq!(short_hash("task-1"), short_hash("task-1"));
        assert_ne!(short_hash("task-1"), short_hash("task-2"));
        let hash = short_hash("task-42");
        assert!(hash.starts_with('#'), "{hash}");
        assert_eq!(hash.len(), 1 + 8, "{hash}");
    }

    /// payload 摘要: 空载荷 "—", 短载荷全量, 长载荷截断标 `…` (摘要不冒充全文)。
    #[test]
    fn payload_summary_truncates_honestly() {
        assert_eq!(payload_summary(&Value::Null, 72), HONEST_PLACEHOLDER);
        assert_eq!(
            payload_summary(&serde_json::json!({}), 72),
            HONEST_PLACEHOLDER
        );
        assert_eq!(
            payload_summary(&serde_json::json!([]), 72),
            HONEST_PLACEHOLDER
        );
        assert_eq!(
            payload_summary(&serde_json::json!({"step": 3}), 72),
            "{\"step\":3}"
        );
        let long = payload_summary(&serde_json::json!({"step": "x".repeat(200)}), 12);
        assert!(long.ends_with('…'), "{long}");
        assert!(long.chars().count() <= 13, "{long}");
    }

    /// 分组计数行: 有快照给六态真数 (零也是真数), 无快照显式 "—"。
    #[test]
    fn counts_line_shows_six_groups_or_the_honest_placeholder() {
        assert_eq!(counts_line(None), HONEST_PLACEHOLDER);
        let line = counts_line(Some(TaskCounts {
            failed: 1,
            total: 1,
            ..TaskCounts::default()
        }));
        assert!(
            line.contains("待派 0") && line.contains("失败 1") && line.contains("合计 1"),
            "{line}"
        );
    }

    /// 选中跟随任务 id 锚定: 刷新后不跳行; 选择被列表长度钳制。
    #[test]
    fn selection_is_anchored_and_clamped() {
        let mut model = TaskBoardModel::default();
        assert!(model.selected_row().is_none(), "无快照 = 无选中");
        model.select_next();
        assert_eq!(model.selected, 0);

        model.absorb(TaskBoardSnapshot {
            rows: vec![
                failed_row("task-1"),
                failed_row("task-2"),
                failed_row("task-3"),
            ],
            counts: TaskCounts::default(),
        });
        model.select_next();
        model.select_next();
        assert_eq!(model.selected_id(), Some("task-3"));
        model.select_next();
        assert_eq!(model.selected, 2, "底行不动");
        // 刷新锚定: 同 id 仍在, 选中不跳。
        model.absorb(TaskBoardSnapshot {
            rows: vec![
                failed_row("task-1"),
                failed_row("task-2"),
                failed_row("task-3"),
            ],
            counts: TaskCounts::default(),
        });
        assert_eq!(model.selected_id(), Some("task-3"));
        // 列表缩短: 选中被钳到新底行。
        model.absorb(TaskBoardSnapshot {
            rows: vec![failed_row("task-1")],
            counts: TaskCounts::default(),
        });
        assert_eq!(model.selected_id(), Some("task-1"));
    }

    /// 只有 failed 能人判重试 (G 组核心)。
    #[test]
    fn only_failed_rows_are_judge_retryable() {
        let mut row = failed_row("task-1");
        assert!(row.judge_retryable());
        for state in ALL_STATES
            .iter()
            .filter(|state| **state != TaskStateView::Failed)
        {
            row.state = Some(*state);
            assert!(!row.judge_retryable(), "{state:?} 不得可重试");
        }
        row.state = None;
        assert!(!row.judge_retryable(), "合同外标签不猜可重试");
    }

    /// 合同 JSON 解析: 缺字段如实留空, 未知状态不猜分组。
    #[test]
    fn rows_and_snapshots_parse_from_the_contract_shape() {
        let value = serde_json::json!({
            "tasks": [{
                "id": "task-1", "title": "校对", "state": "failed", "attempts": 2,
                "assignee": "worker-a", "created_at_ms": 1, "updated_at_ms": 2,
                "payload": {"step": 3},
                "timeline": [{"kind": "enqueued", "at_ms": 1}]
            }, {
                "id": "task-2", "title": "怪态", "state": "maybe", "attempts": 0,
                "assignee": null, "created_at_ms": 3, "updated_at_ms": 3,
                "payload": null, "timeline": []
            }],
            "counts": {"pending": 1, "assigned": 0, "running": 0, "done": 0,
                       "failed": 1, "blocked": 0, "total": 2}
        });
        let snapshot = snapshot_from_json(&value);
        assert_eq!(snapshot.rows.len(), 2);
        assert_eq!(snapshot.rows[0].state, Some(TaskStateView::Failed));
        assert_eq!(snapshot.rows[0].attempts, 2);
        assert_eq!(snapshot.rows[0].assignee.as_deref(), Some("worker-a"));
        assert_eq!(snapshot.rows[0].payload_summary, "{\"step\":3}");
        assert_eq!(snapshot.rows[0].timeline.len(), 1);
        // 合同外状态: 不猜分组, 原样标签如实。
        assert_eq!(snapshot.rows[1].state, None);
        assert_eq!(snapshot.rows[1].state_label, "maybe");
        assert_eq!(snapshot.rows[1].payload_summary, HONEST_PLACEHOLDER);
        assert_eq!(snapshot.counts.total, 2);
    }
}
