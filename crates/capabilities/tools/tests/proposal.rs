//! 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`) 回归: 提案生成四 kind /
//! 只入账不落真面 (工作区零变化) / IM 审批卡温柔呈现 (含「建议你亲自确认」行) /
//! accepted 标记不执行 (执行仍走既有工具面) / 账 append-only / 开关 fail-closed /
//! 拒绝即帧 / 列表真值 / 自述提案节同账。

/// 本集成测试二进制内的 env 锁（进程级全局量 ⇒ 串行）。
/// 放在**模块级**（不在测试函数内）：同文件多个测试函数共用同一把。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::proposal::{
    proposals_enabled_from_env, ProposalBoundaryGuard, ProposalConfig, ProposalDecision,
    ProposalDraft, ProposalLedger, ProposalLedgerEntry, ProposalReview, ProposalState,
    ProposalTool, PROPOSALS_ENABLE_ENV, PROPOSAL_ACTIONS, PROPOSAL_CAPABILITY_ID,
    PROPOSAL_EVIDENCE_HIGHLIGHT, PROPOSAL_NO_EXECUTION_NOTE, PROPOSAL_PERSONAL_CONFIRM_MARKER,
    PROPOSAL_REVIEW_TONE,
};
use apeireth_tools_canonical::proposal_push::{
    ProposalPushLedger, ProposalPushOutcome, ProposalPushRecord,
};
use apeireth_tools_canonical::{capability_roster, render_snapshot, SelfStatusSnapshot};
use serde_json::{json, Value};

/// 工具调用一帧 (按动作拼参数)。
fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: "proposal_draft".into(),
        arguments,
    }
}

/// 临时目录 (每测试一个, 落系统临时区)。
fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-proposal-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 工具 + 提案账 (账本落在数据目录, 工作区另置 —— 两边必须分开测)。
fn tool_with(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, ProposalTool) {
    let root = temp_root(tag);
    let workspace = root.join("workspace");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&workspace).expect("创建工作区");
    std::fs::create_dir_all(&data_dir).expect("创建数据目录");
    // 工作区里放几个真文件, 零变化断言要盯住它们。
    std::fs::write(workspace.join("a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(workspace.join("b.md"), "# b\n").unwrap();
    std::fs::create_dir_all(workspace.join("src")).unwrap();
    std::fs::write(workspace.join("src").join("c.rs"), "fn c() {}\n").unwrap();
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&data_dir));
    (workspace, data_dir, tool)
}

/// 目录内容快照 (相对路径 + 字节): 零变化断言的对比基线。
fn snapshot_dir(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("读目录") {
            let entry = entry.expect("目录项");
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .expect("相对路径")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, std::fs::read(&path).expect("读文件")));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// 一份合法草稿参数 (kind 可换)。
fn draft_arguments(kind: &str, title: &str) -> Value {
    json!({
        "action": "draft",
        "kind": kind,
        "title": title,
        "detail": "这几处是重复劳动留下的边角料。",
        "evidence": ["src/a.rs:12 临时注释", "src/c.rs:3 重复片段", "b.md 旧段落"],
        "draft_payload": {"files": ["src/a.rs", "src/c.rs", "b.md"]},
    })
}

async fn ok_value(tool: &ProposalTool, arguments: Value) -> Value {
    let result = tool.invoke(&call(arguments)).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("结构化 json")
}

async fn refused(tool: &ProposalTool, arguments: Value) -> String {
    let result = tool.invoke(&call(arguments)).await;
    assert!(!result.is_ok(), "必须拒绝即帧: {}", result.render());
    result.render()
}

