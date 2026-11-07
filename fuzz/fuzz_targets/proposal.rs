#![no_main]
// 仓根 `clippy.toml` 禁 `std::fs::write` / `create_dir_all` 等直调 —— 那条规矩是给
// **产品代码**用的（写盘必须走原子写助手）。fuzz harness 的职责恰恰是**真的落盘**
// 去喂被测账本的 IO 路径（追加 / 撕裂 / 坏行 / 并发），故在此显式豁免并说明理由。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

//! fuzz: **提案工具面**（`apeireth_tools_canonical::proposal`，`tool.proposal_draft`）
//! + **提案推送账**（`apeireth_tools_canonical::proposal_push`）。
//!
//! **面**：`crates/capabilities/tools/src/proposal.rs`（提案账写入 / 决定落账 /
//! 人审卡载荷）+ `crates/capabilities/tools/src/proposal_push.rs`（推送账幂等 /
//! 失败留账 / 坏行容错）。
//!
//! **与 `im_payload` target 的分工**：`im_payload` 覆盖 **gateway 侧**
//! (`im_proposal.rs` 的卡片信封 / 按钮载荷 / 待推送选择)。本 target 覆盖 **工具面
//! 本体**：提案账追加、决定落账、人审卡载荷构造、推送账。
//!
//! ## 判据全部取自被测模块自己写明的契约（逐条出处标在断言旁的 `proposal.rs:` 注释里）
//!
//! 1. **先斩后奏 = 不存在**：模块头「**先斩后奏 = 不存在**: 提案只入提案账
//!    ([`ProposalLedger`], `proposals.jsonl` append-only), **不落任何真面**
//!    (工作区零文件系统写, 记忆 / 任务簿 / git 一概不碰)」
//!    （`proposal.rs:6-8`）⇒ 起草后工作区**逐字节零变化**；数据目录里只多出提案账
//!    一个文件（工具本身**不推送** ⇒ 不许多出推送账）。
//! 2. **提案不直接执行**：模块头「真正的执行永远走既有工具面 ... 的正常审批链 ——
//!    本工具**无执行权**, 结果帧显式 `executed: false`」（`proposal.rs:10-11`）
//!    ⇒ 每一个成功帧都必须显式带 `executed: false` + 不执行说明。
//! 3. **一提案一决定**：工具描述「one proposal gets exactly one decision」
//!    （`proposal.rs:102`）+ 状态机注释「提案状态机 (一提案一决定, 决定只追加不改史)」
//!    （`proposal.rs:141`）⇒ 第二个决定必被拒且**账上只有一条决定行**；**并发**
//!    点头同一提案也只能落一条决定行。
//! 4. **查无提案必被拒**：`decide` 的拒绝分支「拒绝标记: 提案 {} 不在提案账上」
//!    （`proposal.rs:835`）⇒ 不存在的 id 必被拒，且**不得新建提案**（账上零新增行）。
//! 5. **缺署名必拒且账面零写**：`proposal_review_refusal`「`decided_by` 不能为空
//!    (谁拍的板要如实留痕)」（`proposal.rs:666`）+ 拒绝即帧约定「提案入账失败
//!    (未入账, 无半帧)」（`proposal.rs:777`）⇒ 署名缺失/空白 ⇒ 拒，**提案账与
//!    推送账的行数都不变**。
//! 6. **人审温柔**：`ProposalReviewCard`「detail 恒含「建议你亲自确认」行 + 证据逐条
//!    「请亲自过目」+ 「标出的 N 处你最好亲自看一遍」」（`proposal.rs:568-569`）
//!    ⇒ 卡面必含标记、每条证据行都带高亮前缀、语气行的 N == 证据条数。
//! 7. **append-only**：`ProposalLedger`「**append-only**: 逐行一个 [`StoredDoc`]
//!    信封, 追加写 + 落盘, 不重写、不删行 ... 坏行读作不存在 (跳过并计入
//!    `skipped`), 不砖化整本」（`proposal.rs:322-324`）⇒ 旧字节前缀逐字节不变；
//!    **追加不许把两行挤成一行**（撕裂写后追加、并发追加都算）。
//! 8. **推送幂等**：`ProposalPushLedger`「推送幂等键 = 提案 id ... `failed` 行不入
//!    集合 —— 失败留账待重试, 重试成功才进幂等集」（`proposal_push.rs:158-160`）
//!    +「已推送数 (成功送达的**去重提案 id** 计数)」「最近推送 (有界 5 条, 新在前;
//!    含 `failed` 留账行 —— 失败也是真值, 不藏)」（`proposal_push.rs:88-92`）。
//! 9. **提案 id 只当身份，不当路径**：id 全文只用于**逐字比较**（`proposal.rs:394`
//!    `draft.id == proposal_id`）与落账 ⇒ 路径穿越形态的 id 既不会被当路径用，
//!    也不会在账外落任何文件。
//!
//! ## 刻意不造的期望（自造期望 = 假阳性，本项目历史已栽 9 次）
//!
//! 1. **账本 id 冲突/行序被外部改动**后的 `status()` 与 `state_of()` 分歧：契约没有
//!    承诺「手改账本后两个读面必须一致」，本 target **不**断言二者一致（只分别断言
//!    各自**内部**自洽：`state_of` = 「最后一条决定行定状态」，`status` 的分态计数
//!    自洽）。
//! 2. **详情为空的草稿**：`normalize_detail` 会补标准行（`proposal.rs:246-252`
//!    明写「缺行 = 追加标准行」），所以工具面**不**把「详情为空」当拒绝理由；
//!    本 target 只断言**纯判定函数** `proposal_draft_refusal` 对空详情给拒绝
//!    （`proposal.rs:640`）。
//! 3. **幂等性**不作判据（提案面是 append-only 追加语义，不是幂等函数）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Once};
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::stored_doc::StoredDoc;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::proposal::{
    PROPOSAL_DOC_NAME, PROPOSAL_DOC_VERSION, PROPOSAL_EVIDENCE_HIGHLIGHT,
    PROPOSAL_NO_EXECUTION_NOTE, PROPOSAL_PERSONAL_CONFIRM_MARKER, PROPOSAL_REVIEW_TONE,
    PROPOSAL_TOOL_NAME, ProposalDecision, ProposalDraft, ProposalKind, ProposalLedger,
    ProposalLedgerEntry, ProposalReview, ProposalState, ProposalTool, normalize_detail,
    proposal_draft_refusal, proposal_review_refusal,
};
use apeireth_tools_canonical::proposal_push::{
    PROPOSAL_PUSH_DOC_NAME, PROPOSAL_PUSH_DOC_VERSION, PROPOSAL_PUSH_LEDGER_FILE,
    ProposalPushLedger, ProposalPushOutcome, ProposalPushRecord,
};
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};

// ===========================================================================
// 基础设施
// ===========================================================================

static SEQ: AtomicU64 = AtomicU64::new(0);

/// 本次运行独有的落盘根目录 (pid + 纳秒 + 原子计数); 建不出来 ⇒ `None`
/// (环境故障不是库的缺陷, 本轮不作断言)。
fn scratch_root(tag: &str) -> Option<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "apeireth-fuzz-proposal-{}-{nanos}-{seq}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 递归快照 (相对路径 + 字节): 「零副作用」「没落任何文件」判据的对比基线。
fn snapshot_tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, std::fs::read(&path).unwrap_or_default()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// 极简执行器 (本面无 I/O 等待, 一次 poll 即完成; 不引新依赖)。
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

