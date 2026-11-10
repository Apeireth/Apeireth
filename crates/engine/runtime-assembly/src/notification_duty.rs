//! 问责闭环 · 第 4 步：通知义务（纠正发生时）。
//!
//! 上位：`docs/04-internal/backend-enhancement/accountability-loop-design-2026-10-23.md` §2.4。
//! 承诺措辞对应句：「……并在**发现影响时通知你**」。
//!
//! # 判据
//!
//! 顾问的硬话：
//! > 若系统知道某记忆被纠正、且知道旧结论曾驱动求职/购买/发信，却只改视图然后宣布
//! > 「已纠正」—— **那叫数据纠正，不叫责任闭环**。
//!
//! 闭环的最后一环：纠正一条事实时，**查影响索引 → 生成待办**。
//!
//! # 三条硬纪律
//!
//! 1. **不自动外发**（设计文档 §7）。通知义务 ≠ 替用户到处发更正。
//!    本模块只**生成待办**，由用户决定要不要发、发给谁。
//! 2. **不被引用就不打扰**（负例必测）：事实没影响过任何行动 ⇒ **不生成通知**。
//!    无差别地通知一切纠正是另一种噪声。
//! 3. **零真值**：待办里只带行动 id / 时间 / 目的地，**不带内容原文**。

use serde::{Deserialize, Serialize};

use crate::accountability::{AccountabilityLedger, ImpactStatement, Outcome};

/// 纠正的程度（决定补救建议的力度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionSeverity {
    /// 仅措辞调整（结论方向没变）。
    Wording,
    /// 部分修正（结论的一部分不再成立）。
    Partial,
    /// 事实被推翻（结论方向反了）。
    Refuted,
}

impl CorrectionSeverity {
    /// 稳定标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Wording => "wording",
            Self::Partial => "partial",
            Self::Refuted => "refuted",
        }
    }

    /// 是否需要用户授权才能采取补救动作。
    ///
    /// 措辞调整**不需要**（改个说法是我们该做的）；
    /// 部分修正与事实推翻**需要**（可能要联系别人、撤回、重新决策）。
    pub fn needs_authorization(&self) -> bool {
        !matches!(self, Self::Wording)
    }
}

/// 一条待办通知（**不自动外发**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactNotification {
    /// 被纠正的事实 id（**只存 id**）。
    pub fact_id: String,
    /// 纠正程度。
    pub severity: CorrectionSeverity,
    /// 受影响的事项（哪几次行动）。
    pub affected_actions: Vec<AffectedAction>,
    /// 建议补救动作（人类可读，**不含内容原文**）。
    pub suggested_remedies: Vec<String>,
    /// 是否需要用户授权才能执行补救。
    pub needs_authorization: bool,
}

/// 一条受影响事项（**零真值**：只指涉，不带内容）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedAction {
    /// 行动 id。
    pub action_id: String,
    /// 何时发生（epoch 毫秒）。
    pub at_ms: i64,
    /// 去了哪。
    pub destination: String,
    /// 能否撤回。
    pub reversible: bool,
    /// 后果状态（`unknown` 是合法且必要的一档）。
    pub outcome: Outcome,
}

impl From<&ImpactStatement> for AffectedAction {
    fn from(statement: &ImpactStatement) -> Self {
        Self {
            action_id: statement.action_id.clone(),
            at_ms: statement.at_ms,
            destination: statement.destination.clone(),
            reversible: statement.reversible,
            outcome: statement.outcome,
        }
    }
}

/// 通知义务：纠正发生时，查影响索引 → 生成待办。
///
/// 纯函数式：输入是「被纠正的事实 id + 纠正程度 + 账本」，输出是**要不要通知**。
/// 副作用（写文件、推面板）留给调用方 —— 这样「不自动外发」这条纪律
/// 由**类型**保证，而不是靠调用方自觉。
pub fn assess_correction(
    ledger: &AccountabilityLedger,
    fact_id: &str,
    severity: CorrectionSeverity,
) -> Option<ImpactNotification> {
    assess_correction_from(&ledger.actions_referencing(fact_id), fact_id, severity)
}

