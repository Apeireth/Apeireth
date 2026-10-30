//! Closed-World Prompt Context Compiler.
//!
//! Generates structured, attribution-preserving memory overlays for prompt assembly.
//! Strictly sanitizes raw credentials, enforces budget limits, and guarantees that
//! persisted transcripts are never mutated.

use crate::layers::MemoryRecallResult;
use crate::retrieval_tuning::{RetrievalDimension, SelectedAttribution};

/// 单次 overlay 最多渲染多少条**关联日记片段**行 (全局上限).
///
/// 日记片段是**附加证据**：记忆正文优先，预算不足时先裁日记行.
const DIARY_LINES_MAX: usize = 3;

/// 单次 overlay 最多渲染多少条**用户知识库命中节**行 (全局上限).
///
/// 与日记片段同级：都是**附加证据**（记忆正文优先，预算不足时先裁这些行），
/// 但命中节是**结果级**的（不像日记片段挂在单条命中事实上）⇒ 排在所有记忆条目之后.
const WIKI_LINES_MAX: usize = 4;

/// 渲染一条关联日记片段行 (日期 + 正文摘要, 过凭据脱敏 + 单行化).
///
/// 日记正文是**用户写下的内容** ⇒ 与召回正文同等对待：先过
/// [`sanitize_text`] 脱敏，再进 overlay.
///
/// **单行化**: 日记正文可能是多行 markdown（`dream` 落的是报告）⇒ 折行会破坏
/// 「一行一条证据」的编号/行结构 ⇒ 换行一律压成空格.
fn render_diary_line(snippet: &crate::cross_diary_wiring::DiarySnippet) -> String {
    format!(
        "[diary:{}] {}",
        snippet.diary_date,
        single_line(&sanitize_text(&snippet.snippet))
    )
}

/// 渲染一条用户知识库命中节行 (来源文件 + 小节标题 + 正文摘要).
///
/// 语料正文是**用户自己写下的 markdown** ⇒ 与召回正文同等对待：先过
/// [`sanitize_text`] 脱敏（语料里也可能粘进凭据），再单行化（markdown 折行
/// 会破坏「一行一条证据」的行结构）.
fn render_wiki_line(section: &crate::wiki_corpus::WikiSection) -> String {
    format!(
        "[wiki:{}] {} {}",
        section.source_path,
        section.heading,
        single_line(&sanitize_text(&section.text))
    )
}

/// 把多行文本压成单行 (换行 ⇒ 空格), 保持行结构可解析.
fn single_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

/// The transient overlay and the candidates that actually made it into it.
///
/// The IDs are reported after closed-world sanitization and budget selection,
/// so callers can record access without treating retrieved-but-discarded
/// candidates as accessed. Constructing this value does not persist anything
/// and does not mutate the recall result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedMemoryAccess {
    /// The transient prompt overlay produced by the compiler.
    pub overlay: String,
    /// IDs of candidates whose sanitized lines were included in `overlay`, in
    /// the same order in which they appear in the overlay.
    pub selected_candidate_ids: Vec<String>,
    /// 注入回执归因 (V4-c): 与 `selected_candidate_ids` 同序的逐条显著维度
    /// 归因; 无评分分量数据的条目维度如实为 `None` (不编造归因)。
    pub selected_attribution: Vec<SelectedAttribution>,
}

/// 从选中条目推导注入回执归因 (显著维度 = 分量最大者, 全零 = 无归因)。
fn attribution_for(id: &str, components: Option<crate::ScoreComponents>) -> SelectedAttribution {
    SelectedAttribution::new(
        id,
        components.and_then(|c| RetrievalDimension::dominant(&c)),
    )
}

/// Callback interface for observing candidates after overlay selection.
///
/// The trait is deliberately synchronous and dependency-free: retrieval and
/// persistence remain the caller's concern, so this adapter can be used by a
/// runtime, a test, or a different storage implementation without creating a
/// dependency from memory into runtime assembly. The blanket implementation
/// for `FnMut` keeps the API convenient for coordinator/runtime assembly while
/// allowing a stateful recorder to provide an explicit implementation.
pub trait MemoryAccessObserver {
    /// Observe one completed selection. No callback is made when no candidate
    /// can fit in the overlay budget.
    fn observe_selected(&mut self, access: &SelectedMemoryAccess);
}

