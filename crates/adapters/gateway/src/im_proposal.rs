//! IM 快捷接入: 提案卡推送 + 按钮落账 (件四 · 自主度提案面接线)。
//!
//! 杀手锏链路 (提案真的到手机上): 提案入账 → 人审卡 [`ProposalReviewCard`]
//! **同链同信封**推到 IM 渠道 (与 IM 审批卡同一条出站链 [`crate::im_bridge::ImBridge`]、
//! 同一个 [`ImApprovalCard`] 信封 —— 0 第二套卡信封), 「建议你亲自确认」行随卡
//! 到手机 → 人在手机上点批准/拒绝 → **`proposal accept/reject` 落账** (走既有
//! `tool.proposal_draft` 工具面, 与模型侧同一份代码: 一提案一决定、重复决定
//! 拒绝即帧、append-only 不改史), `decided_by` = IM 用户名 + 渠道标注 (如实署名)。
//!
//! 三条硬语义 (全部有测试钉):
//!
//! - **推送幂等键 = 提案 id**: 同提案不重复推送 —— 推送账
//!   [`ProposalPushLedger`] 的 `pushed` 行是真值幂等集;
//! - **失败留账, IM 失败不丢提案**: 推送失败写 `failed` 行 (带真因) 待重试;
//!   提案本体永远在提案账 (`proposals.jsonl`), 外联失败零丢失;
//! - **点头也不执行**: 决定只是账面标记 ([`PROPOSAL_NO_EXECUTION_NOTE`]),
//!   执行仍走既有工具面的正常审批链。
//!
//! 提案卡**不设超时**: 提案账没有过期态, 什么时候看都行 —— 信封必填的超时戳
//! 取 [`PROPOSAL_CARD_NO_DEADLINE_MS`] 远期哨兵, 提案按钮回调不按超时判废
//! (一提案一决定只看账面状态)。

use std::path::Path;
use std::sync::Arc;

use apeireth_governance::action_binding::canonical_json_bytes;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_sdk::im::{ImApprovalCard, ImButtonPayload, IM_BUTTON_APPROVE, IM_BUTTON_REJECT};
use apeireth_tools_canonical::proposal::{
    ProposalDecision, ProposalDraft, ProposalLedger, ProposalLedgerEntry, ProposalReviewCard,
    ProposalState, ACTION_ACCEPT, ACTION_REJECT, PROPOSAL_CAPABILITY_ID,
    PROPOSAL_NO_EXECUTION_NOTE, PROPOSAL_TOOL_NAME,
};
use apeireth_tools_canonical::proposal_push::{
    ProposalPushLedger, ProposalPushOutcome, ProposalPushRecord,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 提案卡的「无期限」哨兵 (信封必填的超时戳)。
///
/// 提案账没有过期态 —— 提案什么时候看都行; 本哨兵是远期值, 只为满足信封
/// 形状 (`expires_at_ms ≥ created_at_ms`)。提案按钮回调**不按超时判废**
/// (见 [`ImProposalDecider::decide_from_click`])。
pub const PROPOSAL_CARD_NO_DEADLINE_MS: i64 = i64::MAX;

/// 按钮载荷是不是提案卡 (回调路由标记: subject = `tool.proposal_draft`)。
///
/// 提案面 grant 即用零审批, `tool.proposal_draft` 永远不产生工具审批暂停 ——
/// 该 subject 只属于提案卡, 路由无歧义。
pub fn is_proposal_payload(payload: &ImButtonPayload) -> bool {
    payload.subject == PROPOSAL_CAPABILITY_ID
}

/// IM 按钮落账的署名 (`decided_by` = IM 用户名 + 渠道标注, 如实)。
pub fn im_decided_by(channel_id: &str, operator_id: &str) -> String {
    format!("im:{operator_id}@{channel_id}")
}

/// 提案草稿 → IM 审批卡信封 (同信封: 与审批卡同一个 [`ImApprovalCard`] 形状、
/// 同一条出站链; 0 第二套卡信封)。字段映射即承诺:
///
/// - `approval_ref` = `pair_id` = 提案 id (**推送幂等键**, 也是一提案一决定的
///   闭合身份);
/// - `subject` = `tool.proposal_draft` (回调路由标记, 见 [`is_proposal_payload`]);
/// - `command_text` = 人审卡全文 ([`ProposalReviewCard::render`], 「建议你亲自
///   确认」行 + 逐条「请亲自过目」+ 温柔语气行随卡到手机);
/// - `arguments_summary` = [`ProposalDraft::summary`] (与工具面同一份摘要口径);
/// - `risk_level` = `low` (提案面风险档: 读面 + append 账, 无执行/无外联);
/// - `governance_reason` = 卡面同文的不执行声明 ([`PROPOSAL_NO_EXECUTION_NOTE`]);
/// - `action_binding_digest` = 卡载荷规范字节的 SHA-256 (小写 hex, 与动作绑定
///   同一摘要口径): 按钮绑的是这份提案, 载荷被改 → 摘要不符;
/// - `expires_at_ms` = [`PROPOSAL_CARD_NO_DEADLINE_MS`] (提案不设超时)。
pub fn proposal_card_envelope(draft: &ProposalDraft) -> ImApprovalCard {
    let card = draft.review_card();
    ImApprovalCard {
        approval_ref: draft.id.clone(),
        pair_id: draft.id.clone(),
        round: 0,
        subject: PROPOSAL_CAPABILITY_ID.to_string(),
        command_text: card.render(),
        arguments_summary: draft.summary(),
        authorization_families: Vec::new(),
        action_binding_digest: review_card_digest(&card),
        risk_level: "low".to_string(),
        governance_reason: PROPOSAL_NO_EXECUTION_NOTE.to_string(),
        created_at_ms: draft.created_at_ms,
        expires_at_ms: PROPOSAL_CARD_NO_DEADLINE_MS,
    }
}

/// 人审卡载荷的绑定摘要 (规范字节的 SHA-256, 小写 hex)。
fn review_card_digest(card: &ProposalReviewCard) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json_bytes(&card.to_json()));
    format!("{:x}", hasher.finalize())
}

