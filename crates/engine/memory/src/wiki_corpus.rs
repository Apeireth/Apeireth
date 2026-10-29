//! `apeireth-memory::wiki_corpus` — 用户知识库（Markdown 语料）的目录树检索面。
//!
//! # 为什么另起一个检索面
//!
//! `three_tier_vault` 的 `TreeReasoningRouter` 按空白分词打分, 中文整句会退化成
//! **一个** token, 命中不了中文标题; 而 TOC 节点的 `byte_range` 只覆盖标题行本身,
//! 要取「整节正文」必须自己推导区间。两者都不够用, 但 `three_tier_vault` 是别处的
//! 公共依赖, 所以这里自带一套路由 (CJK bigram 分词) 与区间推导, 不动它。
//!
//! # 行为边界
//!
//! - 语料根不存在 / 目录里没有 `.md` / 查询分词后为空 ⇒ 返回空上下文, 不报错、不 panic。
//! - 只读文件: 不写盘、不联网、不读环境变量。
//! - 输出完全确定: 同样输入两次 `build` 逐字相等 (排序键无随机、无哈希序依赖)。

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::three_tier_vault::{TocTreeIndexer, TocTreeNode};

/// 正文被 `max_section_chars` 截断时追加在尾部的省略标记。
pub const TRUNCATION_MARKER: &str = "…";

/// 语料检索的预算（全部有界，缺省即保守值）。
#[derive(Debug, Clone, Copy)]
pub struct WikiLimits {
    /// 最多取多少「节」。
    pub max_sections: usize,
    /// 单节最多多少字符（超出截断）。
    pub max_section_chars: usize,
    /// 最多扫描多少个 .md 文件。
    pub max_files: usize,
    /// 全部节的字符总预算。
    pub max_total_chars: usize,
}

impl Default for WikiLimits {
    fn default() -> Self {
        Self {
            max_sections: 4,
            max_section_chars: 1200,
            max_files: 200,
            max_total_chars: 4800,
        }
    }
}

/// 一个命中的「节」（= 一个标题下的整段正文）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WikiSection {
    /// 相对语料根的路径，用 `/` 分隔（出处展示用）。
    pub source_path: String,
    /// 标题文字（去掉 `#` 与空白）。
    pub heading: String,
    /// 该节正文（可能按 `max_section_chars` 截断）。
    ///
    /// **不含标题行** —— 标题另有 `heading` 字段, 含进来会让渲染出的证据行
    /// 把标题说两遍。
    pub text: String,
}

/// 一次检索的结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WikiContext {
    /// 命中的节，按「得分降序 → 文档顺序升序 → 路径字典序升序」排好。
    pub sections: Vec<WikiSection>,
}

/// 一个命中节在检索过程中的中间态。
struct SectionHit {
    /// 文档顺序序号（文件按相对路径排序，文件内按标题出现顺序）。
    doc_index: usize,
    /// 正文所在文件在 `contents` 中的下标。
    file_index: usize,
    /// 相对语料根的来源路径。
    source_path: String,
    /// 标题文字。
    heading: String,
    /// 路由得分。
    score: usize,
    /// 该节的字节区间。
    range: (usize, usize),
}

/// 打分后的候选节（还没决定取不取）。
struct ScoredSection {
    /// 文档顺序序号。
    doc_index: usize,
    /// 命中的标题节点。
    node: TocTreeNode,
    /// 该节的字节区间。
    range: (usize, usize),
    /// 路由得分。
    score: usize,
}

