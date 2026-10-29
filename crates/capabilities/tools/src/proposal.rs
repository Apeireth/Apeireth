//! 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`, 声明名 `proposal_draft`)。
//!
//! 保守版设计 (愿景小说第四节的保守裁剪): **它提案, 人点头**。它对发现的
//! 边角料 (重复劳动 / 小错漏 / 待整理) 起草结构化提案, 但——
//!
//! - **先斩后奏 = 不存在**: 提案只入提案账 ([`ProposalLedger`],
//!   `proposals.jsonl` append-only), **不落任何真面** (工作区零文件系统写,
//!   记忆 / 任务簿 / git 一概不碰);
//! - **提案不直接执行**: 人点头 = 账面标记 `accepted` ([`ProposalReview`]),
//!   真正的执行永远走既有工具面 (`tool.apply_patch` / `tool.task_board` 等)
//!   的正常审批链 —— 本工具无执行权, 结果帧显式 `executed: false`;
//! - **人审温柔**: 每份提案带一张人审卡载荷 ([`ProposalReviewCard`]), detail
//!   恒含「建议你亲自确认」行, 证据行逐条「请亲自过目」, 语气结构 =
//!   「标出的 N 处你最好亲自看一遍」(IM 审批卡渲染面消费同一份载荷)。
//!
//! # 治理 (风险档 = 低)
//!
//! 权力面 = **只读 + 写提案账** (append-only 追加行, 不写工作面): 无文件写、
//! 无执行、无外联。注册即闸 (`APEIRETH_ENABLE_PROPOSALS`, 默认关 fail-closed,
//! 见 [`ProposalConfig::from_env`]), grant 即用零审批 (与 `tool.self_status`
//! 同档)。提案的"通过"不是本工具的放行动作 —— 审批链管执行, 本账只管留痕。
//!
//! # 自述随行
//!
//! 提案账的待审数 / 最近提案随 `tool.self_status` 的 `proposal` 节上报
//! ([`ProposalLedger::status`] → [`crate::self_status::ProposalStatus`]),
//! 与工具面同账同源; 推送列 (已推送数 / 最近推送) 与同目录推送账
//! [`crate::proposal_push::ProposalPushLedger`] 同源 (提案卡推没推到 IM 渠道
//! 照实报)。提案工具本身**不推送** —— 推送是 IM 桥的外联动作, 痕迹进推送账。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::CapabilityId;
use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::exec_pipeline::{AroundPolicy, OutputSchema, SchemaKind, DEFAULT_MAX_TIMEOUT_MS};
use crate::self_status::{ProposalStatus, ProposalStatusEntry};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const PROPOSAL_CAPABILITY_ID: &str = "tool.proposal_draft";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const PROPOSAL_TOOL_NAME: &str = "proposal_draft";

/// 生产执行时限 (毫秒): 起草/标记是账本追加 + 只读扫描, 到期即归 `timeout.*`。
pub const PROPOSAL_TIMEOUT_MS: u64 = 5_000;

/// 提案账文件 (数据目录落点: `<data_dir>/proposals.jsonl`)。
pub const PROPOSAL_LEDGER_FILE: &str = "proposals.jsonl";

/// 提案账 StoredDoc 的身份戳。
pub const PROPOSAL_DOC_NAME: &str = "proposal-ledger";

/// 提案账 StoredDoc 的格式版本戳。
pub const PROPOSAL_DOC_VERSION: u32 = 1;

/// 提案面总开关 (`APEIRETH_ENABLE_PROPOSALS`): 注册即闸, 默认关 fail-closed。
///
/// 只认显式 `"1"` (前后空白容忍); 未设 / `"0"` / 其它任何取值 = 关。
pub const PROPOSALS_ENABLE_ENV: &str = "APEIRETH_ENABLE_PROPOSALS";

/// detail 必含的人审行标记 (人审温柔: 「建议你亲自确认」)。
pub const PROPOSAL_PERSONAL_CONFIRM_MARKER: &str = "建议你亲自确认";

/// detail 缺人审行时温柔补上的标准行 (含 [`PROPOSAL_PERSONAL_CONFIRM_MARKER`])。
pub const PROPOSAL_PERSONAL_CONFIRM_LINE: &str = "建议你亲自确认: 这份提案只是草稿, \
  拟改动一处都还没动手 —— 最好你亲自看一遍再点头。";

/// 人审卡的温柔语气结构词 (「…你最好亲自看一遍」)。
pub const PROPOSAL_REVIEW_TONE: &str = "你最好亲自看一遍";

/// 证据行的逐条提示 (标黄处)。
pub const PROPOSAL_EVIDENCE_HIGHLIGHT: &str = "【请亲自过目】";

