//! Context budget assembly and progressive-disclosure catalog.
//!
//! Recovered from donor `apeireth-companion::{context,progressive}`:
//! - [`ContextAssembler`]: ordered injection blocks + total char budget +
//!   core-block protection + greedy "cut the largest non-core first".
//! - [`ProgressiveCatalog`]: catalog-first injection (`topic — summary (N)`)
//!   with honest omission notes when the catalog budget overflows.
//!
//! Complementary to [`crate::context_rot`] (`compact_then_budget` is rot-then-
//! truncate-from-the-back) and to [`crate::context_fold`] (string collapse).
//! This module is the *selection policy* for named injection blocks.
//!
//! Library primitive only. DEFAULT OFF: not production-wired, owns no session.

/// Named injection block (content + core/cap policy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBlock {
    /// Block name (debug / budget report). Owned so callers need not leak
    /// `'static` strings.
    pub name: String,
    /// Block body (may contain newlines).
    pub content: String,
    /// Core blocks are never truncated by the total-budget pass.
    pub core: bool,
    /// Per-block character cap (`None` = unlimited, still subject to total).
    pub cap_chars: Option<usize>,
}

impl ContextBlock {
    /// Construct a non-core, uncapped block.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            core: false,
            cap_chars: None,
        }
    }

    /// Mark as core (never truncated by the total-budget pass).
    #[must_use]
    pub fn core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }

    /// Set a per-block character cap.
    #[must_use]
    pub fn with_cap(mut self, cap: usize) -> Self {
        self.cap_chars = Some(cap);
        self
    }
}

/// Ordered injection pipeline: per-block cap, then greedy total-budget cut.
pub struct ContextAssembler {
    blocks: Vec<ContextBlock>,
    total_budget_chars: usize,
}

impl ContextAssembler {
    /// Total budget in characters. Values below 100 are raised to 100 so a
    /// core persona block always has a usable floor (donor contract).
    pub fn new(total_budget_chars: usize) -> Self {
        Self {
            blocks: Vec::new(),
            total_budget_chars: total_budget_chars.max(100),
        }
    }

    /// Configured total budget (after the 100-char floor).
    pub fn total_budget_chars(&self) -> usize {
        self.total_budget_chars
    }

    /// Register a block (order preserved; core blocks are safer first).
    #[must_use]
    pub fn push(mut self, block: ContextBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// Diagnostic: `(name, char_count)` per registered block (pre-budget).
    pub fn budget_report(&self) -> Vec<(String, usize)> {
        self.blocks
            .iter()
            .map(|b| (b.name.clone(), b.content.chars().count()))
            .collect()
    }

    /// Budgeted assembly: contents only, empty/whitespace blocks dropped.
    pub fn assemble_budgeted(&self) -> Vec<String> {
        self.assemble_budgeted_blocks()
            .into_iter()
            .map(|b| b.content)
            .collect()
    }

    /// Budgeted assembly keeping names / core / cap metadata.
    ///
    /// Algorithm:
    /// 1. Apply per-block `cap_chars`.
    /// 2. If the total still exceeds the budget, sort non-core blocks by
    ///    descending length and cut from the largest first (greedy).
    /// 3. Drop blocks whose remaining content is empty/whitespace.
    pub fn assemble_budgeted_blocks(&self) -> Vec<ContextBlock> {
        let mut capped: Vec<String> = self
            .blocks
            .iter()
            .map(|b| {
                b.content
                    .chars()
                    .take(b.cap_chars.unwrap_or(usize::MAX))
                    .collect()
            })
            .collect();
        let mut total: usize = capped.iter().map(|s| s.chars().count()).sum();
        if total > self.total_budget_chars {
            let mut order: Vec<usize> = (0..self.blocks.len())
                .filter(|&i| !self.blocks[i].core)
                .collect();
            order.sort_by_key(|&i| std::cmp::Reverse(capped[i].chars().count()));
            for i in order {
                if total <= self.total_budget_chars {
                    break;
                }
                let len = capped[i].chars().count();
                if len == 0 {
                    continue;
                }
                let over = total - self.total_budget_chars;
                let cut = len.min(over);
                capped[i] = capped[i].chars().take(len - cut).collect();
                total -= cut;
            }
        }
        self.blocks
            .iter()
            .zip(capped)
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(b, s)| ContextBlock {
                name: b.name.clone(),
                content: s,
                core: b.core,
                cap_chars: b.cap_chars,
            })
            .collect()
    }
}

/// Catalog entry (topic-level summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// Topic name.
    pub topic: String,
    /// One-line summary (typically a representative memory).
    pub summary: String,
    /// Number of items under this topic (retrieval-depth signal).
    pub count: usize,
}

impl CatalogEntry {
    /// Construct a catalog entry.
    pub fn new(topic: impl Into<String>, summary: impl Into<String>, count: usize) -> Self {
        Self {
            topic: topic.into(),
            summary: summary.into(),
            count,
        }
    }
}

/// Progressive-disclosure catalog: directory first, expand on demand.
///
/// This module does **not** fetch memory items. [`ProgressiveCatalog::expand`]
/// returns the topic summary plus an honest note that details come from the
/// caller.
#[derive(Debug, Clone)]
pub struct ProgressiveCatalog {
    entries: Vec<CatalogEntry>,
    /// Catalog-block budget in characters (`token ≈ chars/2` conservative).
    pub catalog_budget_chars: usize,
}

