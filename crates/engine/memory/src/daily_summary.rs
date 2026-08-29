//! `apeireth-memory::daily_summary` — 每日记忆摘要与结构化统计.
//!
//! **设计哲学 (日粒度宏观活动归纳)**:
//! - 汇总单日内的记忆条目 (`mem-*`)、做梦整合 (`mem-dream-*`)、反思周期 (`reflect-*`) 与工具调用记录；
//! - 提取核心正文前缀片段 (`excerpts`)，以结构化 Markdown 形式渲染；
//! - 纯确定性算法，0 LLM 依赖。

use serde::{Deserialize, Serialize};

/// 单日摘要 (结构化).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailySummary {
    /// 归档日期 (YYYY-MM-DD)
    pub date: String,
    /// 事件与条目总数
    pub episode_count: usize,
    /// 显式记忆写入数
    pub memory_writes: usize,
    /// 做梦整合生成数
    pub dreams: usize,
    /// 反思记录数
    pub reflections: usize,
    /// 工具执行记录数
    pub tool_records: usize,
    /// 正文摘录列表 (截断有界)
    pub excerpts: Vec<String>,
}

impl DailySummary {
    /// 渲染为人类与模型可读的结构化文本.
    pub fn render(&self) -> String {
        let mut s = format!("【今日摘要 · {}】\n", self.date);
        s.push_str(&format!(
            "记忆条目 {} · 做梦整合 {} · 反思记录 {} · 工具调用 {} · 总事件 {}\n",
            self.memory_writes,
            self.dreams,
            self.reflections,
            self.tool_records,
            self.episode_count
        ));
        if !self.excerpts.is_empty() {
            s.push_str("今日活动摘录:\n");
            for e in self.excerpts.iter().take(8) {
                s.push_str(&format!("  • {}\n", e));
            }
        }
        s
    }
}

/// 从 `(id, content)` 元组序列确定性构建单日摘要.
pub fn build_daily_summary(
    date: &str,
    entries: &[(&str, &str)],
    tool_records: usize,
) -> DailySummary {
    let memory_writes = entries
        .iter()
        .filter(|(id, _)| id.starts_with("mem-") && !id.starts_with("mem-dream-"))
        .count();
    let dreams = entries
        .iter()
        .filter(|(id, _)| id.starts_with("mem-dream-"))
        .count();
    let reflections = entries
        .iter()
        .filter(|(id, _)| id.starts_with("reflect-"))
        .count();
    let excerpts: Vec<String> = entries
        .iter()
        .map(|(_, c)| c.chars().take(80).collect())
        .collect();

    DailySummary {
        date: date.to_string(),
        episode_count: entries.len(),
        memory_writes,
        dreams,
        reflections,
        tool_records,
        excerpts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_by_kind() {
        let entries: Vec<(&str, &str)> = vec![
            ("mem-1", "学习 Rust 所有权与生命周期"),
            ("mem-dream-1", "【做梦整合】编译安全与确定性"),
            ("reflect-1", "【反思周期】第 1 轮完成"),
            ("e-other", "普通日常事件"),
        ];
        let summary = build_daily_summary("2026-08-29", &entries, 6);
        assert_eq!(summary.episode_count, 4);
        assert_eq!(summary.memory_writes, 1);
        assert_eq!(summary.dreams, 1);
        assert_eq!(summary.reflections, 1);
        assert_eq!(summary.tool_records, 6);

        let rendered = summary.render();
        assert!(rendered.contains("【今日摘要 · 2026-08-29】"));
        assert!(rendered.contains("工具调用 6"));
        assert!(rendered.contains("学习 Rust 所有权与生命周期"));
    }
}
