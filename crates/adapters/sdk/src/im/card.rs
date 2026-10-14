//! IM 快捷接入: 审批卡片 (命令文本 / 风险级 / 批准·拒绝按钮) + 按钮回调载荷。
//!
//! 卡片是**展示面**, 不是授权面: 按钮载荷 [`ImButtonPayload`] 只携带闭合所需的
//! 身份 (approval_ref / pair_id / round / subject) 与本地超时戳 (`expires_at_ms`),
//! 治理闭合仍在网关侧走 `approval_closure` 四态词表。`im-feishu` 渠道的卡片
//! wire 消费 [`crate::lark::message::CardContent`] (同形状元素数组), 其余 kind
//! 走本适配器的模板信封 (按钮载荷一致)。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::im::channel::ImChannelKind;
use crate::im::error::{ImError, ImResult};

/// 批准按钮的 action id。
pub const IM_BUTTON_APPROVE: &str = "approve";
/// 拒绝按钮的 action id。
pub const IM_BUTTON_REJECT: &str = "reject";
/// 取消的 action id (卡片不渲染, 闭合词表可达)。
pub const IM_BUTTON_CANCEL: &str = "cancel";

/// 风险级词表 (与治理侧 risk 序一致: info/low < medium < high < critical < nuclear)。
pub const IM_RISK_LEVELS: &[&str] = &["info", "low", "medium", "high", "critical", "nuclear"];

/// 校验风险级标签 (词表外 = fail-closed)。
pub fn validate_risk_level(level: &str) -> ImResult<()> {
    if IM_RISK_LEVELS.contains(&level) {
        Ok(())
    } else {
        Err(ImError::InvalidConfig {
            reason: format!("unknown risk level {level:?}"),
        })
    }
}

/// 按钮回调载荷 (随卡片往返; 闭合身份 + 超时戳)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImButtonPayload {
    /// 被审批操作的稳定引用 (approval id 字符串)。
    pub approval_ref: String,
    /// 审计配对身份 (asked ↔ decision 同一 pair)。
    pub pair_id: String,
    /// 配对所属轮次。
    pub round: u64,
    /// 被审批操作的能力身份 (审计 subject)。
    pub subject: String,
    /// 本地审批超时戳 (epoch 毫秒; 与本地 `expires_at` 同源)。
    pub expires_at_ms: i64,
    /// 按钮 action id (approve / reject / cancel)。
    pub action: String,
}

impl ImButtonPayload {
    /// 载荷结构校验 (身份非空 + action 在词表内)。
    pub fn validate(&self) -> ImResult<()> {
        if self.approval_ref.trim().is_empty() || self.pair_id.trim().is_empty() {
            return Err(ImError::Decode {
                reason: "button payload is missing its identity".to_string(),
            });
        }
        if !is_known_action(&self.action) {
            return Err(ImError::Decode {
                reason: format!("unknown button action {:?}", self.action),
            });
        }
        Ok(())
    }
}

/// action id 是否在按钮词表内。
pub fn is_known_action(action: &str) -> bool {
    matches!(
        action,
        IM_BUTTON_APPROVE | IM_BUTTON_REJECT | IM_BUTTON_CANCEL
    )
}

/// 一张待人审批的卡片 (展示字段全集)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImApprovalCard {
    /// 被审批操作的稳定引用。
    pub approval_ref: String,
    /// 审计配对身份。
    pub pair_id: String,
    /// 配对所属轮次。
    pub round: u64,
    /// 被审批操作的能力身份。
    pub subject: String,
    /// 一行命令文本 (人在手机上判断的主信息)。
    pub command_text: String,
    /// 参数短摘要。
    pub arguments_summary: String,
    /// 审批卡族标注 / 授权对象 (E-P2 审批族归一): 被批命令的族 id 列表
    /// (如 `["删除族"]`)。批"删除族" = 该族所有形态生效, 同族换形重放被
    /// 本次授权覆盖, 跨族仍逐族审批。空 = 非命令面操作 (授权对象 = 操作
    /// 本身, 逐操作授权)。`#[serde(default)]`: 旧卡片载荷无此字段照读。
    #[serde(default)]
    pub authorization_families: Vec<String>,
    /// 风险级 (词表标签)。
    pub risk_level: String,
    /// 治理给出的原因 (展示用)。
    pub governance_reason: String,
    /// 卡片创建时间 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 本地审批超时戳 (epoch 毫秒)。
    pub expires_at_ms: i64,
}

impl ImApprovalCard {
    /// 卡片结构校验 (命令文本非空 + 风险级在词表 + 超时戳不早于创建)。
    pub fn validate(&self) -> ImResult<()> {
        if self.command_text.trim().is_empty() {
            return Err(ImError::InvalidConfig {
                reason: "approval card needs a command text".to_string(),
            });
        }
        validate_risk_level(&self.risk_level)?;
        if self.expires_at_ms < self.created_at_ms {
            return Err(ImError::InvalidConfig {
                reason: "approval card expiry must not precede creation".to_string(),
            });
        }
        ImButtonPayload::from_card(self, IM_BUTTON_APPROVE).validate()
    }