/// 调一次真实工具面 (`tool.proposal_draft`)。
fn call_tool(tool: &ProposalTool, arguments: Value) -> ToolResult {
    block_on(tool.invoke(&ToolCall {
        id: "call_fuzz".to_string(),
        name: PROPOSAL_TOOL_NAME.to_string(),
        arguments,
    }))
}

/// 成功帧的值 (失败帧 ⇒ `None`)。
fn ok_value(result: &ToolResult) -> Option<Value> {
    match &result.outcome {
        ToolOutcome::Ok { value } => Some(value.clone()),
        ToolOutcome::Error { .. } => None,
    }
}

/// 失败帧的真因 (成功帧 ⇒ `None`)。
fn error_reason(result: &ToolResult) -> Option<String> {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => Some(message.clone()),
        ToolOutcome::Ok { .. } => None,
    }
}

/// 提案账的草稿行 (提案 id 列表, 保序)。
fn draft_ids(ledger: &ProposalLedger) -> Vec<String> {
    ledger
        .scan()
        .expect("提案账可读")
        .records
        .iter()
        .filter_map(|record| match &record.body {
            ProposalLedgerEntry::Draft(draft) => Some(draft.id.clone()),
            ProposalLedgerEntry::Review(_) => None,
        })
        .collect()
}

/// 提案账的决定行。
fn review_rows(ledger: &ProposalLedger) -> Vec<ProposalReview> {
    ledger
        .scan()
        .expect("提案账可读")
        .records
        .iter()
        .filter_map(|record| match &record.body {
            ProposalLedgerEntry::Review(review) => Some(review.clone()),
            ProposalLedgerEntry::Draft(_) => None,
        })
        .collect()
}

/// 提案账的总行数 (草稿 + 决定)。
fn ledger_rows(ledger: &ProposalLedger) -> usize {
    ledger.scan().expect("提案账可读").records.len()
}

/// 推送账的行数。
fn push_rows(ledger: &ProposalPushLedger) -> usize {
    ledger.scan().expect("推送账可读").records.len()
}

/// 一份合法草稿参数 (kind / title / detail / evidence / payload 全部可换)。
fn draft_arguments(kind: &str, title: &str, detail: &str, evidence: &[&str]) -> Value {
    json!({
        "action": "draft",
        "kind": kind,
        "title": title,
        "detail": detail,
        "evidence": evidence,
        "draft_payload": {"files": ["src/a.rs"]},
    })
}

// ===========================================================================
// 输入解释: 字节 → 定长字段槽 (NUL 分隔; 缺失槽 = 空串)
// ===========================================================================

struct Segments {
    slots: Vec<String>,
}

impl Segments {
    fn new(body: &[u8]) -> Self {
        let mut slots = Vec::new();
        let mut current: Vec<u8> = Vec::new();
        for byte in body {
            if *byte == 0 {
                slots.push(String::from_utf8_lossy(&current).into_owned());
                current.clear();
            } else {
                current.push(*byte);
            }
        }
        slots.push(String::from_utf8_lossy(&current).into_owned());
        Self { slots }
    }

    fn get(&self, index: usize) -> &str {
        self.slots.get(index).map(String::as_str).unwrap_or("")
    }

    fn byte(&self, index: usize) -> u8 {
        self.slots
            .get(index)
            .and_then(|slot| slot.as_bytes().first().copied())
            .unwrap_or(0)
    }
}

mod slot {
    pub const ACTION: usize = 0;
    pub const KIND: usize = 1;
    pub const TITLE: usize = 2;
    pub const DETAIL: usize = 3;
    pub const EVIDENCE_A: usize = 4;
    pub const EVIDENCE_B: usize = 5;
    pub const PAYLOAD: usize = 6;
    pub const PROPOSAL_ID: usize = 7;
    pub const DECIDED_BY: usize = 8;
    pub const NOTE: usize = 9;
    pub const CONTROL: usize = 10;
}

/// 文本变体: 0 = 原样, 1 = NUL 注入, 2 = 超长, 3 = 零宽 + RTL 覆写。
fn variant(text: &str, which: usize) -> String {
    match which % 4 {
        0 => text.to_string(),
        1 => format!("{text}\u{0}{text}"),
        2 => text.chars().take(64).collect::<String>().repeat(200),
        _ => format!("\u{200b}\u{202e}{text}\u{202c}"),
    }
}

// ===========================================================================
// 固定契约检查 (每进程一次: 这些断言靠夹具事实, 不靠随机输入)
// ===========================================================================

