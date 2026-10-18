//! IM 快捷接入: 设备审批卡推送 + 按钮批准/拒绝落账 (K 轴 L3/L4 · 设备审批卡→IM)。
//!
//! 杀手锏链路 (不可逆/动钱动作的审批真的到手机上): `tool.device_control` 预览拍
//! 产出的 [`DeviceApprovalCard`] 卡载荷 → **同链同信封**推到 IM 渠道 (与提案卡/
//! 工具审批卡同一条出站链 [`crate::im_bridge::ImBridge`]、同一个 [`ImApprovalCard`]
//! 信封 —— 0 第二套卡信封) → 人在手机上点批准/拒绝 → **设备动作批准/拒绝落账**:
//!
//! - **批准 = 批准即执行**: 走既有 [`DeviceControlRunner::execute`] (与模型侧/
//!   面板侧同一台执行核心, 0 第二套执行语义) —— 三拍同计划绑定 + 先模拟后执行
//!   + 分级审批证据照常生效; 执行落 [`DeviceActionLedger`] 状态账, `approved_by`
//!   = IM 用户名 + 渠道标注 (如实署名)。**money 档必须 IM 卡**: 执行证据的
//!   `im_card_id` 取本卡**送达回执** (`message_ref`) —— 卡没到手机就没有回执,
//!   动钱动作无从批准 (面板侧执行缺 `im_card_id` 仍被既有 `ImCardRequired` 拒)。
//!   **irreversible 可面板或 IM**: 面板侧带 `approved_by` 执行与本链批准互为
//!   独立入口, 语义一致;
//! - **拒绝 = 不执行 + 关账**: 落按钮决定账 ([`DeviceApprovalDecisionLedger`]),
//!   **一计划一决定** —— 拍过板的计划再点拒绝即帧 (卡是一次性拍板面);
//! - **推送幂等键 = 动作 plan_id**: 同计划不重复推送 (推送账
//!   [`DeviceApprovalPushLedger::pushed_plan_ids`] 是真值幂等集);
//! - **失败留账重试**: 推送失败写 `failed` 行 (带真因 + 卡载荷) 待重试 —— IM
//!   失败不丢卡, 重试只凭推送账即可重建出站 ([`crate::im_bridge::ImBridge::retry_device_approval_pushes`])。
//!
//! 回调路由 (与工具审批卡无歧义): 本卡 `subject` = [`DEVICE_APPROVAL_CARD_SUBJECT`]
//! (动作计划卡的闭合身份 = plan_id), **不是**能力 id —— `tool.device_control`
//! 的 execute 拍本身会停工具审批, 那张工具审批卡的 subject 是能力 id、走四态
//! 闭合链 ([`crate::im_approval`]); 两个 subject 分家, 两条路由各自闭合。
//!
//! 本面只记**推送痕迹与人的拍板**, 设备动作语义 (计划/预览/执行/状态账) 全在
//! [`apeireth_tools_canonical::device::device_control`], 零第二套。

use std::path::Path;
use std::sync::Arc;

