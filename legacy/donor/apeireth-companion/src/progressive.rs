//! `apeireth-companion::progressive` — TP21 渐进式披露注入 (目录先行 → 按需展开).
//!
//! ## 借鉴 (60+ 项目调研, claude-mem 39k stars)
//!
//! 记忆目录 ~800 token 常驻 + 按需详情 ~120/条 (35k→920, -97%);
//! "注意力预算经济学, 全量注入仅 6% 相关" — 两级注入: 目录先行→按需展开.
//!
//! ## 机制 (确定性, 无 LLM)
//!
//! - **ProgressiveCatalog**: 记忆主题目录 (主题 + 一行摘要 + 条目数), 常驻注入.
//! - **expand(topic)**: 按需详情 (主题下的具体条目) — 查询/预载触发展开.
//! - **预算**: 目录块截断到 `catalog_budget_chars` (token ≈ chars/2 估算, 诚实标注
//!   是近似; 中文 1 char ≈ 1 token, 英文 4 chars ≈ 1 token — 按 2 chars/token 保守).
//! - **与 L0/L1 常驻共存**: 本模块只提供"目录块 + 展开"原语, 注入顺序/预算由
//!   ContextAssembler 决定, 不推翻现有预算机制.
//!
//! ## 挂接 (集成而非分立)
//!
//! - 目录来源: `topic_groups` (主题分组已存在) — 本模块消费其分组结果.
//! - 下游: ContextAssembler 注入 (目录块常驻) + `proactive_memory` 预载 (展开触发).

/// 目录条目 (主题级摘要).
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub topic: String,
    /// 一行摘要 (来自主题内代表性记忆).
    pub summary: String,
    /// 主题内条目数 (检索深度信号).
    pub count: usize,
}

impl CatalogEntry {
    pub fn new(topic: impl Into<String>, summary: impl Into<String>, count: usize) -> Self {
        Self {
            topic: topic.into(),
            summary: summary.into(),
            count,
        }
    }
}

/// 渐进式披露目录 (确定性).
#[derive(Debug)]
pub struct ProgressiveCatalog {
    entries: Vec<CatalogEntry>,
    /// 目录块预算 (字符; token ≈ chars/2 保守估算).
    pub catalog_budget_chars: usize,
}

impl ProgressiveCatalog {
    pub fn new(entries: Vec<CatalogEntry>) -> Self {
        Self {
            entries,
            catalog_budget_chars: 1600, // ~800 token
        }
    }

    /// 目录块: "主题 — 摘要 (N 条)" 列表, 按预算截断.
    /// 预算内放不下的条目被省略, 末尾标注省略数 (0 装 PASS: 不假装全量).
    pub fn block(&self) -> String {
        let mut lines = Vec::new();
        let mut used = 0usize;
        let mut omitted = 0usize;
        for e in &self.entries {
            let line = format!("- {}: {} ({}条)", e.topic, e.summary, e.count);
            let cost = line.chars().count();
            if used + cost > self.catalog_budget_chars && !lines.is_empty() {
                omitted += 1;
                continue;
            }
            lines.push(line);
            used += cost;
        }
        if omitted > 0 {
            lines.push(format!("…另有 {omitted} 个主题未展开 (目录预算内)"));
        }
        lines.join("\n")
    }

    /// 按需展开: 主题 → 详情 (此处为占位: 详情条目由调用方从记忆取,
    /// 本模块返回主题摘要 + 引导; 0 装 PASS: 不假装已拉取记忆).
    pub fn expand(&self, topic: &str) -> Option<String> {
        let e = self.entries.iter().find(|e| e.topic == topic)?;
        Some(format!(
            "## {}\n{}\n(共 {} 条, 详情条目由调用方按需从记忆检索 — 本模块不假装已拉取)",
            e.topic, e.summary, e.count
        ))
    }

    /// 预算内实际能容纳的主题数 (诊断).
    pub fn fit_count(&self) -> usize {
        let mut used = 0usize;
        let mut n = 0usize;
        for e in &self.entries {
            let cost = format!("- {}: {} ({}条)", e.topic, e.summary, e.count)
                .chars()
                .count();
            if used + cost > self.catalog_budget_chars && n > 0 {
                break;
            }
            used += cost;
            n += 1;
        }
        n
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<CatalogEntry> {
        vec![
            CatalogEntry::new("主人的工作", "投资套件开发进展", 42),
            CatalogEntry::new("熬夜规律", "深夜活跃 + 次日效率低", 7),
            CatalogEntry::new("绿萝", "前女友留下的盆栽, 喜阳", 3),
            CatalogEntry::new("代码审计", "双洋葱安全机制记录", 15),
        ]
    }

    #[test]
    fn block_generates_catalog_lines() {
        let cat = ProgressiveCatalog::new(sample_entries());
        let block = cat.block();
        assert!(block.contains("主人的工作"));
        assert!(block.contains("42条"), "应含条目数");
        assert!(block.contains("绿萝"), "所有条目都在预算内");
        assert!(!block.contains("…另有"), "4 条都在预算内无省略");
    }

    #[test]
    fn budget_truncates_and_notes_omission() {
        let cat = ProgressiveCatalog {
            entries: sample_entries(),
            catalog_budget_chars: 60, // 极小预算 → 只够 1 条 + 省略标注
        };
        let block = cat.block();
        assert!(block.contains("…另有"), "应诚实标注省略: {block}");
        assert!(cat.fit_count() < cat.len(), "预算内放不下全部");
    }

    #[test]
    fn expand_returns_topic_detail() {
        let cat = ProgressiveCatalog::new(sample_entries());
        let detail = cat.expand("熬夜规律").unwrap();
        assert!(detail.contains("熬夜规律"));
        assert!(detail.contains("7"), "应含条目数");
        assert!(detail.contains("不假装"), "0 装 PASS 标注在展开里");
        assert!(cat.expand("不存在的主题").is_none());
    }

    #[test]
    fn empty_catalog_block_is_empty() {
        let cat = ProgressiveCatalog::new(vec![]);
        assert_eq!(cat.block(), "");
        assert_eq!(cat.fit_count(), 0);
    }

    #[test]
    fn budget_approx_half_chars() {
        // token ≈ chars/2: 1600 chars 预算 ≈ 800 token (与调研目录预算一致)
        let cat = ProgressiveCatalog::new(sample_entries());
        let block = cat.block();
        let tokens_est = block.chars().count() / 2;
        assert!(tokens_est <= 800, "估算 token 应在预算内: {tokens_est}");
    }
}