/// ① 提案生成覆盖四个 kind: 逐 kind 入账, kind 标签稳定, id 唯一。
#[tokio::test]
async fn draft_covers_all_four_kinds() {
    let (_workspace, _data_dir, tool) = tool_with("four-kinds");
    for (kind, title) in [
        ("tidy", "待整理: 三个临时文件该归档"),
        ("fix_suggest", "小错漏: 一处拼写修正建议"),
        ("summarize", "重复劳动: 三段日志可合并摘要"),
        ("list", "清单: 待查项五条"),
    ] {
        let value = ok_value(&tool, draft_arguments(kind, title)).await;
        assert_eq!(value["action"], "draft");
        assert_eq!(value["status"], "proposed");
        assert_eq!(value["proposal"]["kind"], kind, "kind 标签必须稳定");
        assert_eq!(value["proposal"]["title"], title);
        let id = value["proposal"]["id"].as_str().expect("id 在场");
        assert!(id.starts_with("prop-"), "提案身份前缀: {id}");
    }
    // 账上四条草稿行, kind 全齐。
    let scan = tool.ledger().scan().expect("账可读");
    let kinds: Vec<String> = scan
        .records
        .iter()
        .map(|record| match &record.body {
            ProposalLedgerEntry::Draft(draft) => draft.kind.as_str().to_string(),
            ProposalLedgerEntry::Review(_) => panic!("本用例无决定行"),
        })
        .collect();
    assert_eq!(kinds, ["tidy", "fix_suggest", "summarize", "list"]);
}

/// ② 只入账不落真面: 起草后**工作区逐字节零变化**, 工作区里也没有提案账。
#[tokio::test]
async fn drafting_lands_only_in_the_ledger_and_the_workspace_stays_byte_identical() {
    let (workspace, data_dir, tool) = tool_with("zero-change");
    let before = snapshot_dir(&workspace);
    let value = ok_value(&tool, draft_arguments("tidy", "待整理: 归档")).await;
    assert_eq!(value["status"], "proposed");
    let after = snapshot_dir(&workspace);
    assert_eq!(before, after, "提案不落任何真面: 工作区必须逐字节零变化");
    // 真面里只有提案账一行 (append-only 落数据目录)。
    let data_entries: Vec<String> = std::fs::read_dir(&data_dir)
        .expect("数据目录可读")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(data_entries, ["proposals.jsonl"], "只入提案账, 不落别的面");
    assert!(
        !workspace.join("proposals.jsonl").exists(),
        "提案账不进工作区"
    );
}

/// ③ IM 审批卡呈现: detail 恒含「建议你亲自确认」行 + 温柔语气结构
/// (「标出的 N 处你最好亲自看一遍」) + 证据逐条「请亲自过目」+ 卡面声明不执行。
#[tokio::test]
async fn im_review_card_carries_the_personal_confirm_line_and_the_gentle_tone() {
    let (_workspace, _data_dir, tool) = tool_with("im-card");
    let value = ok_value(&tool, draft_arguments("fix_suggest", "小错漏: 拼写修正")).await;
    let card = value["card"].as_str().expect("卡呈现文本在场");
    assert!(
        card.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "卡面必须含「建议你亲自确认」行: {card}"
    );
    assert!(
        value["proposal"]["detail"]
            .as_str()
            .expect("detail 在场")
            .contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "detail 必含「建议你亲自确认」行"
    );
    assert!(
        card.contains(PROPOSAL_REVIEW_TONE),
        "温柔语气结构 (你最好亲自看一遍): {card}"
    );
    assert!(card.contains("标出的 3 处"), "点名处数: {card}");
    assert!(
        card.matches(PROPOSAL_EVIDENCE_HIGHLIGHT).count() >= 3,
        "证据逐条标黄请人过目: {card}"
    );
    // 卡载荷结构面: 风险档低 + 批准/拒绝两按钮 + 不执行声明。
    let payload = &value["card_payload"];
    assert_eq!(payload["risk"], "low");
    assert_eq!(payload["buttons"], json!(["approve", "reject"]));
    assert_eq!(value["executed"], false);
    assert_eq!(value["execute_note"], PROPOSAL_NO_EXECUTION_NOTE);
}