/// 自主度边界声明 (工具描述与结果帧同文): **提案不直接执行**。
pub const PROPOSAL_NO_EXECUTION_NOTE: &str = "A proposal is a draft, never an \
  action: recording it touches only the append-only proposal ledger, and marking \
  it accepted records a human decision without executing anything. Execution \
  always goes through the existing tool faces (apply_patch, task_board, ...) and \
  their normal approval chain.";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const PROPOSAL_TOOL_DESCRIPTION: &str = "Draft a proposal for discovered \
  leftovers (repeated work, small errors or omissions, things to tidy) and record \
  it in the append-only proposal ledger. Four kinds: tidy / fix_suggest / \
  summarize / list. A proposal is a structured draft {id, kind, title, detail, \
  evidence, draft_payload} - drafting writes only proposals.jsonl and touches no \
  working face at all (no file is created, modified or deleted anywhere). The \
  detail always carries a personal-confirm line and the review card tells the \
  human which spots to look at personally. Actions: draft (record a proposal), \
  accept / reject (record a human review decision - nothing is executed), list \
  (read the ledger's true counts and recent proposals). Marking a decision needs \
  proposal_id and decided_by; one proposal gets exactly one decision. ";

/// 提案的四个 kind (边角料的四种处置草稿)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    /// 待整理 (收纳/归档/清理类草稿)。
    Tidy,
    /// 小错漏 (修正建议类草稿)。
    FixSuggest,
    /// 重复劳动 (摘要/合并类草稿)。
    Summarize,
    /// 清单 (列出待办/待查项的草稿)。
    List,
}

impl ProposalKind {
    /// 稳定标签 (`tidy` / `fix_suggest` / `summarize` / `list`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tidy => "tidy",
            Self::FixSuggest => "fix_suggest",
            Self::Summarize => "summarize",
            Self::List => "list",
        }
    }

    /// 闭合词表解析 (词表外 = `None`, 调用方拒绝即帧)。
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "tidy" => Some(Self::Tidy),
            "fix_suggest" => Some(Self::FixSuggest),
            "summarize" => Some(Self::Summarize),
            "list" => Some(Self::List),
            _ => None,
        }
    }
}

/// 提案状态机 (一提案一决定, 决定只追加不改史)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    /// 待审 (已入账, 还没点头/摇头)。
    Pending,
    /// 人点头 (标记 accepted; **不执行**, 执行走既有工具面)。
    Accepted,
    /// 人摇头 (标记 rejected; 提案留痕, 不执行)。
    Rejected,
}

impl ProposalState {
    /// 稳定标签 (`pending` / `accepted` / `rejected`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

/// 提案账里的决定 (人审结果的留痕行)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalDecision {
    /// 点头 (标记 accepted)。
    Accepted,
    /// 摇头 (标记 rejected)。
    Rejected,
}

impl ProposalDecision {
    /// 稳定标签 (`accepted` / `rejected`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    /// 决定对应的提案状态。
    pub const fn state(self) -> ProposalState {
        match self {
            Self::Accepted => ProposalState::Accepted,
            Self::Rejected => ProposalState::Rejected,
        }
    }
}

/// 一份结构化提案草稿 (提案账的 draft 行主体)。
///
/// 字段合同: `{id, kind, title, detail, evidence, draft_payload}` + 入账时刻。
/// `detail` 恒含「建议你亲自确认」行 (见 [`normalize_detail`]); `evidence` 是
/// 发现边角料的证据行 (至少一条, 人审卡逐条标黄请人过目); `draft_payload` 是
/// 「如果点头, 打算怎么改」的结构化设想 —— **只是设想, 不是动作**。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalDraft {
    /// 提案稳定身份 (`prop-<时刻毫秒>-<进程内序号>`)。
    pub id: String,
    /// 提案 kind ([`ProposalKind`])。
    pub kind: ProposalKind,
    /// 标题 (人读首行, 非空)。
    pub title: String,
    /// 详情 (恒含「建议你亲自确认」行)。
    pub detail: String,
    /// 证据行 (至少一条, 每条非空)。
    pub evidence: Vec<String>,
    /// 「点头后打算怎么改」的结构化设想 (JSON 对象; 只是设想, 不落任何真面)。
    pub draft_payload: Value,
    /// 入账时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
}

/// 提案身份 (`prop-<时刻毫秒>-<进程内序号>`)。
pub fn proposal_id(created_at_ms: i64, ordinal: u64) -> String {
    format!("prop-{created_at_ms}-{ordinal}")
}

/// 进程内提案序号 (身份唯一性用, 非安全边界)。
static PROPOSAL_ORDINAL: AtomicU64 = AtomicU64::new(1);

/// 生成下一个提案身份 (当前时刻 + 进程内序号)。
pub fn next_proposal_id() -> String {
    proposal_id(
        now_epoch_ms(),
        PROPOSAL_ORDINAL.fetch_add(1, Ordering::Relaxed),
    )
}

