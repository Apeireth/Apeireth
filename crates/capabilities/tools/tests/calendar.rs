//! 主人日程面 (`tool.calendar`) 回归: 空账本列 / 建后可查 / 改期 / 取消 /
//! 非法时间拒绝 / 表外动作拒绝 / 坏行按不存在 / 写动作逐次人批 (读零审批) /
//! 审计行 / **与 `tool.care_schedule` 的主体边界** / 对抗 (超长标题 · 巨量条目
//! 宁拒不截) / 开关 fail-closed。
//!
//! 主体边界说明 (钉死, 不靠记忆): 本面是**主人日程** (owner 自己的安排); 而
//! `tool.care_schedule` (`crates/engine/organ/src/care/tool.rs:40`) 是**代理
//! 自身的关怀例程** (代理何时该主动关怀主人) ⇒ 两者主体不同、治理档不同、
//! 账本不同、互不可见 ⇒ 关怀触发器不是日程条目, 日程条目也不是关怀触发器。

/// 本集成测试二进制内的 env 锁（进程级全局量 ⇒ 串行）。
/// 放在**模块级**（不在测试函数内）：同文件多个测试函数共用同一把。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::calendar::{
    calendar_approval_reason, calendar_enabled_from_env, calendar_write_face_for_action,
    CalendarApprovalHook, CalendarAuditRow, CalendarConfig, CalendarEntry, CalendarLedgerEntry,
    CalendarStatus, CalendarStore, CalendarTool, CALENDAR_ACTIONS, CALENDAR_AUDIT_DOC_NAME,
    CALENDAR_AUDIT_FILE, CALENDAR_CAPABILITY_ID, CALENDAR_DIR_NAME, CALENDAR_ENABLE_ENV,
    CALENDAR_ENTRY_ID_MAX_BYTES, CALENDAR_LEDGER_DOC_NAME, CALENDAR_LEDGER_FILE,
    CALENDAR_MAX_ENTRIES, CALENDAR_MAX_LEDGER_ROWS, CALENDAR_MAX_SPAN_MS, CALENDAR_NOTE_MAX_BYTES,
    CALENDAR_SCOPE_NOTE, CALENDAR_TITLE_MAX_BYTES, CALENDAR_TOOL_NAME,
};
use serde_json::{json, Value};

/// 工具调用一帧 (按动作拼参数)。
fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: CALENDAR_TOOL_NAME.into(),
        arguments,
    }
}

/// 临时数据目录 (每测试一个, 落系统临时区)。
fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-calendar-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 工具 + 数据目录 (账本落在 `<data_dir>/calendar/`)。
fn tool_with(tag: &str) -> (std::path::PathBuf, CalendarTool) {
    let data_dir = temp_data_dir(tag);
    let tool = CalendarTool::for_data_dir(&data_dir);
    (data_dir, tool)
}

async fn ok_value(tool: &CalendarTool, arguments: Value) -> Value {
    let result = tool.invoke(&call(arguments)).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("结构化 json")
}

async fn refused(tool: &CalendarTool, arguments: Value) -> String {
    let result = tool.invoke(&call(arguments)).await;
    assert!(!result.is_ok(), "必须拒绝即帧: {}", result.render());
    result.render()
}

/// 造一条合法的新建参数 (时刻固定, 便于断言)。
fn create_arguments(entry_id: &str, title: &str, start_ms: i64, end_ms: i64) -> Value {
    json!({
        "action": "create",
        "entry_id": entry_id,
        "title": title,
        "start_ms": start_ms,
        "end_ms": end_ms,
    })
}

/// 合成一条条目账行 (StoredDoc 信封 JSON, 与写端同形)。
fn synthetic_created_row(index: usize) -> String {
    json!({
        "name": CALENDAR_LEDGER_DOC_NAME,
        "version": 1,
        "compatible_versions": [1],
        "body": {
            "event": "created",
            "id": format!("e{index}"),
            "title": format!("合成条目 {index}"),
            "start_ms": 1_700_000_000_000i64 + index as i64,
            "end_ms": 1_700_000_003_600i64 + index as i64,
            "note": null,
            "status": "scheduled",
            "created_at_ms": 1_700_000_000_000i64,
            "updated_at_ms": 1_700_000_000_000i64,
        }
    })
    .to_string()
}