/// ④ 温柔补行: detail 缺「建议你亲自确认」行也**入账即含** (结构性保证, 不靠
/// 模型记得写); 已含则原样保留不重复。
#[tokio::test]
async fn detail_without_the_confirm_line_gets_it_before_it_lands() {
    let (_workspace, _data_dir, tool) = tool_with("confirm-line");
    let mut arguments = draft_arguments("tidy", "待整理");
    arguments["detail"] = json!("没写确认行的详情。");
    let value = ok_value(&tool, arguments).await;
    let landed = value["proposal"]["detail"].as_str().expect("detail");
    assert!(landed.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER));
    let scan = tool.ledger().scan().expect("账可读");
    match &scan.records[0].body {
        ProposalLedgerEntry::Draft(draft) => {
            assert!(draft.detail.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER));
        }
        ProposalLedgerEntry::Review(_) => panic!("应是草稿行"),
    }
    // 已含标记 = 原样保留 (不重复贴行)。
    let mut arguments = draft_arguments("tidy", "待整理二");
    arguments["detail"] = json!("详情。\n建议你亲自确认: 我自己写的确认行。");
    let value = ok_value(&tool, arguments).await;
    let landed = value["proposal"]["detail"].as_str().expect("detail");
    assert_eq!(
        landed.matches(PROPOSAL_PERSONAL_CONFIRM_MARKER).count(),
        1,
        "确认行只贴一次: {landed}"
    );
}

/// ⑤ accepted 标记: 点头 = 账面标记 accepted + `executed: false` + 执行仍走
/// 既有工具面的正常审批链 (工作区零变化, 无任何执行发生)。
#[tokio::test]
async fn accept_marks_accepted_without_executing_and_points_at_the_existing_chain() {
    let (workspace, _data_dir, tool) = tool_with("accept");
    let drafted = ok_value(&tool, draft_arguments("tidy", "待整理")).await;
    let id = drafted["proposal"]["id"].as_str().expect("id").to_string();
    let before = snapshot_dir(&workspace);

    let value = ok_value(
        &tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner", "note": "亲自看过, 点头"}),
    )
    .await;
    assert_eq!(value["action"], "accept");
    assert_eq!(value["status"], "accepted");
    assert_eq!(value["proposal_id"], id);
    assert_eq!(value["decided_by"], "owner");
    // 提案不直接执行: 结果帧显式不执行 + 指向既有工具面的正常审批链。
    assert_eq!(value["executed"], false);
    let note = value["execute_note"].as_str().expect("执行说明在场");
    assert!(
        note.contains("existing tool faces"),
        "执行仍走既有工具面: {note}"
    );
    assert_eq!(
        before,
        snapshot_dir(&workspace),
        "标记 accepted 不落任何真面"
    );
    // 账面状态闭合: 草稿行 + 决定行, 状态 accepted。
    assert_eq!(
        tool.ledger().state_of(&id).expect("状态可读"),
        Some(ProposalState::Accepted)
    );
    let scan = tool.ledger().scan().expect("账可读");
    assert_eq!(scan.records.len(), 2);
    match &scan.records[1].body {
        ProposalLedgerEntry::Review(review) => {
            assert_eq!(review.decision, ProposalDecision::Accepted);
            assert_eq!(review.decided_by, "owner");
        }
        ProposalLedgerEntry::Draft(_) => panic!("应是决定行"),
    }
}

/// ⑥ 一提案一决定: 摇头/点头都只记一次; 重复决定与不存在的提案都拒绝即帧。
#[tokio::test]
async fn one_proposal_gets_exactly_one_decision() {
    let (_workspace, _data_dir, tool) = tool_with("one-decision");
    let drafted = ok_value(&tool, draft_arguments("list", "清单")).await;
    let id = drafted["proposal"]["id"].as_str().expect("id").to_string();

    let value = ok_value(
        &tool,
        json!({"action": "reject", "proposal_id": id, "decided_by": "owner"}),
    )
    .await;
    assert_eq!(value["status"], "rejected");
    assert_eq!(value["executed"], false);

    // 二次决定 (点头) = 拒绝即帧, 不改史。
    let frame = refused(
        &tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
    )
    .await;
    assert!(frame.contains("rejected"), "真因如实: {frame}");
    assert_eq!(
        tool.ledger().state_of(&id).expect("状态可读"),
        Some(ProposalState::Rejected),
        "拒绝后状态不被翻回"
    );
    // 不存在的提案 = 拒绝即帧。
    let frame = refused(
        &tool,
        json!({"action": "accept", "proposal_id": "prop-none-1", "decided_by": "owner"}),
    )
    .await;
    assert!(frame.contains("不在提案账上"), "真因如实: {frame}");
}

