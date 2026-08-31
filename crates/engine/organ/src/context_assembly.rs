//! Budgeted context-block assembler (relational prompt injection).
//!
//! Canonical implementation module. (the block
//! pipeline only — rot scoring already lives in
//! `apeireth-orchestration::context_rot` and is a different algorithm).
//!
//! - Ordered named blocks with a total character budget.
//! - Core blocks (persona / identity / essential story) are never truncated.
//! - Non-core overflow is cut greedily from the longest remaining block.
//! - Per-block `cap_chars` applies before the total budget.
//!
//! Default-off library primitive. Not a second session, transcript, or
//! prompt-cache owner (`PromptCacheStabilizer` remains the cache-prefix owner).

/// Named injection block.
#[derive(Debug, Clone)]
pub struct ContextBlock {
    /// Block name (debug / budget report).
    pub name: &'static str,
    /// Block body (may contain newlines).
    pub content: String,
    /// Core block: never truncated by the total budget.
    pub core: bool,
    /// Per-block cap (`None` = unlimited, still subject to total budget).
    pub cap_chars: Option<usize>,
}

impl ContextBlock {
    pub fn new(name: &'static str, content: impl Into<String>) -> Self {
        Self {
            name,
            content: content.into(),
            core: false,
            cap_chars: None,
        }
    }

    pub fn core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }

    pub fn with_cap(mut self, cap: usize) -> Self {
        self.cap_chars = Some(cap);
        self
    }
}

/// Ordered block pipeline: total budget + core protection + greedy truncate.
pub struct ContextAssembler {
    blocks: Vec<ContextBlock>,
    total_budget_chars: usize,
}

impl ContextAssembler {
    /// Total budget in Unicode scalar values (minimum 100).
    pub fn new(total_budget_chars: usize) -> Self {
        Self {
            blocks: Vec::new(),
            total_budget_chars: total_budget_chars.max(100),
        }
    }

    pub fn total_budget_chars(&self) -> usize {
        self.total_budget_chars
    }

    /// Register a block (order preserved; core blocks should be registered first).
    pub fn push(mut self, block: ContextBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// Diagnostic: `(name, char_count)` per registered block (pre-budget).
    pub fn budget_report(&self) -> Vec<(String, usize)> {
        self.blocks
            .iter()
            .map(|b| (b.name.to_string(), b.content.chars().count()))
            .collect()
    }

    /// Budgeted assembly: truncated contents, empty / whitespace-only dropped.
    pub fn assemble_budgeted(&self) -> Vec<String> {
        self.assemble_budgeted_blocks()
            .into_iter()
            .map(|b| b.content)
            .collect()
    }

    /// Budgeted assembly keeping name / core / cap metadata.
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
                name: b.name,
                content: s,
                core: b.core,
                cap_chars: b.cap_chars,
            })
            .collect()
    }
}

/// Hydra-style tiered concatenation: smaller tier numbers sort first.
/// Complementary to `PromptCacheStabilizer` (cache prefix) — this only orders
/// system-prompt *parts*, it does not own the message list.
pub fn assemble_tiered(parts: &[(u8, &str)]) -> String {
    let mut sorted = parts.to_vec();
    sorted.sort_by_key(|(tier, _)| *tier);
    let mut s = String::new();
    for (_, content) in sorted {
        s.push_str(content);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_blocks_never_truncated() {
        let a = ContextAssembler::new(300)
            .push(ContextBlock::new("persona", "核心人格内容".repeat(30)).core(true))
            .push(ContextBlock::new("mem", "记忆内容".repeat(50)))
            .push(ContextBlock::new("prefs", "偏好内容".repeat(20)));
        let out = a.assemble_budgeted();
        let total: usize = out.iter().map(|s| s.chars().count()).sum();
        assert!(total <= 300, "总预算应约束 (核心保护下 total=300)");
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
        assert_eq!(out[0], "abcabcabcabc".to_string());
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
    fn budget_floor_is_100() {
        let a = ContextAssembler::new(10);
        assert_eq!(a.total_budget_chars(), 100);
    }

    #[test]
    fn budget_report_is_pre_truncate() {
        let a = ContextAssembler::new(10)
            .push(ContextBlock::new("a", "abcdef"))
            .push(ContextBlock::new("b", "xyz"));
        assert_eq!(a.budget_report(), vec![("a".into(), 6), ("b".into(), 3)]);
    }

    #[test]
    fn assemble_tiered_orders_by_tier() {
        let s = assemble_tiered(&[
            (100, "工具指引\n"),
            (0, "身份: 阿佩瑞斯\n"),
            (50, "记忆证据\n"),
        ]);
        let i0 = s.find("身份").unwrap();
        let i1 = s.find("记忆").unwrap();
        let i2 = s.find("工具").unwrap();
        assert!(i0 < i1 && i1 < i2, "tier 0 身份应最前: {s}");
    }

    #[test]
    fn core_blocks_can_exceed_budget_if_they_alone_overflow() {
        let a = ContextAssembler::new(100)
            .push(ContextBlock::new("persona", "核".repeat(150)).core(true))
            .push(ContextBlock::new("mem", "记".repeat(50)));
        let blocks = a.assemble_budgeted_blocks();
        assert_eq!(blocks[0].name, "persona");
        assert_eq!(blocks[0].content.chars().count(), 150);
        assert!(
            !blocks.iter().any(|b| b.name == "mem"),
            "非核心块应被砍空并过滤"
        );
    }
}
