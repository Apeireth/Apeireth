#![no_main]

//! fuzz: **IM 提案载荷面**（`apeireth_gateway::im_proposal`）—— 卡片内容 /
//! 按钮决定 / 审批引用 / 操作者标识 / 落账回执 / 推送账幂等。
//!
//! **面**：提案卡出站信封（`proposal_card_envelope`）、按钮回调落账
//! （`ImProposalDecider::decide_from_click` / `decide`）、待推送选择
//! （`pending_push_drafts`）。IM 侧收到的一切**提案载荷**都从这三个入口进：
//! 卡片内容（模型起草 → 人读）、按钮决定（人点）、审批引用（提案 id）。
//!
//! **为什么这个面值得 fuzz**：它是「模型起草的文本」与「人点的按钮」汇合的
//! 地方 —— 载荷里的每个字段都是**外部可控**（模型起草的标题/详情/证据；
//! IM 回传的按钮载荷/操作者 id）。而它的承诺全是**强承诺**：
//! 一提案一决定、重复点击拒绝即帧、点头也不执行、坏载荷不得静默。
//!
//! **判据全部取自被测函数自己写明的契约**（逐条出处见各处注释）：
//!
//! 1. **卡片面**（`im_proposal.rs` `proposal_card_envelope` 的字段映射承诺 +
//!    SDK `ImApprovalCard::validate` / `ImButtonPayload::validate` 的两条形状
//!    承诺）：approval_ref = pair_id = 提案 id（闭合身份）、risk_level = low、
//!    expires_at_ms = 无期限哨兵、摘要 = 卡载荷规范字节的 SHA-256、
//!    「载荷被改 → 摘要不符」、信封形状只取决于提案 id 是否非空。
//! 2. **载荷面**（`im_proposal.rs` `decide_from_click` 的拒绝即帧词表 +
//!    SDK `ImButtonPayload::validate`）：路由标记 = subject；身份不自洽 /
//!    词表外按钮 / 缺 IM 用户标识 ⇒ 拒绝即帧（真因非空）且**零工具面调用**；
//!    落账帧缺状态标签 / 状态标签词表外 ⇒ 拒绝即帧；落账成功 ⇒ **恒不执行**。
//! 3. **真实工具面**（同 `tool.proposal_draft` 工具面 = 模型侧同一份代码）：
//!    一提案一决定（第二次点击必被拒）、查无提案必被拒、缺署名必被拒且
//!    **账面零写**、署名 = `im:<用户>@<渠道>`、点头只落账面不执行。
//! 4. **账本面**（`pending_push_drafts` 的选择承诺）：待推送 = 提案账里
//!    **还没推送过的待审草稿**；已拍板的不再待审；已推送的不再推；失败留账
//!    的照推；坏行读作不存在（不砖化整本）。
//!
//! **不做的事**：不假设任何「我以为应该这样」的期望值；不把 `;` `|` `&`
//! 当作有 shell 语义（本面不经 shell）；不把「幂等性」当判据。

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_gateway::im_proposal::{
    im_decided_by, is_proposal_payload, pending_push_drafts, proposal_card_envelope,
    ImProposalClosureResult, ImProposalDecider, PROPOSAL_CARD_NO_DEADLINE_MS,
};
use apeireth_governance::action_binding::canonical_json_bytes;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolOutcome, ToolResult};
use apeireth_sdk::im::{ImButtonPayload, IM_BUTTON_APPROVE, IM_BUTTON_REJECT};
use apeireth_tools_canonical::proposal::{
    normalize_detail, ProposalDecision, ProposalDraft, ProposalKind, ProposalLedger,
    ProposalLedgerEntry, ProposalReview, ProposalState, ProposalTool, PROPOSAL_CAPABILITY_ID,
    PROPOSAL_DOC_NAME, PROPOSAL_DOC_VERSION, PROPOSAL_EVIDENCE_HIGHLIGHT,
    PROPOSAL_NO_EXECUTION_NOTE, PROPOSAL_PERSONAL_CONFIRM_MARKER, PROPOSAL_REVIEW_TONE,
    PROPOSAL_TOOL_NAME,
};
use apeireth_tools_canonical::proposal_push::{
    ProposalPushLedger, ProposalPushOutcome, ProposalPushRecord, PROPOSAL_PUSH_DOC_NAME,
    PROPOSAL_PUSH_DOC_VERSION,
};
use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// 输入解释: 字节 → 定长字段槽 (NUL 分隔; 缺失槽 = 空串)
// ---------------------------------------------------------------------------

/// fuzz 字节切成的字段槽 (NUL 分隔; 槽内可含换行/制表符/零宽/RTL 等任意字节)。
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

    /// 第 n 个槽 (越界 = 空串)。
    fn get(&self, index: usize) -> &str {
        self.slots.get(index).map(String::as_str).unwrap_or("")
    }

    /// 第 n 个槽的首字节 (控制位来源; 越界 = 0)。
    fn byte(&self, index: usize) -> u8 {
        self.slots
            .get(index)
            .and_then(|slot| slot.as_bytes().first().copied())
            .unwrap_or(0)
    }
}