impl WikiContext {
    /// 扫描 root 下的 *.md（递归），按 query 路由出命中节。
    ///
    /// **根不存在 / 无 .md / query 为空 ⇒ 返回空上下文（零行为变化）**。
    pub fn build(root: &Path, query: &str, limits: WikiLimits) -> Self {
        let tokens = tokenize_query(query);
        if tokens.is_empty() {
            return Self::default();
        }

        let files = collect_markdown_files(root, limits.max_files);
        if files.is_empty() {
            return Self::default();
        }

        // 正文只对「命中的文件」保留: 未命中的文件读进来也只是白读一遍。
        let mut contents: Vec<String> = Vec::new();
        let mut hits: Vec<SectionHit> = Vec::new();
        let mut doc_index = 0usize;

        for (source_path, abs_path) in files {
            // 读不动的文件（权限 / 非 UTF-8）直接跳过, 不当作错误抛给调用方。
            let Ok(content) = std::fs::read_to_string(&abs_path) else {
                continue;
            };
            let tree = build_toc_tree(&abs_path, &content);
            let mut file_hits: Vec<ScoredSection> = Vec::new();
            for (node, range, own_range) in section_ranges_with_own_body(&tree) {
                // 标题信号（主）+ **正文信号（弱）**。
                // 正文信号的存在理由：**纯目录树路由只认标题** ⇒ 用户问
                // 「哲学门禁」而标题是「治理层的取舍」时**命中不了**
                // （2026-10-25 真机验证实测：0 命中）⇒ 加一条弱信号，
                // 让「知识库里正文提到过」也能被找到。仍**无向量、仍确定性**。
                // 正文信号只看**本节自有正文**（不含子节）⇒ 命中的是最具体的那节
                // （否则祖先节会因为正文更长而压过子节）。
                let score = score_node(&node, &tokens)
                    + body_score(&section_body(&content, own_range), &tokens);
                // 文档序号按**所有**节递增 (命中与否都占位), 这样并列时的
                // 「文档顺序升序」才是全文口径。
                let current_index = doc_index;
                doc_index += 1;
                if score > 0 {
                    file_hits.push(ScoredSection {
                        doc_index: current_index,
                        node,
                        range,
                        score,
                    });
                }
            }
            if file_hits.is_empty() {
                continue;
            }
            let file_index = contents.len();
            contents.push(content);
            for hit in file_hits {
                hits.push(SectionHit {
                    doc_index: hit.doc_index,
                    file_index,
                    source_path: source_path.clone(),
                    heading: hit.node.title.clone(),
                    score: hit.score,
                    range: hit.range,
                });
            }
        }

        if hits.is_empty() {
            return Self::default();
        }

        // 得分降序 → 文档顺序升序 → 路径字典序升序: 全序, 不依赖哈希序。
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.doc_index.cmp(&b.doc_index))
                .then_with(|| a.source_path.cmp(&b.source_path))
        });

        let mut sections: Vec<WikiSection> = Vec::new();
        let mut total_chars = 0usize;
        for hit in hits.iter().take(limits.max_sections) {
            let content = &contents[hit.file_index];
            // 正文**不含标题行** —— 标题另有 `heading` 字段，
            // 含进来会让渲染出的证据行把标题说两遍。
            let raw = section_body(content, hit.range);
            let text = truncate_chars(&raw, limits.max_section_chars);
            let text_chars = text.chars().count();
            // 累计超预算就停: 语料注入的上下文总量必须有硬上限。
            if total_chars + text_chars > limits.max_total_chars {
                break;
            }
            total_chars += text_chars;
            sections.push(WikiSection {
                source_path: hit.source_path.clone(),
                heading: hit.heading.clone(),
                text,
            });
        }

        Self { sections }
    }

    /// 是否没有命中任何节。
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// 全部命中节的字符总量（截断后的实际注入量）。
    pub fn total_chars(&self) -> usize {
        self.sections.iter().map(|s| s.text.chars().count()).sum()
    }
}

/// 查询分词：空白分隔的小写词 + CJK 相邻 bigram（连续 CJK 字符两两组合）。
///
/// ASCII 字母/数字/下划线组成的词保留；标点丢弃。确定性：按出现顺序去重。
///
/// 为什么要有 CJK bigram: 中文没有词边界, 按空白分词会把整句压成一个 token,
/// 命中不了标题。这里把**连续**的 CJK 字符两两组合, 让「记忆引擎」这类查询
/// 能落到标题「记忆引擎」上。单个孤立 CJK 字成不了 bigram, 按约定丢弃。
pub fn tokenize_query(query: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut ascii_word = String::new();
    let mut cjk_run: Vec<char> = Vec::new();

    for ch in query.chars() {
        if is_ascii_word_char(ch) {
            // ASCII 会打断 CJK 连续段, 所以先结算 CJK run。
            flush_cjk_run(&mut cjk_run, &mut tokens, &mut seen);
            ascii_word.push(ch);
        } else if is_cjk(ch) {
            flush_ascii_word(&mut ascii_word, &mut tokens, &mut seen);
            cjk_run.push(ch);
        } else {
            // 标点 / 空白: 丢弃, 同时结算两段缓冲。
            flush_ascii_word(&mut ascii_word, &mut tokens, &mut seen);
            flush_cjk_run(&mut cjk_run, &mut tokens, &mut seen);
        }
    }
    flush_ascii_word(&mut ascii_word, &mut tokens, &mut seen);
    flush_cjk_run(&mut cjk_run, &mut tokens, &mut seen);

    tokens
}

