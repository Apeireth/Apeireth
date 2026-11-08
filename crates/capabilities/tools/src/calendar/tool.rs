//! 主人日程的生产工具面 (`tool.calendar`, 声明名 `calendar`)。
//!
//! 四动作 (闭合词表):
//!
//! | 动作 | 写面 | 语义 |
//! |---|---|---|
//! | `list` | 纯读 | 列主人的日程条目 (零审批、零写) |
//! | `create` | 写日程账 | 新建条目 (逐次人批 + 审计行) |
//! | `update` | 写日程账 | 改期 / 改标题 / 改备注 (逐次人批 + 审计行) |
//! | `cancel` | 写日程账 | 取消条目 (逐次人批 + 审计行; 取消是终态) |
//!
//! # 零外部集成
//!
//! 只读写本地日程账 (`<data_dir>/calendar/`): 不出网、不连外部日历服务、
//! 不同步 (owner 暂缓跨设备漫游) ⇒ 本面**没有**出网面与同步面。
//!
//! # 边界 (拒绝即帧 · 宁拒不截)
//!
//! 词表外动作 / 非法时刻 (倒挂 / 零时长 / 超长时段) / 空标题 / 超长标题 /
//! 超长备注 / 非法条目 id / 重复 id / 改期已取消条目 / 取消不存在条目 ⇒
//! **永久错误帧** (真因如实, 不静默降级、不截断)。
//!
//! # 主体边界
//!
//! 本面是**主人日程** (owner 自己的安排); 与 `tool.care_schedule` (代理自身的
//! 关怀例程) 主体不同、治理档不同、账本不同、互不可见 ⇒ 详见父模块的说明。

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::exec_pipeline::{OutputSchema, SchemaKind};

use super::store::{
    next_calendar_entry_id, now_epoch_ms, validate_calendar_entry_id, validate_calendar_note,
    validate_calendar_span, validate_calendar_title, CalendarEntry, CalendarStatus, CalendarStore,
};
use super::{
    ACTION_CANCEL, ACTION_CREATE, ACTION_LIST, ACTION_UPDATE, CALENDAR_ACTIONS,
    CALENDAR_ENTRY_ID_MAX_BYTES, CALENDAR_MAX_ENTRIES, CALENDAR_NOTE_MAX_BYTES,
    CALENDAR_SCOPE_NOTE, CALENDAR_TITLE_MAX_BYTES, CALENDAR_TOOL_NAME,
};

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalendarParams {
    action: Option<String>,
    entry_id: Option<String>,
    title: Option<String>,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    note: Option<String>,
    from_ms: Option<i64>,
    to_ms: Option<i64>,
    include_cancelled: Option<bool>,
}

/// 新建动作的字段边界 (纯判定: 返回 `Some(拒绝理由)` 即拒绝)。
fn create_refusal(params: &CalendarParams) -> Option<String> {
    let Some(title) = params.title.as_deref() else {
        return Some("拒绝建条目: create 必须给 title".to_string());
    };
    if let Err(error) = validate_calendar_title(title) {
        return Some(format!("拒绝建条目: {error}"));
    }
    let (Some(start_ms), Some(end_ms)) = (params.start_ms, params.end_ms) else {
        return Some("拒绝建条目: create 必须给 start_ms 与 end_ms (epoch 毫秒)".to_string());
    };
    if let Err(error) = validate_calendar_span(start_ms, end_ms) {
        return Some(format!("拒绝建条目: {error}"));
    }
    if let Some(note) = params.note.as_deref() {
        if let Err(error) = validate_calendar_note(note) {
            return Some(format!("拒绝建条目: {error}"));
        }
    }
    if let Some(entry_id) = params.entry_id.as_deref() {
        if let Err(error) = validate_calendar_entry_id(entry_id) {
            return Some(format!("拒绝建条目: {error}"));
        }
    }
    None
}