use apeireth_governance::action_binding::canonical_json_bytes;
use apeireth_sdk::im::{ImApprovalCard, ImButtonPayload, IM_BUTTON_APPROVE, IM_BUTTON_REJECT};
use apeireth_tools_canonical::device::approval::{
    DeviceActionDecision, DeviceApprovalDecisionLedger, DeviceApprovalDecisionRecord,
    DeviceApprovalPushLedger, DeviceApprovalPushOutcome, DeviceApprovalPushRecord,
};
use apeireth_tools_canonical::device::device_control::{
    grade_risk_label, ApprovalEvidence, DeviceApprovalCard, DeviceControlRunner,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::im_proposal::im_decided_by;

/// 设备动作计划卡的回调路由标记 (`subject`)。
///
/// 为什么不是能力 id `tool.device_control`: 该能力的 execute 拍本身会停工具
/// 审批, 那张**工具审批卡** (闭合身份 = approval id) 的 subject 是能力 id、走
/// 四态闭合链; 本卡是**动作计划卡** (闭合身份 = plan_id), 独立 subject 让两条
/// 路由无歧义、各有各的闭合语义。
pub const DEVICE_APPROVAL_CARD_SUBJECT: &str = "tool.device_control.plan";

/// 设备审批卡的「无期限」哨兵 (信封必填的超时戳)。
///
/// 设备审批卡**不设卡面超时**: 点按本身就是底线②③要求的那次逐次人批, 卡面
/// 载荷的动作摘要把「批的是什么」钉死; 真正的一次性闸门在执行面 —— 三拍的
/// 预览票一次性 (先模拟后执行), 没预览/已核销的计划点批准也进不了执行。
/// 本哨兵是远期值, 只为满足信封形状 (`expires_at_ms ≥ created_at_ms`)。
pub const DEVICE_CARD_NO_DEADLINE_MS: i64 = i64::MAX;

/// 按钮载荷是不是设备动作计划卡 (回调路由标记: subject = [`DEVICE_APPROVAL_CARD_SUBJECT`])。
pub fn is_device_approval_payload(payload: &ImButtonPayload) -> bool {
    payload.subject == DEVICE_APPROVAL_CARD_SUBJECT
}

/// 卡载荷的绑定摘要 (规范字节的 SHA-256, 小写 hex; 与提案卡同一摘要口径)。
/// 按钮绑的是这份卡载荷, 载荷被改 → 摘要不符。
fn device_card_digest(card: &DeviceApprovalCard) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json_bytes(&card.to_json()));
    format!("{:x}", hasher.finalize())
}

/// 设备动作计划卡 → IM 审批卡信封 (同信封: 与提案卡/工具审批卡同一个
/// [`ImApprovalCard`] 形状、同一条出站链, 0 第二套卡信封)。字段映射即承诺:
///
/// - `approval_ref` = `pair_id` = 动作 plan_id (**推送幂等键**, 也是一计划一
///   决定的闭合身份);
/// - `subject` = [`DEVICE_APPROVAL_CARD_SUBJECT`] (回调路由标记, 与工具审批卡
///   分家);
/// - `command_text` = 卡面全文 ([`DeviceApprovalCard::render`]: 将发生什么 +
///   分级语义 + money 档 IM 卡标注随卡到手机);
/// - `arguments_summary` = 设备 / 动作 / 分级 / 动作参数 (闭合键集, 有界);
/// - `risk_level` = 分级风险标注 ([`grade_risk_label`], 卡载荷同文);
/// - `governance_reason` = 分级审批语义说明 (卡面同文);
/// - `action_binding_digest` = 卡载荷规范字节的 SHA-256 (小写 hex);
/// - `expires_at_ms` = [`DEVICE_CARD_NO_DEADLINE_MS`] (卡不设超时, 见哨兵注)。
pub fn device_card_envelope(card: &DeviceApprovalCard, created_at_ms: i64) -> ImApprovalCard {
    let params_text = serde_json::to_string(&card.params).unwrap_or_default();
    ImApprovalCard {
        approval_ref: card.plan_id.clone(),
        pair_id: card.plan_id.clone(),
        round: 0,
        subject: DEVICE_APPROVAL_CARD_SUBJECT.to_string(),
        command_text: card.render(),
        arguments_summary: format!(
            "{} · {} · 分级 {} · 参数 {}",
            card.device_name, card.device_action, card.grade, params_text
        ),
        authorization_families: Vec::new(),
        action_binding_digest: device_card_digest(card),
        risk_level: grade_risk_label(&card.grade).to_string(),
        governance_reason: card.approval_note.clone(),
        created_at_ms,
        expires_at_ms: DEVICE_CARD_NO_DEADLINE_MS,
    }
}

/// 一次设备审批卡推送的结果 (逐卡一条: 回执或真因)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceApprovalPushResult {
    /// 动作 plan_id (推送幂等键)。
    pub plan_id: String,
    /// 推送结果 (`pushed` / `failed`)。
    pub outcome: DeviceApprovalPushOutcome,
    /// 事实细节 (对端回执引用 / 失败真因)。
    pub detail: String,
}