/// ⑦ 账 append-only: 追加不重写不删行 —— 追加前的字节前缀逐字节不变。
#[tokio::test]
async fn the_proposal_ledger_is_append_only() {
    let (_workspace, _data_dir, tool) = tool_with("append-only");
    let drafted = ok_value(&tool, draft_arguments("tidy", "待整理")).await;
    let id = drafted["proposal"]["id"].as_str().expect("id").to_string();
    let ledger_path = tool.ledger().path().to_path_buf();
    let before = std::fs::read(&ledger_path).expect("读账");

    ok_value(
        &tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
    )
    .await;
    ok_value(&tool, draft_arguments("summarize", "重复劳动")).await;

    let after = std::fs::read(&ledger_path).expect("读账");
    assert!(after.len() > before.len(), "只追加: 新行必须加长账本");
    assert_eq!(
        &after[..before.len()],
        &before[..],
        "append-only: 旧字节前缀逐字节不变 (不重写不删行)"
    );
}

/// ⑧ 开关 fail-closed: `APEIRETH_ENABLE_PROPOSALS` 只认显式 "1" (未设/0/杂值
/// 全关), 且 `ProposalConfig::from_env` 同口径。
#[test]
fn the_enable_switch_is_fail_closed_except_the_exact_one() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // 串行护住 env 读写 (进程级全局量, 同二进制内其它用例不读这把开关)。
    assert!(ProposalConfig::from_env(".").is_none());
    for value in ["0", "2", "true", "yes", ""] {
        std::env::set_var(PROPOSALS_ENABLE_ENV, value);
        assert!(!proposals_enabled_from_env(), "{value:?} 必须是关");
        assert!(
            ProposalConfig::from_env(".").is_none(),
            "{value:?} 必须不装配"
        );
    }
    std::env::set_var(PROPOSALS_ENABLE_ENV, "1");
    assert!(proposals_enabled_from_env(), "显式 1 才开");
    assert!(ProposalConfig::from_env(".").is_some());
    std::env::remove_var(PROPOSALS_ENABLE_ENV);
}

/// ⑨ 拒绝即帧: 空标题 / 空证据 / 非对象 payload / 词表外 kind / 词表外动作 /
/// 缺动作 —— 全部带真因拒绝, 不静默降级, 账上不留半帧。
#[tokio::test]
async fn bad_requests_are_refused_with_real_frames() {
    let (_workspace, _data_dir, tool) = tool_with("refusals");
    let mut bad = draft_arguments("tidy", " ");
    let frame = refused(&tool, bad.clone()).await;
    assert!(frame.contains("标题"), "真因如实: {frame}");

    bad = draft_arguments("tidy", "好标题");
    bad["evidence"] = json!([]);
    let frame = refused(&tool, bad.clone()).await;
    assert!(frame.contains("证据"), "真因如实: {frame}");

    bad = draft_arguments("tidy", "好标题");
    bad["draft_payload"] = json!(["不是对象"]);
    let frame = refused(&tool, bad.clone()).await;
    assert!(frame.contains("draft_payload"), "真因如实: {frame}");

    bad = draft_arguments("rm -rf", "好标题");
    let frame = refused(&tool, bad.clone()).await;
    assert!(frame.contains("closed vocabulary"), "闭合词表: {frame}");

    let frame = refused(&tool, json!({"action": "explode"})).await;
    assert!(frame.contains("closed vocabulary"), "闭合动作词表: {frame}");

    let frame = refused(&tool, json!({"kind": "tidy"})).await;
    assert!(frame.contains("action"), "缺动作拒绝: {frame}");

    // 全部拒绝后账上零行 (无半帧)。
    let scan = tool.ledger().scan().expect("账可读");
    assert!(scan.records.is_empty(), "拒绝不留痕");
}

