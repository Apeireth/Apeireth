//! `apeireth-companion::daily_summary` — 每日摘要 (§6.4 后端数据源).
//!
//! 从真 SQLite 记忆汇总「他今天做了什么」:
//! - episodes 统计: 记忆条目 (mem-*) / 做梦整合 (mem-dream-*) / 反思记录 (reflect-*)
//! - 工具调用记录数 (action_stream, 由调用方经 `history_streams::query` 传入)
//! - 今日内容摘录 (最多 N 条)
//!
//! 0 假装: 这里是「统计 + 结构化」数据源; 展示 (UI/消息) 由上层决定.
//! 签名用 (id, content) 元组 — `apeireth_memory::Episode` 类型未公开导出, 不依赖其形状.

/// 单日摘要 (结构化).
#[derive(Debug, Clone, Default)]
pub struct DailySummary {
    pub date: String,
    pub episode_count: usize,
    pub memory_writes: usize,
    pub dreams: usize,
    pub reflections: usize,
    pub tool_records: usize,
    pub excerpts: Vec<String>,
}

impl DailySummary {
    /// 渲染成可读文本 (给「他今天干了什么」).
    pub fn render(&self) -> String {
        let mut s = format!("【今日摘要 · {}】\n", self.date);
        s.push_str(&format!(
            "记忆条目 {} · 做梦整合 {} · 反思记录 {} · 工具调用 {} · 总事件 {}\n",
            self.memory_writes, self.dreams, self.reflections, self.tool_records, self.episode_count
        ));
        if !self.excerpts.is_empty() {
            s.push_str("摘录:\n");
            for e in self.excerpts.iter().take(8) {
                s.push_str(&format!("  • {}\n", e));
            }
        }
        s
    }
}

/// 从 (id, content) 条目构建每日摘要 (纯函数, 可测).
pub fn build_daily_summary(date: &str, entries: &[(&str, &str)], tool_records: usize) -> DailySummary {
    let memory_writes = entries
        .iter()
        .filter(|(id, _)| id.starts_with("mem-") && !id.starts_with("mem-dream-"))
        .count();
    let dreams = entries.iter().filter(|(id, _)| id.starts_with("mem-dream-")).count();
    let reflections = entries.iter().filter(|(id, _)| id.starts_with("reflect-")).count();
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
            ("mem-1", "线代: 特征值卡住"),
            ("mem-dream-1", "【做梦整合】a ◆ b"),
            ("reflect-1", "【反思周期】第 1 轮完成"),
            ("e-other", "普通事件"),
        ];
        let s = build_daily_summary("2026-08-16", &entries, 5);
        assert_eq!(s.episode_count, 4);
        assert_eq!(s.memory_writes, 1);
        assert_eq!(s.dreams, 1);
        assert_eq!(s.reflections, 1);
        assert_eq!(s.tool_records, 5);
        assert!(s.render().contains("【今日摘要 · 2026-08-16】"));
        assert!(s.render().contains("工具调用 5"));
    }
}