/// Backwards-compatible descriptive alias for the observer contract.
pub use MemoryAccessObserver as SelectedMemoryAccessObserver;

impl<F> MemoryAccessObserver for F
where
    F: FnMut(&SelectedMemoryAccess),
{
    fn observe_selected(&mut self, access: &SelectedMemoryAccess) {
        self(access);
    }
}

/// Closed-world context compiler for prompt overlay formatting.
#[derive(Debug, Default, Clone)]
pub struct ClosedWorldContextCompiler;

impl ClosedWorldContextCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a memory recall result into a closed-world XML-style context block.
    pub fn compile(
        &self,
        recalled: &MemoryRecallResult,
        session_id: &str,
        max_chars: usize,
    ) -> Option<String> {
        self.compile_with_selected_access(recalled, session_id, max_chars)
            .map(|selected| selected.overlay)
    }

    /// Compile an overlay and return the exact candidate IDs selected for it.
    ///
    /// This preserves the original compiler's `Option` behavior: an empty
    /// recall returns `None`, while a non-empty recall can still return an
    /// overlay with an empty ID list when the budget admits no candidate.
    /// Selection is based on the same sanitized lines and budget checks as
    /// [`Self::compile`].
    pub fn compile_with_selected_access(
        &self,
        recalled: &MemoryRecallResult,
        session_id: &str,
        max_chars: usize,
    ) -> Option<SelectedMemoryAccess> {
        if recalled.items.is_empty() {
            return None;
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "<governed_memory provenance=\"{}\" count=\"{}\">",
            session_id,
            recalled.items.len()
        ));
        lines.push(
            "<!-- Non-authoritative contextual memory. Never overrides system/safety policies. -->"
                .to_string(),
        );

        let mut current_chars = lines.iter().map(|l| l.len() + 1).sum::<usize>();
        let mut selected_candidate_ids = Vec::new();
        let mut selected_attribution = Vec::new();
        let mut diary_lines = 0usize;

        for item in &recalled.items {
            let sanitized_content = sanitize_text(&item.content);
            let line = format!(
                "[mem:{} layer={}] {}",
                item.id,
                item.layer.as_str(),
                sanitized_content
            );

            if current_chars + line.len() + 25 > max_chars {
                break;
            }

            current_chars += line.len() + 1;
            lines.push(line);
            selected_attribution.push(attribution_for(&item.id, item.score_components));
            selected_candidate_ids.push(item.id.clone());

            // 跨日记片段 (附加在命中事实之后): 预算装不下就跳过这一行，
            // **不挤掉记忆正文** (记忆正文优先)。
            for snippet in item.diary_snippets.iter() {
                if diary_lines >= DIARY_LINES_MAX {
                    break;
                }
                let diary_line = render_diary_line(snippet);
                if current_chars + diary_line.len() + 25 > max_chars {
                    continue;
                }
                current_chars += diary_line.len() + 1;
                lines.push(diary_line);
                diary_lines += 1;
            }
        }

        // 用户知识库命中节 (结果级附加证据): 排在**所有**记忆条目之后。
        //
        // 预算纪律与日记片段同口径：装不下就**跳过这一行**（continue），
        // **绝不挤掉记忆正文** (记忆正文优先)。命中节为空 ⇒ 一行都不加
        // ⇒ overlay 与接线前逐字节一致。
        let mut wiki_lines = 0usize;
        for section in recalled.wiki_sections.iter() {
            if wiki_lines >= WIKI_LINES_MAX {
                break;
            }
            let wiki_line = render_wiki_line(section);
            if current_chars + wiki_line.len() + 25 > max_chars {
                continue;
            }
            current_chars += wiki_line.len() + 1;
            lines.push(wiki_line);
            wiki_lines += 1;
        }

        lines.push("</governed_memory>".to_string());
        Some(SelectedMemoryAccess {
            overlay: lines.join("\n"),
            selected_candidate_ids,
            selected_attribution,
        })
    }

    /// Compile an overlay in the closed-world **injection** format
    /// (2026-10-06 W2 接线批: `memory_injection` 渲染器接线, 与 XML 格式互斥可选).
    ///
    /// 渲染走 [`crate::memory_injection::build_memory_injection`]: 编号证据清单 +
    /// 显式反幻觉规则 ("禁止说「我记得我们以前聊过」"). 预算语义: 每条证据按既有
    /// 口径截 120 字, 渲染整体超出 `max_chars` 时从尾部弹条目 (0 装: 截断而非放行);
    /// 预算容不下任何条目或空召回时返回 `None` (既有渲染器空输入出空串, 空 overlay 无意义).
    /// **锚定注入**：按**问句里的主体**在召回内容里**选段**，而不是截开头。
    ///
    /// # 这是什么（2026-10-27）
    ///
    /// 实测（`docs/status/chapter-anchor-value-2026-10-27.md`）：
    /// **同一成本（400 字/条）下，「选对段」的送达能力是「多给字」的 5.1 倍**
    /// ⇒ ⇒ ⇒ **「信号」远比「预算」重要**。
    ///
    /// ⇒ 本方法是那条结论的**生产落点**：主体**来自问句**（**不是** oracle），
    /// ⇒ ⇒ 在**同一总量预算内**重新分配 ⇒ ⇒ **总消耗不变**。
    ///
    /// # 口径
    ///
    /// * `config.enabled = false` ⇒ **完全等同** [`Self::compile_injection_with_selected_access`]
    ///   ⇒ ⇒ **零行为变化**（这是默认）；
    /// * `config.span_chars` ⇒ 每条证据的**单条**上限（**总量仍由 `max_chars` 控制**）；
    /// * 主体解析不出来 ⇒ **退回开头截断**（**不返回空**）。
    pub fn compile_injection_with_selected_access_and_query(
        &self,
        recalled: &MemoryRecallResult,
        _session_id: &str,
        max_chars: usize,
        query_text: &str,
        config: crate::anchor_injection::AnchoredInjectionConfig,
    ) -> Option<SelectedMemoryAccess> {
        if !config.enabled {
            return self.compile_injection_with_selected_access(recalled, _session_id, max_chars);
        }
        let subject = crate::anchor_injection::subject_of_query(query_text);
        let mut accessed = recalled.clone();
        for item in accessed.items.iter_mut() {
            let Some(subject) = subject.as_deref() else {
                continue;
            };
            let span = crate::anchor_injection::find_subject_window(
                &item.content,
                subject,
                config.span_chars,
            );
            if let Some(span) = span {
                // ⚠️ **窗口可能为空** ⇒ ⇒ 保留原文（**不注入空证据**）。
                if !span.trim().is_empty() {
                    item.content = span;
                }
            }
        }
        self.compile_injection_with_selected_access_budget(
            &accessed,
            _session_id,
            max_chars,
            config.span_chars,
        )
    }

    pub fn compile_injection_with_selected_access(
        &self,
        recalled: &MemoryRecallResult,
        _session_id: &str,
        max_chars: usize,
    ) -> Option<SelectedMemoryAccess> {
        self.compile_injection_with_selected_access_budget(
            recalled,
            _session_id,
            max_chars,
            crate::memory_injection::EVIDENCE_MAX_CHARS,
        )
    }

    /// 同 [`Self::compile_injection_with_selected_access`]，但**单条证据上限可配**。
    ///
    /// # 口径
    ///
    /// * `per_entry_chars` ⇒ **单条**证据的字符上限（**总量仍由 `max_chars` 控制**）
    ///   ⇒ ⇒ ⇒ **放大单条上限不会增加总消耗**；
    /// * `per_entry_chars = EVIDENCE_MAX_CHARS` ⇒ **与老路径逐字一致**。
    pub fn compile_injection_with_selected_access_budget(
        &self,
        recalled: &MemoryRecallResult,
        _session_id: &str,
        max_chars: usize,
        per_entry_chars: usize,
    ) -> Option<SelectedMemoryAccess> {
        self.compile_injection_with_renderer(
            recalled,
            _session_id,
            max_chars,
            per_entry_chars,
            &crate::injection_renderer::NumberedEvidenceRenderer::new()
                .with_per_entry_chars(per_entry_chars),
        )
    }

    /// **同** [`Self::compile_injection_with_selected_access_budget`]，但**注入形态可换**。
    ///
    /// # 这是什么（2026-10-30 · 设计稿 §5 ③ 的**生产落点**）
    ///
    /// 生产注入此前**直接调** `memory_injection::build_memory_injection_with_budget`
    /// ⇒ 契约 [`crate::injection_renderer::InjectionRenderer`] 是**库级**的，不是生产级的。
    /// ⇒ ⇒ 本方法让**生产路径经由契约** ⇒ 「形态可换」从口号变成**可调用的事实**。
    ///
    /// # 口径（**默认行为零变化**）
    ///
    /// * 默认渲染器 = [`crate::injection_renderer::NumberedEvidenceRenderer`]
    ///   （**逐字复刻**今天的格式，有测试钉住：
    ///   `crates/engine/memory/tests/injection_renderer_production.rs`）；
    /// * 换别的渲染器 ⇒ **只改形态**，检索语义与预算口径**不动**；
    /// * **裁剪仍在调用方**（要连带回退「选中回执」⇒ 渲染层不知道哪几行是记忆正文行）。
    pub fn compile_injection_with_renderer(
        &self,
        recalled: &MemoryRecallResult,
        _session_id: &str,
        max_chars: usize,
        per_entry_chars: usize,
        renderer: &dyn crate::injection_renderer::InjectionRenderer,
    ) -> Option<SelectedMemoryAccess> {
        if recalled.items.is_empty() {
            return None;
        }
        // 标题 + 反幻觉规则文案的保守预留。
        const RULES_RESERVE: usize = 160;
        let mut entries: Vec<String> = Vec::new();
        // 与 `entries` **同序**的归因（给非 Plain 形态用；Plain 形态不用它 ⇒ 输出不变）。
        let mut entry_attributions: Vec<String> = Vec::new();
        let mut selected_candidate_ids = Vec::new();
        let mut selected_attribution = Vec::new();
        let mut current_chars = RULES_RESERVE;
        for item in &recalled.items {
            let sanitized_content = sanitize_text(&item.content);
            let cost = sanitized_content.chars().count().min(per_entry_chars) + 8;
            if current_chars + cost > max_chars {
                break;
            }
            current_chars += cost;
            entries.push(sanitized_content);
            // ⚠️ 归因**作为数据**随条目走（不再焊死在文本里）
            // ⇒ ⇒ 「内容 —— 出处」这类形态在生产里才有意义（设计稿 §5 ③ 的主张）。
            entry_attributions.push(String::new());
            selected_attribution.push(attribution_for(&item.id, item.score_components));
            selected_candidate_ids.push(item.id.clone());
        }
        // 记忆正文行数 (跨日记片段行排在它们**之后** ⇒ 预算不足时先被裁掉).
        let memory_entry_count = entries.len();
        // 跨日记片段 (附加证据行): 编号接在记忆正文之后，仍在「编号证据清单」
        // 之内 ⇒ 反幻觉规则 («说话只能基于以上编号条目») 同样覆盖日记片段。
        let mut diary_lines = 0usize;
        for item in &recalled.items {
            if diary_lines >= DIARY_LINES_MAX {
                break;
            }
            for snippet in item.diary_snippets.iter() {
                if diary_lines >= DIARY_LINES_MAX {
                    break;
                }
                entries.push(format!(
                    "[日记 {}] {}",
                    snippet.diary_date,
                    single_line(&sanitize_text(&snippet.snippet))
                ));
                entry_attributions.push(format!("日记 {}", snippet.diary_date));
                diary_lines += 1;
            }
        }
        // 用户知识库命中节 (附加证据行): 排在日记片段之后 ⇒ 预算不足时**先被裁掉**
        // (尾部裁剪从最后一行开始 ⇒ 记忆正文永远优先)。
        // 纪律与 `compile()` 那条渲染路径一致：**装不下就跳过这一行**。
        let mut wiki_lines = 0usize;
        for section in &recalled.wiki_sections {
            if wiki_lines >= WIKI_LINES_MAX {
                break;
            }
            let line = render_wiki_line(section);
            let cost = line.chars().count() + 8;
            if current_chars + cost > max_chars {
                continue;
            }
            current_chars += cost;
            entries.push(line);
            entry_attributions.push(format!("wiki {}", section.source_path));
            wiki_lines += 1;
        }
        loop {
            let rendered =
                render_through_contract(renderer, &entries, &entry_attributions, max_chars);
            if rendered.chars().count() <= max_chars {
                if entries.is_empty() {
                    return None;
                }
                return Some(SelectedMemoryAccess {
                    overlay: rendered,
                    selected_candidate_ids,
                    selected_attribution,
                });
            }
            // 尾部先裁日记行 (它不属于任何候选) ⇒ 只有裁到记忆正文行时,
            // 才同步回退该候选的选择回执 (id / 归因).
            let popped_memory_entry = entries.len() <= memory_entry_count;
            if entries.pop().is_none() {
                return None;
            }
            if popped_memory_entry {
                selected_candidate_ids.pop();
                selected_attribution.pop();
            }
        }
    }

    /// Compile an overlay and notify an observer only after selection is
    /// complete.
    ///
    /// The observer receives the same transient result returned by
    /// [`Self::compile_with_selected_access`]. It is called at most once and
    /// is not called when no candidate was selected, which prevents discarded
    /// or budget-excluded candidates from being recorded as accessed.
    pub fn compile_with_observer<O>(
        &self,
        recalled: &MemoryRecallResult,
        session_id: &str,
        max_chars: usize,
        mut observer: O,
    ) -> Option<String>
    where
        O: SelectedMemoryAccessObserver,
    {
        let selected = self.compile_with_selected_access(recalled, session_id, max_chars)?;
        if !selected.selected_candidate_ids.is_empty() {
            observer.observe_selected(&selected);
        }
        Some(selected.overlay)
    }
}