/// ⑩ 列表真值: `list` 报账上真计数 (待审数 / accepted / rejected / total) 与
/// 最近提案 (新在前, 不回正文)。
#[tokio::test]
async fn list_reports_the_true_counts_of_the_ledger() {
    let (_workspace, _data_dir, tool) = tool_with("list");
    let first = ok_value(&tool, draft_arguments("tidy", "待整理甲")).await;
    let first_id = first["proposal"]["id"].as_str().unwrap().to_string();
    ok_value(&tool, draft_arguments("summarize", "重复劳动乙")).await;
    ok_value(
        &tool,
        json!({"action": "accept", "proposal_id": first_id, "decided_by": "owner"}),
    )
    .await;

    let value = ok_value(&tool, json!({"action": "list"})).await;
    assert_eq!(value["action"], "list");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["pending"], 1);
    assert_eq!(value["accepted"], 1);
    assert_eq!(value["rejected"], 0);
    assert_eq!(value["total"], 2);
    let recent = value["recent"].as_array().expect("最近提案在场");
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0]["title"], "重复劳动乙", "新提案在前");
    assert_eq!(recent[0]["state"], "pending");
    assert_eq!(recent[1]["state"], "accepted");
    assert!(
        recent[0].get("detail").is_none() && recent[0].get("draft_payload").is_none(),
        "最近提案不回正文"
    );
}

/// ⑪ 自述提案节与工具面同账同源: 待审数 / 分态计数 / 最近提案读同一本
/// `proposals.jsonl` 的真值 (render_snapshot 帧形)。
#[tokio::test]
async fn the_self_status_proposal_section_reads_the_same_ledger() {
    let (_workspace, data_dir, tool) = tool_with("self-status");
    let drafted = ok_value(&tool, draft_arguments("fix_suggest", "小错漏丙")).await;
    let id = drafted["proposal"]["id"].as_str().expect("id").to_string();
    ok_value(&tool, draft_arguments("list", "清单丁")).await;
    ok_value(
        &tool,
        json!({"action": "reject", "proposal_id": id, "decided_by": "owner"}),
    )
    .await;

    let status = ProposalLedger::under_data_dir(&data_dir)
        .status()
        .expect("提案节可探测");
    assert_eq!(status.pending, 1);
    assert_eq!(status.rejected, 1);
    assert_eq!(status.total, 2);
    assert_eq!(status.recent.len(), 2);
    assert_eq!(status.recent[1].id, id);

    // 帧形: 自述渲染面把提案节投影成 {pending, accepted, rejected, total, recent}。
    let mut snapshot = apeireth_tools_canonical::SelfStatusSnapshot::minimal(
        Default::default(),
        apeireth_tools_canonical::capability_roster([("proposals", true)]),
    );
    snapshot.proposal = Some(status);
    let frame = render_snapshot(&snapshot);
    assert_eq!(frame["proposal"]["pending"], 1);
    assert_eq!(frame["proposal"]["rejected"], 1);
    assert_eq!(frame["proposal"]["recent"][1]["id"], id);
}

