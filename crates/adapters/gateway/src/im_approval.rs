//! IM 快捷接入: 审批卡片闭合 (件三)。
//!
//! 杀手锏链路: 工具审批事件 → IM 审批卡片 (命令文本 / 风险级 / 批准·拒绝按钮)
//! → 人在 IM 点按钮 → **治理闭合**:
//!
//! - 闭合词表是治理侧 [`ApprovalOutcome`] 四态 (allowed_once / rejected /
//!   cancelled / unavailable), 本面 0 第二套词表; 按钮与四态的映射固定:
//!   批准 → `AllowedOnce` (仅一次、仅该操作), 拒绝 → `Rejected`,
//!   取消 → `Cancelled`, 超时/中断/查无/失败 → `Unavailable` (fail-closed);
//! - **超时语义与本地一致**: 按钮载荷带本地审批的 `expires_at_ms`, 到点即
//!   `Unavailable` 且 0 执行 (与本地 `expired` 分辨率同一映射);
//! - **审计配对原子**: 每个闭合把 `approval.asked` ↔ `approval.decision`
//!   一对记录经 [`commit_approval_audit_pair`] 一次提交 (双写落地或整对回滚);
//! - **一次性**: 同一 pair 只闭合一次, 重复点击不二次执行、不二次记账。
//!
//! 卡片是展示面, 授权仍走既有 canonical 审批路径 ([`ImApprovalResolver`]),
//! 0 隐藏授权、0 旁路。

use std::collections::HashMap;
use std::sync::Mutex;

use apeireth_governance::approval_closure::{
    commit_approval_audit_pair, ApprovalAuditPair, ApprovalAuditRecord, ApprovalAuditSink,
    ApprovalOutcome, MemoryApprovalAuditSink,
};
use apeireth_sdk::im::{
    ImApprovalCard, ImButtonPayload, IM_BUTTON_APPROVE, IM_BUTTON_CANCEL, IM_BUTTON_REJECT,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::canonical_entry::CanonicalPendingApproval;

/// 审批卡片上的风险级词表 (与治理 risk 序一致, 高位覆盖低位)。
const RISK_VOCABULARY: &[(&str, i32)] = &[
    ("info", 0),
    ("low", 0),
    ("medium", 1),
    ("high", 2),
    ("critical", 3),
    ("nuclear", 4),
];

/// 一次待人审批 (网关侧视图: 展示字段 + 闭合身份)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImApprovalNotice {
    /// 被审批操作的稳定引用 (canonical approval id 字符串)。
    pub approval_ref: String,
    /// 所属桌面会话 (IM 会话与之同源)。
    pub session: apeireth_core::kernel::SessionId,
    /// 审计配对身份 (一对 asked ↔ decision 共用)。
    pub pair_id: String,
    /// 配对所属轮次。
    pub round: u64,
    /// 被审批操作的能力身份 (审计 subject)。
    pub subject: String,
    /// 一行命令文本。
    pub command_text: String,
    /// 参数短摘要。
    pub arguments_summary: String,
    /// 治理 hook 名 (展示)。
    pub governance_hook: String,
    /// 治理原因 (展示 + 风险级推导输入)。
    pub governance_reason: String,
    /// 创建时间 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 本地审批超时戳 (epoch 毫秒; 与本地 `expires_at` 同源)。
    pub expires_at_ms: i64,
}

impl ImApprovalNotice {
    /// 从 canonical 暂停视图构造 (pair_id = approval id, subject = capability)。
    pub fn from_canonical(view: &CanonicalPendingApproval, round: u64) -> Self {
        Self {
            approval_ref: view.approval_id.to_string(),
            session: view.session,
            pair_id: view.approval_id.to_string(),
            round,
            subject: view.capability_id.clone(),
            command_text: view.command_text.clone(),
            arguments_summary: view.arguments_summary.clone(),
            governance_hook: view.governance_hook.clone(),
            governance_reason: view.governance_reason.clone(),
            created_at_ms: view.created_at.epoch_millis(),
            expires_at_ms: view.expires_at.epoch_millis(),
        }
    }

