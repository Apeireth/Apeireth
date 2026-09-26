//! Context budget assembly and progressive-disclosure catalog.
//!
//! Two concerns:
//! - [`ContextAssembler`]: ordered injection blocks + total char budget +
//!   core-block protection + greedy "cut the largest non-core first".
//! - [`ProgressiveCatalog`]: catalog-first injection (`topic — summary (N)`)
//!   with honest omission notes when the catalog budget overflows.
//!
//! Complementary to [`crate::context_rot`] (`compact_then_budget` is rot-then-
//! truncate-from-the-back) and to [`crate::context_fold`] (string collapse).
//! This module is the *selection policy* for named injection blocks.
//!
//! # Long-tail cuts never evaporate content
//!
//! A budget cut used to drop the tail outright, and the dropped characters were
//! unrecoverable. The cut point now keeps a head/tail preview with a marker
//! line and a one-line retrieval guide, while the full original is written to
//! the data directory's `spill/` area ([`truncate_with_spill`]). When the write
//! cannot happen the full original stays inline: prefer a too-long context over
//! lost information, and never turn a successful call into an error.
//!
//! Library primitive only. DEFAULT OFF: not production-wired, owns no session.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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

/// Sub-directory, under the caller's data directory, that receives the full
/// originals of truncated long tails.
pub const SPILL_SUBDIR: &str = "spill";

/// How many times a spill write retries with a fresh name before giving up.
const MAX_SPILL_WRITE_ATTEMPTS: usize = 16;

/// Head and tail previews kept around one long-tail cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSplit {
    /// First `keep_chars / 2` characters of the content.
    pub head: String,
    /// How many characters the omitted middle lost.
    pub omitted_chars: usize,
    /// Last `keep_chars - keep_chars / 2` characters of the content.
    pub tail: String,
}

/// Split `content` into head and tail previews totalling `keep_chars`
/// characters (halved; the tail keeps the odd character).
///
/// Counting is per Unicode scalar value, so every cut point lands on a UTF-8
/// boundary: a surrogate-pair (4-byte) character is never split between a
/// preview and the omitted middle. `keep_chars >= content length` is a no-op
/// (whole content as head, nothing omitted).
pub fn split_head_tail(content: &str, keep_chars: usize) -> PreviewSplit {
    let total = content.chars().count();
    if keep_chars >= total {
        return PreviewSplit {
            head: content.to_string(),
            omitted_chars: 0,
            tail: String::new(),
        };
    }
    let head_chars = keep_chars / 2;
    let tail_chars = keep_chars - head_chars;
    PreviewSplit {
        head: content.chars().take(head_chars).collect(),
        omitted_chars: total - keep_chars,
        tail: content.chars().skip(total - tail_chars).collect(),
    }
}

/// The marker line that stands in for the omitted middle of a truncated long
/// tail.
pub fn omission_marker(omitted_chars: usize) -> String {
    format!("…[中间 {omitted_chars} 字符已省略]…")
}

/// The one-line retrieval guide attached to truncated content.
///
/// The line doubles as the UI's signal that a full original exists on disk; the
/// text always embeds the path, and structured callers can read
/// [`SpilledTruncation::spilled_path`] instead of parsing the line.
pub fn retrieval_guide(path: &Path) -> String {
    format!(
        "完整内容已存于 {}，可用文件读取按偏移取回或检索关键词定位",
        path.display()
    )
}

/// The outcome of a spill-backed truncation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpilledTruncation {
    /// What goes into the context: the head/tail preview with marker and
    /// retrieval guide, or — when spilling was impossible — the full inline
    /// original.
    pub text: String,
    /// Where the full original now lives. `None` means nothing was spilled:
    /// either no truncation happened, or the write failed and `text` holds the
    /// full original inline.
    pub spilled_path: Option<PathBuf>,
    /// The write failure that forced the inline fallback, when one happened.
    pub spill_error: Option<String>,
}