/// 槽位语义 (固定顺序; 缺槽 = 空)。
mod slot {
    pub const DRAFT_ID: usize = 0;
    pub const TITLE: usize = 1;
    pub const DETAIL: usize = 2;
    pub const EVIDENCE_A: usize = 3;
    pub const EVIDENCE_B: usize = 4;
    pub const DRAFT_PAYLOAD: usize = 5;
    pub const APPROVAL_REF: usize = 6;
    pub const PAIR_ID: usize = 7;
    pub const SUBJECT: usize = 8;
    pub const ACTION: usize = 9;
    pub const OPERATOR: usize = 10;
    pub const CHANNEL: usize = 11;
    pub const KIND: usize = 12;
    pub const CREATED_AT: usize = 13;
    pub const ROUND: usize = 14;
    pub const EXPIRES_AT: usize = 15;
    pub const FRAME: usize = 16;
    pub const CONTROL: usize = 17;
    pub const EXTRA: usize = 18;
}

// ---------------------------------------------------------------------------
// 文本变体: 空 / 超长 / NUL / 零宽 + RTL 覆写 (决定文本面的边界)
// ---------------------------------------------------------------------------

/// 取前 n 个字符 (字符边界安全)。
fn take_chars(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

/// 文本变体: 0 = 原样, 1 = NUL 注入, 2 = 超长, 3 = 零宽 + RTL 覆写。
fn variant(text: &str, which: usize) -> String {
    match which % 4 {
        0 => text.to_string(),
        1 => format!("{text}\u{0}{text}"),
        2 => take_chars(text, 64).repeat(200),
        _ => format!("\u{200b}\u{202e}{text}\u{202c}"),
    }
}

/// 按槽位构造一份提案草稿 (字段合同: `{id, kind, title, detail, evidence,
/// draft_payload}` + 入账时刻; detail 恒含「建议你亲自确认」行 —— 走
/// `normalize_detail`, 与工具面入账前同一道手续)。
fn build_draft(segments: &Segments, which: usize) -> ProposalDraft {
    let kind = match ProposalKind::parse(segments.get(slot::KIND)) {
        Some(kind) => kind,
        None => [
            ProposalKind::Tidy,
            ProposalKind::FixSuggest,
            ProposalKind::Summarize,
            ProposalKind::List,
        ][which % 4],
    };
    let draft_payload = serde_json::from_str::<Value>(segments.get(slot::DRAFT_PAYLOAD))
        .unwrap_or_else(|_| json!({}));
    ProposalDraft {
        id: segments.get(slot::DRAFT_ID).to_string(),
        kind,
        title: variant(segments.get(slot::TITLE), which),
        detail: normalize_detail(&variant(segments.get(slot::DETAIL), which)),
        evidence: vec![
            variant(segments.get(slot::EVIDENCE_A), which),
            variant(segments.get(slot::EVIDENCE_B), which + 1),
        ],
        draft_payload,
        created_at_ms: segments
            .get(slot::CREATED_AT)
            .trim()
            .parse::<i64>()
            .unwrap_or(0),
    }
}

// ---------------------------------------------------------------------------
// 独立 SHA-256 (摘要判据不借库内的实现; NIST 向量自检)
// ---------------------------------------------------------------------------

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in message.chunks(64) {
        let mut w = [0u32; 64];
        for (index, word) in w.iter_mut().enumerate().take(16) {
            let base = index * 4;
            *word = u32::from_be_bytes([
                chunk[base],
                chunk[base + 1],
                chunk[base + 2],
                chunk[base + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut out = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    sha256(data).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// SHA-256 实现自检 (NIST 向量; 自检不过 = harness 自己的错, 立即炸)。
fn assert_sha256_self_check() {
    for (input, expected) in [
        (
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ] {
        assert_eq!(
            sha256_hex(input.as_bytes()),
            expected,
            "harness 自己的 SHA-256 实现不过 NIST 向量 —— 判据不可信"
        );
    }
}

// ---------------------------------------------------------------------------
// 工具面剧本 (落账帧由 fuzz 决定) + 极简执行器 (0 新依赖)
// ---------------------------------------------------------------------------

/// 工具面回什么帧。
#[derive(Clone, Debug)]
enum Frame {
    Ok(Value),
    Error(String),
}

/// 剧本工具面: 回固定帧, 并记录被调用了几次 (拒绝即帧时必须是 0 次)。
struct ScriptedFace {
    id: CapabilityId,
    frame: Frame,
    calls: AtomicUsize,
}

impl ScriptedFace {
    fn new(frame: Frame) -> Self {
        Self {
            id: CapabilityId::new(PROPOSAL_CAPABILITY_ID).expect("valid capability id"),
            frame,
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl ToolCapability for ScriptedFace {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(PROPOSAL_TOOL_NAME)
    }

    fn invoke<'life0, 'life1, 'async_trait>(
        &'life0 self,
        call: &'life1 ToolCall,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match self.frame.clone() {
                Frame::Ok(value) => ToolResult::ok(call.id.clone(), value),
                Frame::Error(message) => {
                    ToolResult::permanent_error(call.id.clone(), message)
                }
            }
        })
    }
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

// ---------------------------------------------------------------------------
// 面 1: 卡片内容面 (proposal_card_envelope)
// ---------------------------------------------------------------------------

fn card_surface(segments: &Segments) {
    assert_sha256_self_check();

    let draft = build_draft(segments, 0);
    let card = draft.review_card();
    let envelope = proposal_card_envelope(&draft);

    // ── 契约 1: 字段映射 (im_proposal.rs `proposal_card_envelope` 的逐字承诺) ──
    assert_eq!(
        envelope.approval_ref, draft.id,
        "approval_ref = 提案 id (推送幂等键)"
    );
    assert_eq!(
        envelope.pair_id, draft.id,
        "pair_id = 提案 id (一提案一决定的闭合身份)"
    );
    assert_eq!(envelope.round, 0, "提案卡轮次 = 0");
    assert_eq!(
        envelope.subject, PROPOSAL_CAPABILITY_ID,
        "subject = tool.proposal_draft (回调路由标记)"
    );
    assert_eq!(
        envelope.command_text, card.render(),
        "command_text = 人审卡全文"
    );
    assert_eq!(
        envelope.arguments_summary, draft.summary(),
        "arguments_summary = 提案摘要 (与工具面同一份口径)"
    );
    assert!(
        envelope.authorization_families.is_empty(),
        "提案面授权族为空 (逐操作授权)"
    );
    assert_eq!(envelope.risk_level, "low", "提案面风险档 = low");
    assert_eq!(
        envelope.governance_reason, PROPOSAL_NO_EXECUTION_NOTE,
        "governance_reason = 卡面同文的不执行声明"
    );
    assert_eq!(
        envelope.created_at_ms, draft.created_at_ms,
        "created_at_ms = 提案入账时刻"
    );
    assert_eq!(
        envelope.expires_at_ms, PROPOSAL_CARD_NO_DEADLINE_MS,
        "expires_at_ms = 无期限哨兵 (提案不设超时)"
    );

    // ── 契约 2: 动作绑定摘要 = 卡载荷规范字节的 SHA-256 (小写 hex) ────────────
    let expected_digest = sha256_hex(&canonical_json_bytes(&card.to_json()));
    assert_eq!(
        envelope.action_binding_digest, expected_digest,
        "action_binding_digest 必须是卡载荷规范字节的 SHA-256"
    );
    assert_eq!(
        envelope.action_binding_digest.len(),
        64,
        "摘要 = 64 位 hex"
    );
    assert!(
        envelope
            .action_binding_digest
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "摘要必须是小写 hex"
    );

    // ── 契约 3: 信封形状 (SDK `ImApprovalCard::validate` + `ImButtonPayload::validate`) ──
    // 两张校验表: 命令文本非空 + 风险级在词表 + 超时戳不早于创建 + 身份非空
    // + action 在词表内。本信封的其余字段由上面的映射钉死 ⇒ 信封成立与否
    // 只取决于提案 id 是否非空。
    assert_eq!(
        envelope.validate().is_ok(),
        !draft.id.trim().is_empty(),
        "信封形状: 其余字段由映射钉死, 成立与否只取决于提案 id 是否非空"
    );

    // ── 契约 4: 卡面温柔行 (proposal.rs `ProposalReviewCard` 的结构承诺) ──────
    assert!(
        envelope.command_text.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
        "卡面必含「建议你亲自确认」行"
    );
    assert!(
        envelope.command_text.contains(PROPOSAL_EVIDENCE_HIGHLIGHT),
        "卡面必含逐条「请亲自过目」"
    );
    assert!(
        envelope.command_text.contains(PROPOSAL_REVIEW_TONE),
        "卡面必含温柔语气行"
    );
    assert!(
        envelope.command_text.contains(&draft.id),
        "卡面带提案 id"
    );

    // ── 契约 5: 确定性 + 「载荷被改 → 摘要不符」 ───────────────────────────
    let again = proposal_card_envelope(&draft);
    assert_eq!(
        envelope.action_binding_digest, again.action_binding_digest,
        "同一份提案载荷 → 同一摘要"
    );
    let mut changed = draft.clone();
    changed.title = format!("{}\u{1}", draft.title);
    assert_ne!(
        envelope.action_binding_digest,
        proposal_card_envelope(&changed).action_binding_digest,
        "载荷被改 → 摘要不符 (按钮绑的是这份提案)"
    );
    // 摘要只由卡载荷决定: 卡载荷之外的字段 (入账时刻) 改动不改摘要。
    let mut outside = draft.clone();
    outside.created_at_ms = draft.created_at_ms.wrapping_add(1);
    assert_eq!(
        envelope.action_binding_digest,
        proposal_card_envelope(&outside).action_binding_digest,
        "摘要 = 卡载荷规范字节的摘要, 卡载荷之外的字段不入摘要"
    );

    // ── 契约 6: 决定文本的边界变体 (空 / 超长 / NUL / 零宽 + RTL) ───────────
    for which in 1..4 {
        let mutated = build_draft(segments, which);
        let mutated_card = mutated.review_card();
        let mutated_envelope = proposal_card_envelope(&mutated);
        assert_eq!(
            mutated_envelope.action_binding_digest,
            sha256_hex(&canonical_json_bytes(&mutated_card.to_json())),
            "变体卡载荷的摘要仍须是规范字节的 SHA-256 (which={which})"
        );
        assert_eq!(
            mutated_envelope.validate().is_ok(),
            !mutated.id.trim().is_empty(),
            "变体信封形状只取决于提案 id 是否非空 (which={which})"
        );
        assert_eq!(
            mutated_envelope.command_text, mutated_card.render(),
            "变体 command_text 仍是人审卡全文 (which={which})"
        );
        assert!(
            mutated_envelope
                .command_text
                .contains(PROPOSAL_PERSONAL_CONFIRM_MARKER),
            "变体卡面仍必含「建议你亲自确认」行 (which={which})"
        );
    }
    trace("card", &draft.id);
}

// ---------------------------------------------------------------------------
// 面 2: 按钮载荷面 (ImButtonPayload 形状 + 决定器拒绝即帧)
// ---------------------------------------------------------------------------

/// 造一个按钮载荷 (字段全来自 fuzz 槽位)。
fn build_payload(segments: &Segments) -> ImButtonPayload {
    ImButtonPayload {
        approval_ref: segments.get(slot::APPROVAL_REF).to_string(),
        pair_id: segments.get(slot::PAIR_ID).to_string(),
        round: segments.get(slot::ROUND).trim().parse::<u64>().unwrap_or(0),
        subject: segments.get(slot::SUBJECT).to_string(),
        expires_at_ms: segments
            .get(slot::EXPIRES_AT)
            .trim()
            .parse::<i64>()
            .unwrap_or(PROPOSAL_CARD_NO_DEADLINE_MS),
        action: segments.get(slot::ACTION).to_string(),
    }
}

/// 造落账帧剧本 (0 = 合法帧, 1 = 缺状态标签, 2 = 状态词表外, 3 = 工具面报错,
/// 4 = 合法帧但自称已执行)。
fn build_frame(segments: &Segments, payload: &ImButtonPayload) -> Frame {
    let user: Option<Value> = serde_json::from_str(segments.get(slot::FRAME).trim()).ok();
    let control = segments.byte(slot::CONTROL) % 5;
    if control == 3 {
        return Frame::Error(segments.get(slot::EXTRA).to_string());
    }
    let status = if segments.byte(slot::EXTRA) % 2 == 0 {
        "accepted"
    } else {
        "rejected"
    };
    let mut value = json!({
        "action": "accept",
        "status": status,
        "proposal_id": payload.approval_ref,
        "decided_by": "im:fuzz@primary",
        "executed": false,
    });
    if let Some(Value::Object(extra)) = &user {
        if let Value::Object(base) = &mut value {
            for (key, item) in extra {
                base.insert(key.clone(), item.clone());
            }
        }
    }
    match control {
        1 => {
            if let Value::Object(base) = &mut value {
                base.remove("status");
            }
        }
        2 => {
            if let Value::Object(base) = &mut value {
                base.insert(
                    "status".to_string(),
                    Value::String(segments.get(slot::EXTRA).to_string()),
                );
            }
        }
        4 => {
            if let Value::Object(base) = &mut value {
                base.insert("executed".to_string(), Value::Bool(true));
            }
        }
        _ => {}
    }
    Frame::Ok(value)
}

fn payload_surface(segments: &Segments) {
    let payload = build_payload(segments);

    // ── 契约 A: 路由标记 = subject (im_proposal.rs `is_proposal_payload`) ─────
    assert_eq!(
        is_proposal_payload(&payload),
        payload.subject == PROPOSAL_CAPABILITY_ID,
        "按钮载荷是不是提案卡 = subject 是不是 tool.proposal_draft"
    );

    // ── 契约 B: 载荷结构校验口径 (SDK `ImButtonPayload::validate`: 身份非空 +
    //    action 在词表内) ───────────────────────────────────────────────────
    let identity_ok = !payload.approval_ref.trim().is_empty()
        && !payload.pair_id.trim().is_empty();
    let action_ok = matches!(payload.action.as_str(), "approve" | "reject" | "cancel");
    assert_eq!(
        payload.validate().is_ok(),
        identity_ok && action_ok,
        "载荷结构校验 = 身份非空 + action 在按钮词表内"
    );

    // ── 契约 C: 载荷往返不变 (serde 形状) ──────────────────────────────────
    let encoded = serde_json::to_value(&payload).expect("按钮载荷可序列化");
    let decoded: ImButtonPayload =
        serde_json::from_value(encoded).expect("按钮载荷可反序列化");
    assert_eq!(decoded, payload, "按钮载荷往返不变");

    // ── 契约 D: 决定器 (im_proposal.rs `decide_from_click` 的拒绝即帧词表) ───
    let frame = build_frame(segments, &payload);
    let face = Arc::new(ScriptedFace::new(frame.clone()));
    let decider = ImProposalDecider::new(face.clone());
    let operator = segments.get(slot::OPERATOR);
    let channel = segments.get(slot::CHANNEL);
    let result = block_on(decider.decide_from_click(channel, operator, &payload));

    let subject_ok = payload.subject == PROPOSAL_CAPABILITY_ID;
    let identity_ok = payload.approval_ref == payload.pair_id;
    let action_ok = matches!(payload.action.as_str(), IM_BUTTON_APPROVE | IM_BUTTON_REJECT);
    let operator_ok = !operator.trim().is_empty();

    if !(subject_ok && identity_ok && action_ok && operator_ok) {
        // 拒绝即帧 (真因如实, 0 静默): 必须是拒绝帧, 且零工具面调用。
        match result {
            ImProposalClosureResult::Refused { reason } => {
                assert!(
                    !reason.trim().is_empty(),
                    "拒绝即帧必须给真因 (0 静默): 载荷={payload:?}"
                );
            }
            ImProposalClosureResult::Decided(closure) => {
                panic!("非法载荷被落账了: {closure:?} payload={payload:?}")
            }
        }
        assert_eq!(
            face.calls(),
            0,
            "拒绝即帧时不得调用工具面 (0 静默副作用): payload={payload:?}"
        );
        trace("payload-refused", &payload);
        return;
    }

    // 合法载荷 → 结果由落账帧决定 (词表外状态 / 缺状态 = 拒绝即帧)。
    match frame {
        Frame::Error(message) => match result {
            ImProposalClosureResult::Refused { reason } => {
                assert_eq!(reason, message, "工具面报错真因如实回传")
            }
            ImProposalClosureResult::Decided(closure) => {
                panic!("工具面报错却落了账: {closure:?}")
            }
        },
        Frame::Ok(value) => {
            match value.get("status").and_then(|status| status.as_str()) {
                None => assert!(
                    matches!(result, ImProposalClosureResult::Refused { .. }),
                    "落账帧缺状态标签 → 拒绝即帧: value={value}"
                ),
                Some(status) if !matches!(status, "accepted" | "rejected") => {
                    assert!(
                        matches!(result, ImProposalClosureResult::Refused { .. }),
                        "落账帧状态标签词表外 → 拒绝即帧: status={status:?}"
                    )
                }
                Some(status) => match result {
                    ImProposalClosureResult::Decided(closure) => {
                        assert!(
                            !closure.executed,
                            "点头也不执行 (是否执行恒 false): closure={closure:?}"
                        );
                        let expected = if status == "accepted" {
                            ProposalDecision::Accepted
                        } else {
                            ProposalDecision::Rejected
                        };
                        assert_eq!(
                            closure.decision, expected,
                            "落账帧的状态标签即账面决定"
                        );
                    }
                    ImProposalClosureResult::Refused { reason } => {
                        panic!("状态合法的落账帧却被拒: {reason}")
                    }
                },
            }
        }
    }
    trace("payload-decided", &payload);
}

// ---------------------------------------------------------------------------
// 面 3: 真实工具面 (与模型侧同一份 `tool.proposal_draft` 代码)
// ---------------------------------------------------------------------------

/// 本次运行独有的落盘目录 (每次迭代用独有文件名 ⇒ 迭代之间零状态泄漏)。
fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join(format!("apeireth-fuzz-im-payload-{}", std::process::id()))
}

/// 本次运行独有的账本路径 (每个面一个文件; 迭代内先截断 ⇒ 迭代之间零状态泄漏)。
fn scratch_path(tag: &str) -> PathBuf {
    scratch_dir().join(format!("{tag}.jsonl"))
}

/// 截断账本文件 (迭代之间零状态泄漏)。
fn truncate_ledger(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, b"").expect("账本可写");
}

/// 读回提案账的决定行 (真值账)。
fn review_rows(ledger: &ProposalLedger) -> Vec<ProposalReview> {
    ledger
        .scan()
        .expect("提案账可读")
        .records
        .iter()
        .filter_map(|record| match &record.body {
            apeireth_tools_canonical::proposal::ProposalLedgerEntry::Review(review) => {
                Some(review.clone())
            }
            _ => None,
        })
        .collect()
}

fn real_tool_surface(segments: &Segments) {
    let ledger_path = scratch_path("tool");
    truncate_ledger(&ledger_path);
    let proposals = ProposalLedger::at(&ledger_path);
    let tool = Arc::new(ProposalTool::new(ProposalLedger::at(&ledger_path)));

    // 经真实工具面起草一份提案 (提案入账; 工具面自带边界校验)。
    let drafted = block_on(tool.invoke(&ToolCall {
        id: "call_draft".to_string(),
        name: PROPOSAL_TOOL_NAME.to_string(),
        arguments: json!({
            "action": "draft",
            "kind": "tidy",
            "title": "fuzz 提案标题",
            "detail": "fuzz 提案详情",
            "evidence": ["src/a.rs:1"],
            "draft_payload": {"files": ["src/a.rs"]},
        }),
    }));
    let ToolOutcome::Ok { value } = drafted.outcome else {
        panic!("真实工具面起草失败: {}", drafted.render())
    };
    let proposal_id = value
        .get("proposal")
        .and_then(|proposal| proposal.get("id"))
        .and_then(|id| id.as_str())
        .expect("提案 id 在场")
        .to_string();

    // 按钮载荷: 闭合身份 = 提案 id (im_proposal.rs: `approval_ref` = `pair_id`
    // = 提案 id, 一提案一决定的闭合身份)。
    let payload = ImButtonPayload {
        approval_ref: proposal_id.clone(),
        pair_id: proposal_id.clone(),
        round: 0,
        subject: PROPOSAL_CAPABILITY_ID.to_string(),
        expires_at_ms: PROPOSAL_CARD_NO_DEADLINE_MS,
        action: segments.get(slot::ACTION).to_string(),
    };
    let operator = segments.get(slot::OPERATOR);
    let channel = segments.get(slot::CHANNEL);
    let decider = ImProposalDecider::new(tool.clone());
    let first = block_on(decider.decide_from_click(channel, operator, &payload));

    let action_ok = matches!(payload.action.as_str(), IM_BUTTON_APPROVE | IM_BUTTON_REJECT);
    let operator_ok = !operator.trim().is_empty();

    if !(action_ok && operator_ok) {
        // 词表外按钮 / 缺 IM 用户标识 ⇒ 拒绝即帧, 账面零写。
        match first {
            ImProposalClosureResult::Refused { reason } => {
                assert!(!reason.trim().is_empty(), "拒绝即帧必须给真因")
            }
            ImProposalClosureResult::Decided(closure) => {
                panic!("词表外按钮/缺署名却落了账: {closure:?}")
            }
        }
        assert!(
            review_rows(&proposals).is_empty(),
            "拒绝即帧 = 账面零写"
        );
        trace("tool-refused", &payload);
        return;
    }

    // 合法载荷 + 提案在账上 ⇒ 落账成功, 且点头只是账面标记。
    let closure = match first {
        ImProposalClosureResult::Decided(closure) => closure,
        ImProposalClosureResult::Refused { reason } => {
            panic!("合法载荷的落账被拒: {reason}")
        }
    };
    assert!(
        !closure.executed,
        "点头也不执行 (是否执行恒 false): {closure:?}"
    );
    assert_eq!(
        closure.decided_by,
        im_decided_by(channel, operator),
        "decided_by = IM 用户名 + 渠道标注 (如实署名)"
    );
    assert_eq!(
        closure.proposal_id, proposal_id,
        "落账回执带的提案 id = 被点的提案 id"
    );
    let expected_decision = if payload.action == IM_BUTTON_APPROVE {
        ProposalDecision::Accepted
    } else {
        ProposalDecision::Rejected
    };
    assert_eq!(
        closure.decision, expected_decision,
        "批准 → accepted / 拒绝 → rejected"
    );

    // 真值账: 恰好一行决定, 署名如实。
    let rows = review_rows(&proposals);
    assert_eq!(rows.len(), 1, "一提案一决定: 账面恰好一行决定");
    assert_eq!(rows[0].proposal_id, proposal_id);
    assert_eq!(rows[0].decision, expected_decision);
    assert_eq!(rows[0].decided_by, im_decided_by(channel, operator));
    assert_eq!(
        proposals.state_of(&proposal_id).unwrap(),
        Some(match expected_decision {
            ProposalDecision::Accepted => ProposalState::Accepted,
            ProposalDecision::Rejected => ProposalState::Rejected,
        }),
        "提案账状态 = 决定状态"
    );

    // 一提案一决定: 重复点击 ⇒ 拒绝即帧, 不改史。
    let again = block_on(decider.decide_from_click(channel, operator, &payload));
    match again {
        ImProposalClosureResult::Refused { reason } => {
            assert!(
                !reason.trim().is_empty(),
                "重复点击必须给真因 (拒绝即帧)"
            )
        }
        ImProposalClosureResult::Decided(closure) => {
            panic!("重复点击又落了一次账: {closure:?}")
        }
    }
    assert_eq!(
        review_rows(&proposals).len(),
        1,
        "一提案一决定: 重复点击不改史 (append-only)"
    );

    // 查无提案 ⇒ 拒绝即帧 (按钮指向不存在的提案 id)。
    let unknown = ImButtonPayload {
        approval_ref: format!("{proposal_id}-no-such"),
        pair_id: format!("{proposal_id}-no-such"),
        ..payload.clone()
    };
    match block_on(decider.decide_from_click(channel, operator, &unknown)) {
        ImProposalClosureResult::Refused { reason } => {
            assert!(!reason.trim().is_empty(), "查无提案必须给真因")
        }
        ImProposalClosureResult::Decided(closure) => {
            panic!("查无提案却落了账: {closure:?}")
        }
    }
    assert_eq!(review_rows(&proposals).len(), 1, "查无提案 = 账面零写");
    trace("tool-decided", &proposal_id);
}

// ---------------------------------------------------------------------------
// 面 4: 待推送选择面 (pending_push_drafts)
// ---------------------------------------------------------------------------

/// 把一串账本主体写成 jsonl 账本 (逐行一个 StoredDoc 信封; 可选追加一条坏行)。
fn write_ledger<T: serde::Serialize>(
    path: &PathBuf,
    name: &str,
    version: u32,
    bodies: &[T],
    bad_row: bool,
) {
    let mut buffer: Vec<u8> = Vec::new();
    for body in bodies {
        let doc = apeireth_core::stored_doc::StoredDoc::new(name, version, vec![version], body);
        buffer.extend_from_slice(&serde_json::to_vec(&doc).expect("账本信封可序列化"));
        buffer.push(b'\n');
    }
    if bad_row {
        buffer.extend_from_slice(b"{ not json\n");
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, &buffer).expect("账本可写");
}

fn ledger_surface(segments: &Segments) {
    let proposals_path = scratch_path("proposals");
    let pushes_path = scratch_path("pushes");
    let proposals = ProposalLedger::at(&proposals_path);
    let pushes = ProposalPushLedger::at(&pushes_path);

    let draft_count = 1 + (segments.byte(slot::CONTROL) % 3) as usize;
    let mut draft_ids: Vec<String> = Vec::new();
    let mut proposal_bodies: Vec<ProposalLedgerEntry> = Vec::new();
    for index in 0..draft_count {
        let raw = segments.get(index);
        let id = if raw.trim().is_empty() {
            format!("prop-fuzz-{index}")
        } else {
            raw.to_string()
        };
        let draft = ProposalDraft {
            id: id.clone(),
            kind: ProposalKind::Tidy,
            title: format!("fuzz 提案 {index}"),
            detail: normalize_detail("fuzz 提案详情"),
            evidence: vec![format!("src/{index}.rs:1")],
            draft_payload: json!({"files": []}),
            created_at_ms: 1_700_000_000_000 + index as i64,
        };
        proposal_bodies.push(ProposalLedgerEntry::Draft(draft));
        draft_ids.push(id);
    }

    // 决定行: 每个提案至多一行 (工具面口径: 一提案一决定, 查无提案即帧)。
    let mut reviewed_ids: Vec<String> = Vec::new();
    for (index, id) in draft_ids.iter().enumerate() {
        if segments.byte(slot::EXTRA + index) % 2 == 0 {
            let decision = if segments.byte(slot::CONTROL) % 4 < 2 {
                ProposalDecision::Accepted
            } else {
                ProposalDecision::Rejected
            };
            proposal_bodies.push(ProposalLedgerEntry::Review(ProposalReview {
                proposal_id: id.clone(),
                decision,
                decided_by: "owner".to_string(),
                note: None,
                at_ms: 1_700_000_001_000 + index as i64,
            }));
            reviewed_ids.push(id.clone());
        }
    }

    // 推送账行: `pushed` = 幂等集, `failed` = 留账待重试 (不是终态)。
    let mut pushed_ids: Vec<String> = Vec::new();
    let mut failed_ids: Vec<String> = Vec::new();
    let mut push_bodies: Vec<ProposalPushRecord> = Vec::new();
    for (index, id) in draft_ids.iter().enumerate() {
        match segments.byte(slot::DRAFT_ID).wrapping_add(index as u8) % 3 {
            0 => {
                push_bodies.push(ProposalPushRecord {
                    proposal_id: id.clone(),
                    channel_id: "primary".to_string(),
                    conversation_id: "oc_fuzz".to_string(),
                    outcome: ProposalPushOutcome::Pushed,
                    message_ref: Some("m-fuzz".to_string()),
                    reason: None,
                    at_ms: 1_700_000_002_000 + index as i64,
                });
                pushed_ids.push(id.clone());
            }
            1 => {
                push_bodies.push(ProposalPushRecord {
                    proposal_id: id.clone(),
                    channel_id: "primary".to_string(),
                    conversation_id: "oc_fuzz".to_string(),
                    outcome: ProposalPushOutcome::Failed,
                    message_ref: None,
                    reason: Some("delivery failed: 503".to_string()),
                    at_ms: 1_700_000_002_000 + index as i64,
                });
                failed_ids.push(id.clone());
            }
            _ => {}
        }
    }

    // 坏行: 读作不存在 (跳过并计入 skipped), 不砖化整本。
    let bad_row = segments.byte(slot::ROUND) % 2 == 0;
    write_ledger(
        &proposals_path,
        PROPOSAL_DOC_NAME,
        PROPOSAL_DOC_VERSION,
        &proposal_bodies,
        bad_row,
    );
    write_ledger(
        &pushes_path,
        PROPOSAL_PUSH_DOC_NAME,
        PROPOSAL_PUSH_DOC_VERSION,
        &push_bodies,
        false,
    );

    let pending = pending_push_drafts(&proposals, &pushes).expect("读提案账/推送账");
    let pending_ids: Vec<&str> = pending.iter().map(|draft| draft.id.as_str()).collect();

    // ── 契约 1: 待推送清单不得含重复提案 id (推送幂等键 = 提案 id) ───────────
    let mut unique = pending_ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        pending_ids.len(),
        "待推送清单含重复提案 id: {pending_ids:?}"
    );

    // ── 契约 2: 待推送 ⊆ 提案账的草稿 id (只有草稿可推) ─────────────────────
    for id in &pending_ids {
        assert!(
            draft_ids.iter().any(|draft_id| draft_id == id),
            "待推送清单含非草稿 id: {id} (草稿={draft_ids:?})"
        );
    }

    // ── 契约 3: review 行里真出现过的提案 id 不得在 pending 里 ───────────────
    for id in &reviewed_ids {
        assert!(
            !pending_ids.contains(&id.as_str()),
            "已拍板的提案不得回到待审 (只推待审草稿): id={id} pending={pending_ids:?}"
        );
    }

    // ── 契约 4: 已推送 (推送账 `pushed` 行) 的不再推 (推送幂等键 = 提案 id) ──
    for id in &pushed_ids {
        assert!(
            !pending_ids.contains(&id.as_str()),
            "已推送的提案又被推了一次 (同提案不重复推送): id={id} pending={pending_ids:?}"
        );
    }

    // ── 契约 5: 完整性 —— 每个待审且未推送的草稿都必在待推送清单里 ─────────
    for id in &draft_ids {
        let state = proposals.state_of(id).expect("提案账可读");
        let expected = state == Some(ProposalState::Pending) && !pushed_ids.contains(id);
        assert_eq!(
            pending_ids.contains(&id.as_str()),
            expected,
            "待推送清单与「待审且未推送」口径不一致: id={id} state={state:?} pending={pending_ids:?}"
        );
    }

    // ── 契约 6: 失败留账的提案照推 (失败不是终态, 待重试) ───────────────────
    for id in &failed_ids {
        if !pushed_ids.contains(id) && !reviewed_ids.contains(id) {
            assert!(
                pending_ids.contains(&id.as_str()),
                "失败留账的提案必须照推 (待重试): id={id} pending={pending_ids:?}"
            );
        }
    }

    // ── 契约 7: 坏行读作不存在 (跳过并计入 skipped), 不砖化整本 ────────────
    let scan = proposals.scan().expect("提案账可读");
    if segments.byte(slot::ROUND) % 2 == 0 {
        assert_eq!(scan.skipped, 1, "坏行如实计数 (跳过不砖化整本)");
        assert_eq!(
            scan.records.len(),
            draft_ids.len() + reviewed_ids.len(),
            "坏行不得挤掉好行"
        );
    }
    trace("ledger", &pending_ids);
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------

fn trace(label: &str, detail: impl std::fmt::Debug) {
    if std::env::var_os("DSH_FUZZ_TRACE").is_some() {
        eprintln!("[im_payload] {label}: {detail:?}");
    }
}

fuzz_target!(|data: &[u8]| {
    let mode = data.first().copied().unwrap_or(0);
    let body = data.get(1..).unwrap_or(&[]);
    let segments = Segments::new(body);
    let forced = std::env::var("DSH_FUZZ_MODE").ok().and_then(|v| v.parse::<u8>().ok());
    match forced.unwrap_or(mode % 4) {
        0 => card_surface(&segments),
        1 => payload_surface(&segments),
        2 => real_tool_surface(&segments),
        _ => ledger_surface(&segments),
    }
});