/// 当前时刻 (epoch 毫秒); 时钟回拨不编造, 如实取系统读数。
fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// 温柔补行: detail 必含「建议你亲自确认」行。
///
/// 已含标记 = 原样保留 (不重复贴); 缺行 = 追加标准行
/// ([`PROPOSAL_PERSONAL_CONFIRM_LINE`]) —— 人审温柔是**结构性保证**, 不靠
/// 模型记得写。
pub fn normalize_detail(detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER) {
        return trimmed.to_string();
    }
    format!("{trimmed}\n{PROPOSAL_PERSONAL_CONFIRM_LINE}")
}

impl ProposalDraft {
    /// 审批卡摘要 (标题首行 + kind + 证据条数)。
    pub fn summary(&self) -> String {
        format!(
            "提案摘要: [{}] {} (证据 {} 条)",
            self.kind.as_str(),
            first_line(&self.title),
            self.evidence.len()
        )
    }

    /// 人审温柔的语气结构行: 「标出的 N 处你最好亲自看一遍」。
    pub fn tone_line(&self) -> String {
        format!("标出的 {} 处{PROPOSAL_REVIEW_TONE}", self.evidence.len())
    }

    /// 渲染成 IM 审批卡载荷 (呈现文本见 [`ProposalReviewCard::render`])。
    pub fn review_card(&self) -> ProposalReviewCard {
        ProposalReviewCard {
            proposal_id: self.id.clone(),
            kind: self.kind.as_str().to_string(),
            title: self.title.clone(),
            detail: self.detail.clone(),
            evidence: self.evidence.clone(),
            tone_line: self.tone_line(),
            decision_note: PROPOSAL_NO_EXECUTION_NOTE.to_string(),
        }
    }
}

/// 标题首行 (卡片不整段贴长标题)。
fn first_line(text: &str) -> String {
    text.trim()
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// 一条人审决定 (提案账的 review 行主体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalReview {
    /// 被决定的提案 id。
    pub proposal_id: String,
    /// 决定 ([`ProposalDecision`])。
    pub decision: ProposalDecision,
    /// 谁拍的板 (人审来源如实留痕: 人名 / IM 卡回调标识)。
    pub decided_by: String,
    /// 决定备注 (可空)。
    pub note: Option<String>,
    /// 决定时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

/// 提案账的一行 (append-only 的行主体: 草稿行 or 决定行)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProposalLedgerEntry {
    /// 草稿入账 (提出一份提案)。
    Draft(ProposalDraft),
    /// 决定入账 (人点头/摇头的留痕)。
    Review(ProposalReview),
}

/// 提案账 (数据目录 `proposals.jsonl`)。
///
/// **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写、不删
/// 行 (与语句账本 / 技能卡版本审计同一姿势); 坏行读作不存在 (跳过并计入
/// `skipped`), 不砖化整本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalLedger {
    path: PathBuf,
}