/// 改期动作的字段边界 (纯判定)。
fn update_refusal(params: &CalendarParams) -> Option<String> {
    let Some(entry_id) = params.entry_id.as_deref() else {
        return Some("拒绝改期: update 必须给 entry_id".to_string());
    };
    if let Err(error) = validate_calendar_entry_id(entry_id) {
        return Some(format!("拒绝改期: {error}"));
    }
    if let Some(title) = params.title.as_deref() {
        if let Err(error) = validate_calendar_title(title) {
            return Some(format!("拒绝改期: {error}"));
        }
    }
    if let Some(note) = params.note.as_deref() {
        if let Err(error) = validate_calendar_note(note) {
            return Some(format!("拒绝改期: {error}"));
        }
    }
    // 起止: 给了就得自洽 (倒挂 / 零时长 / 超长即拒)。
    if let (Some(start_ms), Some(end_ms)) = (params.start_ms, params.end_ms) {
        if let Err(error) = validate_calendar_span(start_ms, end_ms) {
            return Some(format!("拒绝改期: {error}"));
        }
    }
    // 至少要给一个待改字段 (否则是无意义调用)。
    let changes_something = params.title.is_some()
        || params.note.is_some()
        || params.start_ms.is_some()
        || params.end_ms.is_some();
    if !changes_something {
        return Some(
            "拒绝改期: update 至少要给一个待改字段 (title / start_ms / end_ms / note)".to_string(),
        );
    }
    None
}

/// 取消动作的字段边界 (纯判定)。
fn cancel_refusal(params: &CalendarParams) -> Option<String> {
    let Some(entry_id) = params.entry_id.as_deref() else {
        return Some("拒绝取消: cancel 必须给 entry_id".to_string());
    };
    validate_calendar_entry_id(entry_id)
        .err()
        .map(|error| format!("拒绝取消: {error}"))
}

/// 单调边界 Guard (只拒不放): 字段边界不过即拒 —— 空标题 / 倒挂或零时长 / 超长
/// 标题 / 超长备注 / 非法条目 id / 改期缺待改字段。后续任何阶段都不能翻回放行。
#[derive(Debug, Clone, Copy, Default)]
pub struct CalendarBoundaryGuard;

impl CalendarBoundaryGuard {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

impl ToolGuard for CalendarBoundaryGuard {
    fn name(&self) -> &str {
        "calendar_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != super::CALENDAR_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判字段形状边界。
        let Ok(params) = serde_json::from_value::<CalendarParams>(request.arguments.clone()) else {
            return None;
        };
        match params.action.as_deref() {
            Some(ACTION_CREATE) => create_refusal(&params),
            Some(ACTION_UPDATE) => update_refusal(&params),
            Some(ACTION_CANCEL) => cancel_refusal(&params),
            _ => None,
        }
    }
}

/// 主人日程工具 (`tool.calendar`, 声明名 `calendar`)。
///
/// 零外部集成: 只读写本地日程账 (`<data_dir>/calendar/`), 不出网、不同步。
pub struct CalendarTool {
    id: CapabilityId,
    store: CalendarStore,
}

impl CalendarTool {
    /// 绑定日程账落位。
    pub fn new(store: CalendarStore) -> Self {
        Self {
            id: CapabilityId::new(super::CALENDAR_CAPABILITY_ID).expect("valid capability id"),
            store,
        }
    }