/// Writes the full originals of truncated long tails under
/// `<data_dir>/spill/`.
///
/// File names carry the scope (session), a monotonic sequence number and a
/// short content hash (`spill-<scope>-<seq>-<hash>.txt`), so concurrent
/// sessions and repeated spills of identical content cannot collide. Creation
/// is exclusive (`create_new`); a rare name collision simply advances the
/// sequence and retries.
#[derive(Debug, Clone)]
pub struct SpillWriter {
    spill_dir: PathBuf,
    sequence: Arc<AtomicU64>,
}

impl SpillWriter {
    /// Spill files land in `data_dir/spill/` ([`SPILL_SUBDIR`]).
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            spill_dir: data_dir.into().join(SPILL_SUBDIR),
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// The directory spill files are written to.
    pub fn spill_dir(&self) -> &Path {
        &self.spill_dir
    }

    /// Write `content` in full and return the path it now lives at.
    ///
    /// Failures come back as legible strings so the caller can report them;
    /// the truncation path never turns one into a failed call (it falls back
    /// to the inline original instead).
    pub fn spill(&self, scope: &str, content: &str) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.spill_dir)
            .map_err(|error| format!("spill directory create failed: {error}"))?;
        for _ in 0..MAX_SPILL_WRITE_ATTEMPTS {
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
            let path = self
                .spill_dir
                .join(spill_file_name(scope, sequence, content));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    if let Err(error) = std::io::Write::write_all(&mut file, content.as_bytes()) {
                        drop(file);
                        let _ = std::fs::remove_file(&path);
                        return Err(format!("spill content write failed: {error}"));
                    }
                    return Ok(path);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(format!("spill file create failed: {error}")),
            }
        }
        Err("spill file name could not be made unique within the attempt limit".to_string())
    }
}

/// Collision-resistant spill file name: `spill-<scope>-<seq>-<hash>.txt`.
///
/// The scope is reduced to a safe path segment, the sequence number comes from
/// the writer (monotonic per writer instance), and the short hash comes from
/// the content — so the same session spilling the same content twice still
/// gets two distinct files.
pub fn spill_file_name(scope: &str, sequence: u64, content: &str) -> String {
    format!(
        "spill-{}-{sequence:06}-{:08x}.txt",
        sanitize_scope(scope),
        short_hash(content.as_bytes())
    )
}

/// Reduce `scope` to a safe path segment (ASCII letters, digits, `-`, `_`).
fn sanitize_scope(scope: &str) -> String {
    let cleaned: String = scope
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    let cleaned = cleaned.trim_matches('_');
    if cleaned.is_empty() {
        "session".to_string()
    } else {
        cleaned.to_string()
    }
}