/// 与 [`assess_correction`] 同语义，但直接吃**已读出的语句切片**。
///
/// # 为什么需要这个入口
///
/// 调用方（`apeireth egress notify`）必须先把 **consequence 账**叠加到语句上
/// （后果可能已被回填为 `confirmed`/`failed`），再据此生成补救建议 ——
/// 否则建议会基于**过期状态**给出错误指引（例如已知 `failed` 还建议
/// 「先确认这些行动的结果」）。`assess_correction` 只接受 ledger、拿不到
/// 叠加后的视图，故拆出本函数作为真源，旧入口委托它（行为不变）。
pub fn assess_correction_from(
    statements: &[ImpactStatement],
    fact_id: &str,
    severity: CorrectionSeverity,
) -> Option<ImpactNotification> {
    if statements.is_empty() {
        // 纪律 ②：没被引用过 ⇒ 不打扰。
        return None;
    }
    let affected_actions: Vec<AffectedAction> =
        statements.iter().map(AffectedAction::from).collect();
    let remedies = suggest_remedies(&affected_actions, severity);
    Some(ImpactNotification {
        fact_id: fact_id.to_string(),
        severity,
        affected_actions,
        suggested_remedies: remedies,
        needs_authorization: severity.needs_authorization(),
    })
}

/// 按「能不能撤回 + 后果状态」给补救建议。
fn suggest_remedies(actions: &[AffectedAction], severity: CorrectionSeverity) -> Vec<String> {
    let mut remedies: Vec<String> = Vec::new();
    if actions.iter().any(|action| action.reversible) {
        remedies.push("撤回仍可撤回的行动（它们可逆）".to_string());
    }
    if actions.iter().any(|action| !action.reversible) {
        remedies.push("为已不可撤回的行动补一条更正说明".to_string());
    }
    if actions
        .iter()
        .any(|action| action.outcome == Outcome::Unknown)
    {
        remedies.push("先确认这些行动的结果（当前状态未知，系统不知道是否真的生效）".to_string());
    }
    if severity == CorrectionSeverity::Refuted {
        remedies.push("重新审视基于该事实做出的决定".to_string());
    }
    if remedies.is_empty() {
        remedies.push("仅记录，无需补救动作".to_string());
    }
    remedies
}

/// 决策回执：一次外部行动后，**面向被影响的人**的一行话。
///
/// 与日志的区别（必须写进实现注释 —— 混淆这两者是本设计最容易犯的错）：
///
/// | | 日志 | 回执 |
/// |---|---|---|
/// | 面向 | 审计 | **被影响的人** |
/// | 内容 | 全量结构 | **一句话** + 可展开依据 |
/// | 渠道 | 文件 | 会话 / 面板 / 通知 |
///
/// 「一句话」是**要求**：能一句话说清，就不要说三段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionReceipt {
    /// 行动 id。
    pub action_id: String,
    /// 一行话（面向人）。
    pub headline: String,
    /// 可展开的依据（**零真值**：只指涉）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub basis: Vec<String>,
    /// 还能不能撤回。
    pub reversible: bool,
    /// 后果状态（`unknown` 是合法且必要的一档）。
    pub outcome: Outcome,
}

impl DecisionReceipt {
    /// 从一条影响声明生成回执。
    ///
    /// **不做的事**：不解释「为什么这么做」（那是依据面板的事）、
    /// 不承诺结果（`outcome` 就是 `unknown`，不粉饰）。
    pub fn from_statement(statement: &ImpactStatement) -> Self {
        let mut headline = format!(
            "我把这条内容发给了 {}（用途：{}）。",
            statement.destination,
            describe_boundary(statement.boundary)
        );
        if !statement.reversible {
            headline.push_str("这一步发出去就收不回了。");
        }
        if statement.outcome == Outcome::Unknown {
            headline.push_str("结果我还不确定。");
        }
        Self {
            action_id: statement.action_id.clone(),
            headline,
            basis: statement
                .grants
                .iter()
                .map(|grant| format!("授权：{grant}"))
                .chain(
                    statement
                        .facts_used
                        .iter()
                        .map(|fact| format!("依据：{fact}")),
                )
                .collect(),
            reversible: statement.reversible,
            outcome: statement.outcome,
        }
    }
}