/// ⑫ 单调边界 Guard 只拒不放: 同样的坏形状在 Guard 层也拒 (任何后续阶段都
/// 不能翻回放行), 好形状零拦截。
#[test]
fn the_boundary_guard_refuses_the_same_shapes_monotonically() {
    let guard = ProposalBoundaryGuard::new();
    let capability = apeireth_core::kernel::CapabilityId::new(PROPOSAL_CAPABILITY_ID).unwrap();

    let empty_title = json!({"action": "draft", "kind": "tidy", "title": " ", "detail": "d", "evidence": ["e"], "draft_payload": {}});
    let deny = guard.deny(&ToolGuardRequest::new(
        &capability,
        "proposal_draft",
        &empty_title,
    ));
    assert!(deny.is_some(), "空标题在 Guard 层也拒");

    let empty_decider = json!({"action": "accept", "proposal_id": "prop-1", "decided_by": " "});
    let deny = guard.deny(&ToolGuardRequest::new(
        &capability,
        "proposal_draft",
        &empty_decider,
    ));
    assert!(deny.is_some(), "空拍板人在 Guard 层也拒");

    let good_draft = json!({"action": "draft", "kind": "tidy", "title": "t", "detail": "d", "evidence": ["e"], "draft_payload": {"a": 1}});
    let deny = guard.deny(&ToolGuardRequest::new(
        &capability,
        "proposal_draft",
        &good_draft,
    ));
    assert!(deny.is_none(), "好形状零拦截");

    let listing = json!({"action": "list"});
    let deny = guard.deny(&ToolGuardRequest::new(
        &capability,
        "proposal_draft",
        &listing,
    ));
    assert!(deny.is_none(), "列表零拦截");
}

/// 动作词表闭合 (draft / accept / reject / list), 供声明面与治理面同词。
#[test]
fn the_action_vocabulary_is_closed_and_named_honestly() {
    assert_eq!(PROPOSAL_ACTIONS, ["draft", "accept", "reject", "list"]);
}

// ---------------------------------------------------------------------------
// 回归 (提案面 fuzz 实测, 2026-10-25): 账本追加的**行完整性**与**一提案一决定**
// ---------------------------------------------------------------------------

/// 极简执行器 (工具面 `invoke` 的体是同步的, 一次 poll 即完成; 不引新依赖)。
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = std::task::Waker::noop();
    let mut context = std::task::Context::from_waker(waker);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => return value,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 账本上的行 (提案 id 列表)。
fn ledger_draft_ids(ledger: &ProposalLedger) -> Vec<String> {
    ledger
        .scan()
        .expect("账可读")
        .records
        .iter()
        .filter_map(|record| match &record.body {
            ProposalLedgerEntry::Draft(draft) => Some(draft.id.clone()),
            ProposalLedgerEntry::Review(_) => None,
        })
        .collect()
}

/// 账本上的决定行。
fn ledger_review_rows(ledger: &ProposalLedger) -> Vec<ProposalReview> {
    ledger
        .scan()
        .expect("账可读")
        .records
        .iter()
        .filter_map(|record| match &record.body {
            ProposalLedgerEntry::Review(review) => Some(review.clone()),
            ProposalLedgerEntry::Draft(_) => None,
        })
        .collect()
}