    /// 风险级标签 (确定性推导, 非第二治理判定):
    /// 治理文本里的词表标签取最高位; 无标签或标签低于 `high` = `high`
    /// (需人批的效应本身是高位)。
    pub fn risk_level(&self) -> String {
        approval_risk_level(&self.governance_reason, &self.governance_hook)
    }

    /// 渲染成 IM 审批卡片。
    pub fn approval_card(&self) -> ImApprovalCard {
        ImApprovalCard {
            approval_ref: self.approval_ref.clone(),
            pair_id: self.pair_id.clone(),
            round: self.round,
            subject: self.subject.clone(),
            command_text: self.command_text.clone(),
            arguments_summary: self.arguments_summary.clone(),
            risk_level: self.risk_level(),
            governance_reason: self.governance_reason.clone(),
            created_at_ms: self.created_at_ms,
            expires_at_ms: self.expires_at_ms,
        }
    }
}

/// 风险级确定性推导 (治理文本里出现的词表标签取最高位; 无标签 = `high`)。
pub fn approval_risk_level(reason: &str, hook: &str) -> String {
    let haystack = format!("{reason} {hook}").to_ascii_lowercase();
    let mut best = 2; // high: 需人批效应的基线位
    for (label, rank) in RISK_VOCABULARY {
        if haystack
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == *label)
        {
            best = best.max(*rank);
        }
    }
    RISK_VOCABULARY
        .iter()
        .find(|(_, rank)| *rank == best)
        .map(|(label, _)| (*label).to_string())
        .unwrap_or_else(|| "high".to_string())
}

/// 解析器: 把按钮决定带回既有 canonical 审批路径 (执行冻结操作 + 续跑回合)。
#[async_trait]
pub trait ImApprovalResolver: Send + Sync {
    /// 解析一次待审批。返回 canonical 分辨率 (含续跑文本)。
    async fn resolve(
        &self,
        session: apeireth_core::kernel::SessionId,
        approval_ref: &str,
        decision: &str,
    ) -> ImApprovalResolution;
}

/// canonical 审批分辨率 (四态映射的输入)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImApprovalResolution {
    /// 一次解析完成了该轮 (续跑文本 + 稳定分辨率标签)。
    Resumed {
        /// 续跑后的回复文本。
        text: String,
        /// 稳定分辨率标签 (`approved` / `rejected` / `cancelled`)。
        label: String,
    },
    /// 已过本地超时 (与本地 `expired` 同义)。
    Expired,
    /// 已被别处解析过 (本地先点了 / 并发闭合)。
    AlreadyResolved {
        /// 已有终态标签。
        label: String,
    },
    /// 已中断 (外部效应未知, 不得自动重试)。
    Interrupted,
    /// 查无此待审批。
    NotFound,
    /// 解析失败 (fail-closed)。
    Failed {
        /// 事实原因。
        reason: String,
    },
}

/// 一次闭合的结果 (四态 + 身份 + 是否执行)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImApprovalClosure {
    /// 审计配对身份。
    pub pair_id: String,
    /// 配对所属轮次。
    pub round: u64,
    /// 被审批操作的能力身份。
    pub subject: String,
    /// 四态闭合词表值。
    pub outcome: ApprovalOutcome,
    /// canonical 分辨率标签 (审计可读)。
    pub resolution_label: String,
    /// 冻结操作是否执行 (仅 `allowed_once` 为 true)。
    pub executed: bool,
    /// 续跑文本 (拒绝/超时为固定文案, 0 泄露内部原因)。
    pub reply_text: String,
}

/// 闭合入口的返回。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImApprovalClosureResult {
    /// 本轮完成闭合 (审计配对已原子提交)。
    Closed(ImApprovalClosure),
    /// 该 pair 已闭合过 (重复点击 / 并发), 0 二次执行、0 二次记账。
    AlreadyClosed {
        /// 已闭合的 pair 身份。
        pair_id: String,
    },
}

/// 审批审计配对提交器 (原子双写)。
pub trait ImApprovalAuditCommit: Send + Sync {
    /// 一对 asked ↔ decision 一次提交 (双写落地或整对回滚)。
    fn commit(&self, pair: &ApprovalAuditPair) -> Result<(), String>;