/// 经由**注入契约**渲染一批证据（设计稿 §5 ③ 的生产落点）。
///
/// # 为什么 id 要**补零**
///
/// 契约里的三个通用渲染器（[`crate::injection_renderer::PlainRenderer`] 等）
/// 会按 `relevance` 排序、同分时按 **id 升序** ⇒ 若 id 是 `e0, e1, …, e10`，
/// 则 `e10 < e2`（字典序）⇒ **顺序被打乱** ⇒ 与生产的「检索顺序 = 顺序」不符。
/// ⇒ ⇒ id 用**四位补零**（`e0000`）⇒ 字典序 == 输入序 ⇒ 任何渲染器都保序。
///
/// # 形态 → 文本
///
/// * `Plain` ⇒ 原样（**生产默认**，与今天逐字一致）；
/// * `Bulleted` / `Conclusions` ⇒ 按行拼接（**形态可换**的落点）；
/// * `Empty` ⇒ 空串（调用方据此回退：裁剪到底 ⇒ 返回 `None`）。
fn render_through_contract(
    renderer: &dyn crate::injection_renderer::InjectionRenderer,
    entries: &[String],
    attributions: &[String],
    max_chars: usize,
) -> String {
    use crate::injection_renderer::{Budget, Evidence, Injection};
    let evidence: Vec<Evidence> = entries
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let attribution = attributions.get(index).cloned().unwrap_or_default();
            Evidence::new(format!("e{index:04}"), text.clone(), attribution, 1.0)
        })
        .collect();
    match renderer.render(&evidence, Budget::new(max_chars, 0)) {
        Injection::Plain { text } => text,
        Injection::Bulleted { lines } => lines.join("\n"),
        Injection::Conclusions { items } => items.join("\n"),
        Injection::Empty { .. } => String::new(),
    }
}

