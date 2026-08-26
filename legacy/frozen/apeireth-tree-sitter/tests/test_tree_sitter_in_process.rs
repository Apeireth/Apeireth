//! Fixture 5 + K-1 强校验: in-process tree-sitter 行为验证
//!
//! (per RIVAL 蓝图 §3.7 缺口 5 + 5 P0 / 9 skeleton / i18n / observability crate 共享 fixture 模式)
//!
//! 测 7 件事 (in-process, 不走 HTTP, 直接调 lib API):
//! 1. 8 编译期 hardcode 常量守门 (K-1 强校验 #2: 8 Language 枚举 + 8 常量值)
//! 2. `TOOL_WHITELIST` 编译期 hardcode 包含 8 tree-sitter 工具 (K-1 强校验 #3)
//! 3. `validate_tool_call` 接受白名单内 + 拒绝白名单外 (m3 防御)
//! 4. 8 Language 解析循环 (8 语言 highlight/parse/search/fold/indent 全部返 NotImplemented)
//! 5. NodeKind 推断 (function/class/variable/import/comment + 兜底)
//! 6. LSP dispatch (workspace/languages 是唯一真实输出, 其它 5 method 返 Internal error)
//! 7. 5 K-1 字样守门 (apeireth / tree_sitter / highlight / parse / must-do)
//!
//! 5 P0 crate + 9 skeleton + i18n + observability + tree-sitter 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_tree_sitter::{
    contains_k1_keyword, detect_indent, fold, highlight, lsp_dispatch, parse, search, validate_tool_call,
    HighlightKind, HighlightSpan, IndentStyle, Language, LspErrorBody, LspMessage, NodeKind, ParseOptions,
    ParseResult, SearchQuery, TreeSitterError, AST_MAX_DEPTH, FOLDING_DEFAULT_LEVEL, K1_KEYWORDS,
    K1_KEYWORDS_COUNT, MAX_FILE_SIZE_BYTES, PLATFORM_NAME, SEARCH_MAX_RESULTS,
    SUPPORTED_LANGUAGES, SUPPORTED_LANGUAGES_COUNT, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
    TREE_SITTER_SCHEMA_VERSION, HIGHLIGHT_MAX_TOKEN_LENGTH,
};

// ----- Fixture 1: 8 编译期 hardcode 常量守门 (K-1 强校验 #1 + #2) -----

#[test]
fn test_compile_time_constants_pinned() {
    // 8 编译期 hardcode 常量
    assert_eq!(
        TREE_SITTER_SCHEMA_VERSION, "1",
        "TREE_SITTER_SCHEMA_VERSION 编译期 hardcode = 1"
    );
    assert_eq!(PLATFORM_NAME, "apeireth", "K-1 强校验 #1: 平台名必须 apeireth");
    assert_eq!(
        SUPPORTED_LANGUAGES.len(),
        SUPPORTED_LANGUAGES_COUNT,
        "K-1 强校验 #2: SUPPORTED_LANGUAGES 长度 == SUPPORTED_LANGUAGES_COUNT"
    );
    assert_eq!(SUPPORTED_LANGUAGES_COUNT, 8, "K-1 强校验 #2: 8 Language 枚举");
    assert_eq!(
        MAX_FILE_SIZE_BYTES,
        10 * 1024 * 1024,
        "MAX_FILE_SIZE_BYTES 10 MB (10 * 1024 * 1024)"
    );
    assert_eq!(HIGHLIGHT_MAX_TOKEN_LENGTH, 1024, "HIGHLIGHT_MAX_TOKEN_LENGTH 1024");
    assert_eq!(AST_MAX_DEPTH, 64, "AST_MAX_DEPTH 64");
    assert_eq!(SEARCH_MAX_RESULTS, 1000, "SEARCH_MAX_RESULTS 1000");
    assert_eq!(FOLDING_DEFAULT_LEVEL, 2, "FOLDING_DEFAULT_LEVEL 2");

    // 8 Language 全部 hardcode 列出 (per K-1 强校验 #2)
    assert!(SUPPORTED_LANGUAGES.contains(&Language::Rust));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::TypeScript));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::JavaScript));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::Python));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::Go));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::Bash));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::Yaml));
    assert!(SUPPORTED_LANGUAGES.contains(&Language::Json));
}