/// Short non-cryptographic content hash (multiply-xor fold) used only to make
/// spill file names collision-resistant — not a security primitive.
fn short_hash(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Truncate `content` to a `keep_chars` head/tail preview with an omission
/// marker and a retrieval guide, after spilling the full original.
///
/// Contract (the fallback is part of the contract, not an afterthought):
/// - `content` at or below `keep_chars` is returned byte-for-byte unchanged
///   and nothing is spilled (zero-change below the threshold).
/// - The full original is spilled first; only then is the preview built. The
///   preview holds `keep_chars` content characters (halved head/tail) plus two
///   bounded overhead lines (marker + guide).
/// - If the spill cannot be written, the full original is kept inline: a
///   truncation must never lose content, and overflow handling must never turn
///   a successful call into an error.
pub fn truncate_with_spill(
    content: &str,
    keep_chars: usize,
    spill: &SpillWriter,
    scope: &str,
) -> SpilledTruncation {
    if content.chars().count() <= keep_chars {
        return SpilledTruncation {
            text: content.to_string(),
            spilled_path: None,
            spill_error: None,
        };
    }
    match spill.spill(scope, content) {
        Ok(path) => {
            let split = split_head_tail(content, keep_chars);
            let text = format!(
                "{}\n{}\n{}\n{}",
                split.head,
                omission_marker(split.omitted_chars),
                split.tail,
                retrieval_guide(&path),
            );
            SpilledTruncation {
                text,
                spilled_path: Some(path),
                spill_error: None,
            }
        }
        Err(error) => SpilledTruncation {
            text: content.to_string(),
            spilled_path: None,
            spill_error: Some(error),
        },
    }
}

/// Where the assembler's long-tail cuts spill their full originals, and under
/// which naming scope (the session).
#[derive(Debug, Clone)]
struct SpillBinding {
    writer: SpillWriter,
    scope: String,
}

/// Ordered injection pipeline: per-block cap, then greedy total-budget cut.
///
/// Cuts are prefix cuts by default. Binding a spill sink with
/// [`ContextAssembler::with_spill`] upgrades every cut to a head/tail preview
/// with a retrieval guide ([`truncate_with_spill`]); a failed spill keeps the
/// full original inline.
pub struct ContextAssembler {
    blocks: Vec<ContextBlock>,
    total_budget_chars: usize,
    spill: Option<SpillBinding>,
}

impl ContextAssembler {
    /// Total budget in characters. Values below 100 are raised to 100 so a
    /// core persona block always has a usable floor (the floor contract).
    pub fn new(total_budget_chars: usize) -> Self {
        Self {
            blocks: Vec::new(),
            total_budget_chars: total_budget_chars.max(100),
            spill: None,
        }
    }

    /// Bind a spill sink: long-tail cuts keep a head/tail preview and spill the
    /// full original under the writer's `spill/` directory, named by `scope`
    /// (the session). Without this binding, cuts keep the plain prefix shape.
    #[must_use]
    pub fn with_spill(mut self, writer: SpillWriter, scope: impl Into<String>) -> Self {
        self.spill = Some(SpillBinding {
            writer,
            scope: scope.into(),
        });
        self
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
    /// 1. Plan each block's retained length: per-block `cap_chars` first.
    /// 2. If the total still exceeds the budget, sort non-core blocks by
    ///    descending length and cut from the largest first (greedy).
    /// 3. Materialize every block at its planned length in one cut, then drop
    ///    blocks whose remaining content is empty/whitespace. With a bound
    ///    spill sink the cut is a head/tail preview whose full original is
    ///    spilled ([`truncate_with_spill`]); the marker and guide lines are
    ///    bounded overhead on top of the planned width.
    pub fn assemble_budgeted_blocks(&self) -> Vec<ContextBlock> {
        let mut planned: Vec<usize> = self
            .blocks
            .iter()
            .map(|b| {
                b.content
                    .chars()
                    .count()
                    .min(b.cap_chars.unwrap_or(usize::MAX))
            })
            .collect();
        let mut total: usize = planned.iter().sum();
        if total > self.total_budget_chars {
            let mut order: Vec<usize> = (0..self.blocks.len())
                .filter(|&i| !self.blocks[i].core)
                .collect();
            order.sort_by_key(|&i| std::cmp::Reverse(planned[i]));
            for i in order {
                if total <= self.total_budget_chars {
                    break;
                }
                let len = planned[i];
                if len == 0 {
                    continue;
                }
                let over = total - self.total_budget_chars;
                let cut = len.min(over);
                planned[i] = len - cut;
                total -= cut;
            }
        }
        self.blocks
            .iter()
            .zip(planned)
            .filter_map(|(b, keep)| {
                let content = self.materialize_cut(&b.content, keep);
                if content.trim().is_empty() {
                    return None;
                }
                Some(ContextBlock {
                    name: b.name.clone(),
                    content,
                    core: b.core,
                    cap_chars: b.cap_chars,
                })
            })
            .collect()
    }

    /// Apply one planned cut to a block body.
    ///
    /// Without a spill sink this is a plain prefix cut (the historical shape).
    /// With a sink the cut becomes a head/tail preview whose full original is
    /// spilled; when the spill cannot be written the full original is kept
    /// inline (prefer too long over lost information).
    fn materialize_cut(&self, content: &str, keep_chars: usize) -> String {
        match &self.spill {
            Some(binding) => {
                truncate_with_spill(content, keep_chars, &binding.writer, &binding.scope).text
            }
            None => content.chars().take(keep_chars).collect(),
        }
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
    /// Catalog with the default budget (~800 tokens ≈ 1600 chars).
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

    // -----------------------------------------------------------------------
    // Long-tail spill: head/tail preview + retrieval guide, never losing content.
    // -----------------------------------------------------------------------

    /// The preview halves land on UTF-8 boundaries: a 4-byte (surrogate-pair)
    /// character is never split between the preview and the omitted middle.
    #[test]
    fn head_tail_preview_keeps_both_ends_on_char_boundaries() {
        // 8 scalar values; the cut at keep=5 falls right where 4-byte
        // characters sit on both sides of the boundary.
        let content = "ab🦀cd🦀ef";
        let split = split_head_tail(content, 5);
        assert_eq!(split.head, "ab", "头预览为前 keep/2 个字符");
        assert_eq!(split.tail, "🦀ef", "尾预览为后 keep-keep/2 个字符");
        assert_eq!(split.omitted_chars, 3);
        assert_eq!(
            split.head.chars().count() + split.tail.chars().count(),
            5,
            "头尾对半且总保留量恰为 keep"
        );

        // All-4-byte content: the halves are whole characters, never halves of
        // one (a byte-based cut would emit invalid UTF-8 here).
        let crabs = "🦀".repeat(10);
        let split = split_head_tail(&crabs, 5);
        assert_eq!(split.head, "🦀🦀");
        assert_eq!(split.tail, "🦀🦀🦀");
        assert_eq!(split.omitted_chars, 5);

        // Boundary at the very ends stays sane (the tail keeps the odd char).
        let split = split_head_tail(content, 2);
        assert_eq!(split.head, "a");
        assert_eq!(split.tail, "f");
        let split = split_head_tail(content, 1);
        assert_eq!(split.head, "");
        assert_eq!(split.tail, "f");
        let split = split_head_tail(content, 0);
        assert_eq!(split.head, "");
        assert_eq!(split.tail, "");
        assert_eq!(split.omitted_chars, 8);
    }

    /// The retrieval guide carries the real spill path, and the spilled file
    /// holds the full original.
    #[test]
    fn truncation_guide_line_carries_the_real_spill_path() {
        let temp = tempfile::tempdir().unwrap();
        let writer = SpillWriter::new(temp.path());
        let content = "X".repeat(500);

        let out = truncate_with_spill(&content, 100, &writer, "session-abc");

        let path = out.spilled_path.expect("截断必须先落盘成功");
        assert!(path.is_file(), "落盘路径必须真实存在: {path:?}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            content,
            "落盘文件保存完整原文"
        );
        assert!(
            path.to_string_lossy().contains("session-abc"),
            "文件名含会话"
        );

        let lines: Vec<&str> = out.text.lines().collect();
        assert_eq!(lines.len(), 4, "头 / 标记行 / 尾 / 取回指引行: {lines:?}");
        assert_eq!(lines[0], "X".repeat(50), "头预览保留前一半");
        assert!(
            lines[1].contains("已省略") && lines[1].contains("400"),
            "标记行"
        );
        assert_eq!(lines[2], "X".repeat(50), "尾预览保留后一半");
        assert_eq!(
            lines[3],
            retrieval_guide(&path),
            "指引行是含真实路径的取回指引"
        );
        assert!(
            lines[3].contains(&path.display().to_string()),
            "指引行文本必须含路径"
        );
    }

    /// When the spill cannot be written the full original stays inline: prefer
    /// too long over lost information.
    #[test]
    fn spill_failure_keeps_the_full_original_inline() {
        let temp = tempfile::tempdir().unwrap();
        // A regular file where the spill directory should be: every write fails.
        let blocked = temp.path().join("occupied-by-a-file");
        std::fs::write(&blocked, "not a directory").unwrap();
        let writer = SpillWriter::new(&blocked);
        let content = "Y".repeat(500);

        let out = truncate_with_spill(&content, 100, &writer, "session-abc");

        assert_eq!(out.text, content, "落盘失败必须回退为内联原文");
        assert!(out.spilled_path.is_none());
        assert!(out.spill_error.is_some(), "回退原因应可诊断");
    }

    /// Below the threshold nothing happens at all: no truncation, no spill.
    #[test]
    fn below_threshold_is_zero_change() {
        let temp = tempfile::tempdir().unwrap();
        let writer = SpillWriter::new(temp.path());
        let content = "z".repeat(100);

        let out = truncate_with_spill(&content, 100, &writer, "session-abc");
        assert_eq!(out.text, content);
        assert!(out.spilled_path.is_none());
        assert!(out.spill_error.is_none());
        assert!(!writer.spill_dir().exists(), "低于阈值不得写盘 (零变化)");

        let out = truncate_with_spill(&content, 200, &writer, "session-abc");
        assert_eq!(out.text, content);
        assert!(out.spilled_path.is_none());
    }

    /// Names carry scope + sequence + short hash, so repeated spills of the
    /// same content in the same session still get distinct paths.
    #[test]
    fn spill_paths_are_unique_across_repeated_writes() {
        let temp = tempfile::tempdir().unwrap();
        let writer = SpillWriter::new(temp.path());
        let content = "identical payload";

        let mut paths = std::collections::HashSet::new();
        for _ in 0..5 {
            let out = truncate_with_spill(content, 4, &writer, "same-session");
            let path = out.spilled_path.expect("落盘成功");
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.starts_with("spill-"), "统一前缀: {name}");
            assert!(name.contains("same-session"), "文件名含会话: {name}");
            assert!(name.ends_with(".txt"), "文件名: {name}");
            assert!(paths.insert(path), "路径必须唯一");
        }
        assert_eq!(paths.len(), 5);

        // The name formula itself: scope + sequence + short hash.
        let name = spill_file_name("sess/1", 7, "abc");
        assert_eq!(
            name,
            format!("spill-sess_1-000007-{:08x}.txt", short_hash(b"abc"))
        );
    }

    /// The assembler's cut point, with a spill sink bound, keeps the head/tail
    /// preview and points at the full original instead of dropping the tail.
    #[test]
    fn assembler_with_spill_keeps_head_tail_and_marks_the_middle() {
        let temp = tempfile::tempdir().unwrap();
        let writer = SpillWriter::new(temp.path());
        let mem_full = "记忆内容".repeat(50); // 200 chars
        let a = ContextAssembler::new(300)
            .with_spill(writer.clone(), "sess-1")
            .push(ContextBlock::new("persona", "核心人格内容".repeat(30)).core(true))
            .push(ContextBlock::new("mem", mem_full.clone()))
            .push(ContextBlock::new("prefs", "偏好内容".repeat(20)));

        let out = a.assemble_budgeted_blocks();
        let core = out.iter().find(|b| b.name == "persona").unwrap();
        assert_eq!(core.content, "核心人格内容".repeat(30), "核心块不受影响");
        let prefs = out.iter().find(|b| b.name == "prefs").unwrap();
        assert_eq!(prefs.content, "偏好内容".repeat(20), "未截断块零变化");

        let mem = out.iter().find(|b| b.name == "mem").unwrap();
        let lines: Vec<&str> = mem.content.lines().collect();
        assert_eq!(lines.len(), 4, "头/标记/尾/指引: {lines:?}");
        assert_eq!(lines[0], "记忆内容".repeat(5), "头 20 字符");
        assert!(lines[1].contains("160"), "中间 160 字符被标记替代");
        assert_eq!(lines[2], "记忆内容".repeat(5), "尾 20 字符");
        assert!(lines[3].contains("完整内容已存于"), "附取回指引行");

        // The full original is retrievable from disk.
        let spilled: Vec<_> = std::fs::read_dir(writer.spill_dir())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(spilled.len(), 1);
        assert_eq!(std::fs::read_to_string(&spilled[0]).unwrap(), mem_full);
    }
}