fn section_ranges_with_own_body(
    root: &TocTreeNode,
) -> Vec<(TocTreeNode, (usize, usize), (usize, usize))> {
    let nodes = flatten_headings(root);
    let file_end = root.byte_range.1;

    // 逆序扫一遍, 维护「每个层级往后第一个同级或更高级标题」的字节起点:
    // 标题只有 1..=6 级, 用定长数组就能把 O(n²) 的前瞻压成 O(6n)。
    let mut next_at_or_above = [file_end; 8];
    let mut ends: Vec<usize> = vec![file_end; nodes.len()];
    // 「往后第一个**更深**标题（level ≥ 2）」的字节起点。
    //
    // **为什么单独维护**：一级标题（`# 文档标题`）按「到下一个同级或更高级标题」
    // 算的话 = **到文件末尾** ⇒ 整篇文档变成一节 ⇒ 注入上下文时会把
    // **整篇**灌进去（2026-10-25 真机验证实测到的真问题）。
    // ⇒ 一级标题只拥有**它的前言**：到第一个更深标题（`##`）为止。
    //   没有更深标题时，仍按「下一个同级标题」收尾（纯 `#` 平铺文档照常工作）。
    let mut next_deeper_start = file_end;
    // 「本节自有正文」的结束位置：本节之后**第一个更深标题**的起点。
    let mut own_ends: Vec<usize> = vec![file_end; nodes.len()];
    for index in (0..nodes.len()).rev() {
        let node = &nodes[index];
        let level = node.level.clamp(1, 7);
        let start = node.byte_range.0;
        ends[index] = if level == 1 {
            next_at_or_above[1].min(next_deeper_start)
        } else {
            next_at_or_above[level]
        };
        own_ends[index] = next_deeper_start.min(ends[index]);
        for slot in level..=7 {
            if start < next_at_or_above[slot] {
                next_at_or_above[slot] = start;
            }
        }
        if level >= 2 {
            next_deeper_start = start;
        }
    }

    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let start = node.byte_range.0;
            let end = ends[index].max(start);
            let own_end = own_ends[index].max(start).min(end);
            ((*node).clone(), (start, end), (start, own_end))
        })
        .collect()
}

/// 前序展开标题节点（= 文档顺序：先父后子，兄弟按出现顺序），不含传入的根节点。
fn flatten_headings(root: &TocTreeNode) -> Vec<&TocTreeNode> {
    let mut out: Vec<&TocTreeNode> = Vec::new();
    // 显式栈做前序遍历: 孩子倒序入栈, 出栈顺序才是文档顺序。
    let mut stack: Vec<&TocTreeNode> = root.children.iter().rev().collect();
    while let Some(node) = stack.pop() {
        out.push(node);
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }
    out
}

/// 路由打分：标题命中 +3、summary/标题行命中 +1。
///
/// 只统计非 root 节点（root 是合成容器, 它的标题是文件名, 命中它没有出处意义）。
fn score_node(node: &TocTreeNode, tokens: &[String]) -> usize {
    let title = node.title.to_lowercase();
    let summary = node.summary.to_lowercase();
    let mut score = 0usize;
    for token in tokens {
        if title.contains(token.as_str()) {
            score += 3;
        }
        if summary.contains(token.as_str()) {
            score += 1;
        }
    }
    score
}

/// 正文命中分（**弱信号**，封顶 3）。
///
/// 每个查询词在正文里出现 ⇒ +1，**总贡献封顶 3** ⇒
/// 目录结构（标题 +3 / 标题行 +1）始终是主排序依据，正文只是「兜底可找」。
fn body_score(body: &str, tokens: &[String]) -> usize {
    let body = body.to_lowercase();
    let mut score = 0usize;
    for token in tokens {
        if body.contains(token.as_str()) {
            score += 1;
        }
    }
    score.min(3)
}

/// 取一节的**正文**（**不含标题行** —— 标题另有 `heading` 字段，
/// 且标题已在 `score_node` 里算过，不重复计分）。
fn section_body(content: &str, range: (usize, usize)) -> String {
    let raw = slice_range(content, range);
    match raw.split_once('\n') {
        Some((_heading, rest)) => rest.trim_start_matches('\n').to_string(),
        None => String::new(),
    }
}

/// 建 TOC 树, 并把标题行的字节起点修正成原文里的**真实**起点。
///
/// 为什么还要修正: `TocTreeIndexer::build_from_markdown` 用 `line.len() + 1` 累加偏移,
/// 这在 LF 行尾下正确, 但 CRLF 文件每行少算 1 字节, 偏移会逐行漂移; 末行无换行时
/// 还会越界。切原文必须落在真实字节边界上, 所以这里按实际行尾重扫一遍。
fn build_toc_tree(path: &Path, content: &str) -> TocTreeNode {
    let mut tree = TocTreeIndexer::build_from_markdown(path, content);
    let headings = scan_heading_lines(content);
    // 标题数与树节点数不一致时说明两边的判定口径出了偏差, 这时宁可用索引器
    // 自带的偏移 (LF 下正确), 也不用错位的偏移去切正文。
    if headings.len() == count_heading_nodes(&tree) {
        let mut cursor = 0usize;
        fixup_heading_offsets(&mut tree, &headings, &mut cursor);
    }
    tree
}