#[test]
fn test_language_from_str_covers_8() {
    // 8 Language 全部能从 string 解析回
    let cases = [
        ("rust", Language::Rust),
        ("Rust", Language::Rust),
        ("RUST", Language::Rust),
        ("rs", Language::Rust),
        ("typescript", Language::TypeScript),
        ("ts", Language::TypeScript),
        ("javascript", Language::JavaScript),
        ("js", Language::JavaScript),
        ("python", Language::Python),
        ("py", Language::Python),
        ("go", Language::Go),
        ("golang", Language::Go),
        ("bash", Language::Bash),
        ("sh", Language::Bash),
        ("yaml", Language::Yaml),
        ("yml", Language::Yaml),
        ("json", Language::Json),
    ];
    for (s, expected) in cases {
        let back = Language::from_str(s).unwrap_or_else(|| panic!("from_str({s}) 失败"));
        assert_eq!(back, expected, "Language::from_str({s}) -> {expected:?}");
    }
    // 不支持的语言
    assert_eq!(Language::from_str("cobol"), None);
    assert_eq!(Language::from_str(""), None);
}

// ----- Fixture 2: TOOL_WHITELIST 8 项 (K-1 强校验 #3) -----

#[test]
fn test_whitelist_contains_eight_tree_sitter_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 8, "TOOL_WHITELIST 8 项");
    assert_eq!(TOOL_WHITELIST_COUNT, 8, "TOOL_WHITELIST_COUNT 编译期守门");
    for tool in [
        "apeireth_tree_sitter_highlight",
        "apeireth_tree_sitter_parse",
        "apeireth_tree_sitter_search",
        "apeireth_tree_sitter_fold",
        "apeireth_tree_sitter_indent",
        "apeireth_tree_sitter_lsp",
        "apeireth_tree_sitter_list_languages",
        "apeireth_tree_sitter_validate",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

// ----- Fixture 3: m3 防御 — 白名单校验 (per m3-hallucination-defense §2.4) -----

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    let args = serde_json::json!({});
    // m3 hallucination 经典: "apeireth_tree_sitter_format" 实际不存在
    let result = validate_tool_call("apeireth_tree_sitter_format", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        TreeSitterError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_tree_sitter_format");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

// ----- Fixture 4: 8 Language 解析循环 (5 核心 API 全部返 NotImplemented) -----

#[test]
fn test_eight_languages_skeleton_returns_not_implemented() {
    let source = "fn main() { println!(\"hi\"); }";
    for &lang in SUPPORTED_LANGUAGES {
        // §1 highlight
        let hl = highlight(source, lang);
        assert!(
            matches!(hl, Err(TreeSitterError::NotImplemented("highlight"))),
            "{lang:?} highlight 应返 NotImplemented: {hl:?}"
        );
        // §2 parse
        let ps = parse(source, &ParseOptions::new(lang));
        assert!(
            matches!(ps, Err(TreeSitterError::NotImplemented("parse"))),
            "{lang:?} parse 应返 NotImplemented: {ps:?}"
        );
        // §3 search
        let sr = search(source, lang, &SearchQuery::new());
        assert!(
            matches!(sr, Err(TreeSitterError::NotImplemented("search"))),
            "{lang:?} search 应返 NotImplemented: {sr:?}"
        );
        // §4 fold
        let fd = fold(source, lang, FOLDING_DEFAULT_LEVEL);
        assert!(
            matches!(fd, Err(TreeSitterError::NotImplemented("fold"))),
            "{lang:?} fold 应返 NotImplemented: {fd:?}"
        );
        // §5 indent
        let id = detect_indent(source, lang);
        assert!(
            matches!(id, Err(TreeSitterError::NotImplemented("indent"))),
            "{lang:?} indent 应返 NotImplemented: {id:?}"
        );
    }
}

#[test]
fn test_node_kind_inference_covers_5_categories() {
    // 5 NodeKind 类别 + Other 兜底
    assert_eq!(NodeKind::from_type_str("function_item"), NodeKind::Function);
    assert_eq!(NodeKind::from_type_str("function_declaration"), NodeKind::Function);
    assert_eq!(NodeKind::from_type_str("method_declaration"), NodeKind::Function);
    assert_eq!(NodeKind::from_type_str("fn"), NodeKind::Function);
    assert_eq!(NodeKind::from_type_str("class_definition"), NodeKind::Class);
    assert_eq!(NodeKind::from_type_str("struct_item"), NodeKind::Class);
    assert_eq!(NodeKind::from_type_str("impl_item"), NodeKind::Class);
    assert_eq!(NodeKind::from_type_str("interface_declaration"), NodeKind::Class);
    assert_eq!(NodeKind::from_type_str("let_declaration"), NodeKind::Variable);
    assert_eq!(NodeKind::from_type_str("variable_declaration"), NodeKind::Variable);
    assert_eq!(NodeKind::from_type_str("const_item"), NodeKind::Variable);
    assert_eq!(NodeKind::from_type_str("use_declaration"), NodeKind::Import);
    assert_eq!(NodeKind::from_type_str("import_statement"), NodeKind::Import);
    assert_eq!(NodeKind::from_type_str("line_comment"), NodeKind::Comment);
    assert_eq!(NodeKind::from_type_str("block_comment"), NodeKind::Comment);
    // 兜底
    assert_eq!(
        NodeKind::from_type_str("expression_statement"),
        NodeKind::Other("expression_statement".to_string())
    );
    assert_eq!(
        NodeKind::from_type_str("identifier"),
        NodeKind::Other("identifier".to_string())
    );
}

// ----- Fixture 5: LSP dispatch + 5 K-1 字样 -----

#[test]
fn test_lsp_dispatch_workspace_languages_returns_8() {
    let msg = LspMessage::Languages { id: 1 };
    let response = lsp_dispatch(msg).unwrap();
    assert_eq!(response.id, 1);
    let result = response.result.expect("workspace/languages 应有 result");
    let langs = result.get("languages").expect("languages 字段");
    let arr = langs.as_array().expect("languages 应为数组");
    assert_eq!(arr.len(), 8, "workspace/languages 应返 8 Language");
}

#[test]
fn test_lsp_dispatch_highlight_returns_internal_error() {
    use apeireth_tree_sitter::lsp::HighlightParams;
    let msg = LspMessage::Highlight {
        id: 42,
        params: HighlightParams {
            text: "fn main() {}".to_string(),
            language: Language::Rust,
        },
    };
    let response = lsp_dispatch(msg).unwrap();
    assert_eq!(response.id, 42);
    assert!(response.result.is_none());
    let err = response.error.expect("skeleton 阶段 highlight 应有 error");
    assert_eq!(err.code, -32603, "Internal error code per LSP 3.17");
    assert!(err.message.contains("skeleton"));
}

#[test]
fn test_lsp_error_body_method_not_found_uses_lsp_code() {
    // per LSP 3.17 §ErrorCodes, -32601 = Method not found
    let err = LspErrorBody::method_not_found("textDocument/ast");
    assert_eq!(err.code, -32601);
    assert!(err.message.contains("textDocument/ast"));
}

#[test]
fn test_indent_default_for_languages() {
    // 8 语言 per language 默认缩进
    assert_eq!(IndentStyle::default_for(Language::Rust), IndentStyle::Space { size: 4 });
    assert_eq!(IndentStyle::default_for(Language::Python), IndentStyle::Space { size: 4 });
    assert_eq!(IndentStyle::default_for(Language::Go), IndentStyle::Tab);
    assert_eq!(IndentStyle::default_for(Language::Bash), IndentStyle::Space { size: 2 });
    assert_eq!(IndentStyle::default_for(Language::Yaml), IndentStyle::Space { size: 2 });
    assert_eq!(IndentStyle::default_for(Language::Json), IndentStyle::Space { size: 2 });
    assert_eq!(
        IndentStyle::default_for(Language::TypeScript),
        IndentStyle::Space { size: 2 }
    );
    assert_eq!(
        IndentStyle::default_for(Language::JavaScript),
        IndentStyle::Space { size: 2 }
    );
}

#[test]
fn test_highlight_span_length_tracking() {
    let span = HighlightSpan::new(0, 100, HighlightKind::String);
    assert_eq!(span.len(), 100);
    assert!(!span.is_empty());

    let empty = HighlightSpan::new(5, 5, HighlightKind::Comment);
    assert!(empty.is_empty(), "start >= end 视为空");
}

#[test]
fn test_parse_result_empty_helper() {
    let r = ParseResult::empty(Language::Python);
    assert_eq!(r.language, Language::Python);
    assert_eq!(r.elapsed_ms, 0);
    assert!(r.highlights.is_empty());
    assert!(r.ast_nodes.is_empty());
    assert!(r.fold_ranges.is_empty());
    assert!(r.indent.is_none());
}

#[test]
fn test_k1_must_do_invariants() {
    // 5 K-1 字样 (per 任务 K-1 强校验 #4):
    // 1) "apeireth" 平台名
    assert_eq!(PLATFORM_NAME, "apeireth", "K-1 字样 #1: apeireth");
    // 2) "tree_sitter" crate 名
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(
        crate_name.contains("tree-sitter") || crate_name.contains("tree_sitter"),
        "K-1 字样 #2: tree_sitter in crate name ({crate_name})"
    );
    // 3) "highlight" (TOOL_WHITELIST 守门)
    assert!(
        TOOL_WHITELIST.contains(&"apeireth_tree_sitter_highlight"),
        "K-1 字样 #3: highlight (in WHITELIST)"
    );
    // 4) "parse" (TOOL_WHITELIST 守门)
    assert!(
        TOOL_WHITELIST.contains(&"apeireth_tree_sitter_parse"),
        "K-1 字样 #4: parse (in WHITELIST)"
    );
    // 5) "must-do" (K-1 字样守门 helper 守门)
    assert!(
        K1_KEYWORDS.contains(&"must-do"),
        "K-1 字样 #5: must-do (in K1_KEYWORDS)"
    );
    assert_eq!(K1_KEYWORDS.len(), 5, "K1_KEYWORDS 5 个");
    assert_eq!(K1_KEYWORDS_COUNT, 5, "K1_KEYWORDS_COUNT 编译期守门");

    // contains_k1_keyword helper 守门
    assert!(contains_k1_keyword("apeireth-tree-sitter"));
    assert!(contains_k1_keyword("apeireth_tree_sitter_highlight"));
    assert!(contains_k1_keyword("must-do skeleton"));
    assert!(!contains_k1_keyword("hello world"), "无关字符串应返 false");
}

#[test]
fn test_highlight_kind_scope_strings_12_categories() {
    // 12 HighlightKind 都有非空 scope
    let kinds = [
        HighlightKind::Keyword,
        HighlightKind::Function,
        HighlightKind::Type,
        HighlightKind::String,
        HighlightKind::Number,
        HighlightKind::Comment,
        HighlightKind::Operator,
        HighlightKind::Variable,
        HighlightKind::Constant,
        HighlightKind::Import,
        HighlightKind::Punctuation,
        HighlightKind::Other,
    ];
    assert_eq!(kinds.len(), 12, "HighlightKind 12 类");
    for k in kinds {
        assert!(!k.scope().is_empty(), "HighlightKind {k:?} scope 不能为空");
    }
}

#[test]
fn test_fold_level_range_1_to_5() {
    // level 0 / 6 越界
    let r0 = fold("x", Language::Rust, 0);
    assert!(matches!(r0, Err(TreeSitterError::FoldingLevelOutOfRange(0))));
    let r6 = fold("x", Language::Rust, 6);
    assert!(matches!(r6, Err(TreeSitterError::FoldingLevelOutOfRange(6))));
    // level 1-5 都走通到 NotImplemented
    for level in 1..=5u32 {
        let r = fold("x", Language::Rust, level);
        assert!(matches!(r, Err(TreeSitterError::NotImplemented("fold"))));
    }
}

#[test]
fn test_search_max_results_limit() {
    let q = SearchQuery::new().with_max_results(SEARCH_MAX_RESULTS + 1);
    let r = search("x", Language::Rust, &q);
    assert!(matches!(r, Err(TreeSitterError::SearchResultLimitExceeded { .. })));
}