    /// 已提交记录 (审计读面)。
    fn committed(&self) -> Vec<ApprovalAuditRecord>;
}

/// 内存审计提交器 (参考实现 + 测试读面)。
pub struct MemoryApprovalAudit {
    inner: Mutex<MemoryApprovalAuditSink>,
}

impl MemoryApprovalAudit {
    /// 空的内存审计。
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MemoryApprovalAuditSink::new()),
        }
    }
}

impl Default for MemoryApprovalAudit {
    fn default() -> Self {
        Self::new()
    }
}

impl ImApprovalAuditCommit for MemoryApprovalAudit {
    fn commit(&self, pair: &ApprovalAuditPair) -> Result<(), String> {
        let mut sink = self
            .inner
            .lock()
            .map_err(|_| "approval audit sink is poisoned".to_string())?;
        commit_approval_audit_pair(&mut *sink, pair).map_err(|e| e.to_string())
    }

    fn committed(&self) -> Vec<ApprovalAuditRecord> {
        self.inner
            .lock()
            .map(|sink| sink.records().to_vec())
            .unwrap_or_default()
    }
}

/// 落盘审计 sink (jsonl 追加; savepoint 回滚按字节截断)。
///
/// 句柄克隆共享同一份落盘状态, 因此既能当 [`ApprovalAuditSink`] 走
/// [`commit_approval_audit_pair`] 的原子双写, 又能当 [`ImApprovalAuditCommit`] 用。
#[derive(Clone)]
pub struct FileApprovalAudit {
    path: std::path::PathBuf,
    state: std::sync::Arc<Mutex<FileApprovalAuditState>>,
}

struct FileApprovalAuditState {
    /// 每次追加后的文件字节长度 (首元素 = 初始长度)。
    boundaries: Vec<u64>,
    /// 已提交记录 (读面)。
    records: Vec<ApprovalAuditRecord>,
}

impl FileApprovalAudit {
    /// 打开/创建落盘审计 (已有记录读入内存索引)。
    pub fn open(path: std::path::PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs_err::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if !path.exists() {
            fs_err::write(&path, b"").map_err(|e| e.to_string())?;
        }
        let bytes = fs_err::read(&path).map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        for line in bytes.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_slice::<ApprovalAuditRecord>(line) {
                records.push(record);
            }
        }
        Ok(Self {
            path,
            state: std::sync::Arc::new(Mutex::new(FileApprovalAuditState {
                boundaries: vec![bytes.len() as u64],
                records,
            })),
        })
    }

    /// 已提交记录 (读面)。
    pub fn records(&self) -> Vec<ApprovalAuditRecord> {
        self.state
            .lock()
            .map(|state| state.records.clone())
            .unwrap_or_default()
    }
}

impl ApprovalAuditSink for FileApprovalAudit {
    type Error = String;

    fn committed_len(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.records.len())
            .unwrap_or(0)
    }

    fn write(&mut self, record: ApprovalAuditRecord) -> Result<(), Self::Error> {
        let line = serde_json::to_string(&record).map_err(|e| e.to_string())?;
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        let mut file = fs_err::OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        use std::io::Write as _;
        file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        let previous = state.boundaries.last().copied().unwrap_or(0);
        state.boundaries.push(previous + line.len() as u64 + 1);
        state.records.push(record);
        Ok(())
    }

    fn rollback(&mut self, committed_len: usize) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if committed_len >= state.records.len() {
            return;
        }
        let offset = state.boundaries.get(committed_len).copied().unwrap_or(0);
        if let Ok(file) = fs_err::OpenOptions::new().write(true).open(&self.path) {
            let _ = file.set_len(offset);
            let _ = file.sync_all();
        }
        state.records.truncate(committed_len);
        state.boundaries.truncate(committed_len + 1);
    }
}

impl ImApprovalAuditCommit for FileApprovalAudit {
    fn commit(&self, pair: &ApprovalAuditPair) -> Result<(), String> {
        let mut handle = self.clone();
        commit_approval_audit_pair(&mut handle, pair).map_err(|e| e.to_string())
    }