/// 待推送提案 = 提案账里**还没推送过**的待审草稿 (推送幂等键 = 提案 id)。
///
/// - 只推**待审**草稿: 已点头/摇头的提案人已拍板, 无待审卡可言;
/// - 已推送 (推送账 `pushed` 行) 的不再推 (同提案不重复推送);
/// - 失败过的照推 (推送账 `failed` 行 = 留账待重试, 不是终态)。
pub fn pending_push_drafts(
    proposals: &ProposalLedger,
    pushes: &ProposalPushLedger,
) -> Result<Vec<ProposalDraft>, String> {
    let scan = proposals.scan()?;
    let pushed = pushes.pushed_ids()?;
    let mut order: Vec<String> = Vec::new();
    let mut drafts: std::collections::BTreeMap<String, ProposalDraft> =
        std::collections::BTreeMap::new();
    let mut states: std::collections::BTreeMap<String, ProposalState> =
        std::collections::BTreeMap::new();
    for record in &scan.records {
        match &record.body {
            ProposalLedgerEntry::Draft(draft) => {
                if !drafts.contains_key(&draft.id) {
                    order.push(draft.id.clone());
                }
                drafts.insert(draft.id.clone(), draft.clone());
                states.insert(draft.id.clone(), ProposalState::Pending);
            }
            ProposalLedgerEntry::Review(review) => {
                states.insert(review.proposal_id.clone(), review.decision.state());
            }
        }
    }
    Ok(order
        .iter()
        .filter(|id| states.get(*id) == Some(&ProposalState::Pending) && !pushed.contains(*id))
        .filter_map(|id| drafts.get(id).cloned())
        .collect())
}

/// 一次提案卡推送的结果 (逐提案一条: 回执或真因)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalPushResult {
    /// 提案 id (推送幂等键)。
    pub proposal_id: String,
    /// 推送结果 (`pushed` / `failed`)。
    pub outcome: ProposalPushOutcome,
    /// 事实细节 (对端回执引用 / 失败真因)。
    pub detail: String,
}