/// 治理请求 (派发本面, 动作词表由调用方给)。
fn dispatch(action: Option<&str>) -> GovernanceRequest<'static> {
    let capability: &'static CapabilityId = Box::leak(Box::new(
        CapabilityId::new(CALENDAR_CAPABILITY_ID).expect("capability id"),
    ));
    let arguments: &'static Value = Box::leak(Box::new(match action {
        Some(action) => json!({ "action": action }),
        None => json!({}),
    }));
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
}

// ---------------------------------------------------------------------------
// ① 空账本: 列日程 = 空 (零审批、零写)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_ledger_lists_nothing_and_writes_nothing() {
    let (data_dir, tool) = tool_with("empty");

    let value = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(value["action"], "list");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["count"], 0);
    assert_eq!(value["entries"], json!([]));
    assert_eq!(value["skipped"], 0);
    assert!(value["note"].is_null(), "无坏行时 note 显式 null: {value}");
    assert_eq!(value["scope"], CALENDAR_SCOPE_NOTE, "每帧带主体边界说明");

    // 读动作零写: 数据目录里一个文件都没有。
    let entries = std::fs::read_dir(&data_dir).expect("数据目录可读").count();
    assert_eq!(entries, 0, "list 是纯读: 数据目录必须零变化");
}

// ---------------------------------------------------------------------------
// ② 建后可查: 落盘位置 + 条目可查
// ---------------------------------------------------------------------------

