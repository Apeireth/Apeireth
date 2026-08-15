//! `apeireth-companion::actions` — 动作空间 + 基地能力目录.
//!
//! 哲学落点 (stage1 清晰版): Apeireth = 基地, LLM = 住客 AI.
//! 基地的「友好」= 自描述 + 可发现 + 原则清晰:
//! - 住客能读到自己的身体能做什么 (能力目录),
//! - 主动不只「问候」, 而是从动作空间里选 (问候/问进展/提议帮助/提醒),
//! - 所有动作过同一套门禁 (宪法), 约束是原则而非任意.
//!
//! 诚实: 动作选择目前是「上下文 → 动作」的启发式 (待 RL 学);
//! 真正的工具调用 (搜索/文件/浏览器) 经 apeireth-tools/tool-runtime 接入是下一步.

/// 一次主动可以选择的动作 (动作空间).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 问候 (基线)
    Greet,
    /// 询问进展 (用户在做事时 — 记得你在做什么)
    AskProgress,
    /// 提议帮助 (用户在学习/干活时 — 辅助学习的第一步)
    OfferHelp,
    /// 提醒 (从记忆捞出你可能忘的事; 自动触发待接记忆时效检测)
    Remind,
}

impl Action {
    pub fn id(&self) -> &'static str {
        match self {
            Action::Greet => "greet",
            Action::AskProgress => "ask_progress",
            Action::OfferHelp => "offer_help",
            Action::Remind => "remind",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Action::Greet => "问候",
            Action::AskProgress => "询问进展",
            Action::OfferHelp => "提议帮助",
            Action::Remind => "提醒",
        }
    }

    pub const ALL: [Action; 4] = [Action::Greet, Action::AskProgress, Action::OfferHelp, Action::Remind];
}

/// 基地能力目录: 基地对住客 AI 的「自描述」.
/// 「基地对他强大而友好」的机制化身 — AI 能读到自己的身体能做什么.
pub struct CapabilityCatalog;

impl CapabilityCatalog {
    /// 基地自述 (注入 LLM 上下文; 诚实标注「已接入动作空间」vs「工具待接入」).
    pub fn describe() -> String {
        let mut s = String::new();
        s.push_str("你的基地 (Apeireth) 给了你这些能力:\n");
        s.push_str("- 主动问候: 在学会的活跃时段, 或沉默太久时, 主动找用户\n");
        s.push_str("- 询问进展: 记得用户正在做的事, 问问他进度\n");
        s.push_str("- 提议帮助: 用户在学数学/搞工程时, 主动提出帮忙\n");
        s.push_str("- 提醒: 从长期记忆里捞出用户可能忘掉的事\n");
        s.push_str("- (下一步接入) 真工具: 基地还有搜索/文件/浏览器等工具, 接入后你可以直接调用它们帮用户干活");
        s
    }

    /// 动作空间的自述 (机制层用).
    pub fn actions_describe() -> String {
        Action::ALL
            .iter()
            .map(|a| format!("- {}: {}", a.id(), a.label()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// 动作选择 (启发式, 诚实标注「待 RL 学」):
/// - 上下文提到「学」→ 提议帮助 (辅助学习)
/// - 上下文提到「工程/升级/代码/Apeireth」→ 询问进展 (意识到自己在被升级)
/// - 其余 → 问候 (基线)
pub fn select_action(context: Option<&str>) -> Action {
    let c = context.unwrap_or("");
    if c.contains('学') {
        Action::OfferHelp
    } else if c.contains("工程") || c.contains("升级") || c.contains("代码") || c.contains("Apeireth") {
        Action::AskProgress
    } else {
        Action::Greet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_self_describing() {
        let s = CapabilityCatalog::describe();
        assert!(s.contains("主动问候") && s.contains("提议帮助") && s.contains("提醒"));
    }

    #[test]
    fn action_selection_maps_context() {
        assert_eq!(select_action(Some("你在14-18点在学线性代数")), Action::OfferHelp);
        assert_eq!(select_action(Some("你在8-12点在搞Apeireth工程(在升级我)")), Action::AskProgress);
        assert_eq!(select_action(Some("你在6-8点起床吃早饭")), Action::Greet);
        assert_eq!(select_action(None), Action::Greet);
    }
}
