//! `apeireth-companion::memory_injection` — 反幻觉记忆注入 (吸收 hydra EMI/NEC, 重写).
//!
//! 问题: LLM 检索到记忆后容易幻觉「我记得我们以前聊过…」。
//! 方案: 记忆注入模板 = **闭世界证据**: 编号列表 + 来源标注 + 反幻觉指令
//! (禁止声称记得列表之外的事), 消除「我记得」幻觉。
//!
//! 对齐 hydra: `You do NOT know this user personally... NEVER say "based on our
//! previous conversations" — that is fabrication`.

/// 反幻觉记忆注入: 把检索到的记忆条目渲染成「闭世界证据」块.
pub fn build_memory_injection(entries: &[String]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut s = String::from("[记忆证据 — 你只知道以下条目, 不要声称记得列表之外的任何对话]\n");
    for (i, e) in entries.iter().enumerate() {
        s.push_str(&format!(
            "{}. {}\n",
            i + 1,
            e.chars().take(120).collect::<String>()
        ));
    }
    s.push_str(
        "规则: 说话只能基于以上编号条目; 不确定就说「我猜」; \
         禁止说「我记得我们以前聊过」— 那是编造。",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_entries_no_injection() {
        assert_eq!(build_memory_injection(&[]), "");
    }

    #[test]
    fn entries_numbered_with_closure_rules() {
        let s = build_memory_injection(&[
            "主人明天要交线代作业".to_string(),
            "主人换元法常忘换 dx".to_string(),
        ]);
        assert!(s.contains("[记忆证据"));
        assert!(s.contains("1. 主人明天要交线代作业"));
        assert!(s.contains("2. 主人换元法常忘换 dx"));
        assert!(
            s.contains("禁止说「我记得我们以前聊过」"),
            "反幻觉指令必须存在: {s}"
        );
        assert!(s.contains("我猜"), "不确定就说我猜");
    }

    #[test]
    fn long_entries_truncated() {
        let long = "x".repeat(300);
        let s = build_memory_injection(&[long]);
        assert!(
            s.matches('x').count() <= 120,
            "条目应截断到 120 字: {}",
            s.matches('x').count()
        );
        assert!(s.contains("禁止说"), "反幻觉指令仍在");
    }
}