#[tokio::test]
async fn created_entry_is_visible_in_a_later_list() {
    let (data_dir, tool) = tool_with("create");

    let created = ok_value(
        &tool,
        create_arguments("dentist", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;
    assert_eq!(created["action"], "create");
    assert_eq!(created["status"], "created");
    assert_eq!(created["entry"]["id"], "dentist");
    assert_eq!(created["entry"]["title"], "看牙");
    assert_eq!(created["entry"]["status"], "scheduled");

    // 落盘位置: `<data_dir>/calendar/entries.jsonl` + `audit.jsonl`。
    let calendar_dir = data_dir.join(CALENDAR_DIR_NAME);
    assert!(
        calendar_dir.join(CALENDAR_LEDGER_FILE).exists(),
        "条目账落位"
    );
    assert!(
        calendar_dir.join(CALENDAR_AUDIT_FILE).exists(),
        "审计账落位"
    );

    let listed = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["entries"][0]["id"], "dentist");
    assert_eq!(listed["entries"][0]["title"], "看牙");
    assert_eq!(listed["entries"][0]["start_ms"], 1_700_000_000_000i64);
}

// ---------------------------------------------------------------------------
// ③ 改期: update 改起止
// ---------------------------------------------------------------------------

#[tokio::test]
async fn update_reschedules_the_entry() {
    let (_data_dir, tool) = tool_with("update");
    ok_value(
        &tool,
        create_arguments("dentist", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;

    let updated = ok_value(
        &tool,
        json!({
            "action": "update",
            "entry_id": "dentist",
            "start_ms": 1_700_100_000_000i64,
            "end_ms": 1_700_100_003_600i64,
        }),
    )
    .await;
    assert_eq!(updated["action"], "update");
    assert_eq!(updated["status"], "updated");
    assert_eq!(updated["entry"]["start_ms"], 1_700_100_000_000i64);
    assert_eq!(updated["entry"]["end_ms"], 1_700_100_003_600i64);

    // 账上折出的当前态 = 改期后的时刻 (不是旧值)。
    let listed = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(listed["entries"][0]["start_ms"], 1_700_100_000_000i64);
    assert_eq!(listed["entries"][0]["end_ms"], 1_700_100_003_600i64);

    // 改不存在的条目 = 拒绝即帧。
    let refused_text = refused(
        &tool,
        json!({ "action": "update", "entry_id": "ghost", "title": "x" }),
    )
    .await;
    assert!(refused_text.contains("不在账上"), "{refused_text}");
}

// ---------------------------------------------------------------------------
// ④ 取消: 终态 + 默认列表不显示
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cancel_marks_the_entry_cancelled_and_is_terminal() {
    let (_data_dir, tool) = tool_with("cancel");
    ok_value(
        &tool,
        create_arguments("dentist", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;

    let cancelled = ok_value(&tool, json!({ "action": "cancel", "entry_id": "dentist" })).await;
    assert_eq!(cancelled["action"], "cancel");
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["entry"]["status"], "cancelled");

    // 默认列表不显示已取消条目。
    let listed = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(listed["count"], 0, "已取消条目默认不出现");

    // 显式要才看得到 (条目仍在账上, 不删行)。
    let listed = ok_value(
        &tool,
        json!({ "action": "list", "include_cancelled": true }),
    )
    .await;
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["entries"][0]["status"], "cancelled");

    // 取消是终态: 已取消的条目不可再改期, 也不可再取消。
    let refused_text = refused(
        &tool,
        json!({ "action": "update", "entry_id": "dentist", "title": "改标题" }),
    )
    .await;
    assert!(refused_text.contains("已取消"), "{refused_text}");
    let refused_text = refused(&tool, json!({ "action": "cancel", "entry_id": "dentist" })).await;
    assert!(refused_text.contains("已取消"), "{refused_text}");
}

// ---------------------------------------------------------------------------
// ⑤ 非法时间 / 非法标题: 拒绝即帧 (不截断、不静默改)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn illegal_spans_and_shapes_are_refused() {
    let (_data_dir, tool) = tool_with("illegal");

    // 结束早于开始 (倒挂)。
    let text = refused(
        &tool,
        create_arguments("a", "倒挂", 1_700_000_003_600, 1_700_000_000_000),
    )
    .await;
    assert!(text.contains("结束时刻必须晚于开始时刻"), "{text}");

    // 零时长。
    let text = refused(
        &tool,
        create_arguments("b", "零时长", 1_700_000_000_000, 1_700_000_000_000),
    )
    .await;
    assert!(text.contains("结束时刻必须晚于开始时刻"), "{text}");

    // 非正时刻。
    let text = refused(&tool, create_arguments("c", "非正", 0, 1_700_000_000_000)).await;
    assert!(text.contains("必须为正"), "{text}");

    // 超长时段。
    let text = refused(
        &tool,
        create_arguments(
            "d",
            "超长时段",
            1_700_000_000_000,
            1_700_000_000_000 + CALENDAR_MAX_SPAN_MS + 1,
        ),
    )
    .await;
    assert!(text.contains("跨度"), "{text}");

    // 空标题。
    let text = refused(&tool, create_arguments("e", "   ", 1, 2)).await;
    assert!(text.contains("标题不得为空"), "{text}");

    // 拒绝即帧 ⇒ 账上一个条目都没有。
    let listed = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(listed["count"], 0, "被拒的调用不许留下半个条目");
}

// ---------------------------------------------------------------------------
// ⑥ 表外动作: 拒绝即帧 (闭合词表)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn out_of_vocabulary_actions_are_refused() {
    let (_data_dir, tool) = tool_with("vocab");

    for action in ["delete", "remove", "purge", "care_add", "rm -rf /", ""] {
        let text = refused(&tool, json!({ "action": action })).await;
        assert!(
            text.contains("unknown calendar action") || text.contains("必须显式给 action"),
            "{action}: {text}"
        );
    }

    // 动作缺失 = 拒绝即帧。
    let text = refused(&tool, json!({})).await;
    assert!(text.contains("必须显式给 action"), "{text}");

    // 未知字段 = 拒绝即帧 (参数面闭合)。
    let text = refused(&tool, json!({ "action": "list", "who": "someone" })).await;
    assert!(text.contains("invalid calendar parameters"), "{text}");
}

// ---------------------------------------------------------------------------
// ⑦ 坏行按不存在: 跳过并计数, 不砖化整本
// ---------------------------------------------------------------------------

#[tokio::test]
async fn broken_rows_read_as_nonexistent_without_breaking_the_ledger() {
    let (data_dir, tool) = tool_with("broken-row");
    ok_value(
        &tool,
        create_arguments("dentist", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;

    // 追加一行坏 JSON (撕裂写 / 外部改坏都可能长这样)。
    let ledger = data_dir.join(CALENDAR_DIR_NAME).join(CALENDAR_LEDGER_FILE);
    let mut raw = std::fs::read_to_string(&ledger).expect("读账");
    raw.push_str("{ this is not json\n");
    std::fs::write(&ledger, raw).expect("写坏行");

    // 坏行读作不存在: 好行照读, 坏行计数上报。
    let listed = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(listed["count"], 1, "好行照读");
    assert_eq!(listed["skipped"], 1, "坏行计数如实上报");
    let note = listed["note"].as_str().expect("note 在场");
    assert!(note.contains("坏行读作不存在"), "{note}");
    assert!(note.contains("不砖化整本"), "{note}");

    // 写动作也不被坏行砖化 (账可继续写)。
    let created = ok_value(
        &tool,
        create_arguments("gym", "健身", 1_700_000_100_000, 1_700_000_103_600),
    )
    .await;
    assert_eq!(created["status"], "created");
}

// ---------------------------------------------------------------------------
// ⑧ 治理: 写动作逐次人批, 读动作零审批, 词表外 fail-closed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_actions_require_human_approval_and_reads_do_not() {
    let hook = CalendarApprovalHook::new();

    // 写面: 每次都停人工审批, 无自动放行档。
    for action in ["create", "update", "cancel"] {
        let decision = hook.evaluate(&dispatch(Some(action))).await;
        let Decision::RequireApproval { reason } = decision else {
            panic!("{action} 必须停人工审批: {decision:?}");
        };
        assert!(reason.contains("逐次人批"), "{reason}");
        assert!(reason.contains("无自动放行档"), "{reason}");
        assert!(reason.contains("写主人日程账"), "{reason}");
    }

    // 读面: 零审批。
    let decision = hook.evaluate(&dispatch(Some("list"))).await;
    assert_eq!(decision, Decision::Allow, "{decision:?}");

    // 词表外 / 动作缺失: fail-closed 拒绝 (不静默放行)。
    for action in [None, Some("delete"), Some("care_add")] {
        let decision = hook.evaluate(&dispatch(action)).await;
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "动作 {action:?} 必须 fail-closed 拒绝: {decision:?}"
        );
    }

    // 本钩子只管自己的面: 其它能力一律放行 (不越权误伤)。
    let capability = CapabilityId::new("tool.care_schedule").expect("capability id");
    let arguments = json!({ "action": "care_add" });
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    assert_eq!(
        hook.evaluate(&request).await,
        Decision::Allow,
        "本钩子不得误伤关怀例程面"
    );
}

// ---------------------------------------------------------------------------
// ⑨ 审计行: 写动作留痕, 读动作不留
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_actions_leave_audit_rows_and_reads_do_not() {
    let (data_dir, tool) = tool_with("audit");
    let store = CalendarStore::under_data_dir(&data_dir);

    ok_value(
        &tool,
        create_arguments("dentist", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;
    ok_value(
        &tool,
        json!({
            "action": "update",
            "entry_id": "dentist",
            "start_ms": 1_700_100_000_000i64,
            "end_ms": 1_700_100_003_600i64,
        }),
    )
    .await;
    ok_value(&tool, json!({ "action": "cancel", "entry_id": "dentist" })).await;
    ok_value(&tool, json!({ "action": "list" })).await;

    let audit = store.scan_audit().expect("审计账可读");
    let rows: Vec<&CalendarAuditRow> = audit.records.iter().map(|record| &record.body).collect();
    assert_eq!(rows.len(), 3, "三个写动作 = 三行审计 (list 不留痕)");
    assert_eq!(rows[0].action, "create");
    assert_eq!(rows[1].action, "update");
    assert_eq!(rows[2].action, "cancel");
    for row in &rows {
        assert_eq!(row.entry_id, "dentist");
        assert_eq!(row.outcome, "ok");
        assert!(row.at_ms > 0, "审计行留时刻");
    }

    // 条目账是 append-only: 三次写动作 = 三行 (新建 + 改期 + 取消)。
    let ledger = store.scan().expect("条目账可读");
    let kinds: Vec<&str> = ledger
        .records
        .iter()
        .map(|record| match &record.body {
            CalendarLedgerEntry::Created(_) => "created",
            CalendarLedgerEntry::Updated(_) => "updated",
            CalendarLedgerEntry::Cancelled { .. } => "cancelled",
        })
        .collect();
    assert_eq!(kinds, ["created", "updated", "cancelled"]);
}

// ---------------------------------------------------------------------------
// ⑩ 主体边界: 主人日程 ≠ 代理自身的关怀例程 (tool.care_schedule)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_care_schedule_boundary_is_pinned() {
    // ① 能力 id / 模型面名不同: 关怀例程面照旧是 `tool.care_schedule`。
    assert_eq!(CALENDAR_CAPABILITY_ID, "tool.calendar");
    assert_eq!(CALENDAR_TOOL_NAME, "calendar");
    assert_ne!(CALENDAR_CAPABILITY_ID, "tool.care_schedule");
    assert_ne!(CALENDAR_TOOL_NAME, "care_schedule");

    // ② 动作词表不相交: 关怀动作 (care_add / care_remove / care_list) 在本面
    //    一律是表外动作 (拒绝即帧)。
    for care_action in ["care_add", "care_remove", "care_list"] {
        assert!(
            !CALENDAR_ACTIONS.contains(&care_action),
            "关怀动作 {care_action} 不得进主人日程动作词表"
        );
        assert_eq!(calendar_write_face_for_action(care_action), None);
    }

    // ③ 边界说明自带主体口径 (写进每帧 + 工具描述)。
    assert!(CALENDAR_SCOPE_NOTE.contains("主人日程"));
    assert!(CALENDAR_SCOPE_NOTE.contains("tool.care_schedule"));
    assert!(CALENDAR_SCOPE_NOTE.contains("互不可见"));

    let (_data_dir, tool) = tool_with("boundary");
    let description = tool.declaration().description.clone().unwrap_or_default();
    assert!(
        description.contains("tool.care_schedule"),
        "工具描述必须带主体边界: {description}"
    );

    // ④ 账本目录不同: 主人日程账在 `<data_dir>/calendar/`, 与关怀规则存储
    //    不是同一处 (关怀例程的规则账在 organ 侧, 本面不碰)。
    assert_eq!(CALENDAR_DIR_NAME, "calendar");
    assert!(!CALENDAR_DIR_NAME.contains("care"));
    assert!(!CALENDAR_LEDGER_FILE.contains("care"));
    assert!(!CALENDAR_AUDIT_FILE.contains("care"));
}

// ---------------------------------------------------------------------------
// ⑪ 对抗: 超长标题 / 巨量条目 ⇒ 拒绝, 绝不截断
// ---------------------------------------------------------------------------

#[tokio::test]
async fn overlong_titles_are_refused_not_truncated() {
    let (_data_dir, tool) = tool_with("overlong-title");

    // 超长标题 ⇒ 拒绝即帧。
    let long_title = "长".repeat(CALENDAR_TITLE_MAX_BYTES + 1);
    let text = refused(
        &tool,
        create_arguments("a", &long_title, 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;
    assert!(text.contains("超长即拒"), "{text}");
    assert!(text.contains("不截断"), "{text}");

    // 超长备注 ⇒ 拒绝即帧。
    let long_note = "备".repeat(CALENDAR_NOTE_MAX_BYTES + 1);
    let text = refused(
        &tool,
        json!({
            "action": "create",
            "entry_id": "b",
            "title": "正常标题",
            "start_ms": 1_700_000_000_000i64,
            "end_ms": 1_700_000_003_600i64,
            "note": long_note,
        }),
    )
    .await;
    assert!(text.contains("超长即拒"), "{text}");

    // 拒绝 ⇒ 账上零条目 (**没有**被截断的半个标题落账)。
    let listed = ok_value(&tool, json!({ "action": "list" })).await;
    assert_eq!(listed["count"], 0);
}

#[tokio::test]
async fn oversized_ledgers_are_refused_not_truncated() {
    let (data_dir, tool) = tool_with("bomb-ledger");
    let calendar_dir = data_dir.join(CALENDAR_DIR_NAME);
    std::fs::create_dir_all(&calendar_dir).expect("创建日程面目录");

    // 合成一个「炸弹账本」: 超过有效条目上限的条目行。
    let mut raw = String::new();
    for index in 0..(CALENDAR_MAX_ENTRIES + 8) {
        raw.push_str(&synthetic_created_row(index));
        raw.push('\n');
    }
    std::fs::write(calendar_dir.join(CALENDAR_LEDGER_FILE), raw).expect("写炸弹账本");

    // 列日程 ⇒ 拒绝即帧 (**不截断成 512 条**)。
    let text = refused(&tool, json!({ "action": "list" })).await;
    assert!(text.contains("超上限"), "{text}");
    assert!(text.contains("不截断"), "{text}");

    // 建条目 ⇒ 拒绝即帧 (条目上限: 不静默丢已有条目)。
    let text = refused(
        &tool,
        create_arguments("new", "新条目", 1_800_000_000_000, 1_800_000_003_600),
    )
    .await;
    assert!(text.contains("已满"), "{text}");

    // 账本行数闸也有值 (append-only 的增长上限)。
    assert!(CALENDAR_MAX_LEDGER_ROWS > CALENDAR_MAX_ENTRIES);
}

// ---------------------------------------------------------------------------
// ⑫ 其它边界: 重复 id / 非法 id / 缺字段 / 改期无变更
// ---------------------------------------------------------------------------

#[tokio::test]
async fn duplicate_ids_and_missing_fields_are_refused() {
    let (_data_dir, tool) = tool_with("dup");
    ok_value(
        &tool,
        create_arguments("dentist", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;

    // 重复 id ⇒ 拒绝即帧。
    let text = refused(
        &tool,
        create_arguments(
            "dentist",
            "又一个看牙",
            1_700_000_100_000,
            1_700_000_103_600,
        ),
    )
    .await;
    assert!(text.contains("重复"), "{text}");

    // 非法 id 字符集 ⇒ 拒绝即帧。
    let text = refused(
        &tool,
        create_arguments("has space", "看牙", 1_700_000_000_000, 1_700_000_003_600),
    )
    .await;
    assert!(text.contains("条目 id"), "{text}");

    // 超长 id ⇒ 拒绝即帧。
    let text = refused(
        &tool,
        create_arguments(
            &"x".repeat(CALENDAR_ENTRY_ID_MAX_BYTES + 1),
            "看牙",
            1_700_000_000_000,
            1_700_000_003_600,
        ),
    )
    .await;
    assert!(text.contains("条目 id"), "{text}");

    // 缺字段 ⇒ 拒绝即帧。
    let text = refused(&tool, json!({ "action": "create", "title": "缺时刻" })).await;
    assert!(text.contains("start_ms"), "{text}");
    let text = refused(
        &tool,
        json!({ "action": "create", "start_ms": 1, "end_ms": 2 }),
    )
    .await;
    assert!(text.contains("title"), "{text}");

    // 改期但没给待改字段 ⇒ 拒绝即帧。
    let text = refused(&tool, json!({ "action": "update", "entry_id": "dentist" })).await;
    assert!(text.contains("待改字段"), "{text}");

    // 取消不存在的条目 ⇒ 拒绝即帧。
    let text = refused(&tool, json!({ "action": "cancel", "entry_id": "ghost" })).await;
    assert!(text.contains("不存在"), "{text}");
}

// ---------------------------------------------------------------------------
// ⑬ 开关: fail-closed (只认显式 "1") + 落位约定
// ---------------------------------------------------------------------------

#[test]
fn the_switch_is_fail_closed_except_the_exact_opt_in() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let restore = std::env::var(CALENDAR_ENABLE_ENV).ok();

    for bad in ["", "0", "true", "yes", "on", "01", "1 1"] {
        std::env::set_var(CALENDAR_ENABLE_ENV, bad);
        assert!(!calendar_enabled_from_env(), "开关必须拒绝 {bad:?}");
        assert!(
            CalendarConfig::from_env(temp_data_dir("switch")).is_none(),
            "开关关 ⇒ 配置不构造 (注册即闸)"
        );
    }

    std::env::set_var(CALENDAR_ENABLE_ENV, "1");
    assert!(calendar_enabled_from_env(), "显式 1 必须开");
    let config = CalendarConfig::from_env(temp_data_dir("switch")).expect("配置构造");
    assert_eq!(
        config.store().dir(),
        temp_data_dir("switch").join(CALENDAR_DIR_NAME)
    );

    std::env::set_var(CALENDAR_ENABLE_ENV, " 1 ");
    assert!(calendar_enabled_from_env(), "前后空白容忍");

    match restore {
        Some(value) => std::env::set_var(CALENDAR_ENABLE_ENV, value),
        None => std::env::remove_var(CALENDAR_ENABLE_ENV),
    }
}

// ---------------------------------------------------------------------------
// ⑭ 条目形状: 状态标签 / 审计行身份戳 / 审批理由
// ---------------------------------------------------------------------------

#[test]
fn entry_status_labels_and_audit_identity_are_pinned() {
    assert_eq!(CalendarStatus::Scheduled.as_str(), "scheduled");
    assert_eq!(CalendarStatus::Cancelled.as_str(), "cancelled");
    assert_eq!(CALENDAR_AUDIT_DOC_NAME, "apeireth.calendar_audit");
    assert_eq!(CALENDAR_AUDIT_FILE, "audit.jsonl");
    assert_eq!(CALENDAR_LEDGER_FILE, "entries.jsonl");

    let reason = calendar_approval_reason(Some("create"));
    assert!(reason.contains("写主人日程账"), "{reason}");
    assert!(reason.contains("逐次人批"), "{reason}");
    let reason = calendar_approval_reason(Some("list"));
    assert!(reason.contains("只读日程账"), "{reason}");
    let reason = calendar_approval_reason(None);
    assert!(reason.contains("fail-closed"), "{reason}");

    // 条目结构可序列化 (账上行形态稳定)。
    let entry = CalendarEntry {
        id: "dentist".to_string(),
        title: "看牙".to_string(),
        start_ms: 1_700_000_000_000,
        end_ms: 1_700_000_003_600,
        note: Some("带上牙医名片".to_string()),
        status: CalendarStatus::Scheduled,
        created_at_ms: 1_700_000_000_000,
        updated_at_ms: 1_700_000_000_000,
    };
    let row = CalendarLedgerEntry::Created(entry);
    let encoded = serde_json::to_value(&row).expect("序列化");
    assert_eq!(encoded["event"], "created");
    assert_eq!(encoded["id"], "dentist");
    assert_eq!(encoded["status"], "scheduled");
}