impl ProposalLedger {
    /// 数据目录落点 (`<data_dir>/proposals.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(PROPOSAL_LEDGER_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 账本路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, entry: &ProposalLedgerEntry) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("提案账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            PROPOSAL_DOC_NAME,
            PROPOSAL_DOC_VERSION,
            vec![PROPOSAL_DOC_VERSION],
            entry,
        );
        let line =
            serde_json::to_vec(&doc).map_err(|error| format!("提案账序列化失败: {error}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("提案账打开失败 {}: {error}", self.path.display()))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("提案账写入失败 {}: {error}", self.path.display()))
    }

    /// 追加草稿行。
    pub fn append_draft(&self, draft: &ProposalDraft) -> Result<(), String> {
        self.append(&ProposalLedgerEntry::Draft(draft.clone()))
    }

    /// 追加决定行。
    pub fn append_review(&self, review: &ProposalReview) -> Result<(), String> {
        self.append(&ProposalLedgerEntry::Review(review.clone()))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<ProposalLedgerEntry>, String> {
        scan_records::<ProposalLedgerEntry>(&self.path, &ledger_compat())
            .map_err(|error| format!("提案账读取失败 {}: {error}", self.path.display()))
    }

    /// 某提案的当前状态 (`None` = 账上没有这个 id)。
    ///
    /// 一提案一决定: 最后一条决定行定状态; 没有决定行 = [`ProposalState::Pending`]。
    pub fn state_of(&self, proposal_id: &str) -> Result<Option<ProposalState>, String> {
        let scan = self.scan()?;
        let mut state: Option<ProposalState> = None;
        for record in &scan.records {
            match &record.body {
                ProposalLedgerEntry::Draft(draft) if draft.id == proposal_id => {
                    state.get_or_insert(ProposalState::Pending);
                }
                ProposalLedgerEntry::Review(review) if review.proposal_id == proposal_id => {
                    state = Some(review.decision.state());
                }
                _ => {}
            }
        }
        Ok(state)
    }

    /// 自述提案节的现探测 (待审数 / 分态计数 / 最近提案 + 推送列, 真值如实)。
    ///
    /// 只回 id / kind / 标题 / 状态 (最近提案有界 5 条), 不回 detail/payload
    /// 正文 —— 与自述面「有多少不说是什么」同口径。**推送列** (已推送数 /
    /// 最近推送) 与提案账同目录的推送账 [`crate::proposal_push::ProposalPushLedger`]
    /// (`proposal-pushes.jsonl`) 同源: 提案卡推没推到 IM 渠道照实报, 失败
    /// 推送标 `failed` 留账待重试; 推送账缺文件 = 0 推送真值 (不是错误)。
    pub fn status(&self) -> Result<ProposalStatus, String> {
        let scan = self.scan()?;
        let mut note: Option<String> = None;
        if scan.skipped > 0 {
            note = Some(format!(
                "提案账 {} 行坏行读作不存在 (如实跳过, 不砖化整本)",
                scan.skipped
            ));
        }
        let mut order: Vec<String> = Vec::new();
        let mut entries: std::collections::BTreeMap<String, ProposalStatusEntry> =
            std::collections::BTreeMap::new();
        for record in &scan.records {
            match &record.body {
                ProposalLedgerEntry::Draft(draft) => {
                    if !entries.contains_key(&draft.id) {
                        order.push(draft.id.clone());
                    }
                    entries.insert(
                        draft.id.clone(),
                        ProposalStatusEntry {
                            id: draft.id.clone(),
                            kind: draft.kind.as_str().to_string(),
                            title: first_line(&draft.title),
                            state: ProposalState::Pending.as_str().to_string(),
                        },
                    );
                }
                ProposalLedgerEntry::Review(review) => {
                    if let Some(entry) = entries.get_mut(&review.proposal_id) {
                        entry.state = review.decision.state().as_str().to_string();
                    }
                }
            }
        }
        let mut pending = 0u64;
        let mut accepted = 0u64;
        let mut rejected = 0u64;
        for id in &order {
            match entries.get(id).map(|entry| entry.state.as_str()) {
                Some("pending") => pending += 1,
                Some("accepted") => accepted += 1,
                Some("rejected") => rejected += 1,
                _ => {}
            }
        }
        let total = order.len() as u64;
        let recent: Vec<ProposalStatusEntry> = order
            .iter()
            .rev()
            .take(PROPOSAL_RECENT_LIMIT)
            .filter_map(|id| entries.get(id).cloned())
            .collect();
        if total == 0 && note.is_none() {
            note = Some("尚无提案记录".to_string());
        }
        // 推送列 (自主度提案面接线): 与提案账同目录的推送账同源 (缺文件 = 0
        // 推送真值); 推送账坏行跳过如实并入备注, 读失败照实报不编数。
        let push_stats = crate::proposal_push::ProposalPushLedger::at(
            self.path
                .with_file_name(crate::proposal_push::PROPOSAL_PUSH_LEDGER_FILE),
        )
        .status()?;
        if let Some(push_note) = push_stats.note {
            note = Some(match note {
                Some(existing) => format!("{existing}; {push_note}"),
                None => push_note,
            });
        }
        Ok(ProposalStatus {
            pending,
            accepted,
            rejected,
            total,
            recent,
            pushed: push_stats.pushed,
            recent_pushes: push_stats.recent,
            note,
            reason: None,
        })
    }
}

/// 自述「最近提案」的摘录上限 (条)。
pub const PROPOSAL_RECENT_LIMIT: usize = 5;

/// 账本读端契约 (仅当前格式版本可读)。
fn ledger_compat() -> DocCompat {
    DocCompat::exact(PROPOSAL_DOC_NAME, PROPOSAL_DOC_VERSION)
}

// ---------------------------------------------------------------------------
// 人审卡 (IM 审批卡呈现载荷)
// ---------------------------------------------------------------------------

/// 提案人审卡载荷 (IM 审批卡渲染面消费同一份载荷)。
///
/// 温柔语气结构: detail 恒含「建议你亲自确认」行 + 证据逐条「请亲自过目」+
/// 「标出的 N 处你最好亲自看一遍」。批准/拒绝两个按钮与既有 IM 审批卡同构;
/// **点头也不执行** —— 卡面同文声明 [`PROPOSAL_NO_EXECUTION_NOTE`]。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalReviewCard {
    /// 提案 id (与账本配对)。
    pub proposal_id: String,
    /// 提案 kind 标签。
    pub kind: String,
    /// 标题。
    pub title: String,
    /// 详情 (恒含「建议你亲自确认」行)。
    pub detail: String,
    /// 证据行 (逐条标黄请人过目)。
    pub evidence: Vec<String>,
    /// 温柔语气结构行 (「标出的 N 处你最好亲自看一遍」)。
    pub tone_line: String,
    /// 决定说明 (点头 = 标记 accepted, 不执行)。
    pub decision_note: String,
}