/// F1 起草 ⇒ 账上恰好一行草稿 + 状态 pending + detail 恒含人审行。
///
/// 契约: `proposal.rs:6-8`（提案只入提案账）+ `proposal.rs:194-197`
///（「`detail` 恒含「建议你亲自确认」行 ... `evidence` ... 至少一条」）+
/// `proposal.rs:388`（「没有决定行 = [`ProposalState::Pending`]」）。
fn f01_draft_lands_exactly_one_row() {
    let Some(root) = scratch_root("f01") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let result = call_tool(
        &tool,
        draft_arguments(
            "tidy",
            "待整理: 三处临时注释",
            "这几行注释是调试留下的。",
            &["src/a.rs:12"],
        ),
    );
    let value = ok_value(&result).expect("合法草稿必须入账");
    assert_eq!(value["status"], "proposed");
    assert_eq!(ledger_rows(&ledger), 1, "起草只落一行草稿");
    let id = value["proposal"]["id"]
        .as_str()
        .expect("提案 id 在场")
        .to_string();
    assert_eq!(
        draft_ids(&ledger),
        vec![id.clone()],
        "账上的提案 id 与帧一致"
    );
    assert_eq!(
        ledger.state_of(&id).expect("状态可读"),
        Some(ProposalState::Pending),
        "没有决定行 = 待审"
    );
    let detail = value["proposal"]["detail"].as_str().expect("detail 在场");
    assert!(
        detail.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "入账的 detail 恒含「建议你亲自确认」行: {detail}"
    );
    // 账上那一行的 detail 同样含标记 (结构性保证, 不靠模型记得写)。
    let scan = ledger.scan().expect("账可读");
    match &scan.records[0].body {
        ProposalLedgerEntry::Draft(draft) => assert!(
            draft.detail.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
            "账上 detail 恒含人审行: {}",
            draft.detail
        ),
        ProposalLedgerEntry::Review(_) => panic!("应是草稿行"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F2 一提案一决定: 第二个决定必被拒, 账上只有一条决定行。
///
/// 契约: `proposal.rs:102`（「one proposal gets exactly one decision」）+
/// `proposal.rs:141`（「一提案一决定, 决定只追加不改史」）。
fn f02_one_proposal_gets_exactly_one_decision() {
    let Some(root) = scratch_root("f02") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments("list", "清单", "详情", &["e"]),
    ))
    .expect("起草成功");
    let id = value["proposal"]["id"].as_str().expect("id").to_string();

    let accepted = call_tool(
        &tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
    );
    let accepted = ok_value(&accepted).expect("第一次决定必须落账");
    assert_eq!(accepted["status"], "accepted");
    assert_eq!(
        review_rows(&ledger).len(),
        1,
        "一提案一决定: 恰好一条决定行"
    );

    for action in ["accept", "reject"] {
        let again = call_tool(
            &tool,
            json!({"action": action, "proposal_id": id, "decided_by": "owner"}),
        );
        let reason = error_reason(&again).expect("第二个决定必被拒");
        assert!(!reason.trim().is_empty(), "拒绝即帧必须给真因: {reason}");
        assert_eq!(
            review_rows(&ledger).len(),
            1,
            "第二个决定不许改史 (append-only): action={action}"
        );
        assert_eq!(
            ledger.state_of(&id).expect("状态可读"),
            Some(ProposalState::Accepted),
            "拒绝后状态不被翻回"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F3 查无提案必被拒: 不得新建提案, 账上零新增行。
///
/// 契约: `proposal.rs:835`（「拒绝标记: 提案 {} 不在提案账上」）。
fn f03_unknown_proposal_is_refused_without_writing() {
    let Some(root) = scratch_root("f03") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let before_rows = ledger_rows(&ledger);
    let before_drafts = draft_ids(&ledger);

    for unknown in [
        "prop-none-1",
        "prop/../etc",
        "../../etc/passwd",
        "prop-1\u{0}x",
    ] {
        let result = call_tool(
            &tool,
            json!({"action": "accept", "proposal_id": unknown, "decided_by": "owner"}),
        );
        let reason = error_reason(&result).expect("查无提案必被拒");
        assert!(!reason.trim().is_empty(), "拒绝即帧必须给真因: {reason}");
        assert_eq!(
            ledger_rows(&ledger),
            before_rows,
            "查无提案 = 账面零写 (不许新建提案): id={unknown:?}"
        );
        assert_eq!(
            draft_ids(&ledger),
            before_drafts,
            "草稿集不变: id={unknown:?}"
        );
        assert_eq!(
            ledger.state_of(unknown).expect("状态可读"),
            None,
            "查无提案 = 账上没有这个 id: id={unknown:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F4 缺署名必被拒: **提案账与推送账的行数都不变**。
///
/// 契约: `proposal.rs:666`（「`decided_by` 不能为空 (谁拍的板要如实留痕)」）+
/// `proposal.rs:777`（「提案入账失败 (未入账, 无半帧)」—— 拒绝即帧 ⇒ 账面零写）。
fn f04_missing_signature_is_refused_with_zero_writes() {
    let Some(root) = scratch_root("f04") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let push = ProposalPushLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let id = value["proposal"]["id"].as_str().expect("id").to_string();

    // 推送账先落一行 (证明「推送账行数不变」这个判据不是空判据)。
    push.append(&ProposalPushRecord {
        proposal_id: id.clone(),
        channel_id: "primary".to_string(),
        conversation_id: "oc_fuzz".to_string(),
        outcome: ProposalPushOutcome::Pushed,
        message_ref: Some("m-fuzz".to_string()),
        reason: None,
        at_ms: 1_700_000_000_000,
    })
    .expect("推送账可写");

    let before_rows = ledger_rows(&ledger);
    let before_push = push_rows(&push);

    // ⚠️ 只放**契约内**的缺署名形状: 契约写的是「`decided_by` 不能为空」
    // (proposal.rs:666) ⇒ 判据是 `trim().is_empty()`。纯 NUL 不是空白 ⇒ 不在
    // 「空」的口径内 (那是 harness 自造期望, 已栽一次)。
    for decided_by in [
        json!(null),
        json!(""),
        json!("   "),
        json!("\u{2003}\u{3000}"),
    ] {
        let mut arguments = json!({"action": "accept", "proposal_id": id});
        if !decided_by.is_null() {
            arguments["decided_by"] = decided_by.clone();
        }
        let result = call_tool(&tool, arguments);
        let reason = error_reason(&result).expect("缺署名必被拒");
        assert!(
            reason.contains("decided_by") || reason.contains("拍板"),
            "真因如实 (指名道姓): {reason}"
        );
        assert_eq!(
            ledger_rows(&ledger),
            before_rows,
            "缺署名 = 提案账零写: decided_by={decided_by}"
        );
        assert_eq!(
            push_rows(&push),
            before_push,
            "缺署名 = 推送账零写: decided_by={decided_by}"
        );
        assert_eq!(
            ledger.state_of(&id).expect("状态可读"),
            Some(ProposalState::Pending),
            "缺署名不许把提案拍成已决"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F5 结果帧显式 `executed: false` (+ 不执行说明同文)。
///
/// 契约: `proposal.rs:10-11`（「本工具**无执行权**, 结果帧显式 `executed: false`」）。
fn f05_result_frames_never_claim_execution() {
    let Some(root) = scratch_root("f05") else {
        return;
    };
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let id = value["proposal"]["id"].as_str().expect("id").to_string();
    let frames = [
        value.clone(),
        ok_value(&call_tool(
            &tool,
            json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
        ))
        .expect("决定成功"),
        ok_value(&call_tool(&tool, json!({"action": "list"}))).expect("列表成功"),
    ];
    for frame in frames {
        assert_eq!(frame["executed"], false, "结果帧必须显式说没执行: {frame}");
        assert_eq!(
            frame["execute_note"], PROPOSAL_NO_EXECUTION_NOTE,
            "不执行说明同文: {frame}"
        );
        assert!(
            frame["action"].is_string() && frame["status"].is_string(),
            "输出归一合同: action / status 恒在场: {frame}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F6 人审卡载荷: detail 恒含人审行 + 证据逐条高亮 + 语气行的 N == 证据条数。
///
/// 契约: `proposal.rs:568-569`（「detail 恒含「建议你亲自确认」行 + 证据逐条
/// 「请亲自过目」+ 「标出的 N 处你最好亲自看一遍」」）+ `proposal.rs:265-268`
/// （语气结构行）+ `proposal.rs:616-628`（卡载荷: 风险档低 + 批准/拒绝两按钮）。
fn f06_review_card_carries_the_gentle_structure() {
    let Some(root) = scratch_root("f06") else {
        return;
    };
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let evidence = ["src/a.rs:12 临时注释", "src/c.rs:3 重复片段", "b.md 旧段落"];
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments(
            "fix_suggest",
            "小错漏: 一处拼写",
            "这几处是重复劳动留下的。",
            &evidence,
        ),
    ))
    .expect("起草成功");

    let card_text = value["card"].as_str().expect("卡呈现文本在场");
    let payload = &value["card_payload"];
    assert!(
        card_text.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "卡面必含「建议你亲自确认」行: {card_text}"
    );
    assert!(
        payload["detail"]
            .as_str()
            .expect("detail 在场")
            .contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "卡载荷 detail 恒含人审行"
    );
    assert!(
        card_text.contains(PROPOSAL_REVIEW_TONE),
        "卡面必含温柔语气结构: {card_text}"
    );
    assert_eq!(
        payload["tone_line"],
        format!("标出的 {} 处{PROPOSAL_REVIEW_TONE}", evidence.len()),
        "语气结构的 N 必须等于证据条数"
    );
    for (index, line) in evidence.iter().enumerate() {
        assert!(
            card_text.contains(&format!(
                "{}. {}{line}",
                index + 1,
                PROPOSAL_EVIDENCE_HIGHLIGHT
            )),
            "第 {} 条证据必须逐条标黄请人过目: {card_text}",
            index + 1
        );
    }
    assert_eq!(payload["risk"], "low", "提案面风险档低");
    assert_eq!(payload["buttons"], json!(["approve", "reject"]));
    assert_eq!(payload["decision_note"], PROPOSAL_NO_EXECUTION_NOTE);
    assert_eq!(payload["evidence"], json!(evidence), "卡载荷证据逐条同文");
    let _ = std::fs::remove_dir_all(&root);
}

/// F7 先斩后奏 = 不存在: 工作区逐字节零变化 + 数据目录只多出提案账。
///
/// 契约: `proposal.rs:6-8`（「**不落任何真面** (工作区零文件系统写, 记忆 / 任务簿 /
/// git 一概不碰)」）+ `proposal.rs:29`（「提案工具本身**不推送** —— 推送是 IM 桥的
/// 外联动作」）。
fn f07_drafting_touches_no_working_face() {
    let Some(root) = scratch_root("f07") else {
        return;
    };
    let workspace = root.join("workspace");
    let data_dir = root.join("data");
    std::fs::create_dir_all(workspace.join("src")).expect("建工作区");
    std::fs::create_dir_all(&data_dir).expect("建数据目录");
    std::fs::write(workspace.join("a.rs"), "fn a() {}\n").expect("写工作区文件");
    std::fs::write(workspace.join("src").join("c.rs"), "fn c() {}\n").expect("写工作区文件");
    let before = snapshot_tree(&workspace);

    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&data_dir));
    ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["src/a.rs:12"]),
    ))
    .expect("起草成功");
    let value = ok_value(&call_tool(&tool, json!({"action": "list"}))).expect("列表成功");
    let _ = value;

    assert_eq!(
        snapshot_tree(&workspace),
        before,
        "提案不落任何真面: 工作区必须逐字节零变化"
    );
    let data_entries: Vec<String> = std::fs::read_dir(&data_dir)
        .expect("数据目录可读")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        data_entries,
        vec![
            std::path::Path::new("proposals.jsonl")
                .to_string_lossy()
                .to_string()
        ],
        "只入提案账 (工具本身不推送 ⇒ 不落推送账)"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// F8 append-only: 追加前的字节前缀逐字节不变。
///
/// 契约: `proposal.rs:322-324`（「**append-only** ... 追加写 + 落盘, 不重写、不删行」）。
fn f08_append_only_keeps_the_old_bytes() {
    let Some(root) = scratch_root("f08") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let path = ledger.path().to_path_buf();
    let before = std::fs::read(&path).expect("读账");

    ok_value(&call_tool(&tool, json!({"action": "list"}))).expect("列表成功");
    let after = std::fs::read(&path).expect("读账");
    assert_eq!(after, before, "只读动作不许改账本字节");
    assert!(
        after.starts_with(&before),
        "append-only: 旧字节前缀逐字节不变"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// F9 撕裂写后追加: 新行必须是**独立可读的一行**, 旧行不许被吞掉。
///
/// 契约: `proposal.rs:322-324`（「逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘,
/// 不重写、不删行」）+ `proposal.rs:325-329`（「**行完整性** ... 行体与行尾换行
/// **同在一次写** ⇒ 并发追加不会把两行挤成一行 ... 末行缺换行 (撕裂写) 时先补
/// 一个换行 ⇒ 撕裂的半行不会把下一行吞掉」）—— 追加不许把两行挤成一行 (撕裂写 =
/// 上个进程死在「写行体」与「写换行」之间; 仓内同族修法见
/// `crates/adapters/gateway/src/chain_audit.rs` 的记录分隔符修复)。
fn f09_append_after_a_torn_line_keeps_the_new_row_readable() {
    let Some(root) = scratch_root("f09") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));

    let first = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理甲", "详情甲", &["e"]),
    ))
    .expect("起草成功");
    let first_id = first["proposal"]["id"].as_str().expect("id").to_string();
    let second = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理乙", "详情乙", &["e"]),
    ))
    .expect("起草成功");
    let second_id = second["proposal"]["id"].as_str().expect("id").to_string();

    // 模拟撕裂写: 去掉末行换行 (上个进程死在写行体与写换行之间)。
    let path = ledger.path().to_path_buf();
    let mut bytes = std::fs::read(&path).expect("读账");
    assert!(bytes.ends_with(b"\n"), "正常追加末字节必是换行");
    bytes.pop();
    std::fs::write(&path, &bytes).expect("写撕裂账");

    // 撕裂行自己读作不存在 (坏行读作不存在, 不砖化整本) —— 但**追加不许把它吞掉**。
    let third = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理丙", "详情丙", &["e"]),
    ))
    .expect("撕裂后追加仍成功");
    let third_id = third["proposal"]["id"].as_str().expect("id").to_string();

    let scan = ledger.scan().expect("账可读");
    let ids = draft_ids(&ledger);
    // ⚠️ 撕裂写 = **行尾换行没落**, 行体本身完整 ⇒ 那一行仍是**一条完整记录**
    // (`scan_records` 按 `lines()` 切行, 末行无换行照样切得出来)。
    // 所以正确判据是「追加不许把两行挤成一行」: 三行都还在, 且没有坏行。
    assert!(
        ids.contains(&first_id.to_string()),
        "撕裂写不许毁掉前面那条好行: ids={ids:?}"
    );
    assert!(
        ids.contains(&second_id.to_string()),
        "撕裂那行本身是完整记录, 不许被下一行吞掉: ids={ids:?}"
    );
    assert!(
        ids.contains(&third_id.to_string()),
        "撕裂写后追加的新行必须是自己那一行 (不许被吞进旧行): ids={ids:?}"
    );
    assert_eq!(
        scan.skipped, 0,
        "三行都是完整记录, 不该有坏行 (挤成一行才会出现坏行): ids={ids:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// F10 并发追加不许丢行 (append-only 的「不删行」在并发下也成立)。
///
/// 契约: `proposal.rs:322-324`（「逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘,
/// 不重写、不删行」）+ `proposal.rs:325-329`（行体与行尾换行**同在一次写** ⇒
/// 并发追加不会把两行挤成一行）—— 两行挤成一行 ⇒ 那两行都读作不存在 ⇒ 等于**删行**。
fn f10_concurrent_appends_keep_every_row() {
    let Some(root) = scratch_root("f10") else {
        return;
    };
    let ledger = Arc::new(ProposalLedger::under_data_dir(&root));
    let threads = 4usize;
    let per_thread = 8usize;
    let mut handles = Vec::new();
    for thread in 0..threads {
        let ledger = ledger.clone();
        handles.push(std::thread::spawn(move || {
            for index in 0..per_thread {
                let draft = ProposalDraft {
                    id: format!("prop-t{thread}-{index}"),
                    kind: ProposalKind::Tidy,
                    title: "并发追加".to_string(),
                    detail: normalize_detail("并发追加的详情"),
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
        "并发追加不许丢行 (逐行一个 StoredDoc 信封): records={} skipped={}",
        scan.records.len(),
        scan.skipped
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// F11 并发「点头」同一提案 ⇒ 账上只有一条决定行 (一提案一决定不许被并发破掉)。
///
/// 契约: `proposal.rs:102`（「one proposal gets exactly one decision」）+
/// `proposal.rs:141`（「一提案一决定, 决定只追加不改史」）。
fn f11_concurrent_decisions_keep_one_proposal_one_decision() {
    let Some(root) = scratch_root("f11") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = Arc::new(ProposalTool::new(ProposalLedger::under_data_dir(&root)));
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let id = value["proposal"]["id"].as_str().expect("id").to_string();

    let threads = 4usize;
    let barrier = Arc::new(std::sync::Barrier::new(threads));
    let mut handles = Vec::new();
    for index in 0..threads {
        let tool = tool.clone();
        let barrier = barrier.clone();
        let id = id.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            call_tool(
                &tool,
                json!({"action": "accept", "proposal_id": id, "decided_by": "owner", "note": format!("n{index}")}),
            )
        }));
    }
    let mut accepted = 0usize;
    for handle in handles {
        let result = handle.join().expect("线程回收");
        if ok_value(&result).is_some() {
            accepted += 1;
        } else {
            let reason = error_reason(&result).expect("失败帧必带真因");
            assert!(!reason.trim().is_empty(), "拒绝即帧必须给真因");
        }
    }
    assert_eq!(
        review_rows(&ledger).len(),
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

/// F12 推送账幂等: `pushed` 进幂等集 / `failed` 不进 (失败留账待重试)。
///
/// 契约: `proposal_push.rs:158-160`（「幂等键集合 (**推送幂等键 = 提案 id**):
/// 已成功推送 (`pushed`) 的提案 id。`failed` 行不入集合 —— 失败留账待重试,
/// 重试成功才进幂等集」）+ `proposal_push.rs:88-92`（去重计数 + 最近推送新在前、
/// 含失败留账行）。
fn f12_push_ledger_idempotency_and_failed_rows() {
    let Some(root) = scratch_root("f12") else {
        return;
    };
    let push = ProposalPushLedger::under_data_dir(&root);
    let record = |id: &str, outcome: ProposalPushOutcome| ProposalPushRecord {
        proposal_id: id.to_string(),
        channel_id: "primary".to_string(),
        conversation_id: "oc_fuzz".to_string(),
        outcome,
        message_ref: matches!(outcome, ProposalPushOutcome::Pushed).then(|| "m-out".to_string()),
        reason: matches!(outcome, ProposalPushOutcome::Failed)
            .then(|| "delivery failed: 503".to_string()),
        at_ms: 1_700_000_000_000,
    };
    push.append(&record("prop-1", ProposalPushOutcome::Failed))
        .expect("失败留账可写");
    assert!(
        push.pushed_ids().expect("幂等集可读").is_empty(),
        "失败留账不算已推送 (要重试)"
    );
    push.append(&record("prop-1", ProposalPushOutcome::Pushed))
        .expect("重试成功可写");
    push.append(&record("prop-1", ProposalPushOutcome::Pushed))
        .expect("重复推送可写");
    push.append(&record("prop-2", ProposalPushOutcome::Pushed))
        .expect("推送可写");

    let ids = push.pushed_ids().expect("幂等集可读");
    assert_eq!(ids.len(), 2, "幂等键 = 提案 id (去重): {ids:?}");
    assert!(ids.contains("prop-1") && ids.contains("prop-2"));

    let stats = push.status().expect("推送列可读");
    assert_eq!(stats.pushed, 2, "已推送数 = 去重提案 id 计数");
    assert_eq!(stats.recent.len(), 4, "最近推送含失败留账行 (如实)");
    assert_eq!(stats.recent[0].proposal_id, "prop-2", "新在前");
    assert_eq!(stats.recent[3].proposal_id, "prop-1");
    assert_eq!(stats.recent[3].outcome, "failed", "失败也是真值, 不藏");
    let _ = std::fs::remove_dir_all(&root);
}

/// F13 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。
///
/// 契约: `proposal.rs:322-324` + `proposal_push.rs:12-13`。
fn f13_bad_rows_read_as_absent() {
    let Some(root) = scratch_root("f13") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let id = value["proposal"]["id"].as_str().expect("id").to_string();

    // 追加一条坏行 (坏 JSON)。
    let path = ledger.path().to_path_buf();
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("账可打开");
    {
        use std::io::Write as _;
        file.write_all(b"{ not json\n").expect("写坏行");
    }
    drop(file);

    let scan = ledger.scan().expect("坏行不许砖化整本");
    assert_eq!(scan.records.len(), 1, "好行照读");
    assert_eq!(scan.skipped, 1, "坏行如实计数");
    assert_eq!(
        ledger.state_of(&id).expect("状态可读"),
        Some(ProposalState::Pending),
        "坏行不影响好行的状态"
    );
    // 坏行在账上, 决定照样能落 (账本不砖化)。
    ok_value(&call_tool(
        &tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
    ))
    .expect("坏行不许让工具面瘫掉");
    let _ = std::fs::remove_dir_all(&root);
}

/// F14 提案 id 只当身份不当路径: 路径穿越形态的 id 既不进账也不落文件。
///
/// 契约: `proposal.rs:394`（id 只用于**逐字比较**）+ `proposal.rs:835`
/// （查无提案必被拒）⇒ 穿越形态的 id 必被拒，且**账外零文件**。
fn f14_proposal_ids_are_never_used_as_paths() {
    let Some(root) = scratch_root("f14") else {
        return;
    };
    let workspace = root.join("workspace");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&workspace).expect("建工作区");
    std::fs::create_dir_all(&data_dir).expect("建数据目录");
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&data_dir));
    let value = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "待整理", "详情", &["e"]),
    ))
    .expect("起草成功");
    let real_id = value["proposal"]["id"].as_str().expect("id").to_string();
    let before = snapshot_tree(&root);

    let shapes = [
        "prop/../etc",
        "../../etc/passwd",
        "prop-1/../../x",
        "prop\\..\\windows",
        "prop-1\u{0}",
        "prop-1\n",
        " prop-1",
        "prop-1 ",
        &"p".repeat(4096),
        "prop-é",
        "prop-e\u{301}",
    ];
    for shape in shapes {
        if shape == real_id {
            continue;
        }
        let result = call_tool(
            &tool,
            json!({"action": "accept", "proposal_id": shape, "decided_by": "owner"}),
        );
        assert!(
            error_reason(&result).is_some(),
            "穿越形态的 id 必被拒 (查无提案): {shape:?}"
        );
        assert_eq!(
            snapshot_tree(&root),
            before,
            "id 只当身份不当路径: 账外不许多出任何文件: {shape:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// F15 Unicode 归一化碰撞: NFC 与 NFD 是**两个不同的提案 id**。
///
/// 契约: `proposal.rs:394`（id 逐字比较）+ `proposal.rs:835`（查无提案必被拒）
/// ⇒ 归一化变体不得被当成同一个提案（否则决定会落到别的提案上）。
fn f15_unicode_normalization_does_not_conflate_ids() {
    let Some(root) = scratch_root("f15") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));
    let nfc = "prop-e\u{301}"; // e + 组合重音
    let nfd = "prop-é"; // 预组合重音
    ledger
        .append_draft(&ProposalDraft {
            id: nfc.to_string(),
            kind: ProposalKind::Tidy,
            title: "归一化碰撞".to_string(),
            detail: normalize_detail("详情"),
            evidence: vec!["e".to_string()],
            draft_payload: json!({}),
            created_at_ms: 1_700_000_000_000,
        })
        .expect("草稿可写");

    // NFD 变体不在账上 ⇒ 必被拒 (不许把决定落到 NFC 那条上)。
    let result = call_tool(
        &tool,
        json!({"action": "accept", "proposal_id": nfd, "decided_by": "owner"}),
    );
    assert!(
        error_reason(&result).is_some(),
        "归一化变体不是同一个提案 (查无提案必被拒)"
    );
    assert_eq!(
        ledger.state_of(nfc).expect("状态可读"),
        Some(ProposalState::Pending),
        "被拒的决定不许落到别的提案上"
    );
    // 原形可决。
    ok_value(&call_tool(
        &tool,
        json!({"action": "accept", "proposal_id": nfc, "decided_by": "owner"}),
    ))
    .expect("原形可决");
    assert_eq!(review_rows(&ledger).len(), 1, "恰好一条决定行");
    let _ = std::fs::remove_dir_all(&root);
}

/// F16 纯判定函数 (零 IO): 空标题 / 空详情 / 空证据 / 非对象 payload ⇒ 拒;
/// 缺署名 / 空 id ⇒ 拒。
///
/// 契约: `proposal.rs:636-638`（草稿校验逐项）+ `proposal.rs:660-668`
/// （决定标记校验）。
fn f16_pure_judges_refuse_the_bad_shapes() {
    let good = ProposalDraft {
        id: "prop-1".to_string(),
        kind: ProposalKind::Tidy,
        title: "标题".to_string(),
        detail: normalize_detail("详情"),
        evidence: vec!["e".to_string()],
        draft_payload: json!({}),
        created_at_ms: 1,
    };
    assert!(proposal_draft_refusal(&good).is_none(), "好形状不该被拒");

    let mut bad = good.clone();
    bad.title = "  ".to_string();
    assert!(proposal_draft_refusal(&bad).is_some(), "空标题必拒");
    let mut bad = good.clone();
    bad.detail = String::new();
    assert!(
        proposal_draft_refusal(&bad).is_some(),
        "空详情必拒 (纯判定面)"
    );
    let mut bad = good.clone();
    bad.evidence = vec![];
    assert!(proposal_draft_refusal(&bad).is_some(), "空证据必拒");
    let mut bad = good.clone();
    bad.evidence = vec!["  ".to_string()];
    assert!(proposal_draft_refusal(&bad).is_some(), "空白证据行必拒");
    let mut bad = good.clone();
    bad.draft_payload = json!(["不是对象"]);
    assert!(
        proposal_draft_refusal(&bad).is_some(),
        "非对象 payload 必拒"
    );

    let review = ProposalReview {
        proposal_id: "prop-1".to_string(),
        decision: ProposalDecision::Accepted,
        decided_by: "owner".to_string(),
        note: None,
        at_ms: 1,
    };
    assert!(proposal_review_refusal(&review).is_none(), "好决定不该被拒");
    let mut bad = review.clone();
    bad.decided_by = "   ".to_string();
    assert!(proposal_review_refusal(&bad).is_some(), "缺署名必拒");
    let mut bad = review.clone();
    bad.proposal_id = " ".to_string();
    assert!(
        proposal_review_refusal(&bad).is_some(),
        "空 proposal_id 必拒"
    );
}

/// F17 任意字节当账本 ⇒ 不许 panic (读面要么给出真值, 要么如实报错)。
///
/// 契约: `proposal.rs:322-324`（「坏行读作不存在 ... 不砖化整本」）+
/// `proposal_push.rs:12-13`（同口径）。
fn f17_arbitrary_bytes_never_panic_the_read_face() {
    let Some(root) = scratch_root("f17") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let push = ProposalPushLedger::under_data_dir(&root);
    let payloads: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"\n\n\n".to_vec(),
        b"{ not json".to_vec(),
        vec![0xFF, 0xFE, 0x00],
        b"{\"name\":\"proposal-ledger\",\"version\":1,\"compatible_versions\":[1],\"body\":{}}"
            .to_vec(),
        b"{\"name\":\"proposal-ledger\",\"version\":99,\"compatible_versions\":[99],\"body\":{}}"
            .to_vec(),
        b"{\"name\":\"other\",\"version\":1,\"compatible_versions\":[1],\"body\":{}}".to_vec(),
    ];
    for payload in payloads {
        std::fs::write(ledger.path(), &payload).expect("写账");
        let _ = ledger.scan();
        let _ = ledger.state_of("prop-1");
        let _ = ledger.status();
        std::fs::write(push.path(), &payload).expect("写推送账");
        let _ = push.scan();
        let _ = push.pushed_ids();
        let _ = push.status();
    }
    let _ = std::fs::remove_dir_all(&root);
}

fn fixed_contracts() {
    f01_draft_lands_exactly_one_row();
    f02_one_proposal_gets_exactly_one_decision();
    f03_unknown_proposal_is_refused_without_writing();
    f04_missing_signature_is_refused_with_zero_writes();
    f05_result_frames_never_claim_execution();
    f06_review_card_carries_the_gentle_structure();
    f07_drafting_touches_no_working_face();
    f08_append_only_keeps_the_old_bytes();
    f09_append_after_a_torn_line_keeps_the_new_row_readable();
    f10_concurrent_appends_keep_every_row();
    f11_concurrent_decisions_keep_one_proposal_one_decision();
    f12_push_ledger_idempotency_and_failed_rows();
    f13_bad_rows_read_as_absent();
    f14_proposal_ids_are_never_used_as_paths();
    f15_unicode_normalization_does_not_conflate_ids();
    f16_pure_judges_refuse_the_bad_shapes();
    f17_arbitrary_bytes_never_panic_the_read_face();
}

// ===========================================================================
// 随机面: 按模式分派 (工具面 / 卡片面 / 账本面 / 推送面)
// ===========================================================================

/// 模式 0: 真实工具面 —— 起草 / 决定 / 列表, 参数全由 fuzz 决定。
fn mode_tool_surface(segments: &Segments) {
    let Some(root) = scratch_root("m0") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));

    // 先起一份**真的**提案 (决定面要有账上真 id 才走得到成功分支; 否则决定
    // 永远落在「查无提案」分支, 成功分支就成了一段死代码)。
    let seeded = ok_value(&call_tool(
        &tool,
        draft_arguments("tidy", "fuzz 种子提案", "种子详情", &["src/a.rs:1"]),
    ))
    .expect("种子提案必须入账");
    let real_id = seeded["proposal"]["id"]
        .as_str()
        .expect("种子提案 id 在场")
        .to_string();

    let action = match segments.byte(slot::ACTION) % 5 {
        0 => "draft",
        1 => "accept",
        2 => "reject",
        3 => "list",
        _ => segments.get(slot::ACTION),
    };
    let title = variant(
        segments.get(slot::TITLE),
        segments.byte(slot::CONTROL) as usize,
    );
    let detail = variant(
        segments.get(slot::DETAIL),
        1 + segments.byte(slot::CONTROL) as usize,
    );
    let evidence = vec![
        variant(segments.get(slot::EVIDENCE_A), 0),
        variant(segments.get(slot::EVIDENCE_B), 2),
    ];
    let payload = serde_json::from_str::<Value>(segments.get(slot::PAYLOAD).trim())
        .unwrap_or_else(|_| json!({}));
    let kind = segments.get(slot::KIND);
    // 一半打**账上真 id** (走成功分支), 一半打 fuzz 形状 id (走「查无提案」分支)。
    let proposal_id = if segments.byte(slot::CONTROL) % 2 == 0 {
        real_id.clone()
    } else {
        variant(segments.get(slot::PROPOSAL_ID), 3)
    };
    let decided_by = variant(segments.get(slot::DECIDED_BY), 0);

    let arguments = match action {
        "draft" => json!({
            "action": "draft",
            "kind": kind,
            "title": title,
            "detail": detail,
            "evidence": evidence,
            "draft_payload": payload,
        }),
        "accept" | "reject" => json!({
            "action": action,
            "proposal_id": proposal_id,
            "decided_by": decided_by,
            "note": segments.get(slot::NOTE),
        }),
        "list" => json!({"action": "list"}),
        other => json!({"action": other}),
    };

    let before_rows = ledger_rows(&ledger);
    let result = call_tool(&tool, arguments.clone());
    let after_rows = ledger_rows(&ledger);

    match ok_value(&result) {
        Some(frame) => {
            // 成功帧 ⇒ 显式不执行 + 输出归一合同在场。
            assert_eq!(frame["executed"], false, "结果帧必须显式说没执行: {frame}");
            assert_eq!(frame["execute_note"], PROPOSAL_NO_EXECUTION_NOTE);
            assert!(frame["action"].is_string(), "action 恒在场: {frame}");
            assert!(frame["status"].is_string(), "status 恒在场: {frame}");
            match action {
                "draft" => {
                    assert_eq!(after_rows, before_rows + 1, "起草只落一行");
                    assert_eq!(frame["action"], "draft");
                    assert_eq!(frame["status"], "proposed");
                    let detail = frame["proposal"]["detail"].as_str().expect("detail");
                    assert!(
                        detail.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
                        "入账 detail 恒含人审行: {detail}"
                    );
                    let card = frame["card"].as_str().expect("卡面在场");
                    assert!(
                        card.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
                        "卡面恒含人审行"
                    );
                    let count = frame["proposal"]["evidence"]
                        .as_array()
                        .map(Vec::len)
                        .unwrap_or(0);
                    assert_eq!(
                        frame["card_payload"]["tone_line"],
                        format!("标出的 {} 处{PROPOSAL_REVIEW_TONE}", count),
                        "语气结构的 N == 证据条数"
                    );
                }
                "accept" | "reject" => {
                    assert_eq!(after_rows, before_rows + 1, "决定只落一行");
                    assert_eq!(frame["action"], action);
                    assert_eq!(
                        frame["status"],
                        if action == "accept" {
                            "accepted"
                        } else {
                            "rejected"
                        }
                    );
                    assert_eq!(frame["proposal_id"], proposal_id);
                    assert_eq!(frame["decided_by"], decided_by);
                    let reviews = review_rows(&ledger);
                    assert_eq!(reviews.len(), 1, "决定行恰好一条");
                    assert_eq!(reviews[0].proposal_id, proposal_id);
                    assert_eq!(reviews[0].decided_by, decided_by);
                }
                "list" => {
                    assert_eq!(after_rows, before_rows, "列表是只读动作");
                    let pending = frame["pending"].as_u64().expect("待审数");
                    let accepted = frame["accepted"].as_u64().expect("accepted 数");
                    let rejected = frame["rejected"].as_u64().expect("rejected 数");
                    let total = frame["total"].as_u64().expect("总数");
                    assert_eq!(
                        pending + accepted + rejected,
                        total,
                        "分态计数自洽: {frame}"
                    );
                    let recent = frame["recent"].as_array().expect("最近提案在场");
                    assert!(recent.len() as u64 <= total);
                }
                _ => panic!("词表外动作不该成功: {frame}"),
            }
        }
        None => {
            // 拒绝即帧: 真因如实 + 账面零写。
            let reason = error_reason(&result).expect("失败帧必带真因");
            assert!(!reason.trim().is_empty(), "拒绝即帧必须给真因");
            assert_eq!(
                after_rows, before_rows,
                "拒绝即帧 = 账面零写 (宁拒不损): reason={reason}"
            );
        }
    }

    // 决定落账成功 ⇒ 第二个决定必被拒, 决定行仍恰好一条 (一提案一决定)。
    if (action == "accept" || action == "reject") && ok_value(&result).is_some() {
        let again = call_tool(
            &tool,
            json!({
                "action": if action == "accept" { "reject" } else { "accept" },
                "proposal_id": proposal_id,
                "decided_by": decided_by,
            }),
        );
        assert!(
            error_reason(&again).is_some(),
            "第二个决定必被拒 (一提案一决定): id={proposal_id:?}"
        );
        assert_eq!(
            review_rows(&ledger).len(),
            1,
            "第二个决定不许改史 (append-only): id={proposal_id:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// 模式 1: 人审卡载荷面 —— detail / 证据行全由 fuzz 决定。
fn mode_card_surface(segments: &Segments) {
    let which = segments.byte(slot::CONTROL) as usize;
    let detail = normalize_detail(&variant(segments.get(slot::DETAIL), which));
    let evidence = vec![
        variant(segments.get(slot::EVIDENCE_A), which),
        variant(segments.get(slot::EVIDENCE_B), which + 1),
    ];
    let title = variant(segments.get(slot::TITLE), which + 2);
    let kind = match ProposalKind::parse(segments.get(slot::KIND)) {
        Some(kind) => kind,
        None => ProposalKind::List,
    };
    let draft = ProposalDraft {
        id: format!("prop-fuzz-{}", which),
        kind,
        title: title.clone(),
        detail: detail.clone(),
        evidence: evidence.clone(),
        draft_payload: json!({}),
        created_at_ms: 1_700_000_000_000,
    };
    let card = draft.review_card();
    let rendered = card.render();

    // detail 恒含人审行 (走 normalize_detail 的结构性保证)。
    assert!(
        detail.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "normalize_detail 之后必含人审行: {detail}"
    );
    assert!(
        rendered.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "卡面必含人审行"
    );
    assert!(rendered.contains(PROPOSAL_REVIEW_TONE), "卡面必含温柔语气");
    assert_eq!(
        card.tone_line,
        format!("标出的 {} 处{PROPOSAL_REVIEW_TONE}", evidence.len()),
        "语气结构的 N == 证据条数"
    );
    for (index, line) in evidence.iter().enumerate() {
        assert!(
            rendered.contains(&format!(
                "{}. {}{line}",
                index + 1,
                PROPOSAL_EVIDENCE_HIGHLIGHT
            )),
            "第 {} 条证据必须逐条标黄: {rendered}",
            index + 1
        );
    }
    let payload = card.to_json();
    assert_eq!(payload["proposal_id"], draft.id);
    assert_eq!(payload["kind"], kind.as_str());
    assert_eq!(payload["title"], title);
    assert_eq!(payload["detail"], detail);
    assert_eq!(payload["evidence"], json!(evidence));
    assert_eq!(payload["tone_line"], card.tone_line);
    assert_eq!(payload["decision_note"], PROPOSAL_NO_EXECUTION_NOTE);
    assert_eq!(payload["risk"], "low");
    assert_eq!(payload["buttons"], json!(["approve", "reject"]));
    // 摘要: 标题首行 + kind + 证据条数。
    let summary = draft.summary();
    assert!(summary.contains(kind.as_str()), "摘要带 kind: {summary}");
    assert!(
        summary.contains(&format!("证据 {} 条", evidence.len())),
        "摘要带证据条数: {summary}"
    );
}

/// 模式 2: 提案账面 —— 账本内容全由 fuzz 决定 (好行 / 坏行 / 撕裂行)。
fn mode_ledger_surface(segments: &Segments) {
    let Some(root) = scratch_root("m2") else {
        return;
    };
    let ledger = ProposalLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));

    // 造若干草稿行 + 若干决定行 (id 由 fuzz 决定, 形态各异)。
    let draft_count = 1 + (segments.byte(slot::CONTROL) % 3) as usize;
    let mut ids: Vec<String> = Vec::new();
    let mut bodies: Vec<ProposalLedgerEntry> = Vec::new();
    for index in 0..draft_count {
        let id = variant(segments.get(index % 4), index);
        if id.trim().is_empty() {
            continue;
        }
        ids.push(id.clone());
        bodies.push(ProposalLedgerEntry::Draft(ProposalDraft {
            id: id.clone(),
            kind: ProposalKind::Tidy,
            title: format!("fuzz 提案 {index}"),
            detail: normalize_detail("fuzz 详情"),
            evidence: vec![format!("src/{index}.rs:1")],
            draft_payload: json!({}),
            created_at_ms: 1_700_000_000_000 + index as i64,
        }));
    }
    // 决定行: 至多一条 (一提案一决定的账面形状)。
    if !ids.is_empty() && segments.byte(slot::ACTION) % 2 == 0 {
        bodies.push(ProposalLedgerEntry::Review(ProposalReview {
            proposal_id: ids[0].clone(),
            decision: ProposalDecision::Accepted,
            decided_by: "owner".to_string(),
            note: None,
            at_ms: 1_700_000_001_000,
        }));
    }

    let mut buffer = Vec::new();
    for body in &bodies {
        let doc = StoredDoc::new(
            PROPOSAL_DOC_NAME,
            PROPOSAL_DOC_VERSION,
            vec![PROPOSAL_DOC_VERSION],
            body,
        );
        buffer.extend_from_slice(&serde_json::to_vec(&doc).expect("信封可序列化"));
        buffer.push(b'\n');
    }
    // 可选坏行 (读作存在: 跳过并计数, 不砖化整本)。
    let bad_row = segments.byte(slot::NOTE) % 2 == 0;
    if bad_row {
        buffer.extend_from_slice(b"{ not json\n");
    }
    std::fs::write(ledger.path(), &buffer).expect("账可写");

    let scan = ledger.scan().expect("账可读");
    assert_eq!(
        scan.records.len(),
        bodies.len(),
        "好行照读 (坏行不许挤掉好行)"
    );
    assert_eq!(scan.skipped, usize::from(bad_row), "坏行如实计数");

    // `state_of` 口径: 「最后一条决定行定状态; 没有决定行 = 待审」
    // (proposal.rs:394)。
    for id in &ids {
        let state = ledger.state_of(id).expect("状态可读");
        assert!(state.is_some(), "草稿行在账上 ⇒ 状态在场: {id:?}");
        let decided = bodies.iter().any(|body| match body {
            ProposalLedgerEntry::Review(review) => &review.proposal_id == id,
            _ => false,
        });
        if decided {
            assert_eq!(state, Some(ProposalState::Accepted), "决定定状态: {id:?}");
        } else {
            assert_eq!(state, Some(ProposalState::Pending), "无决定 = 待审: {id:?}");
        }
    }
    assert_eq!(
        ledger.state_of("prop-not-on-ledger").expect("状态可读"),
        None,
        "账上没有这个 id ⇒ None"
    );

    // `status` 内部自洽: 分态计数之和 == 总数。
    let status = ledger.status().expect("提案节可探测");
    assert_eq!(
        status.pending + status.accepted + status.rejected,
        status.total,
        "分态计数自洽"
    );
    assert!(
        status.recent.len() <= status.total as usize,
        "最近提案条数不超过总数"
    );

    // 工具面在坏行账上照样能用 (不砖化)。
    let listed = ok_value(&call_tool(&tool, json!({"action": "list"}))).expect("列表成功");
    assert_eq!(listed["executed"], false);
    let _ = std::fs::remove_dir_all(&root);
}

/// 模式 3: 推送账面 —— 推送行 / 幂等 / 失败留账 / 坏行。
fn mode_push_surface(segments: &Segments) {
    let Some(root) = scratch_root("m3") else {
        return;
    };
    let push = ProposalPushLedger::under_data_dir(&root);
    let tool = ProposalTool::new(ProposalLedger::under_data_dir(&root));

    let count = 1 + (segments.byte(slot::CONTROL) % 4) as usize;
    let mut bodies: Vec<ProposalPushRecord> = Vec::new();
    let mut pushed_ids: Vec<String> = Vec::new();
    for index in 0..count {
        let id = variant(segments.get(index % 4), index);
        if id.trim().is_empty() {
            continue;
        }
        let outcome = match segments.byte(index) % 3 {
            0 => ProposalPushOutcome::Pushed,
            1 => ProposalPushOutcome::Failed,
            _ => {
                if pushed_ids.is_empty() {
                    ProposalPushOutcome::Pushed
                } else {
                    ProposalPushOutcome::Failed
                }
            }
        };
        if outcome == ProposalPushOutcome::Pushed {
            pushed_ids.push(id.clone());
        }
        bodies.push(ProposalPushRecord {
            proposal_id: id,
            channel_id: "primary".to_string(),
            conversation_id: "oc_fuzz".to_string(),
            outcome,
            message_ref: matches!(outcome, ProposalPushOutcome::Pushed)
                .then(|| "m-out".to_string()),
            reason: matches!(outcome, ProposalPushOutcome::Failed)
                .then(|| "delivery failed: 503".to_string()),
            at_ms: 1_700_000_002_000 + index as i64,
        });
    }

    let mut buffer = Vec::new();
    for body in &bodies {
        let doc = StoredDoc::new(
            PROPOSAL_PUSH_DOC_NAME,
            PROPOSAL_PUSH_DOC_VERSION,
            vec![PROPOSAL_PUSH_DOC_VERSION],
            body,
        );
        buffer.extend_from_slice(&serde_json::to_vec(&doc).expect("信封可序列化"));
        buffer.push(b'\n');
    }
    let bad_row = segments.byte(slot::ACTION) % 2 == 0;
    if bad_row {
        buffer.extend_from_slice(b"{ not json\n");
    }
    std::fs::write(push.path(), &buffer).expect("推送账可写");

    let scan = push.scan().expect("推送账可读");
    assert_eq!(scan.records.len(), bodies.len(), "好行照读");
    assert_eq!(scan.skipped, usize::from(bad_row), "坏行如实计数");

    // 幂等集 = `pushed` 行的提案 id (失败留账不入集)。
    let ids = push.pushed_ids().expect("幂等集可读");
    let expected: std::collections::BTreeSet<String> = pushed_ids.iter().cloned().collect();
    assert_eq!(ids, expected, "幂等键 = 提案 id (只收 pushed 行)");

    let stats = push.status().expect("推送列可读");
    assert_eq!(
        stats.pushed,
        expected.len() as u64,
        "已推送数 = 去重提案 id 计数"
    );
    assert_eq!(
        stats.recent.len(),
        bodies.len().min(5),
        "最近推送 = 最近 5 条 (新在前, 含失败留账行)"
    );
    if let Some(last) = bodies.last() {
        assert_eq!(stats.recent[0].proposal_id, last.proposal_id, "新在前");
        assert_eq!(stats.recent[0].outcome, last.outcome.as_str());
    }

    // 工具面自述的推送列与**同目录同文件名**的推送账同源
    // (proposal.rs:27-29: 推送账 = 提案账同目录的 `proposal-pushes.jsonl`)。
    assert_eq!(
        push.path().file_name().and_then(|name| name.to_str()),
        Some(PROPOSAL_PUSH_LEDGER_FILE),
        "推送账落点 = 提案账同目录的推送账文件"
    );
    // 自述推送列 (`ProposalLedger::status` → 自述面 `proposal` 节) 与推送账同源。
    // ⚠️ 工具面的 `list` 动作**不报**推送列 (契约: list 只报账上真计数; 推送列
    // 随 `tool.self_status` 的 `proposal` 节上报 —— proposal.rs:25-29), 所以这里
    // 查的是 `ProposalLedger::status()` 而不是 list 帧。
    let status = ProposalLedger::under_data_dir(&root)
        .status()
        .expect("提案节可探测");
    assert_eq!(
        status.pushed,
        expected.len() as u64,
        "自述推送列与推送账同源"
    );
    let listed = ok_value(&call_tool(&tool, json!({"action": "list"}))).expect("列表成功");
    assert_eq!(listed["executed"], false, "列表帧也显式不执行");
    let _ = std::fs::remove_dir_all(&root);
}

fn trace(label: &str, detail: impl std::fmt::Debug) {
    if std::env::var_os("DSH_FUZZ_TRACE").is_some() {
        eprintln!("[proposal] {label}: {detail:?}");
    }
}

// ===========================================================================
// fuzz 入口
// ===========================================================================

fuzz_target!(|data: &[u8]| {
    // 固定契约检查 (一次性): 不依赖随机输入的事实断言。
    static FIXED: Once = Once::new();
    FIXED.call_once(fixed_contracts);

    let mode = data.first().copied().unwrap_or(0);
    let segments = Segments::new(data.get(1..).unwrap_or(&[]));
    match mode % 4 {
        0 => mode_tool_surface(&segments),
        1 => mode_card_surface(&segments),
        2 => mode_ledger_surface(&segments),
        _ => mode_push_surface(&segments),
    }
    trace("mode", mode);
});