/// 边界的说法（面向人，不是枚举标签）。
fn describe_boundary(boundary: crate::accountability::Boundary) -> &'static str {
    use crate::accountability::Boundary;
    match boundary {
        Boundary::Provider => "模型服务商",
        Boundary::ThirdParty => "第三方系统",
        Boundary::UserToOrg => "你所在的组织",
        Boundary::InferenceToAction => "根据推测采取的行动",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accountability::{AccountabilityLedger, Affected, Boundary, ImpactStatement};
    use std::path::PathBuf;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-notify-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    fn statement(action_id: &str, facts: Vec<&str>, reversible: bool) -> ImpactStatement {
        ImpactStatement {
            action_id: action_id.to_string(),
            at_ms: 1_700_000_000_000,
            boundary: Boundary::Provider,
            facts_used: facts.into_iter().map(String::from).collect(),
            inferences: Vec::new(),
            grants: vec!["grant-1".into()],
            affected: Affected::SelfOnly,
            reversible,
            destination: "provider:anthropic".into(),
            outcome: Outcome::Unknown,
        }
    }

    #[test]
    fn correction_of_unreferenced_fact_produces_no_notification() {
        let path = temp_path("unreferenced");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], true));
        // ep-404 从没被引用过 ⇒ 不打扰。
        assert!(assess_correction(&ledger, "ep-404", CorrectionSeverity::Refuted).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn correction_of_referenced_fact_lists_affected_actions() {
        let path = temp_path("referenced");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], true));
        ledger.append(&statement("act-2", vec!["ep-1"], false));
        let notice =
            assess_correction(&ledger, "ep-1", CorrectionSeverity::Refuted).expect("应生成通知");
        assert_eq!(notice.affected_actions.len(), 2);
        assert_eq!(notice.affected_actions[0].action_id, "act-1");
        assert!(notice.affected_actions[1].reversible == false);
        assert!(notice.needs_authorization, "事实推翻需要授权才能补救");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wording_correction_does_not_need_authorization() {
        let path = temp_path("wording");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], true));
        let notice =
            assess_correction(&ledger, "ep-1", CorrectionSeverity::Wording).expect("应生成通知");
        assert!(!notice.needs_authorization, "措辞调整不需要用户授权");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_outcome_produces_confirm_first_remedy() {
        let path = temp_path("unknown");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], false));
        let notice =
            assess_correction(&ledger, "ep-1", CorrectionSeverity::Partial).expect("应生成通知");
        assert!(
            notice
                .suggested_remedies
                .iter()
                .any(|remedy| remedy.contains("确认")),
            "后果未知的行动应先确认: {:?}",
            notice.suggested_remedies
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn receipt_is_one_line_and_honest_about_unknown() {
        let path = temp_path("receipt");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], false));
        let statements = ledger.statements();
        let receipt = DecisionReceipt::from_statement(&statements[0]);
        // 一行话（不含换行）。
        assert!(
            !receipt.headline.contains('\n'),
            "回执应是一行: {}",
            receipt.headline
        );
        assert!(receipt.headline.contains("收不回"), "不可逆要说出来");
        assert!(receipt.headline.contains("还不确定"), "后果未知要说出来");
        assert_eq!(receipt.outcome, Outcome::Unknown);
        assert!(!receipt.reversible);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn receipt_basis_carries_ids_not_content() {
        let path = temp_path("basis");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], true));
        let statements = ledger.statements();
        let receipt = DecisionReceipt::from_statement(&statements[0]);
        assert!(receipt.basis.iter().any(|b| b.contains("ep-1")));
        let raw = format!("{receipt:?}");
        assert!(!raw.contains("@"), "回执不得带原文: {raw}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn notification_carries_no_raw_content() {
        let path = temp_path("zerotrue");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], false));
        let notice =
            assess_correction(&ledger, "ep-1", CorrectionSeverity::Refuted).expect("应生成通知");
        let raw = format!("{notice:?}");
        assert!(raw.contains("ep-1"));
        assert!(!raw.contains("@"));
        assert!(!raw.contains("alice"));
        let _ = std::fs::remove_file(&path);
    }
}