impl ProposalReviewCard {
    /// 渲染呈现文本 (本地审批面板 / IM 审批卡共用的卡面正文)。
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("[提案待审] ");
        out.push_str(&self.title);
        out.push('\n');
        out.push_str(&format!(
            "类别: {} | 提案 id: {}\n",
            self.kind, self.proposal_id
        ));
        out.push_str("详情:\n");
        out.push_str(&self.detail);
        out.push('\n');
        out.push_str(&format!("证据 ({}):\n", self.tone_line));
        for (index, line) in self.evidence.iter().enumerate() {
            out.push_str(&format!(
                "{}. {PROPOSAL_EVIDENCE_HIGHLIGHT}{line}\n",
                index + 1
            ));
        }
        out.push_str(&self.decision_note);
        out
    }

    /// 结构化卡载荷 (含风险档低标注 + 批准/拒绝两按钮, 与 IM 审批卡同构)。
    pub fn to_json(&self) -> Value {
        json!({
            "proposal_id": self.proposal_id,
            "kind": self.kind,
            "title": self.title,
            "detail": self.detail,
            "evidence": self.evidence,
            "tone_line": self.tone_line,
            "decision_note": self.decision_note,
            "risk": "low",
            "buttons": ["approve", "reject"],
        })
    }
}

// ---------------------------------------------------------------------------
// 校验 (纯判定, 零文件系统写)
// ---------------------------------------------------------------------------

/// 草稿校验 (纯判定): 返回 `Some(拒绝即帧文案)` 即拒绝。
///
/// 逐项: ① 标题非空; ② 详情非空; ③ 证据至少一条且每条非空; ④
/// `draft_payload` 必须是 JSON 对象 (设想要有形)。
pub fn proposal_draft_refusal(draft: &ProposalDraft) -> Option<String> {
    if draft.title.trim().is_empty() {
        return Some("拒绝提案: 标题 (title) 不能为空".to_string());
    }
    if draft.detail.trim().is_empty() {
        return Some("拒绝提案: 详情 (detail) 不能为空".to_string());
    }
    if draft.evidence.is_empty() {
        return Some("拒绝提案: 提案必须带证据 (evidence 至少一条)".to_string());
    }
    for (index, line) in draft.evidence.iter().enumerate() {
        if line.trim().is_empty() {
            return Some(format!("拒绝提案: 证据第 {} 条为空", index + 1));
        }
    }
    if !draft.draft_payload.is_object() {
        return Some("拒绝提案: draft_payload 必须是 JSON 对象 (结构化设想)".to_string());
    }
    None
}

/// 决定标记校验 (纯判定): 返回 `Some(拒绝即帧文案)` 即拒绝。
pub fn proposal_review_refusal(review: &ProposalReview) -> Option<String> {
    if review.proposal_id.trim().is_empty() {
        return Some("拒绝标记: proposal_id 不能为空".to_string());
    }
    if review.decided_by.trim().is_empty() {
        return Some("拒绝标记: decided_by 不能为空 (谁拍的板要如实留痕)".to_string());
    }
    None
}

// ---------------------------------------------------------------------------
// 工具面
// ---------------------------------------------------------------------------

/// 工具动作闭合词表。
pub const ACTION_DRAFT: &str = "draft";
/// 记录人点头 (标记 accepted, 不执行)。
pub const ACTION_ACCEPT: &str = "accept";
/// 记录人摇头 (标记 rejected, 不执行)。
pub const ACTION_REJECT: &str = "reject";
/// 读提案账真值 (待审数 / 分态计数 / 最近提案)。
pub const ACTION_LIST: &str = "list";

/// 闭合动作词表 (draft / accept / reject / list)。
pub const PROPOSAL_ACTIONS: &[&str] = &[ACTION_DRAFT, ACTION_ACCEPT, ACTION_REJECT, ACTION_LIST];

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalParams {
    action: Option<String>,
    kind: Option<String>,
    title: Option<String>,
    detail: Option<String>,
    evidence: Option<Vec<String>>,
    draft_payload: Option<Value>,
    proposal_id: Option<String>,
    decided_by: Option<String>,
    note: Option<String>,
}

/// 边角料提案工具 (`tool.proposal_draft`, 声明名 `proposal_draft`)。
///
/// 零工作面写: 起草/标记/列表都只碰提案账 ([`ProposalLedger`]) 与只读扫描。
pub struct ProposalTool {
    id: CapabilityId,
    ledger: ProposalLedger,
}