/// 一次设备审批卡按钮落账的结果 (两态闭合词表: decided / refused)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImDeviceClosureResult {
    /// 已落账 (批准即执行 / 拒绝即关账)。
    Decided(ImDeviceClosure),
    /// 拒绝即帧 (重复点击 / 查无送达 / 缺署名 / 执行失败: 真因如实, 0 静默)。
    Refused {
        /// 拒绝的真实原因。
        reason: String,
    },
}

/// 一次设备审批卡按钮落账的记录 (回执 + 署名 + 执行事实)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImDeviceClosure {
    /// 动作 plan_id。
    pub plan_id: String,
    /// 拍板结果 (`approved` / `rejected`)。
    pub decision: DeviceActionDecision,
    /// 谁拍的板 (IM 用户名 + 渠道标注, 如实)。
    pub decided_by: String,
    /// IM 审批卡标识 (送达回执; money 档执行证据同源)。
    pub im_card_id: Option<String>,
    /// 是否已执行 —— 批准 = 批准即执行 (`true` + 状态账记录 id);
    /// 拒绝 / 批准但执行未落地 = `false` (回执文本如实说明)。
    pub executed: bool,
    /// 状态账记录 id (已执行时在场)。
    pub record_id: Option<String>,
    /// 回执文本 (回给点击的人)。
    pub reply_text: String,
}

/// 设备审批卡按钮回调 → 设备动作批准/拒绝的落账面 (一套身份, 一条链)。
pub struct ImDeviceApprovalFace {
    /// 推送账 (幂等键 + 失败留账, `device-approval-pushes.jsonl`)。
    pub pushes: DeviceApprovalPushLedger,
    /// 按钮决定账 (一计划一决定 + 如实署名, `device-approval-decisions.jsonl`)。
    pub decisions: DeviceApprovalDecisionLedger,
    /// 执行核心 (与 `tool.device_control` 工具面同一台: 批准即执行走它)。
    runner: Arc<DeviceControlRunner>,
}

impl ImDeviceApprovalFace {
    /// 数据目录落位 (推送账 + 决定账同目录) + 设备受控执行核心。
    pub fn new(data_dir: impl AsRef<Path>, runner: Arc<DeviceControlRunner>) -> Self {
        Self {
            pushes: DeviceApprovalPushLedger::under_data_dir(data_dir.as_ref()),
            decisions: DeviceApprovalDecisionLedger::under_data_dir(data_dir.as_ref()),
            runner,
        }
    }

    /// 留一笔推送账 (append-only; **留账失败必须报真因**, 不静默)。
    pub fn record_push(&self, record: &DeviceApprovalPushRecord) -> Result<(), String> {
        self.pushes.append(record)
    }