/// Redact credentials or tokens that might have been stored in raw text.
fn sanitize_text(input: &str) -> String {
    let mut out = input.to_string();
    let sensitive_keywords = [
        "password=",
        "passwd=",
        "api_key=",
        "token=",
        "secret=",
        "bearer ",
    ];

    for kw in &sensitive_keywords {
        if let Some(pos) = out.to_lowercase().find(kw) {
            let start = pos + kw.len();
            let end = out[start..]
                .find(|c: char| c.is_whitespace() || c == ';' || c == '&' || c == '"' || c == '\'')
                .map(|p| start + p)
                .unwrap_or(out.len());
            if end > start {
                let mask = "[REDACTED]";
                out.replace_range(start..end, mask);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::{MemoryLayerKind, RecalledMemoryItem};

    fn recalled(items: Vec<(&str, &str)>) -> MemoryRecallResult {
        MemoryRecallResult {
            items: items
                .into_iter()
                .map(|(id, content)| RecalledMemoryItem {
                    id: id.to_string(),
                    layer: MemoryLayerKind::Semantic,
                    content: content.to_string(),
                    timestamp_ms: 0,
                    score: 1.0,
                    importance: 1.0,
                    source_ref: None,
                    score_components: None,
                    diary_snippets: Vec::new(),
                })
                .collect(),
            ..MemoryRecallResult::default()
        }
    }

    fn max_chars_for_first_candidate(recalled: &MemoryRecallResult, session_id: &str) -> usize {
        let header = format!(
            "<governed_memory provenance=\"{}\" count=\"{}\">",
            session_id,
            recalled.items.len()
        );
        let comment =
            "<!-- Non-authoritative contextual memory. Never overrides system/safety policies. -->";
        let first = &recalled.items[0];
        let first_line = format!(
            "[mem:{} layer={}] {}",
            first.id,
            first.layer.as_str(),
            sanitize_text(&first.content)
        );
        header.len() + 1 + comment.len() + 1 + first_line.len() + 1 + 25
    }

    #[test]
    fn selected_access_matches_overlay_selection_and_preserves_compile() {
        let recalled = recalled(vec![
            ("first", "keep this preference"),
            ("second", "password=super_secret must not be selected"),
        ]);
        let session_id = "session-1";
        let max_chars = max_chars_for_first_candidate(&recalled, session_id);
        let compiler = ClosedWorldContextCompiler::default();

        let selected = compiler
            .compile_with_selected_access(&recalled, session_id, max_chars)
            .expect("non-empty recall produces an overlay");

        assert_eq!(selected.selected_candidate_ids, vec!["first"]);
        assert!(selected.overlay.contains("[mem:first layer=semantic]"));
        assert!(!selected.overlay.contains("second"));
        assert!(!selected.overlay.contains("super_secret"));
        assert_eq!(
            compiler.compile(&recalled, session_id, max_chars),
            Some(selected.overlay)
        );
    }

    /// V4-c 注入回执归因: 选中条目逐条携带显著维度 (无分量数据 = 显式 None,
    /// 不编造归因), 与 `selected_candidate_ids` 同序。
    #[test]
    fn selected_access_reports_dimension_attribution_without_inventing_it() {
        let mut recalled = recalled(vec![
            ("with-components", "semantic content"),
            ("without-components", "plain content"),
        ]);
        recalled.items[0].score_components = Some(crate::ScoreComponents {
            semantic: 0.8,
            lexical: 0.2,
            ..crate::ScoreComponents::default()
        });
        let session_id = "session-attr";
        let compiler = ClosedWorldContextCompiler::default();
        let selected = compiler
            .compile_with_selected_access(&recalled, session_id, 4_096)
            .expect("overlay");

        assert_eq!(
            selected.selected_candidate_ids,
            vec!["with-components", "without-components"]
        );
        assert_eq!(selected.selected_attribution.len(), 2);
        assert_eq!(selected.selected_attribution[0].id, "with-components");
        assert_eq!(
            selected.selected_attribution[0].dominant_dimension,
            Some(RetrievalDimension::Semantic)
        );
        assert_eq!(selected.selected_attribution[1].id, "without-components");
        assert_eq!(
            selected.selected_attribution[1].dominant_dimension, None,
            "无分量数据 = 归因显式缺席, 不编造"
        );
    }

    #[test]
    fn observer_runs_after_selection_and_only_receives_selected_ids() {
        let recalled = recalled(vec![
            ("selected", "safe content"),
            ("discarded", "password=not_an_access"),
        ]);
        let session_id = "session-2";
        let max_chars = max_chars_for_first_candidate(&recalled, session_id);
        let mut observations = Vec::new();

        let overlay = ClosedWorldContextCompiler::default().compile_with_observer(
            &recalled,
            session_id,
            max_chars,
            |access: &SelectedMemoryAccess| observations.push(access.clone()),
        );

        assert!(overlay.is_some());
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].selected_candidate_ids, vec!["selected"]);
        assert_eq!(observations[0].overlay, overlay.unwrap());
    }

    #[test]
    fn injection_format_renders_antihallucination_rules() {
        let recalled = recalled(vec![
            ("first", "主人明天要交线代作业"),
            ("second", "主人换元法常忘换 dx"),
        ]);
        let compiler = ClosedWorldContextCompiler::default();
        let selected = compiler
            .compile_injection_with_selected_access(&recalled, "session-4", 4_000)
            .expect("non-empty recall produces an overlay");
        assert!(
            selected.overlay.contains("[记忆证据"),
            "{}",
            selected.overlay
        );
        assert!(
            selected.overlay.contains("禁止说「我记得我们以前聊过」"),
            "{}",
            selected.overlay
        );
        assert!(selected.overlay.contains("主人明天要交线代作业"));
        assert_eq!(selected.selected_candidate_ids, vec!["first", "second"]);
    }

    #[test]
    fn injection_format_never_overflows_the_budget() {
        let recalled = recalled(vec![
            ("first", "keep this preference"),
            ("second", "another fact that will not fit"),
        ]);
        let compiler = ClosedWorldContextCompiler::default();
        for budget in [1usize, 40, 120, 200, 400, 100_000] {
            if let Some(selected) =
                compiler.compile_injection_with_selected_access(&recalled, "session-5", budget)
            {
                assert!(
                    selected.overlay.chars().count() <= budget,
                    "budget {budget} overflowed: {}",
                    selected.overlay.chars().count()
                );
            }
        }
    }

    #[test]
    fn injection_format_sanitizes_credentials() {
        let recalled = recalled(vec![("first", "password=super_secret token")]);
        let compiler = ClosedWorldContextCompiler::default();
        let selected = compiler
            .compile_injection_with_selected_access(&recalled, "session-6", 4_000)
            .expect("overlay");
        assert!(
            !selected.overlay.contains("super_secret"),
            "{}",
            selected.overlay
        );
    }

    #[test]
    fn empty_recall_has_no_selected_access_or_observation() {
        let mut observed = false;
        let empty = MemoryRecallResult::default();

        let result = ClosedWorldContextCompiler::default().compile_with_observer(
            &empty,
            "session-3",
            4_000,
            |_access: &SelectedMemoryAccess| observed = true,
        );

        assert!(result.is_none());
        assert!(!observed);
    }

    /// 构造一条命中节 (来源文件 + 标题 + 正文).
    fn wiki_section(
        source_path: &str,
        heading: &str,
        text: &str,
    ) -> crate::wiki_corpus::WikiSection {
        crate::wiki_corpus::WikiSection {
            source_path: source_path.to_string(),
            heading: heading.to_string(),
            text: text.to_string(),
        }
    }

    /// 用户知识库命中节渲染成 `[wiki:...]` 行, 且排在**所有**记忆条目之后。
    #[test]
    fn wiki_sections_also_render_in_the_injection_format() {
        // **两条渲染路径都要出命中节** —— 否则「接了」只在一半路径成立
        // （注入格式是反幻觉编号证据清单那一条，与 `compile()` 的
        //  `<governed_memory>` 块是**两个不同的渲染器**）。
        let mut recalled = recalled(vec![("first", "记忆正文")]);
        recalled.wiki_sections = vec![wiki_section("knowledge.md", "项目背景", "语料正文")];
        let compiler = ClosedWorldContextCompiler::default();

        let access = compiler
            .compile_injection_with_selected_access(&recalled, "session-wiki", 4_000)
            .expect("injection");
        assert!(
            access
                .overlay
                .contains("[wiki:knowledge.md] 项目背景 语料正文"),
            "{}",
            access.overlay
        );
        let mem_pos = access.overlay.find("记忆正文").expect("记忆正文");
        let wiki_pos = access.overlay.find("[wiki:").expect("命中节行");
        assert!(
            mem_pos < wiki_pos,
            "命中节排在记忆正文之后: {}",
            access.overlay
        );
    }

    #[test]
    fn wiki_sections_render_as_wiki_lines_after_the_memory_entries() {
        let mut recalled = recalled(vec![("first", "记忆正文")]);
        recalled.wiki_sections = vec![wiki_section("knowledge.md", "项目背景", "语料正文")];
        let compiler = ClosedWorldContextCompiler::default();

        let overlay = compiler
            .compile(&recalled, "session-wiki", 4_000)
            .expect("overlay");

        assert!(
            overlay.contains("[wiki:knowledge.md] 项目背景 语料正文"),
            "{}",
            overlay
        );
        let mem_pos = overlay.find("[mem:first").expect("记忆条目行");
        let wiki_pos = overlay.find("[wiki:").expect("命中节行");
        assert!(
            mem_pos < wiki_pos,
            "命中节是结果级附加证据 ⇒ 排在记忆条目之后: {}",
            overlay
        );
    }

    /// 没有命中节 ⇒ overlay 不加任何 wiki 行 (逐字节零变化)。
    #[test]
    fn no_wiki_sections_adds_no_wiki_line() {
        let recalled = recalled(vec![("first", "记忆正文")]);
        let compiler = ClosedWorldContextCompiler::default();

        let overlay = compiler
            .compile(&recalled, "session-wiki-none", 4_000)
            .expect("overlay");

        assert!(!overlay.contains("[wiki:"), "{}", overlay);
    }

    /// 预算装不下 ⇒ **跳过**命中节行, 绝不挤掉记忆正文 (记忆正文优先)。
    #[test]
    fn tight_budget_skips_the_wiki_line_and_keeps_the_memory_entry() {
        let mut recalled = recalled(vec![("first", "记忆正文")]);
        recalled.wiki_sections = vec![wiki_section("knowledge.md", "项目背景", "语料正文")];
        let session_id = "session-wiki-tight";
        let max_chars = max_chars_for_first_candidate(&recalled, session_id);
        let compiler = ClosedWorldContextCompiler::default();

        let overlay = compiler
            .compile(&recalled, session_id, max_chars)
            .expect("overlay");

        assert!(overlay.contains("[mem:first"), "{}", overlay);
        assert!(!overlay.contains("[wiki:"), "{}", overlay);
    }

    /// 命中节里的凭据同样要脱敏 (语料是用户写下的内容, 可能粘进密钥)。
    #[test]
    fn wiki_sections_are_sanitized() {
        let mut recalled = recalled(vec![("first", "记忆正文")]);
        recalled.wiki_sections = vec![wiki_section(
            "knowledge.md",
            "项目背景",
            "token=super_secret wikimarker",
        )];
        let compiler = ClosedWorldContextCompiler::default();

        let overlay = compiler
            .compile(&recalled, "session-wiki-secret", 4_000)
            .expect("overlay");

        assert!(!overlay.contains("super_secret"), "{}", overlay);
    }
}
