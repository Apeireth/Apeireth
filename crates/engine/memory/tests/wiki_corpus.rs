//! 用户知识库 (Markdown 语料) 的目录树检索面集成测试。
//!
//! 钉住几件事:
//! 1. **零行为变化**: 语料根不存在 / 目录里没有 `.md` / 查询分词后为空 ⇒ 空上下文, 不 panic。
//! 2. **节区间正确**: 一节的正文是该标题行到下一个同级/更高级标题之间的内容。
//! 3. **中文能命中**: 查询不是整句当一个 token, 中文标题能被路由到。
//! 4. **截断不切坏 UTF-8**, 且各档预算 (节数 / 文件数 / 总字符) 真的生效。
//! 5. **完全确定**: 同一输入两次 `build` 逐字相等。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use apeireth_memory::wiki_corpus::{tokenize_query, WikiContext, WikiLimits, TRUNCATION_MARKER};

/// 临时语料目录: 进程 id + 计数器保证唯一, 退出时清理。
struct TempCorpus {
    path: PathBuf,
}

impl TempCorpus {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "apeireth-wiki-corpus-{}-{tag}-{seq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("创建临时语料目录");
        Self { path }
    }

    fn write(&self, relative: &str, content: &str) {
        let target = self.path.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("创建临时子目录");
        }
        std::fs::write(&target, content).expect("写入临时语料文件");
    }

    fn root(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempCorpus {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn missing_corpus_root_yields_empty_context_without_panic() {
    let missing = std::env::temp_dir().join(format!(
        "apeireth-wiki-corpus-absent-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&missing);

    let ctx = WikiContext::build(&missing, "记忆引擎", WikiLimits::default());
    assert!(ctx.is_empty(), "语料根不存在时必须是空上下文");
    assert_eq!(ctx.total_chars(), 0, "空上下文的字符总量必须是 0");
    assert!(ctx.sections.is_empty());
}

#[test]
fn corpus_without_markdown_yields_empty_context() {
    let corpus = TempCorpus::new("no-md");
    corpus.write("notes.txt", "记忆引擎其实是一段普通文本。");
    corpus.write("readme", "# 不是扩展名 md 的文件\n");

    let ctx = WikiContext::build(corpus.root(), "记忆引擎", WikiLimits::default());
    assert!(ctx.is_empty(), "目录里没有 .md 时必须是空上下文");
}

#[test]
fn blank_query_yields_empty_context() {
    let corpus = TempCorpus::new("blank-query");
    corpus.write("a.md", "# 记忆引擎\n正文。\n");

    for query in ["", "   ", "\t\n", "!!!", "——"] {
        let ctx = WikiContext::build(corpus.root(), query, WikiLimits::default());
        assert!(ctx.is_empty(), "查询 {query:?} 分词后为空时必须是空上下文");
    }
}

#[test]
fn section_text_stops_before_next_sibling_heading() {
    let corpus = TempCorpus::new("section-span");
    corpus.write(
        "doc.md",
        "# 顶层标题\n顶层正文。\n\
         ## 甲节\n甲节正文。\n\
         ## 乙节\n乙节正文。\n",
    );

    let ctx = WikiContext::build(corpus.root(), "甲节", WikiLimits::default());
    assert_eq!(ctx.sections.len(), 1, "只应命中甲节一节");
    let section = &ctx.sections[0];
    assert_eq!(section.heading, "甲节", "标题应去掉 # 与空白");
    assert_eq!(section.source_path, "doc.md");
    assert!(
        section.text.contains("甲节正文"),
        "正文应包含本节内容: {:?}",
        section.text
    );
    assert!(
        !section.text.contains("乙节"),
        "正文不应越过下一个同级标题: {:?}",
        section.text
    );
    assert!(
        !section.text.contains("顶层正文"),
        "正文不应回溯到上一级标题的内容: {:?}",
        section.text
    );
}

#[test]
fn parent_section_covers_its_children_until_next_sibling() {
    let corpus = TempCorpus::new("parent-span");
    corpus.write(
        "doc.md",
        "# 甲章\n甲章正文。\n\
         ## 甲节一\n甲节一正文。\n\
         ## 甲节二\n甲节二正文。\n\
         # 乙章\n乙章正文。\n",
    );

    let ctx = WikiContext::build(corpus.root(), "甲章", WikiLimits::default());
    assert_eq!(ctx.sections.len(), 1);
    let text = &ctx.sections[0].text;
    assert!(text.contains("甲章正文"));
    assert!(text.contains("甲节一正文"), "父节应包含子节正文: {text:?}");
    assert!(
        !text.contains("乙章"),
        "父节应在下一个同级标题处收尾: {text:?}"
    );
}

#[test]
fn chinese_query_matches_chinese_heading() {
    let corpus = TempCorpus::new("chinese");
    corpus.write(
        "engine.md",
        "# 记忆引擎\n记忆引擎负责短期记忆的读写。\n\
         # 向量索引\n向量索引负责相似度检索。\n",
    );

    // 核心: 查询整句若被当成一个 token 就绝无可能命中中文标题。
    let ctx = WikiContext::build(
        corpus.root(),
        "我想了解记忆引擎是怎么实现的",
        WikiLimits::default(),
    );
    assert_eq!(ctx.sections.len(), 1, "中文查询应命中中文标题");
    assert_eq!(ctx.sections[0].heading, "记忆引擎");
    assert_eq!(ctx.sections[0].source_path, "engine.md");
    assert!(ctx.sections[0].text.contains("短期记忆"));
}

#[test]
fn english_query_splits_on_whitespace_and_matches() {
    let corpus = TempCorpus::new("english");
    corpus.write(
        "arch.md",
        "# Memory Engine\nCovers working memory.\n\
         # Governance Guardrails\nCovers OWASP security.\n",
    );

    let ctx = WikiContext::build(corpus.root(), "memory engine", WikiLimits::default());
    assert_eq!(ctx.sections.len(), 1);
    assert_eq!(ctx.sections[0].heading, "Memory Engine");
    assert!(ctx.sections[0].text.contains("working memory"));

    // 单个词也应命中标题里出现的那个词。
    let single = WikiContext::build(corpus.root(), "governance", WikiLimits::default());
    assert_eq!(single.sections.len(), 1);
    assert_eq!(single.sections[0].heading, "Governance Guardrails");
}

#[test]
fn section_text_is_truncated_on_char_boundary() {
    let corpus = TempCorpus::new("truncate");
    let body = "记忆引擎".repeat(40);
    let original = format!("# 记忆引擎\n{body}\n");
    corpus.write("doc.md", &original);

    let limits = WikiLimits {
        max_section_chars: 10,
        ..WikiLimits::default()
    };
    let ctx = WikiContext::build(corpus.root(), "记忆引擎", limits);
    assert_eq!(ctx.sections.len(), 1);
    let text = &ctx.sections[0].text;
    assert!(
        text.chars().count() <= limits.max_section_chars + TRUNCATION_MARKER.chars().count(),
        "截断后字符数必须受预算约束: {}",
        text.chars().count()
    );
    assert!(
        text.ends_with(TRUNCATION_MARKER),
        "截断后应标注省略: {text:?}"
    );
    assert!(
        text.starts_with("# 记忆引擎\n"),
        "截断应发生在字符边界上, 不得切坏 UTF-8: {text:?}"
    );
    // 省略标记之前的字符必须与原文前缀逐字相同 (没有半个汉字)。
    let kept: String = text.chars().take(limits.max_section_chars).collect();
    assert!(original.starts_with(&kept), "截断前缀必须是原文的逐字前缀");
}

#[test]
fn build_is_deterministic_across_runs() {
    let corpus = TempCorpus::new("determinism");
    corpus.write(
        "b.md",
        "# 记忆引擎\n第二份文件。\n\
         ## 细节说明\n细节正文。\n",
    );
    corpus.write(
        "a.md",
        "# 记忆引擎\n第一份文件。\n\
         ## 细节说明\n细节正文。\n",
    );

    let first = WikiContext::build(corpus.root(), "记忆引擎", WikiLimits::default());
    let second = WikiContext::build(corpus.root(), "记忆引擎", WikiLimits::default());
    assert_eq!(first, second, "同一输入两次 build 必须逐字相等");
    assert_eq!(first.sections.len(), 2);
    assert_eq!(
        first.sections[0].source_path, "a.md",
        "同分时按路径字典序: {:?}",
        first.sections
    );
}

#[test]
fn max_sections_budget_is_enforced() {
    let corpus = TempCorpus::new("max-sections");
    corpus.write(
        "doc.md",
        "# 记忆一\n正文一。\n\
         # 记忆二\n正文二。\n\
         # 记忆三\n正文三。\n",
    );

    let limits = WikiLimits {
        max_sections: 2,
        ..WikiLimits::default()
    };
    let ctx = WikiContext::build(corpus.root(), "记忆", limits);
    assert_eq!(ctx.sections.len(), 2, "节数预算必须生效");
    assert_eq!(ctx.sections[0].heading, "记忆一");
    assert_eq!(ctx.sections[1].heading, "记忆二");
}

#[test]
fn max_files_budget_is_enforced() {
    let corpus = TempCorpus::new("max-files");
    corpus.write("a.md", "# 记忆引擎\nA 正文。\n");
    corpus.write("b.md", "# 记忆引擎\nB 正文。\n");
    corpus.write("c.md", "# 记忆引擎\nC 正文。\n");

    let limits = WikiLimits {
        max_files: 2,
        ..WikiLimits::default()
    };
    let ctx = WikiContext::build(corpus.root(), "记忆引擎", limits);
    assert_eq!(ctx.sections.len(), 2, "文件数预算必须生效");
    let paths: Vec<&str> = ctx
        .sections
        .iter()
        .map(|s| s.source_path.as_str())
        .collect();
    assert_eq!(paths, vec!["a.md", "b.md"], "只扫描字典序靠前的文件");
}

#[test]
fn max_total_chars_budget_is_enforced() {
    let corpus = TempCorpus::new("max-total");
    corpus.write(
        "doc.md",
        "# 记忆一\n记忆一正文内容。\n\
         # 记忆二\n记忆二正文内容。\n\
         # 记忆三\n记忆三正文内容。\n",
    );

    let limits = WikiLimits {
        max_total_chars: 30,
        ..WikiLimits::default()
    };
    let ctx = WikiContext::build(corpus.root(), "记忆", limits);
    assert!(
        ctx.total_chars() <= limits.max_total_chars,
        "总字符预算必须生效: {}",
        ctx.total_chars()
    );
    assert!(ctx.sections.len() < 3, "超预算后必须停止取节");
}

#[test]
fn source_path_is_root_relative_with_forward_slashes() {
    let corpus = TempCorpus::new("source-path");
    corpus.write("notes/sub/deep.md", "# 记忆引擎\n深层内容。\n");
    corpus.write("top.md", "# 其它\n别的内容。\n");

    let ctx = WikiContext::build(corpus.root(), "记忆引擎", WikiLimits::default());
    assert_eq!(ctx.sections.len(), 1);
    assert_eq!(
        ctx.sections[0].source_path, "notes/sub/deep.md",
        "来源路径必须相对语料根且用 / 分隔"
    );
    assert!(!ctx.sections[0].source_path.contains('\\'));
}

#[test]
fn markdown_extension_match_is_case_insensitive() {
    let corpus = TempCorpus::new("case-ext");
    corpus.write("upper.MD", "# 记忆引擎\n大写扩展名。\n");

    let ctx = WikiContext::build(corpus.root(), "记忆引擎", WikiLimits::default());
    assert_eq!(ctx.sections.len(), 1, "扩展名大小写不敏感");
    assert_eq!(ctx.sections[0].source_path, "upper.MD");
}

#[test]
fn tokenize_query_splits_cjk_bigrams_and_dedups() {
    assert_eq!(
        tokenize_query("Memory Engine"),
        vec!["memory".to_string(), "engine".to_string()],
        "英文按空白分词并小写"
    );
    assert_eq!(
        tokenize_query("记忆引擎"),
        vec!["记忆".to_string(), "忆引".to_string(), "引擎".to_string()],
        "连续 CJK 字符两两组成 bigram"
    );
    assert_eq!(
        tokenize_query("记忆引擎, 记忆引擎!"),
        vec!["记忆".to_string(), "忆引".to_string(), "引擎".to_string()],
        "标点丢弃、重复按出现顺序去重"
    );
    assert_eq!(
        tokenize_query("rust_api"),
        vec!["rust_api".to_string()],
        "下划线属于词字符"
    );
    assert!(tokenize_query("   ").is_empty(), "空白查询没有 token");
}

#[test]
fn crlf_document_sections_start_at_real_line_boundaries() {
    let corpus = TempCorpus::new("crlf");
    // 前言行让标题不再落在第 0 个字节: 偏移一旦按 LF 估算就会整体漂移。
    corpus.write(
        "doc.md",
        "前言。\r\n# 记忆引擎\r\n记忆引擎正文。\r\n# 其它\r\n其它正文。\r\n",
    );

    let ctx = WikiContext::build(corpus.root(), "记忆引擎", WikiLimits::default());
    assert_eq!(ctx.sections.len(), 1);
    let text = &ctx.sections[0].text;
    assert!(
        text.starts_with("# 记忆引擎\r\n记忆引擎正文。"),
        "CRLF 文件的节起点必须落在真实行边界: {text:?}"
    );
    assert!(!text.contains("其它"), "不得越过下一个同级标题: {text:?}");
}