impl ProposalTool {
    /// 绑定提案账落位 (数据目录 `proposals.jsonl`)。
    pub fn new(ledger: ProposalLedger) -> Self {
        Self {
            id: CapabilityId::new(PROPOSAL_CAPABILITY_ID).expect("valid capability id"),
            ledger,
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 提案账 (自述提案节同账同源)。
    pub fn ledger(&self) -> &ProposalLedger {
        &self.ledger
    }

    fn draft(&self, call: &ToolCall, params: &ProposalParams) -> ToolResult {
        let Some(kind_text) = params.kind.as_deref() else {
            return self.refuse(
                call,
                "拒绝提案: draft 动作必须给 kind (tidy / fix_suggest / summarize / list)",
            );
        };
        let Some(kind) = ProposalKind::parse(kind_text) else {
            return self.refuse(
                call,
                format!(
                    "unknown proposal kind {kind_text:?}; the closed vocabulary is \
                     tidy / fix_suggest / summarize / list"
                ),
            );
        };
        let title = params.title.clone().unwrap_or_default();
        let detail = normalize_detail(&params.detail.clone().unwrap_or_default());
        let evidence = params.evidence.clone().unwrap_or_default();
        let draft_payload = params.draft_payload.clone().unwrap_or(Value::Null);
        let mut draft = ProposalDraft {
            id: next_proposal_id(),
            kind,
            title,
            detail,
            evidence,
            draft_payload,
            created_at_ms: now_epoch_ms(),
        };
        if let Some(refusal) = proposal_draft_refusal(&draft) {
            return self.refuse(call, refusal);
        }
        // 温柔保证: 入账的 detail 恒含「建议你亲自确认」行 (normalize 已补)。
        draft.detail = normalize_detail(&draft.detail);
        if let Err(error) = self.ledger.append_draft(&draft) {
            return self.refuse(call, format!("提案入账失败 (未入账, 无半帧): {error}"));
        }
        let card = draft.review_card();
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_DRAFT,
                "status": "proposed",
                "proposal": {
                    "id": draft.id,
                    "kind": draft.kind.as_str(),
                    "title": draft.title,
                    "detail": draft.detail,
                    "evidence": draft.evidence,
                    "draft_payload": draft.draft_payload,
                    "created_at_ms": draft.created_at_ms,
                },
                "card": card.render(),
                "card_payload": card.to_json(),
                "executed": false,
                "execute_note": PROPOSAL_NO_EXECUTION_NOTE,
            }),
        )
        .with_name(PROPOSAL_TOOL_NAME)
    }

    fn decide(
        &self,
        call: &ToolCall,
        params: &ProposalParams,
        decision: ProposalDecision,
    ) -> ToolResult {
        let action = match decision {
            ProposalDecision::Accepted => ACTION_ACCEPT,
            ProposalDecision::Rejected => ACTION_REJECT,
        };
        let review = ProposalReview {
            proposal_id: params.proposal_id.clone().unwrap_or_default(),
            decision,
            decided_by: params.decided_by.clone().unwrap_or_default(),
            note: params.note.clone(),
            at_ms: now_epoch_ms(),
        };
        if let Some(refusal) = proposal_review_refusal(&review) {
            return self.refuse(call, refusal);
        }
        let state = match self.ledger.state_of(&review.proposal_id) {
            Ok(state) => state,
            Err(error) => return self.refuse(call, format!("提案账读取失败: {error}")),
        };
        match state {
            None => {
                return self.refuse(
                    call,
                    format!("拒绝标记: 提案 {} 不在提案账上", review.proposal_id),
                );
            }
            Some(ProposalState::Pending) => {}
            Some(other) => {
                return self.refuse(
                    call,
                    format!(
                        "拒绝标记: 提案 {} 已是 {} (一提案一决定, append-only 不改史)",
                        review.proposal_id,
                        other.as_str()
                    ),
                );
            }
        }
        if let Err(error) = self.ledger.append_review(&review) {
            return self.refuse(call, format!("决定入账失败 (未入账, 无半帧): {error}"));
        }
        ToolResult::ok(
            &call.id,
            json!({
                "action": action,
                "status": review.decision.state().as_str(),
                "proposal_id": review.proposal_id,
                "decided_by": review.decided_by,
                "note": review.note,
                "executed": false,
                "execute_note": PROPOSAL_NO_EXECUTION_NOTE,
            }),
        )
        .with_name(PROPOSAL_TOOL_NAME)
    }

    fn list(&self, call: &ToolCall) -> ToolResult {
        let status = match self.ledger.status() {
            Ok(status) => status,
            Err(error) => return self.refuse(call, format!("提案账读取失败: {error}")),
        };
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_LIST,
                "status": "ok",
                "pending": status.pending,
                "accepted": status.accepted,
                "rejected": status.rejected,
                "total": status.total,
                "recent": status.recent,
                "note": status.note,
                "executed": false,
                "execute_note": PROPOSAL_NO_EXECUTION_NOTE,
            }),
        )
        .with_name(PROPOSAL_TOOL_NAME)
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, reason: impl Into<String>) -> ToolResult {
        ToolResult::permanent_error(&call.id, reason.into()).with_name(PROPOSAL_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for ProposalTool {
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
                        "enum": ["draft", "accept", "reject", "list"],
                        "description": "draft records a proposal; accept / reject record a human review decision (nothing is executed); list reads the ledger's true counts."
                    },
                    "kind": {
                        "type": "string",
                        "enum": ["tidy", "fix_suggest", "summarize", "list"],
                        "description": "draft only: what kind of leftover this proposal handles."
                    },
                    "title": {
                        "type": "string",
                        "description": "draft only: human-readable title (non-empty)."
                    },
                    "detail": {
                        "type": "string",
                        "description": "draft only: detail; a personal-confirm line is always ensured before it lands in the ledger."
                    },
                    "evidence": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "draft only: evidence lines for the discovered leftover (at least one, each non-empty)."
                    },
                    "draft_payload": {
                        "type": "object",
                        "description": "draft only: the structured change plan if a human nods - a plan, never an action."
                    },
                    "proposal_id": {
                        "type": "string",
                        "description": "accept / reject only: the proposal this decision marks."
                    },
                    "decided_by": {
                        "type": "string",
                        "description": "accept / reject only: who made the decision (recorded truthfully in the ledger)."
                    },
                    "note": {
                        "type": "string",
                        "description": "accept / reject only: optional decision note."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(PROPOSAL_TOOL_NAME)
            .with_description(format!(
                "{PROPOSAL_TOOL_DESCRIPTION}{PROPOSAL_NO_EXECUTION_NOTE}"
            ))
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<ProposalParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(call, format!("invalid proposal_draft parameters: {error}"));
            }
        };
        let Some(action) = params.action.as_deref() else {
            return self.refuse(
                call,
                "拒绝提案: 必须显式给 action (draft / accept / reject / list)",
            );
        };
        match action {
            ACTION_DRAFT => self.draft(call, &params),
            ACTION_ACCEPT => self.decide(call, &params, ProposalDecision::Accepted),
            ACTION_REJECT => self.decide(call, &params, ProposalDecision::Rejected),
            ACTION_LIST => self.list(call),
            other => self.refuse(
                call,
                format!(
                    "unknown proposal_draft action {other:?}; the closed vocabulary is \
                     draft / accept / reject / list"
                ),
            ),
        }
    }
}