impl ProgressiveCatalog {
    /// Catalog with the donor default budget (~800 tokens ≈ 1600 chars).
    pub fn new(entries: Vec<CatalogEntry>) -> Self {
        Self {
            entries,
            catalog_budget_chars: 1600,
        }
    }

    /// Override the catalog character budget.
    #[must_use]
    pub fn with_budget(mut self, chars: usize) -> Self {
        self.catalog_budget_chars = chars;
        self
    }

    /// Catalog block: `"- topic: summary (N条)"` lines, truncated to budget.
    /// Overflow is noted (`…另有 N 个主题未展开`) — never silently dropped.
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

    /// On-demand expand: topic → summary + count. Does not pretend to have
    /// pulled the underlying memory items.
    pub fn expand(&self, topic: &str) -> Option<String> {
        let e = self.entries.iter().find(|e| e.topic == topic)?;
        Some(format!(
            "## {}\n{}\n(共 {} 条, 详情条目由调用方按需从记忆检索 — 本模块不假装已拉取)",
            e.topic, e.summary, e.count
        ))
    }

    /// How many topics actually fit in the catalog budget (diagnostic).
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

    /// Number of registered topics.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_blocks_never_truncated() {
        // 总 430 字, 预算 300: 核心 150 保留, 非核心按大头先砍 (mem 200 → 留 70)
        // "核心人格内容" = 6 chars × 30 = 180; "记忆内容" = 4 × 50 = 200;
        // "偏好内容" = 4 × 20 = 80; total 460. Budget 300.
        // Over = 160. Largest non-core is mem (200) → cut 160 → mem 40.
        let a = ContextAssembler::new(300)
            .push(ContextBlock::new("persona", "核心人格内容".repeat(30)).core(true))
            .push(ContextBlock::new("mem", "记忆内容".repeat(50)))
            .push(ContextBlock::new("prefs", "偏好内容".repeat(20)));
        let out = a.assemble_budgeted();
        let total: usize = out.iter().map(|s| s.chars().count()).sum();
        assert!(total <= 300, "总预算应约束, got {total}");
        assert!(out[0].contains("核心人格内容"), "核心块应完整保留");
        assert!(
            out[1].chars().count() < "记忆内容".repeat(50).chars().count(),
            "非核心块应被截断"
        );
        assert_eq!(out[1].chars().count(), 40, "mem 200 → 砍 160 → 留 40");
    }

    #[test]
    fn per_block_cap() {
        let a = ContextAssembler::new(100_000)
            .push(ContextBlock::new("x", "abc".repeat(10)).with_cap(12));
        let out = a.assemble_budgeted();
        assert_eq!(out[0], "abcabcabcabc");
    }

    #[test]
    fn empty_blocks_filtered() {
        let a = ContextAssembler::new(1000)
            .push(ContextBlock::new("a", "hello"))
            .push(ContextBlock::new("b", "   "));
        let out = a.assemble_budgeted();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "hello");
    }

    #[test]
    fn budget_floor_is_one_hundred() {
        let a = ContextAssembler::new(10);
        assert_eq!(a.total_budget_chars(), 100);
    }

    #[test]
    fn greedy_cut_skips_core_even_if_largest() {
        let a = ContextAssembler::new(120)
            .push(ContextBlock::new("core", "C".repeat(80)).core(true))
            .push(ContextBlock::new("small", "s".repeat(30)))
            .push(ContextBlock::new("big", "b".repeat(50)));
        let blocks = a.assemble_budgeted_blocks();
        let core = blocks.iter().find(|b| b.name == "core").unwrap();
        assert_eq!(core.content.chars().count(), 80);
        let total: usize = blocks.iter().map(|b| b.content.chars().count()).sum();
        assert!(total <= 120);
    }

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
        assert!(block.contains("42条"));
        assert!(block.contains("绿萝"));
        assert!(!block.contains("…另有"));
    }

    #[test]
    fn budget_truncates_and_notes_omission() {
        let cat = ProgressiveCatalog {
            entries: sample_entries(),
            catalog_budget_chars: 60,
        };
        let block = cat.block();
        assert!(block.contains("…另有"), "应诚实标注省略: {block}");
        assert!(cat.fit_count() < cat.len());
    }

    #[test]
    fn expand_returns_topic_detail() {
        let cat = ProgressiveCatalog::new(sample_entries());
        let detail = cat.expand("熬夜规律").unwrap();
        assert!(detail.contains("熬夜规律"));
        assert!(detail.contains('7'));
        assert!(detail.contains("不假装"));
        assert!(cat.expand("不存在的主题").is_none());
    }

    #[test]
    fn empty_catalog_block_is_empty() {
        let cat = ProgressiveCatalog::new(vec![]);
        assert_eq!(cat.block(), "");
        assert_eq!(cat.fit_count(), 0);
        assert!(cat.is_empty());
    }

    #[test]
    fn budget_approx_half_chars() {
        let cat = ProgressiveCatalog::new(sample_entries());
        let block = cat.block();
        let tokens_est = block.chars().count() / 2;
        assert!(tokens_est <= 800, "估算 token 应在预算内: {tokens_est}");
    }
}