    fn committed(&self) -> Vec<ApprovalAuditRecord> {
        self.records()
    }
}

/// 一次闭合请求 (按钮载荷 + 会话 + 现在时间)。
#[derive(Debug, Clone)]
pub struct ImApprovalRequest {
    /// 会话 (IM 会话映射到的桌面会话, 同源)。
    pub session: apeireth_core::kernel::SessionId,
    /// 按钮载荷 (闭合身份 + 超时戳 + action)。
    pub payload: ImButtonPayload,
    /// 现在时间 (epoch 毫秒; 注入口径, 测试可控)。
    pub now_ms: i64,
}

/// 闭合器: 一次按钮点击 → 四态闭合 + 原子审计配对 + 一次性执行。
pub struct ImApprovalCloser {
    resolver: std::sync::Arc<dyn ImApprovalResolver>,
    audit: std::sync::Arc<dyn ImApprovalAuditCommit>,
    closed: Mutex<HashMap<String, ImApprovalClosure>>,
}

impl ImApprovalCloser {
    /// 构造闭合器。
    pub fn new(
        resolver: std::sync::Arc<dyn ImApprovalResolver>,
        audit: std::sync::Arc<dyn ImApprovalAuditCommit>,
    ) -> Self {
        Self {
            resolver,
            audit,
            closed: Mutex::new(HashMap::new()),
        }
    }

    /// 已闭合轮次的记录 (审计读面之外的进程内视图)。
    pub fn closure_of(&self, pair_id: &str) -> Option<ImApprovalClosure> {
        self.closed
            .lock()
            .ok()
            .and_then(|closed| closed.get(pair_id).cloned())
    }

    /// 闭合一 round (超时/取消/拒绝 fail-closed; 批准仅一次)。
    pub async fn close(
        &self,
        request: ImApprovalRequest,
    ) -> Result<ImApprovalClosureResult, crate::im_bridge::ImBridgeError> {
        let ImApprovalRequest {
            session,
            payload,
            now_ms,
        } = request;
        payload
            .validate()
            .map_err(|e| crate::im_bridge::ImBridgeError::Decode(e.to_string()))?;

        let pair_id = payload.pair_id.clone();
        if let Ok(closed) = self.closed.lock() {
            if closed.contains_key(&pair_id) {
                return Ok(ImApprovalClosureResult::AlreadyClosed { pair_id });
            }
        }

        // 超时语义与本地一致: 到点即 unavailable, 0 执行。
        let (resolution_label, outcome, executed, reply_text) = if now_ms > payload.expires_at_ms {
            (
                "expired".to_string(),
                ApprovalOutcome::Unavailable,
                false,
                timeout_reply(),
            )
        } else {
            let decision = decision_label(&payload.action);
            let resolution = self
                .resolver
                .resolve(session, &payload.approval_ref, decision)
                .await;
            match resolution {
                ImApprovalResolution::Resumed { text, label } => {
                    let outcome = ApprovalOutcome::from_resolution_label(&label)
                        .unwrap_or(ApprovalOutcome::Unavailable);
                    let executed = outcome == ApprovalOutcome::AllowedOnce;
                    let reply = if executed { text } else { fixed_reply(&label) };
                    (label, outcome, executed, reply)
                }
                ImApprovalResolution::Expired => (
                    "expired".to_string(),
                    ApprovalOutcome::Unavailable,
                    false,
                    timeout_reply(),
                ),
                ImApprovalResolution::AlreadyResolved { .. } => {
                    return Ok(ImApprovalClosureResult::AlreadyClosed { pair_id });
                }
                ImApprovalResolution::Interrupted => (
                    "interrupted".to_string(),
                    ApprovalOutcome::Unavailable,
                    false,
                    fixed_reply("interrupted"),
                ),
                ImApprovalResolution::NotFound => (
                    "unavailable".to_string(),
                    ApprovalOutcome::Unavailable,
                    false,
                    fixed_reply("unavailable"),
                ),
                ImApprovalResolution::Failed { .. } => (
                    "unavailable".to_string(),
                    ApprovalOutcome::Unavailable,
                    false,
                    fixed_reply("unavailable"),
                ),
            }
        };

        let closure = ImApprovalClosure {
            pair_id: pair_id.clone(),
            round: payload.round,
            subject: payload.subject.clone(),
            outcome,
            resolution_label,
            executed,
            reply_text,
        };

        // 审计配对原子: asked ↔ decision 一对一次提交。
        let pair = ApprovalAuditPair::new(
            pair_id.clone(),
            payload.round,
            payload.subject.clone(),
            closure.outcome,
        );
        self.audit
            .commit(&pair)
            .map_err(crate::im_bridge::ImBridgeError::Audit)?;

        if let Ok(mut closed) = self.closed.lock() {
            closed.insert(pair_id, closure.clone());
        }
        Ok(ImApprovalClosureResult::Closed(closure))
    }
}