/// 单调边界 Guard (只拒不放): 草稿/决定校验不过即拒 —— 空标题 / 空证据 /
/// 非对象 payload / 空拍板人, 任何后续阶段都不能翻回放行。
pub struct ProposalBoundaryGuard;

impl ProposalBoundaryGuard {
    /// 提案面边界 Guard (无状态)。
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProposalBoundaryGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGuard for ProposalBoundaryGuard {
    fn name(&self) -> &str {
        "proposal_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != PROPOSAL_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判草稿/决定边界。
        let Ok(params) = serde_json::from_value::<ProposalParams>(request.arguments.clone()) else {
            return None;
        };
        match params.action.as_deref() {
            Some(ACTION_DRAFT) => {
                let kind = params
                    .kind
                    .as_deref()
                    .and_then(ProposalKind::parse)
                    .unwrap_or(ProposalKind::List);
                let mut draft = ProposalDraft {
                    id: String::new(),
                    kind,
                    title: params.title.clone().unwrap_or_default(),
                    detail: params.detail.clone().unwrap_or_default(),
                    evidence: params.evidence.clone().unwrap_or_default(),
                    draft_payload: params.draft_payload.clone().unwrap_or(Value::Null),
                    created_at_ms: 0,
                };
                // 词表外 kind 由工具即帧; 这里只判草稿形状边界。
                draft.detail = normalize_detail(&draft.detail);
                proposal_draft_refusal(&draft)
            }
            Some(ACTION_ACCEPT) | Some(ACTION_REJECT) => proposal_review_refusal(&ProposalReview {
                proposal_id: params.proposal_id.clone().unwrap_or_default(),
                decision: ProposalDecision::Accepted,
                decided_by: params.decided_by.clone().unwrap_or_default(),
                note: None,
                at_ms: 0,
            }),
            _ => None,
        }
    }
}

/// 组装提案面五段流水线: 低风险档 (读面 + append 账本, 无 ask 档钩子) →
/// 单调边界 Guard → 超时 → 输出归一合同。
pub fn proposal_pipeline() -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_guard(Arc::new(ProposalBoundaryGuard::new()))
            .with_around(
                AroundPolicy::new().with_timeout(PROPOSAL_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(ProposalTool::output_schema()),
    )
}

/// 组装边角料提案工具面 (模型侧一个能力)。
pub fn proposal_capability(config: ProposalConfig) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> = Arc::new(ProposalTool::new(config.ledger()));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        proposal_pipeline(),
    ))
}

// ---------------------------------------------------------------------------
// 开关 (注册即闸, 默认关 fail-closed)
// ---------------------------------------------------------------------------