/// 一次提案按钮落账的结果 (两态闭合词表: decided / refused)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImProposalClosureResult {
    /// 已落账 (accepted / rejected 的账面标记; 恒不执行)。
    Decided(ImProposalClosure),
    /// 拒绝即帧 (重复点击 / 查无提案 / 缺署名 / 词表外按钮: 真因如实, 0 静默)。
    Refused {
        /// 拒绝的真实原因。
        reason: String,
    },
}

/// 一次提案按钮落账的记录 (回执 + 署名 + 恒不执行)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImProposalClosure {
    /// 提案 id。
    pub proposal_id: String,
    /// 决定 (`accepted` / `rejected` 的账面标记)。
    pub decision: ProposalDecision,
    /// 谁拍的板 (IM 用户名 + 渠道标注, 如实)。
    pub decided_by: String,
    /// 回执文本 (回给点击的人)。
    pub reply_text: String,
    /// 是否执行 —— **恒 `false`** (提案不直接执行)。
    pub executed: bool,
}

/// 按钮回调 → `proposal accept/reject` 的落账面。
///
/// 走既有 `tool.proposal_draft` 工具面 (与模型侧同一份代码): 一提案一决定、
/// 重复决定拒绝即帧、append-only 不改史 —— **0 第二套提案语义**。
pub struct ImProposalDecider {
    face: Arc<dyn ToolCapability>,
}

impl ImProposalDecider {
    /// 绑定 `tool.proposal_draft` 工具面 (注册面/裸工具面皆可)。
    pub fn new(face: Arc<dyn ToolCapability>) -> Self {
        Self { face }
    }

    /// IM 按钮回调入口: 署名 = IM 用户名 + 渠道标注。
    ///
    /// 拒绝即帧 (真因如实): 载荷不是提案卡 / 闭合身份不自洽 / 词表外按钮 /
    /// 缺 IM 用户标识 (decided_by 必须如实署名, 缺人不落账)。
    /// 提案卡不设超时 —— 本入口不按 `expires_at_ms` 判废。
    pub async fn decide_from_click(
        &self,
        channel_id: &str,
        operator_id: &str,
        payload: &ImButtonPayload,
    ) -> ImProposalClosureResult {
        if !is_proposal_payload(payload) {
            return ImProposalClosureResult::Refused {
                reason: "拒绝落账: 载荷不是提案卡 (subject 不是 tool.proposal_draft)".to_string(),
            };
        }
        if payload.pair_id != payload.approval_ref {
            return ImProposalClosureResult::Refused {
                reason: format!(
                    "拒绝落账: 提案卡闭合身份不自洽 (approval_ref {:?} ≠ pair_id {:?})",
                    payload.approval_ref, payload.pair_id
                ),
            };
        }
        let decision = match payload.action.as_str() {
            IM_BUTTON_APPROVE => ProposalDecision::Accepted,
            IM_BUTTON_REJECT => ProposalDecision::Rejected,
            other => {
                return ImProposalClosureResult::Refused {
                    reason: format!("拒绝落账: 提案卡只有批准/拒绝两按钮, 词表外按钮 {other:?}"),
                };
            }
        };
        if operator_id.trim().is_empty() {
            return ImProposalClosureResult::Refused {
                reason: "拒绝落账: 按钮回调缺 IM 用户标识 —— decided_by 必须如实署名".to_string(),
            };
        }
        let decided_by = im_decided_by(channel_id, operator_id);
        self.decide(&payload.approval_ref, decision, decided_by)
            .await
    }

