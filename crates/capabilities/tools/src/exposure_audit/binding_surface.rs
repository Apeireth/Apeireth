//! 本地数据面自查 · 项五: 客户端令牌表绑定完整性 (W3-② 绑定一致性)。
//!
//! 防御语自查: 问责链 (令牌 → 用户 → 角色) 的两端绑定必须对得上 —— 令牌条目
//! 声明的 `user_id` 在用户表内、且该用户回绑这个客户端名。判定真源是 W3-②
//! 装载面的绑定一致性校验 (拒开语义), 本项只把它的**判定结果**呈现成体检发现;
//! 令牌值零采集 (只报计数与绑定名, 名可呈现、值不可)。

use crate::exposure_audit::report::{ExposureItem, Finding, RiskLevel};

/// 绑定面的核对结果 (由 W3-② 装载面判定后注入, 本项不自造真源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSurface {
    /// 无可核对面 (用户表/令牌表未配置): 显式未测, 不冒充干净。
    Absent {
        /// 未测原因 (如实)。
        reason: String,
    },
    /// 已核对: 客户端枚数 / 用户名数 / 不一致明细 (空 = 一致)。
    Checked {
        /// 客户端令牌表枚数。
        clients: usize,
        /// 用户表用户名数。
        users: usize,
        /// 不一致明细 (绑定名可呈现; 令牌值零采集)。
        problems: Vec<String>,
    },
}

/// 项五: 客户端令牌表绑定完整性体检项。
pub fn token_binding_item(surface: &BindingSurface) -> ExposureItem {
    match surface {
        BindingSurface::Absent { reason } => ExposureItem::not_measured(
            "token_bindings",
            "客户端令牌表绑定完整性",
            format!("未测原因: {reason}"),
            "配置用户表与令牌表后复检; 发令牌序: 先建用户再写绑定, 删用户前先摘绑定",
        ),
        BindingSurface::Checked {
            clients,
            users,
            problems,
        } => {
            let note = format!(
                "W3-② 绑定一致性口径: 客户端 {clients} 枚 / 用户 {users} 名 (只报计数与绑定名, 令牌值零采集)"
            );
            if problems.is_empty() {
                ExposureItem::measured(
                    "token_bindings",
                    "客户端令牌表绑定完整性",
                    Vec::new(),
                    "保持发令牌序: 先建用户再写绑定, 删用户前先摘绑定; 绑定一致性随周期巡检复查",
                )
                .with_note(note)
            } else {
                let findings = problems
                    .iter()
                    .map(|problem| {
                        Finding::new(
                            "client_tokens↔users",
                            RiskLevel::High,
                            format!("问责链断: {problem}"),
                        )
                    })
                    .collect();
                ExposureItem::measured(
                    "token_bindings",
                    "客户端令牌表绑定完整性",
                    findings,
                    "问责链修复: 按发令牌序更正绑定 (先建用户再写绑定/删用户前先摘绑定), 修复前管理动作按拒开语义处置",
                )
                .with_note(note)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_surface_is_unmeasured_not_clean() {
        let item = token_binding_item(&BindingSurface::Absent {
            reason: "用户表未配置".to_string(),
        });
        assert_eq!(
            item.status,
            crate::exposure_audit::report::ItemStatus::NotMeasured
        );
        assert_eq!(item.score(), None);
    }

    #[test]
    fn consistent_surface_is_clean_and_scores_100() {
        let item = token_binding_item(&BindingSurface::Checked {
            clients: 2,
            users: 2,
            problems: Vec::new(),
        });
        assert_eq!(item.finding_count(), 0);
        assert_eq!(item.score(), Some(100));
        assert!(item.advice.contains("先建用户再写绑定"));
    }

    #[test]
    fn inconsistent_surface_is_high_risk_with_each_problem_a_finding() {
        let item = token_binding_item(&BindingSurface::Checked {
            clients: 1,
            users: 0,
            problems: vec![
                "client \"tui\" declares user id \"ghost\"".to_string(),
                "client \"web\" is not bound back".to_string(),
            ],
        });
        assert_eq!(item.finding_count(), 2);
        assert_eq!(item.risk(), Some(RiskLevel::High));
        assert_eq!(item.score(), Some(50));
    }
}