/// 提案面配置 (提案账落位)。
///
/// 组装根按 [`ProposalConfig::from_env`] 注入 (注册即闸): 缺配置 =
/// `tool.proposal_draft` 不注册, 自述名册 `proposals` 行照实 false。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalConfig {
    /// 提案账所在数据目录 (`<data_dir>/proposals.jsonl`)。
    pub data_dir: PathBuf,
}

impl ProposalConfig {
    /// 提案账落位 = 数据目录。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// 本配置的提案账。
    pub fn ledger(&self) -> ProposalLedger {
        ProposalLedger::under_data_dir(&self.data_dir)
    }

    /// 开关实况: `APEIRETH_ENABLE_PROPOSALS` 只认显式 `"1"` (默认关 fail-closed)。
    pub fn enabled_from_env() -> bool {
        proposals_enabled_from_env()
    }

    /// 从 env 构造: 开关开才 `Some` (默认关 → [`None`], fail-closed)。
    pub fn from_env(data_dir: impl Into<PathBuf>) -> Option<Self> {
        proposals_enabled_from_env().then(|| Self::new(data_dir))
    }
}

/// 提案面开关 (`APEIRETH_ENABLE_PROPOSALS`) 实况: 只认显式 `"1"`。
///
/// 未设 / `"0"` / `"true"` / 其它任何取值 = 关 (fail-closed, 命名即承诺:
/// 没有这把钥匙就是没有提案面)。
pub fn proposals_enabled_from_env() -> bool {
    std::env::var(PROPOSALS_ENABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(id: &str) -> ProposalDraft {
        ProposalDraft {
            id: id.to_string(),
            kind: ProposalKind::Tidy,
            title: "边角料: 三处临时注释待清理".to_string(),
            detail: normalize_detail("这三行注释是调试时留下的。"),
            evidence: vec!["src/a.rs:12".to_string(), "src/b.rs:40".to_string()],
            draft_payload: json!({"files": ["src/a.rs", "src/b.rs"]}),
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn detail_without_the_confirm_line_gets_it_appended_once() {
        let normalized = normalize_detail("这三行注释是调试时留下的。");
        assert!(normalized.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER));
        // 已含标记 = 原样保留, 不重复贴行。
        let again = normalize_detail(&normalized);
        assert_eq!(again, normalized);
        assert_eq!(
            normalized.matches(PROPOSAL_PERSONAL_CONFIRM_MARKER).count(),
            1
        );
    }

    #[test]
    fn all_four_kinds_have_stable_labels_and_parse_back() {
        for kind in [
            ProposalKind::Tidy,
            ProposalKind::FixSuggest,
            ProposalKind::Summarize,
            ProposalKind::List,
        ] {
            assert_eq!(ProposalKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(ProposalKind::parse("rm -rf"), None);
    }

    #[test]
    fn draft_refusal_covers_empty_title_evidence_and_non_object_payload() {
        let mut bad = draft("prop-1");
        bad.title = "  ".to_string();
        assert!(proposal_draft_refusal(&bad).is_some());
        let mut bad = draft("prop-1");
        bad.evidence = vec![];
        assert!(proposal_draft_refusal(&bad).is_some());
        let mut bad = draft("prop-1");
        bad.evidence = vec!["".to_string()];
        assert!(proposal_draft_refusal(&bad).is_some());
        let mut bad = draft("prop-1");
        bad.draft_payload = json!(["not", "an", "object"]);
        assert!(proposal_draft_refusal(&bad).is_some());
        assert!(proposal_draft_refusal(&draft("prop-1")).is_none());
    }

    #[test]
    fn review_refusal_requires_an_honest_decider() {
        let mut review = ProposalReview {
            proposal_id: "prop-1".to_string(),
            decision: ProposalDecision::Accepted,
            decided_by: "owner".to_string(),
            note: None,
            at_ms: 1,
        };
        assert!(proposal_review_refusal(&review).is_none());
        review.decided_by = " ".to_string();
        assert!(proposal_review_refusal(&review).is_some());
    }

    #[test]
    fn review_card_carries_the_gentle_tone_and_personal_confirm_line() {
        let card = draft("prop-1").review_card();
        let rendered = card.render();
        assert!(rendered.contains(PROPOSAL_PERSONAL_CONFIRM_MARKER));
        assert!(rendered.contains(PROPOSAL_REVIEW_TONE));
        assert!(rendered.contains(PROPOSAL_EVIDENCE_HIGHLIGHT));
        assert_eq!(card.tone_line, "标出的 2 处你最好亲自看一遍");
    }

    #[test]
    fn the_switch_is_fail_closed_except_the_exact_opt_in() {
        crate::test_support::assert_switch_fails_closed_except_exact_one(
            PROPOSALS_ENABLE_ENV,
            &["0", "true", "on", " 1 0 ", ""],
            proposals_enabled_from_env,
            Some(|| ProposalConfig::from_env(".").is_some()),
        );
    }
}