    /// 设备审批卡按钮回调入口: 署名 = IM 用户名 + 渠道标注 (与提案链同一署名口径)。
    ///
    /// 拒绝即帧 (真因如实): 载荷不是设备计划卡 / 闭合身份不自洽 / 词表外按钮 /
    /// 缺 IM 用户标识 (decided_by 必须如实署名, 缺人不落账) / 一计划一决定已拍板 /
    /// 查无送达记录 (卡没到手机, 点击不可信 —— money 档无送达回执即无 IM 卡)。
    pub fn decide_from_click(
        &self,
        channel_id: &str,
        operator_id: &str,
        payload: &ImButtonPayload,
    ) -> ImDeviceClosureResult {
        if !is_device_approval_payload(payload) {
            return ImDeviceClosureResult::Refused {
                reason: "拒绝落账: 载荷不是设备审批卡 (subject 不是 tool.device_control.plan)"
                    .to_string(),
            };
        }
        if payload.pair_id != payload.approval_ref {
            return ImDeviceClosureResult::Refused {
                reason: format!(
                    "拒绝落账: 设备审批卡闭合身份不自洽 (approval_ref {:?} ≠ pair_id {:?})",
                    payload.approval_ref, payload.pair_id
                ),
            };
        }
        let decision = match payload.action.as_str() {
            IM_BUTTON_APPROVE => DeviceActionDecision::Approved,
            IM_BUTTON_REJECT => DeviceActionDecision::Rejected,
            other => {
                return ImDeviceClosureResult::Refused {
                    reason: format!(
                        "拒绝落账: 设备审批卡只有批准/拒绝两按钮, 词表外按钮 {other:?}"
                    ),
                }
            }
        };
        if operator_id.trim().is_empty() {
            return ImDeviceClosureResult::Refused {
                reason: "拒绝落账: 按钮回调缺 IM 用户标识 —— decided_by 必须如实署名".to_string(),
            };
        }
        let plan_id = payload.approval_ref.as_str();
        match self.decisions.decision_for(plan_id) {
            Ok(Some(already)) => {
                return ImDeviceClosureResult::Refused {
                    reason: format!(
                        "拒绝落账: 一计划一决定 —— 计划 {plan_id} 已拍板 ({}), 再点不翻案",
                        already.as_str()
                    ),
                }
            }
            Ok(None) => {}
            Err(error) => {
                return ImDeviceClosureResult::Refused {
                    reason: format!("拒绝落账: 决定账读取失败 (真因如实): {error}"),
                }
            }
        }
        let delivered = match self.pushes.delivered_record_for(plan_id) {
            Ok(record) => record,
            Err(error) => {
                return ImDeviceClosureResult::Refused {
                    reason: format!("拒绝落账: 推送账读取失败 (真因如实): {error}"),
                }
            }
        };
        let Some(delivered) = delivered else {
            return ImDeviceClosureResult::Refused {
                reason: format!(
                    "拒绝落账: 查无计划 {plan_id} 的送达记录 —— 审批卡没到手机就没有送达回执, \
                     点击不可信 (money 档无回执 = 无 IM 卡, 拒绝即帧)"
                ),
            };
        };
        let decided_by = im_decided_by(channel_id, operator_id);
        let im_card_id = delivered.message_ref.clone();
        match decision {
            DeviceActionDecision::Rejected => {
                let record = DeviceApprovalDecisionRecord {
                    plan_id: plan_id.to_string(),
                    decision: DeviceActionDecision::Rejected,
                    decided_by: decided_by.clone(),
                    im_card_id: im_card_id.clone(),
                    at_ms: now_epoch_ms(),
                };
                if let Err(error) = self.decisions.append(&record) {
                    return ImDeviceClosureResult::Refused {
                        reason: format!("拒绝落账: 决定账写入失败 (真因如实, 不静默): {error}"),
                    };
                }
                ImDeviceClosureResult::Decided(ImDeviceClosure {
                    plan_id: plan_id.to_string(),
                    decision: DeviceActionDecision::Rejected,
                    decided_by,
                    im_card_id,
                    executed: false,
                    record_id: None,
                    reply_text: format!(
                        "已拒绝: 设备动作计划 {plan_id} 不执行 (拒绝者: {}); 一计划一决定, 本卡关账 —— \
                         拒绝即不执行, 设备零变化。",
                        record.decided_by
                    ),
                })
            }
            DeviceActionDecision::Approved => {
                // 批准即执行: 与模型侧/面板侧同一台执行核心 (三拍绑定 + 先模拟
                // 后执行 + 分级审批证据照常生效)。money 档的 `im_card_id` = 本卡
                // 送达回执 —— 卡没到手机就无从批准。
                let evidence = ApprovalEvidence {
                    approved_by: Some(decided_by.clone()),
                    im_card_id: im_card_id.clone(),
                };
                let card = &delivered.card;
                match self.runner.execute(
                    &card.device,
                    &card.device_action,
                    &card.params,
                    Some(&card.plan_id),
                    &evidence,
                ) {
                    Ok(frame) => {
                        let record = DeviceApprovalDecisionRecord {
                            plan_id: plan_id.to_string(),
                            decision: DeviceActionDecision::Approved,
                            decided_by: decided_by.clone(),
                            im_card_id: im_card_id.clone(),
                            at_ms: now_epoch_ms(),
                        };
                        let bookkeeping_note = match self.decisions.append(&record) {
                            Ok(()) => String::new(),
                            Err(error) => format!(
                                " ⚠ 决定留账失败 (真因如实, 不静默): {error} —— 执行已落地 ({}), \
                                 不可撤",
                                frame.record_id
                            ),
                        };
                        ImDeviceClosureResult::Decided(ImDeviceClosure {
                            plan_id: plan_id.to_string(),
                            decision: DeviceActionDecision::Approved,
                            decided_by,
                            im_card_id,
                            executed: true,
                            record_id: Some(frame.record_id.clone()),
                            reply_text: format!(
                                "已批准并执行: 设备动作计划 {plan_id} (状态账记录 {}); 谁批: {}。",
                                frame.record_id, frame.approved_by
                            ) + &bookkeeping_note,
                        })
                    }
                    Err(error) => ImDeviceClosureResult::Refused {
                        reason: format!(
                            "批准点击未执行 (真因如实): {} —— 决定未落账, 排障后可再点",
                            error.message()
                        ),
                    },
                }
            }
        }
    }
}