    /// 该卡片某按钮的回调载荷。
    pub fn button_payload(&self, action: &str) -> ImResult<ImButtonPayload> {
        if !is_known_action(action) {
            return Err(ImError::InvalidConfig {
                reason: format!("unknown button action {action:?}"),
            });
        }
        Ok(ImButtonPayload::from_card(self, action))
    }
}

impl ImButtonPayload {
    fn from_card(card: &ImApprovalCard, action: &str) -> Self {
        Self {
            approval_ref: card.approval_ref.clone(),
            pair_id: card.pair_id.clone(),
            round: card.round,
            subject: card.subject.clone(),
            expires_at_ms: card.expires_at_ms,
            action: action.to_string(),
        }
    }
}

/// 渲染一张审批卡片为该渠道的 wire 信封体 (POST body 的 `card` 部分)。
pub fn render_approval_card(
    kind: ImChannelKind,
    card: &ImApprovalCard,
) -> ImResult<serde_json::Value> {
    card.validate()?;
    match kind {
        ImChannelKind::ImFeishu => render_feishu_card(card),
        ImChannelKind::ImWecom => render_template_card("im-wecom", card),
        ImChannelKind::ImQq => render_template_card("im-qq", card),
    }
}

/// 审批卡族标注的人读文本: 族 id 以 `、` 连接; 空 = 非命令面操作
/// (授权对象 = 操作本身, 逐操作授权)。批"删除族" = 该族所有形态生效。
pub fn card_family_annotation(card: &ImApprovalCard) -> String {
    if card.authorization_families.is_empty() {
        "（无族·逐操作授权）".to_string()
    } else {
        card.authorization_families.join("、")
    }
}

/// `im-feishu` 渠道: 消费 [`crate::lark::message::CardContent`] 元素形状
/// (header + div 文本 + action 按钮), 按钮 `value` 即 [`ImButtonPayload`]。
fn render_feishu_card(card: &ImApprovalCard) -> ImResult<serde_json::Value> {
    use crate::lark::message::CardContent;

    let mut content = CardContent::plain("待审批操作", card.command_text.clone());
    content.header.insert(
        "template".to_string(),
        serde_json::Value::String("orange".to_string()),
    );

    let mut detail = HashMap::new();
    detail.insert(
        "tag".to_string(),
        serde_json::Value::String("div".to_string()),
    );
    detail.insert(
        "text".to_string(),
        serde_json::json!({
            "tag": "lark_md",
            "content": format!(
                "**授权族**: {}\n**风险级**: {}\n**摘要**: {}\n**治理原因**: {}\n**超时**: {} ms",
                card_family_annotation(card),
                card.risk_level,
                card.arguments_summary,
                card.governance_reason,
                card.expires_at_ms
            ),
        }),
    );
    content.elements.push(detail);

    let mut actions = HashMap::new();
    actions.insert(
        "tag".to_string(),
        serde_json::Value::String("action".to_string()),
    );
    let buttons: Vec<serde_json::Value> = [(IM_BUTTON_APPROVE, "批准"), (IM_BUTTON_REJECT, "拒绝")]
        .into_iter()
        .map(|(action, label)| {
            let payload = card.button_payload(action)?;
            serde_json::to_value(payload)
                .map(|value| {
                    serde_json::json!({
                        "tag": "button",
                        "text": {"tag": "plain_text", "content": label},
                        "type": if action == IM_BUTTON_APPROVE { "primary" } else { "danger" },
                        "value": value,
                    })
                })
                .map_err(|e| ImError::InvalidConfig {
                    reason: format!("button payload serialization failed: {e}"),
                })
        })
        .collect::<ImResult<Vec<_>>>()?;
    actions.insert("actions".to_string(), serde_json::Value::Array(buttons));
    content.elements.push(actions);

    serde_json::to_value(content).map_err(|e| ImError::InvalidConfig {
        reason: format!("approval card serialization failed: {e}"),
    })
}