/// 扫描原文里所有 Markdown 标题行的真实字节起点与长度。
fn scan_heading_lines(content: &str) -> Vec<HeadingLine> {
    let mut out: Vec<HeadingLine> = Vec::new();
    let mut offset = 0usize;
    for raw_line in content.split_inclusive('\n') {
        if is_heading_line(raw_line) {
            out.push(HeadingLine {
                start: offset,
                len: raw_line.len(),
            });
        }
        offset += raw_line.len();
    }
    out
}

/// 一个标题行在原文中的位置。
struct HeadingLine {
    /// 行首第一个字节的偏移。
    start: usize,
    /// 该行（含行尾换行符）的字节长度。
    len: usize,
}

/// 是否是 Markdown 标题行（判定口径与 `TocTreeIndexer` 保持一致: 去掉首尾空白后
/// 以 `#` 开头, 且 `#` 的个数在 1..=6 之间）。
fn is_heading_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return false;
    }
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    (1..=6).contains(&hashes)
}

/// 统计树里的标题节点数（不含根节点）。
fn count_heading_nodes(node: &TocTreeNode) -> usize {
    node.children
        .iter()
        .map(|child| 1 + count_heading_nodes(child))
        .sum()
}

/// 前序（= 文档顺序）把真实偏移写回节点。
fn fixup_heading_offsets(node: &mut TocTreeNode, headings: &[HeadingLine], cursor: &mut usize) {
    for child in &mut node.children {
        if let Some(heading) = headings.get(*cursor) {
            child.byte_range = (heading.start, heading.start + heading.len);
            *cursor += 1;
        }
        fixup_heading_offsets(child, headings, cursor);
    }
}

/// 递归收集 root 下的 *.md（扩展名大小写不敏感），按相对路径字典序返回。
///
/// 目录软链不跟随（避免成环无限递归），指向普通文件的软链照收。
fn collect_markdown_files(root: &Path, max_files: usize) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    if max_files == 0 || !root.is_dir() {
        return out;
    }

    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut sub_dirs: Vec<PathBuf> = Vec::new();
        let mut files: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                sub_dirs.push(path);
            } else if file_type.is_file() {
                if is_markdown_path(&path) {
                    files.push(path);
                }
            } else if file_type.is_symlink() {
                match entry.metadata() {
                    Ok(meta) if meta.is_file() && is_markdown_path(&path) => files.push(path),
                    _ => {}
                }
            }
        }
        stack.extend(sub_dirs);
        for path in files {
            if let Some(source_path) = relative_source_path(&path, root) {
                out.push((source_path, path));
            }
        }
    }

    // 目录遍历顺序不稳定, 排序后才谈得上确定性。
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.truncate(max_files);
    out
}

/// 是不是 Markdown 文件（扩展名大小写不敏感）。
fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// 相对语料根的路径，统一用 `/` 分隔。
fn relative_source_path(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts.join("/").replace('\\', "/"))
}

/// 按字节区间切正文；切之前把两端收敛到字符边界，绝不切坏 UTF-8。
fn slice_range(content: &str, range: (usize, usize)) -> String {
    let len = content.len();
    let start = floor_char_boundary(content, range.0.min(len));
    let end = floor_char_boundary(content, range.1.min(len).max(start));
    content[start..end].to_string()
}

/// 把下标回退到最近的字符边界。
fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    if index > text.len() {
        index = text.len();
    }
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// 按**字符**数截断（不切坏 UTF-8），截断后追加省略标记。
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push_str(TRUNCATION_MARKER);
    out
}

/// ASCII 词字符（字母 / 数字 / 下划线）。
fn is_ascii_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// CJK 表意文字（扩展 A / 基本区 / 兼容区）。
fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

/// 结算 ASCII 词缓冲（小写化后入表，按出现顺序去重）。
fn flush_ascii_word(word: &mut String, tokens: &mut Vec<String>, seen: &mut HashSet<String>) {
    if word.is_empty() {
        return;
    }
    let lowered = word.to_lowercase();
    word.clear();
    if seen.insert(lowered.clone()) {
        tokens.push(lowered);
    }
}

/// 结算 CJK 连续段：相邻两字组成 bigram。
fn flush_cjk_run(run: &mut Vec<char>, tokens: &mut Vec<String>, seen: &mut HashSet<String>) {
    // **单字查询也要能用**：CJK bigram 对「只有一个字」的查询（"猫"）分不出任何
    // token ⇒ 查不到任何节（2026-10-25 真机验证时想到的边界）⇒
    // 单字成段时补一个 unigram；≥2 字仍只出 bigram（避免 token 爆炸）。
    if run.len() == 1 {
        let unigram: String = run.iter().collect();
        if seen.insert(unigram.clone()) {
            tokens.push(unigram);
        }
    }
    for window in run.windows(2) {
        let bigram: String = window.iter().collect();
        if seen.insert(bigram.clone()) {
            tokens.push(bigram);
        }
    }
    run.clear();
}