/// 当前时刻 (epoch 毫秒); 时钟回拨不编造, 如实取系统读数。
fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tools_canonical::device::device_control::ActionGrade;

    fn money_card() -> DeviceApprovalCard {
        DeviceApprovalCard {
            plan_id: "0123456789abcdef".to_string(),
            device: "shop".to_string(),
            device_name: "商店".to_string(),
            device_action: "purchase".to_string(),
            grade: ActionGrade::Money.as_str().to_string(),
            params: serde_json::json!({"item": "牛奶", "amount": 12.5, "currency": "CNY"}),
            effects: vec!["charged_amount: - → 12.5".to_string()],
            requires_im_card: true,
            approval_note: "动钱动作: 永远逐次人批 + IM 审批卡 (无自动档, 底线②)。".to_string(),
        }
    }

    /// 同信封: 设备审批卡就是 ImApprovalCard 信封 (闭合身份 = plan_id, 风险档
    /// 同文, 无期限哨兵)。
    #[test]
    fn the_device_card_rides_the_approval_card_envelope_unchanged() {
        let envelope = device_card_envelope(&money_card(), 1_700_000_000_000);
        envelope.validate().unwrap();
        assert_eq!(
            envelope.approval_ref, "0123456789abcdef",
            "幂等键 = plan_id"
        );
        assert_eq!(
            envelope.pair_id, envelope.approval_ref,
            "一计划一决定闭合身份"
        );
        assert_eq!(envelope.subject, DEVICE_APPROVAL_CARD_SUBJECT);
        assert_eq!(envelope.risk_level, "high", "money 档风险档与卡载荷同文");
        assert_eq!(envelope.expires_at_ms, DEVICE_CARD_NO_DEADLINE_MS);
        assert_eq!(envelope.created_at_ms, 1_700_000_000_000);
        assert!(
            envelope.command_text.contains("将发生什么 (先模拟后执行)"),
            "三拍纪律随卡"
        );
        assert!(
            envelope.command_text.contains("IM 审批卡"),
            "money 档 IM 卡标注随卡"
        );
        assert!(
            envelope.arguments_summary.contains("purchase"),
            "{:?}",
            envelope.arguments_summary
        );
        assert!(
            envelope.arguments_summary.contains("12.5"),
            "参数真值进摘要"
        );
        // 动作摘要 = 卡载荷规范字节的 SHA-256 (64 位小写 hex), 稳定可复算。
        assert_eq!(envelope.action_binding_digest.len(), 64);
        assert_eq!(
            envelope.action_binding_digest,
            device_card_envelope(&money_card(), 999).action_binding_digest,
            "同一份卡载荷 → 同一摘要"
        );
    }

    /// 回调路由: subject 认设备计划卡; 与工具审批卡 (能力 id) 分家无歧义。
    #[test]
    fn device_payloads_are_routed_by_a_subject_of_their_own() {
        let payload = ImButtonPayload {
            approval_ref: "0123456789abcdef".to_string(),
            pair_id: "0123456789abcdef".to_string(),
            round: 0,
            subject: DEVICE_APPROVAL_CARD_SUBJECT.to_string(),
            expires_at_ms: DEVICE_CARD_NO_DEADLINE_MS,
            action: IM_BUTTON_APPROVE.to_string(),
        };
        assert!(is_device_approval_payload(&payload));
        let mut tool_approval = payload.clone();
        tool_approval.subject = "tool.device_control".to_string();
        assert!(
            !is_device_approval_payload(&tool_approval),
            "工具审批卡 (能力 id subject) 不进设备计划卡路由"
        );
    }
}