/// 回归 ①: 撕裂写 (末行缺换行) 之后追加 ⇒ **追加不许把两行挤成一行**。
///
/// 契约: `proposal.rs` 账本头「**append-only**: 逐行一个 [`StoredDoc`] 信封,
/// 追加写 + 落盘, 不重写、不删行」。
///
/// 缺陷原文 (修复前): 行体与行尾换行是**两次写** ⇒ ① 上个进程死在两次写之间
/// (末行缺换行) 时, 下一次追加会把新行**粘在旧行尾** ⇒ 旧行被毁 + 新行被吞
/// (两行都读作不存在 ⇒ 等于删行); ② 两个并发写者会把两行挤成一行 (同样丢行)。
/// 修法: 行体与换行**同在一次写** (O_APPEND 追加是原子的) + 末行缺换行时先补
/// 一个换行 (只追加字节, append-only 不变)。
#[test]
fn appending_after_a_torn_line_keeps_every_row_readable() {
    let root = temp_root("torn-append");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("创建数据目录");
    let ledger = ProposalLedger::under_data_dir(&data_dir);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&data_dir));

    let first = block_on(tool.invoke(&call(draft_arguments("tidy", "待整理甲"))));
    let first_id = serde_json::from_str::<Value>(&first.render()).expect("帧可解析")["proposal"]
        ["id"]
        .as_str()
        .expect("id 在场")
        .to_string();
    let second = block_on(tool.invoke(&call(draft_arguments("tidy", "待整理乙"))));
    let second_id = serde_json::from_str::<Value>(&second.render()).expect("帧可解析")["proposal"]
        ["id"]
        .as_str()
        .expect("id 在场")
        .to_string();

    // 模拟撕裂写: 去掉末行换行 (上个进程死在「写行体」与「写换行」之间)。
    // 行体本身完整 ⇒ 那一行仍是一条完整记录 (只是行尾换行没落)。
    let path = ledger.path().to_path_buf();
    let mut bytes = std::fs::read(&path).expect("读账");
    assert!(bytes.ends_with(b"\n"), "正常追加末字节必是换行");
    bytes.pop();
    std::fs::write(&path, &bytes).expect("写撕裂账");

    let third = block_on(tool.invoke(&call(draft_arguments("tidy", "待整理丙"))));
    let third_id = serde_json::from_str::<Value>(&third.render()).expect("帧可解析")["proposal"]
        ["id"]
        .as_str()
        .expect("id 在场")
        .to_string();

    let scan = ledger.scan().expect("账可读");
    let ids = ledger_draft_ids(&ledger);
    assert!(
        ids.contains(&first_id),
        "撕裂写不许毁掉前面那条好行: {ids:?}"
    );
    assert!(
        ids.contains(&second_id),
        "撕裂那行本身是完整记录, 不许被下一行吞掉: {ids:?}"
    );
    assert!(
        ids.contains(&third_id),
        "撕裂写后追加的新行必须是自己那一行: {ids:?}"
    );
    assert_eq!(scan.skipped, 0, "三行都是完整记录, 不该有坏行: {ids:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 回归 ②: 并发追加不许丢行 (append-only 的「不删行」在并发下也成立)。
///
/// 契约: 同上 (「逐行一个 [`StoredDoc`] 信封 ... 不重写、不删行」)。
///
/// 缺陷原文 (修复前, 实测 32 行只读到 12 行): 行体与换行分两次写 ⇒ 两个写者
/// 的「行体」与「换行」交错 ⇒ 两行挤成一行 ⇒ 那两行都读作不存在。
#[test]
fn concurrent_appends_keep_every_row() {
    let root = temp_root("concurrent-append");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("创建数据目录");
    let ledger = std::sync::Arc::new(ProposalLedger::under_data_dir(&data_dir));
    let threads = 4usize;
    let per_thread = 8usize;
    let mut handles = Vec::new();
    for thread in 0..threads {
        let ledger = ledger.clone();
        handles.push(std::thread::spawn(move || {
            for index in 0..per_thread {
                let draft = ProposalDraft {
                    id: format!("prop-t{thread}-{index}"),
                    kind: apeireth_tools_canonical::proposal::ProposalKind::Tidy,
                    title: "并发追加".to_string(),
                    detail: "并发追加的详情".to_string(),
                    evidence: vec![format!("src/{thread}-{index}.rs:1")],
                    draft_payload: json!({}),
                    created_at_ms: 1_700_000_000_000 + index as i64,
                };
                ledger.append_draft(&draft).expect("并发追加");
            }
        }));
    }
    for handle in handles {
        handle.join().expect("线程回收");
    }
    let scan = ledger.scan().expect("账可读");
    assert_eq!(
        scan.records.len(),
        threads * per_thread,
        "并发追加不许丢行: records={} skipped={}",
        scan.records.len(),
        scan.skipped
    );
    assert_eq!(scan.skipped, 0, "并发追加不许产生坏行");
    let _ = std::fs::remove_dir_all(&root);
}

/// 回归 ③: 并发「点头」同一提案 ⇒ 账上只有一条决定行 (一提案一决定不许被并发破掉)。
///
/// 契约: `proposal.rs` 工具描述「one proposal gets exactly one decision」+
/// 状态机注释「一提案一决定, 决定只追加不改史」。
///
/// 缺陷原文 (修复前, 实测 4 个并发点头 ⇒ 4 个「点头了」帧 + 账上 2 条决定行):
/// `decide` 是「读状态 → 落决定」两步且两步之间没有互斥 ⇒ 并发点头都能读到
/// `Pending` ⇒ 同一提案落两条决定行, 且其中一条被行合并吞掉 (帧说点头了而账上没有)。
/// 修法: 「查状态 → 落决定」放进进程级互斥 (进程内口径)。
#[test]
fn concurrent_decisions_keep_one_proposal_one_decision() {
    let root = temp_root("concurrent-decide");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("创建数据目录");
    let ledger = ProposalLedger::under_data_dir(&data_dir);
    let tool = std::sync::Arc::new(ProposalTool::new(ProposalLedger::under_data_dir(&data_dir)));
    let drafted = block_on(tool.invoke(&call(draft_arguments("tidy", "待整理"))));
    let id = serde_json::from_str::<Value>(&drafted.render()).expect("帧可解析")["proposal"]["id"]
        .as_str()
        .expect("id 在场")
        .to_string();

    let threads = 4usize;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(threads));
    let mut handles = Vec::new();
    for index in 0..threads {
        let tool = tool.clone();
        let barrier = barrier.clone();
        let id = id.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            block_on(tool.invoke(&call(json!({
                "action": "accept",
                "proposal_id": id,
                "decided_by": "owner",
                "note": format!("并发点头 {index}"),
            }))))
        }));
    }
    let mut accepted = 0usize;
    for handle in handles {
        let result = handle.join().expect("线程回收");
        if result.is_ok() {
            accepted += 1;
        } else {
            let frame = result.render();
            assert!(
                frame.contains("一提案一决定"),
                "第二个决定必被拒且真因如实: {frame}"
            );
        }
    }
    assert_eq!(
        ledger_review_rows(&ledger).len(),
        1,
        "一提案一决定: 并发点头也只能落一条决定行"
    );
    assert_eq!(accepted, 1, "并发点头只许一个成功帧 (其余必被拒)");
    assert_eq!(
        ledger.state_of(&id).expect("状态可读"),
        Some(ProposalState::Accepted),
        "状态 = 落账的决定"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 回归 ④: 推送账同姿势 —— 撕裂写之后追加不许把两行挤成一行。
///
/// 契约: `proposal_push.rs` 账本头「**append-only**: 逐行一个 [`StoredDoc`] 信封,
/// 追加写 + 落盘, 不重写、不删行 ... 坏行读作不存在 (跳过并计入 `skipped`),
/// 不砖化整本」+ 推送账与提案账「同一姿势」。
#[test]
fn appending_to_the_push_ledger_after_a_torn_line_keeps_every_row_readable() {
    let root = temp_root("torn-push");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("创建数据目录");
    let push = ProposalPushLedger::under_data_dir(&data_dir);
    let record = |id: &str| ProposalPushRecord {
        proposal_id: id.to_string(),
        channel_id: "primary".to_string(),
        conversation_id: "oc_demo".to_string(),
        outcome: ProposalPushOutcome::Pushed,
        message_ref: Some("m-out".to_string()),
        reason: None,
        at_ms: 1_700_000_000_000,
    };
    push.append(&record("prop-1")).expect("推送 1");
    // 模拟撕裂写: 去掉末行换行。
    let path = push.path().to_path_buf();
    let mut bytes = std::fs::read(&path).expect("读推送账");
    bytes.pop();
    std::fs::write(&path, &bytes).expect("写撕裂推送账");
    push.append(&record("prop-2")).expect("推送 2");

    let scan = push.scan().expect("推送账可读");
    assert_eq!(scan.records.len(), 2, "撕裂写后追加不许丢行: {scan:?}");
    assert_eq!(scan.skipped, 0, "不许产生坏行: {scan:?}");
    let ids = push.pushed_ids().expect("幂等集可读");
    assert!(
        ids.contains("prop-1") && ids.contains("prop-2"),
        "两行都在: {ids:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