    /// 绑定数据目录 (落 `<data_dir>/calendar/`)。
    pub fn for_data_dir(data_dir: impl AsRef<std::path::Path>) -> Self {
        Self::new(CalendarStore::under_data_dir(data_dir))
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 日程账 (自述面与测试同账同源)。
    pub fn store(&self) -> &CalendarStore {
        &self.store
    }

    /// 列日程 (纯读, 零审批, 零写)。
    fn list(&self, call: &ToolCall, params: &CalendarParams) -> ToolResult {
        if let (Some(from_ms), Some(to_ms)) = (params.from_ms, params.to_ms) {
            if from_ms > to_ms {
                return self.refuse(call, "拒绝查询: from_ms 晚于 to_ms (窗口非法)");
            }
        }
        if let Some(entry_id) = params.entry_id.as_deref() {
            if let Err(error) = validate_calendar_entry_id(entry_id) {
                return self.refuse(call, format!("拒绝查询: {error}"));
            }
        }
        let snapshot = match self.store.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => return self.refuse(call, format!("日程账读取失败: {error}")),
        };
        let include_cancelled = params.include_cancelled.unwrap_or(false);
        let mut selected: Vec<CalendarEntry> = snapshot
            .entries
            .iter()
            .filter(|entry| include_cancelled || entry.status == CalendarStatus::Scheduled)
            .filter(|entry| match &params.entry_id {
                Some(wanted) => &entry.id == wanted,
                None => true,
            })
            .filter(|entry| params.from_ms.is_none_or(|from| entry.start_ms >= from))
            .filter(|entry| params.to_ms.is_none_or(|to| entry.start_ms <= to))
            .cloned()
            .collect();
        // 上限保护: 有效条目数有界 ⇒ 这里不会截断; 真超界 (账被外部改坏) 时
        // **拒绝即帧**, 不静默截断。
        if selected.len() > CALENDAR_MAX_ENTRIES {
            return self.refuse(
                call,
                format!(
                    "拒绝查询: 日程条目数 {} 超上限 {CALENDAR_MAX_ENTRIES} (账被外部改坏; 不截断)",
                    selected.len()
                ),
            );
        }
        let note = if snapshot.skipped > 0 {
            Some(format!(
                "日程账 {} 行坏行读作不存在 (如实跳过并计数, 不砖化整本)",
                snapshot.skipped
            ))
        } else {
            None
        };
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_LIST,
                "status": "ok",
                "count": selected.len(),
                "entries": selected,
                "skipped": snapshot.skipped,
                "note": note,
                "scope": CALENDAR_SCOPE_NOTE,
            }),
        )
        .with_name(CALENDAR_TOOL_NAME)
    }

    /// 新建条目 (写账本 ⇒ 逐次人批 + 审计行)。
    fn create(&self, call: &ToolCall, params: &CalendarParams) -> ToolResult {
        if let Some(refusal) = create_refusal(params) {
            return self.refuse(call, refusal);
        }
        let title = params.title.clone().unwrap_or_default();
        let note = match params.note.as_deref().map(validate_calendar_note) {
            Some(Ok(value)) => value,
            Some(Err(error)) => return self.refuse(call, format!("拒绝建条目: {error}")),
            None => None,
        };
        let start_ms = params.start_ms.unwrap_or_default();
        let end_ms = params.end_ms.unwrap_or_default();
        let at_ms = now_epoch_ms();
        let id = match params.entry_id.as_deref() {
            Some(entry_id) => entry_id.to_string(),
            None => next_calendar_entry_id(at_ms),
        };
        let entry = CalendarEntry {
            id,
            title,
            start_ms,
            end_ms,
            note,
            status: CalendarStatus::Scheduled,
            created_at_ms: at_ms,
            updated_at_ms: at_ms,
        };
        match self.store.create(entry) {
            Ok(entry) => ToolResult::ok(
                &call.id,
                json!({
                    "action": ACTION_CREATE,
                    "status": "created",
                    "entry": entry,
                    "scope": CALENDAR_SCOPE_NOTE,
                }),
            )
            .with_name(CALENDAR_TOOL_NAME),
            Err(error) => self.refuse(call, format!("拒绝建条目: {error}")),
        }
    }

    /// 改期 / 改标题 / 改备注 (写账本 ⇒ 逐次人批 + 审计行)。
    fn update(&self, call: &ToolCall, params: &CalendarParams) -> ToolResult {
        if let Some(refusal) = update_refusal(params) {
            return self.refuse(call, refusal);
        }
        let entry_id = params.entry_id.clone().unwrap_or_default();
        let snapshot = match self.store.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => return self.refuse(call, format!("日程账读取失败: {error}")),
        };
        let Some(mut entry) = snapshot
            .entries
            .iter()
            .find(|item| item.id == entry_id)
            .cloned()
        else {
            return self.refuse(call, format!("拒绝改期: 日程条目 {entry_id} 不在账上"));
        };
        if entry.status == CalendarStatus::Cancelled {
            return self.refuse(
                call,
                format!("拒绝改期: 日程条目 {entry_id} 已取消 (取消是终态, 不可再改期)"),
            );
        }
        if let Some(title) = params.title.as_deref() {
            entry.title = title.to_string();
        }
        if let Some(start_ms) = params.start_ms {
            entry.start_ms = start_ms;
        }
        if let Some(end_ms) = params.end_ms {
            entry.end_ms = end_ms;
        }
        if let Some(note) = params.note.as_deref() {
            entry.note = match validate_calendar_note(note) {
                Ok(value) => value,
                Err(error) => return self.refuse(call, format!("拒绝改期: {error}")),
            };
        }
        // 改期后的最终形状仍要过边界 (改期不得把条目改成非法形状)。
        if let Err(error) = validate_calendar_span(entry.start_ms, entry.end_ms) {
            return self.refuse(call, format!("拒绝改期: {error}"));
        }
        if let Err(error) = validate_calendar_title(&entry.title) {
            return self.refuse(call, format!("拒绝改期: {error}"));
        }
        entry.updated_at_ms = now_epoch_ms();
        match self.store.update(entry) {
            Ok(entry) => ToolResult::ok(
                &call.id,
                json!({
                    "action": ACTION_UPDATE,
                    "status": "updated",
                    "entry": entry,
                    "scope": CALENDAR_SCOPE_NOTE,
                }),
            )
            .with_name(CALENDAR_TOOL_NAME),
            Err(error) => self.refuse(call, format!("拒绝改期: {error}")),
        }
    }

    /// 取消条目 (写账本 ⇒ 逐次人批 + 审计行; 取消是终态)。
    fn cancel(&self, call: &ToolCall, params: &CalendarParams) -> ToolResult {
        if let Some(refusal) = cancel_refusal(params) {
            return self.refuse(call, refusal);
        }
        let entry_id = params.entry_id.clone().unwrap_or_default();
        match self.store.cancel(&entry_id, now_epoch_ms()) {
            Ok(entry) => ToolResult::ok(
                &call.id,
                json!({
                    "action": ACTION_CANCEL,
                    "status": "cancelled",
                    "entry": entry,
                    "scope": CALENDAR_SCOPE_NOTE,
                }),
            )
            .with_name(CALENDAR_TOOL_NAME),
            Err(error) => self.refuse(call, format!("拒绝取消: {error}")),
        }
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, reason: impl Into<String>) -> ToolResult {
        ToolResult::permanent_error(&call.id, reason.into()).with_name(CALENDAR_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for CalendarTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": CALENDAR_ACTIONS,
                        "description": "list / create / update / cancel (closed vocabulary)."
                    },
                    "entry_id": {
                        "type": "string",
                        "maxLength": CALENDAR_ENTRY_ID_MAX_BYTES,
                        "description": "Entry id ([A-Za-z0-9._-]). create may omit it (generated); update / cancel require it."
                    },
                    "title": {
                        "type": "string",
                        "maxLength": CALENDAR_TITLE_MAX_BYTES,
                        "description": "Entry title (non-empty). create requires it; update may change it."
                    },
                    "start_ms": {
                        "type": "integer",
                        "description": "Start time in epoch milliseconds (UTC). create requires start_ms + end_ms; update may move them."
                    },
                    "end_ms": {
                        "type": "integer",
                        "description": "End time in epoch milliseconds (UTC); must be strictly after start_ms."
                    },
                    "note": {
                        "type": "string",
                        "maxLength": CALENDAR_NOTE_MAX_BYTES,
                        "description": "Optional note. create / update may set it."
                    },
                    "from_ms": {
                        "type": "integer",
                        "description": "list only: only entries starting at or after this epoch millisecond."
                    },
                    "to_ms": {
                        "type": "integer",
                        "description": "list only: only entries starting at or before this epoch millisecond."
                    },
                    "include_cancelled": {
                        "type": "boolean",
                        "description": "list only: include cancelled entries (default false)."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(CALENDAR_TOOL_NAME)
            .with_description(format!(
                "Owner schedule (local ledger, zero external integration): list / create / update / \
                 cancel the owner's own appointments. Times are epoch milliseconds (UTC); an entry \
                 needs a title and a strictly positive span. Governance: every write (create / \
                 update / cancel) requires a human decision on each call (no auto-pass tier) and \
                 leaves an audit row; list is a pure read with zero approval. Over-long titles or \
                 notes and illegal spans are refused outright, never truncated. {CALENDAR_SCOPE_NOTE}"
            ))
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<CalendarParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(call, format!("invalid calendar parameters: {error}"));
            }
        };
        let Some(action) = params.action.as_deref() else {
            return self.refuse(
                call,
                "拒绝日程调用: 必须显式给 action (list / create / update / cancel)",
            );
        };
        match action {
            ACTION_LIST => self.list(call, &params),
            ACTION_CREATE => self.create(call, &params),
            ACTION_UPDATE => self.update(call, &params),
            ACTION_CANCEL => self.cancel(call, &params),
            other => self.refuse(
                call,
                format!(
                    "unknown calendar action {other:?}; the closed vocabulary is \
                     list / create / update / cancel"
                ),
            ),
        }
    }
}