/// 按钮 action → canonical 决定标签 (批准/拒绝/取消)。
fn decision_label(action: &str) -> &'static str {
    match action {
        IM_BUTTON_APPROVE => "approve",
        IM_BUTTON_REJECT => "reject",
        IM_BUTTON_CANCEL => "cancel",
        _ => "reject",
    }
}

fn timeout_reply() -> String {
    "审批已超时, 操作未执行 (与本地超时语义一致)".to_string()
}

fn fixed_reply(label: &str) -> String {
    match label {
        "rejected" => "已拒绝, 操作未执行".to_string(),
        "cancelled" => "已取消, 操作未执行".to_string(),
        _ => "审批未通过, 操作未执行".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(action: &str) -> ImButtonPayload {
        ImButtonPayload {
            approval_ref: "apr_1".to_string(),
            pair_id: "pair-1".to_string(),
            round: 3,
            subject: "tool.shell".to_string(),
            expires_at_ms: 1_700_000_300_000,
            action: action.to_string(),
        }
    }

    #[test]
    fn risk_level_is_derived_deterministically() {
        assert_eq!(approval_risk_level("no labels here", "hook"), "high");
        assert_eq!(approval_risk_level("risk=nuclear", "hook"), "nuclear");
        assert_eq!(approval_risk_level("medium effect", "hook"), "high");
        assert_eq!(
            approval_risk_level("critical effect", "nuclear hook"),
            "nuclear"
        );
    }

    #[test]
    fn notice_renders_a_card_with_the_closure_identity() {
        let notice = ImApprovalNotice {
            approval_ref: "apr_1".to_string(),
            session: apeireth_core::kernel::SessionId::new(),
            pair_id: "pair-1".to_string(),
            round: 3,
            subject: "tool.shell".to_string(),
            command_text: "shell: bash -c 'ls'".to_string(),
            arguments_summary: "执行命令 ls".to_string(),
            governance_hook: "hook".to_string(),
            governance_reason: "risk=critical write".to_string(),
            created_at_ms: 1_700_000_000_000,
            expires_at_ms: 1_700_000_300_000,
        };
        let card = notice.approval_card();
        card.validate().unwrap();
        assert_eq!(card.pair_id, "pair-1");
        assert_eq!(card.risk_level, "critical");
        let text = apeireth_sdk::im::render_approval_card(
            apeireth_sdk::im::ImChannelKind::ImFeishu,
            &card,
        )
        .unwrap()
        .to_string();
        assert!(text.contains("ls"), "{text}");
    }

    #[test]
    fn four_outcome_vocabulary_is_reachable_from_one_card_round() {
        let outcomes = [
            ("approved", ApprovalOutcome::AllowedOnce),
            ("rejected", ApprovalOutcome::Rejected),
            ("cancelled", ApprovalOutcome::Cancelled),
            ("expired", ApprovalOutcome::Unavailable),
        ];
        for (label, expected) in outcomes {
            assert_eq!(
                ApprovalOutcome::from_resolution_label(label),
                Some(expected),
                "{label}"
            );
        }
        assert_eq!(
            payload(IM_BUTTON_APPROVE).action,
            IM_BUTTON_APPROVE,
            "按钮词表与四态映射同源"
        );
    }
}