    /// 落账一次人审决定 (动作 = `proposal accept` / `proposal reject`)。
    pub async fn decide(
        &self,
        proposal_id: &str,
        decision: ProposalDecision,
        decided_by: String,
    ) -> ImProposalClosureResult {
        let action = match decision {
            ProposalDecision::Accepted => ACTION_ACCEPT,
            ProposalDecision::Rejected => ACTION_REJECT,
        };
        let call = ToolCall {
            id: format!("im-proposal-{proposal_id}-{action}"),
            name: PROPOSAL_TOOL_NAME.to_string(),
            arguments: serde_json::json!({
                "action": action,
                "proposal_id": proposal_id,
                "decided_by": decided_by,
            }),
        };
        let result = self.face.invoke(&call).await;
        match result.outcome {
            ToolOutcome::Ok { value } => {
                let Some(state) = value.get("status").and_then(|s| s.as_str()) else {
                    return ImProposalClosureResult::Refused {
                        reason: format!("拒绝落账: 落账帧缺状态标签 (真因如实): {value}"),
                    };
                };
                let decision = match state {
                    "accepted" => ProposalDecision::Accepted,
                    "rejected" => ProposalDecision::Rejected,
                    other => {
                        return ImProposalClosureResult::Refused {
                            reason: format!("拒绝落账: 落账帧状态标签词表外 {other:?} (真因如实)"),
                        };
                    }
                };
                let landed_id = value
                    .get("proposal_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or(proposal_id)
                    .to_string();
                let landed_by = value
                    .get("decided_by")
                    .and_then(|by| by.as_str())
                    .unwrap_or(decided_by.as_str())
                    .to_string();
                let reply_text = format!(
                    "已落账: 提案 {landed_id} = {state} (decided_by: {landed_by}); \
                     提案不直接执行 —— 点头/摇头只是账面标记, 落地仍走既有工具面的正常审批链。"
                );
                ImProposalClosureResult::Decided(ImProposalClosure {
                    proposal_id: landed_id,
                    decision,
                    decided_by: landed_by,
                    reply_text,
                    executed: false,
                })
            }
            ToolOutcome::Error { message, .. } => {
                ImProposalClosureResult::Refused { reason: message }
            }
        }
    }
}

/// 提案面接线 (一套身份, 一条链): 提案账 + 推送账 + 按钮落账面。
pub struct ImProposalFace {
    /// 提案账 (待推送草稿的来源 + 决定落账的真值账, `proposals.jsonl`)。
    pub proposals: ProposalLedger,
    /// 推送账 (幂等键 + 失败留账, `proposal-pushes.jsonl`)。
    pub pushes: ProposalPushLedger,
    /// 按钮回调落账面 (`proposal accept/reject`)。
    pub decider: ImProposalDecider,
}

impl ImProposalFace {
    /// 数据目录落位 (提案账 + 推送账同目录) + `tool.proposal_draft` 工具面。
    pub fn new(data_dir: impl AsRef<Path>, face: Arc<dyn ToolCapability>) -> Self {
        Self {
            proposals: ProposalLedger::under_data_dir(data_dir.as_ref()),
            pushes: ProposalPushLedger::under_data_dir(data_dir.as_ref()),
            decider: ImProposalDecider::new(face),
        }
    }

    /// 待推送草稿 (只推待审 + 幂等去重, 见 [`pending_push_drafts`])。
    pub fn pending_drafts(&self) -> Result<Vec<ProposalDraft>, String> {
        pending_push_drafts(&self.proposals, &self.pushes)
    }