/// `im-wecom` / `im-qq` 渠道: 本适配器的模板卡信封 (按钮载荷与 `im-feishu` 一致)。
fn render_template_card(kind: &str, card: &ImApprovalCard) -> ImResult<serde_json::Value> {
    let buttons: Vec<serde_json::Value> = [(IM_BUTTON_APPROVE, "批准"), (IM_BUTTON_REJECT, "拒绝")]
        .into_iter()
        .map(|(action, label)| {
            let payload = card.button_payload(action)?;
            serde_json::to_value(payload)
                .map(|value| {
                    serde_json::json!({
                        "button_id": action,
                        "text": label,
                        "value": value,
                    })
                })
                .map_err(|e| ImError::InvalidConfig {
                    reason: format!("button payload serialization failed: {e}"),
                })
        })
        .collect::<ImResult<Vec<_>>>()?;

    let family_label = card_family_annotation(card);
    Ok(serde_json::json!({
        "card_kind": kind,
        "msgtype": "approval_card",
        "card": {
            "title": "待审批操作",
            "command_text": card.command_text,
            "arguments_summary": card.arguments_summary,
            "authorization_families": card.authorization_families,
            "authorization_family_label": family_label,
            "risk_level": card.risk_level,
            "governance_reason": card.governance_reason,
            "expires_at_ms": card.expires_at_ms,
            "buttons": buttons,
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> ImApprovalCard {
        ImApprovalCard {
            approval_ref: "apr_1".to_string(),
            pair_id: "pair-1".to_string(),
            round: 2,
            subject: "tool.shell".to_string(),
            command_text: "shell: bash -c 'rm -rf /tmp/x'".to_string(),
            arguments_summary: "执行命令 rm -rf /tmp/x".to_string(),
            authorization_families: vec!["删除族".to_string()],
            risk_level: "high".to_string(),
            governance_reason: "destructive write".to_string(),
            created_at_ms: 1_700_000_000_000,
            expires_at_ms: 1_700_000_300_000,
        }
    }

    #[test]
    fn card_validation_guards_text_risk_and_expiry() {
        let good = card();
        good.validate().unwrap();

        let mut empty_text = card();
        empty_text.command_text = " ".to_string();
        assert!(matches!(
            empty_text.validate(),
            Err(ImError::InvalidConfig { .. })
        ));

        let mut bad_risk = card();
        bad_risk.risk_level = "extreme".to_string();
        assert!(matches!(
            bad_risk.validate(),
            Err(ImError::InvalidConfig { .. })
        ));

        let mut bad_expiry = card();
        bad_expiry.expires_at_ms = 1;
        assert!(matches!(
            bad_expiry.validate(),
            Err(ImError::InvalidConfig { .. })
        ));

        assert!(validate_risk_level("nuclear").is_ok());
        assert!(validate_risk_level("unknown").is_err());
    }

    #[test]
    fn button_payload_round_trips_the_closure_identity() {
        let card = card();
        let payload = card.button_payload(IM_BUTTON_APPROVE).unwrap();
        assert_eq!(payload.approval_ref, "apr_1");
        assert_eq!(payload.pair_id, "pair-1");
        assert_eq!(payload.round, 2);
        assert_eq!(payload.subject, "tool.shell");
        assert_eq!(payload.expires_at_ms, card.expires_at_ms);
        assert_eq!(payload.action, IM_BUTTON_APPROVE);
        payload.validate().unwrap();

        let restored: ImButtonPayload =
            serde_json::from_str(&serde_json::to_string(&payload).unwrap()).unwrap();
        assert_eq!(restored, payload);

        assert!(card.button_payload("nuke").is_err());
        assert!(is_known_action(IM_BUTTON_CANCEL));
        assert!(!is_known_action("nuke"));
    }

    #[test]
    fn rendered_cards_carry_command_risk_and_two_buttons() {
        let card = card();
        for kind in ImChannelKind::ALL {
            let rendered = render_approval_card(kind, &card).unwrap();
            let text = rendered.to_string();
            assert!(text.contains("rm -rf /tmp/x"), "{kind}: {text}");
            assert!(text.contains("high"), "{kind}: {text}");
            assert!(text.contains("approve"), "{kind}: {text}");
            assert!(text.contains("reject"), "{kind}: {text}");
            assert!(text.contains("pair-1"), "{kind}: {text}");
        }
    }

    /// E-P2 审批卡族标注: 卡片载荷与各渠道渲染都携带授权族（授权对象 =
    /// 族; 批"删除族" = 该族所有形态生效, 人见族而批）; 无族操作明示
    /// "逐操作授权", 不留语义空洞; 旧载荷无字段照读（serde 兼容）。
    #[test]
    fn approval_card_carries_the_family_annotation() {
        let annotated = card();
        assert_eq!(card_family_annotation(&annotated), "删除族");
        for kind in ImChannelKind::ALL {
            let rendered = render_approval_card(kind, &annotated).unwrap().to_string();
            assert!(rendered.contains("删除族"), "{kind}: {rendered}");
            assert!(rendered.contains("授权族"), "{kind}: {rendered}");
        }

        let mut familyless = card();
        familyless.authorization_families = Vec::new();
        assert_eq!(card_family_annotation(&familyless), "（无族·逐操作授权）");
        let rendered = render_approval_card(ImChannelKind::ImWecom, &familyless)
            .unwrap()
            .to_string();
        assert!(rendered.contains("（无族·逐操作授权）"), "{rendered}");

        // 存量卡片载荷（无本字段）读入 = 空族标注, 不读失败。
        let mut object = serde_json::to_value(card()).unwrap();
        object
            .as_object_mut()
            .unwrap()
            .remove("authorization_families");
        let migrated: ImApprovalCard = serde_json::from_value(object).unwrap();
        assert!(migrated.authorization_families.is_empty());
    }
}