    /// 留一笔推送账 (append-only; **留账失败必须报真因**, 不静默)。
    pub fn record_push(&self, record: &ProposalPushRecord) -> Result<(), String> {
        self.pushes.append(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tools_canonical::normalize_detail;

    fn draft(id: &str) -> ProposalDraft {
        ProposalDraft {
            id: id.to_string(),
            kind: apeireth_tools_canonical::proposal::ProposalKind::Tidy,
            title: "边角料: 三处临时注释待清理".to_string(),
            detail: normalize_detail("这三行注释是调试时留下的。"),
            evidence: vec!["src/a.rs:12".to_string(), "src/b.rs:40".to_string()],
            draft_payload: serde_json::json!({"files": ["src/a.rs", "src/b.rs"]}),
            created_at_ms: 1_700_000_000_000,
        }
    }

    fn payload(action: &str) -> ImButtonPayload {
        ImButtonPayload {
            approval_ref: "prop-1".to_string(),
            pair_id: "prop-1".to_string(),
            round: 0,
            subject: PROPOSAL_CAPABILITY_ID.to_string(),
            expires_at_ms: PROPOSAL_CARD_NO_DEADLINE_MS,
            action: action.to_string(),
        }
    }

    /// 同信封: 提案卡就是 ImApprovalCard 信封 (身份/两按钮/无期限哨兵)。
    #[test]
    fn the_proposal_card_rides_the_approval_card_envelope_unchanged() {
        let envelope = proposal_card_envelope(&draft("prop-1"));
        envelope.validate().unwrap();
        assert_eq!(envelope.approval_ref, "prop-1", "幂等键 = 提案 id");
        assert_eq!(envelope.pair_id, "prop-1", "一提案一决定的闭合身份");
        assert_eq!(envelope.subject, PROPOSAL_CAPABILITY_ID);
        assert_eq!(envelope.risk_level, "low", "提案面风险档 = 低");
        assert_eq!(envelope.expires_at_ms, PROPOSAL_CARD_NO_DEADLINE_MS);
        assert_eq!(envelope.created_at_ms, 1_700_000_000_000);
        assert!(
            envelope.command_text.contains("建议你亲自确认"),
            "温柔行随卡"
        );
        assert!(envelope.command_text.contains("【请亲自过目】"), "证据标黄");
        assert!(
            envelope.command_text.contains("你最好亲自看一遍"),
            "温柔语气"
        );
        assert!(envelope.command_text.contains("prop-1"), "卡面带提案 id");
        assert_eq!(envelope.arguments_summary, draft("prop-1").summary());
        // 动作摘要 = 卡载荷规范字节的 SHA-256 (小写 hex, 64 位), 稳定可复算。
        assert_eq!(envelope.action_binding_digest.len(), 64);
        assert_eq!(
            envelope.action_binding_digest,
            proposal_card_envelope(&draft("prop-1")).action_binding_digest,
            "同一份提案载荷 → 同一摘要"
        );
    }

    /// 回调路由: subject 认提案卡; 署名 = IM 用户名 + 渠道标注。
    #[test]
    fn proposal_payloads_are_routed_by_subject_and_signed_honestly() {
        assert!(is_proposal_payload(&payload(IM_BUTTON_APPROVE)));
        let mut approval = payload(IM_BUTTON_APPROVE);
        approval.subject = "tool.calculator".to_string();
        assert!(!is_proposal_payload(&approval), "工具审批卡不进提案路由");
        assert_eq!(im_decided_by("primary", "u_demo"), "im:u_demo@primary");
    }

    /// 待推送选择: 只推待审 + 幂等去重; 已决定/已推送不再推。
    #[test]
    fn pending_push_drafts_only_picks_undecided_and_unpushed_drafts() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-im-proposal-select-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let proposals = ProposalLedger::under_data_dir(&dir);
        let pushes = ProposalPushLedger::under_data_dir(&dir);
        proposals.append_draft(&draft("prop-1")).unwrap();
        proposals.append_draft(&draft("prop-2")).unwrap();
        proposals.append_draft(&draft("prop-3")).unwrap();
        proposals
            .append_review(&apeireth_tools_canonical::proposal::ProposalReview {
                proposal_id: "prop-2".to_string(),
                decision: ProposalDecision::Rejected,
                decided_by: "owner".to_string(),
                note: None,
                at_ms: 1_700_000_000_100,
            })
            .unwrap();
        pushes
            .append(&ProposalPushRecord {
                proposal_id: "prop-3".to_string(),
                channel_id: "primary".to_string(),
                conversation_id: "oc_demo_chat".to_string(),
                outcome: ProposalPushOutcome::Pushed,
                message_ref: Some("m-3".to_string()),
                reason: None,
                at_ms: 1_700_000_000_200,
            })
            .unwrap();
        let pending = pending_push_drafts(&proposals, &pushes).unwrap();
        let ids: Vec<&str> = pending.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["prop-1"], "只推待审且未推送的草稿");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
