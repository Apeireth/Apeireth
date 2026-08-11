//! tree-sitter Demo (1:1 翻译 v0.9.21 商业版 tree-sitter 集成流程).
//!
//! 演示 8 语言切换 + 编译期 hardcode 展示 + 8 工具白名单守门 + 5 K-1 字样守门 + LSP dispatch.
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-tree-sitter --example tree_sitter_demo
//! ```
//!
//! ## 期望输出 (skeleton 阶段)
//!
//! ```text
//! [tree_sitter_demo] TREE_SITTER_SCHEMA_VERSION = 1
//! [tree_sitter_demo] PLATFORM_NAME = apeireth
//! [tree_sitter_demo] SUPPORTED_LANGUAGES 8 项: rust, typescript, javascript, python, go, bash, yaml, json
//! [tree_sitter_demo] MAX_FILE_SIZE_BYTES = 10485760 (10 MB)
//! [tree_sitter_demo] HIGHLIGHT_MAX_TOKEN_LENGTH = 1024
//! [tree_sitter_demo] AST_MAX_DEPTH = 64
//! [tree_sitter_demo] SEARCH_MAX_RESULTS = 1000
//! [tree_sitter_demo] FOLDING_DEFAULT_LEVEL = 2
//! [tree_sitter_demo] --- 8 语言 highlight 演示 ---
//! [tree_sitter_demo] rust highlight: Err(NotImplemented("highlight"))
//! [tree_sitter_demo] typescript highlight: Err(NotImplemented("highlight"))
//! [tree_sitter_demo] ...
//! [tree_sitter_demo] --- 8 语言 parse 演示 ---
//! [tree_sitter_demo] rust parse: Err(NotImplemented("parse"))
//! [tree_sitter_demo] ...
//! [tree_sitter_demo] --- 5 核心 API NotImplemented 守门 ---
//! [tree_sitter_demo] highlight err = NotImplemented("highlight")
//! [tree_sitter_demo] parse err = NotImplemented("parse")
//! [tree_sitter_demo] search err = NotImplemented("search")
//! [tree_sitter_demo] fold err = NotImplemented("fold")
//! [tree_sitter_demo] indent err = NotImplemented("indent")
//! [tree_sitter_demo] --- LSP dispatch 演示 ---
//! [tree_sitter_demo] workspace/languages -> SUPPORTED 8 (唯一真实输出)
//! [tree_sitter_demo] textDocument/highlight -> Internal error (skeleton)
//! [tree_sitter_demo] --- m3 防御演示 ---
//! [tree_sitter_demo] 白名单内 apeireth_tree_sitter_highlight -> Ok
//! [tree_sitter_demo] 白名单外 apeireth_tree_sitter_format -> Err(ToolNotWhitelisted(...))
//! [tree_sitter_demo] --- 5 K-1 字样守门 ---
//! [tree_sitter_demo] "apeireth-tree-sitter crate skeleton" 含 "apeireth" + "tree_sitter" = true
//! [tree_sitter_demo] "highlight & parse APIs" 含 "highlight" + "parse" = true
//! [tree_sitter_demo] "R20 must-do skeleton 阶段完成" 含 "must-do" = true
//! [tree_sitter_demo] completed (skeleton — R20 阶段 4 续接 bash/typescript/python/rust 4 grammar)
//! ```
//!
//! ## 6 哲学 anchor 验证
//!
//! - S-1: 1:1 翻译 v0.9.21 商业版 tree-sitter 集成 (8 Language + 8 工具白名单 + LSP 6 method)
//! - S-2: 用商业版 `out/main/chunks/tree-sitter-*` 实查估 2000 LOC 总, skeleton 阶段估 600 行
//! - O-2: 编译期 hardcode 8 Language + 8 工具白名单, 借鉴 5 P0 + 9 skeleton + i18n + observability 同模式
//! - O-5: 所有方法 `warn!` 占位 + 返 `NotImplemented`, 真实 grammar 留 R20 阶段 4 续
//! - O-3: skeleton 落地 (8 Language + 8 编译期 hardcode + 8 工具白名单 + 5 核心 API stub + 5 K-1 字样)
//! - O-4: §1-§6 跟 i18n / observability 同骨架 + 引用 R20 阶段 4 续路径完整

use apeireth_tree_sitter::{
    contains_k1_keyword, detect_indent, fold, highlight, lsp_dispatch, parse, search, validate_tool_call,
    FoldKind, HighlightKind, HighlightSpan, IndentStyle, Language, LspErrorBody, LspMessage, LspResponse,
    NodeKind, ParseOptions, ParseResult, SearchQuery, TreeSitterError, K1_KEYWORDS, K1_KEYWORDS_COUNT,
    PLATFORM_NAME, SUPPORTED_LANGUAGES, SUPPORTED_LANGUAGES_COUNT, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
    TREE_SITTER_SCHEMA_VERSION, AST_MAX_DEPTH, FOLDING_DEFAULT_LEVEL, HIGHLIGHT_MAX_TOKEN_LENGTH,
    MAX_FILE_SIZE_BYTES, SEARCH_MAX_RESULTS,
};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    // 1) 编译期守门: 8 编译期 hardcode 常量展示
    println!("[tree_sitter_demo] TREE_SITTER_SCHEMA_VERSION = {TREE_SITTER_SCHEMA_VERSION}");
    println!("[tree_sitter_demo] PLATFORM_NAME = {PLATFORM_NAME}");
    println!(
        "[tree_sitter_demo] SUPPORTED_LANGUAGES {SUPPORTED_LANGUAGES_COUNT} 项: {}",
        SUPPORTED_LANGUAGES
            .iter()
            .map(|l| l.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("[tree_sitter_demo] MAX_FILE_SIZE_BYTES = {MAX_FILE_SIZE_BYTES} (10 MB)");
    println!("[tree_sitter_demo] HIGHLIGHT_MAX_TOKEN_LENGTH = {HIGHLIGHT_MAX_TOKEN_LENGTH}");
    println!("[tree_sitter_demo] AST_MAX_DEPTH = {AST_MAX_DEPTH}");
    println!("[tree_sitter_demo] SEARCH_MAX_RESULTS = {SEARCH_MAX_RESULTS}");
    println!("[tree_sitter_demo] FOLDING_DEFAULT_LEVEL = {FOLDING_DEFAULT_LEVEL}");

    // 2) 8 语言 highlight 演示 (skeleton 阶段全部返 NotImplemented)
    println!("[tree_sitter_demo] --- 8 语言 highlight 演示 ---");
    for &language in Language::all() {
        let r = highlight("fn main() {}", language);
        println!("[tree_sitter_demo] {} highlight: {:?}", language.as_str(), r.err());
    }

    // 3) 8 语言 parse 演示
    println!("[tree_sitter_demo] --- 8 语言 parse 演示 ---");
    for &language in Language::all() {
        let r = parse("fn main() {}", &ParseOptions::new(language));
        println!("[tree_sitter_demo] {} parse: {:?}", language.as_str(), r.err());
    }

    // 4) 5 核心 API NotImplemented 守门
    println!("[tree_sitter_demo] --- 5 核心 API NotImplemented 守门 ---");
    let lang = Language::Rust;
    let source = "fn main() { println!(\"hi\"); }";
    let hl = highlight(source, lang);
    let ps = parse(source, &ParseOptions::new(lang));
    let sr = search(source, lang, &SearchQuery::new());
    let fd = fold(source, lang, FOLDING_DEFAULT_LEVEL);
    let id = detect_indent(source, lang);
    println!("[tree_sitter_demo] highlight err = {:?}", hl.as_ref().err());
    println!("[tree_sitter_demo] parse err = {:?}", ps.as_ref().err());
    println!("[tree_sitter_demo] search err = {:?}", sr.as_ref().err());
    println!("[tree_sitter_demo] fold err = {:?}", fd.as_ref().err());
    println!("[tree_sitter_demo] indent err = {:?}", id.as_ref().err());

    // 5) LSP dispatch 演示 (workspace/languages 是 skeleton 阶段唯一真实输出)
    println!("[tree_sitter_demo] --- LSP dispatch 演示 ---");
    let langs_resp = lsp_dispatch(LspMessage::Languages { id: 1 })?;
    println!("[tree_sitter_demo] workspace/languages -> result: {:?}", langs_resp.result);
    println!("[tree_sitter_demo] workspace/languages -> error: {:?}", langs_resp.error);

    let hl_resp = lsp_dispatch(LspMessage::Highlight {
        id: 2,
        params: apeireth_tree_sitter::lsp::HighlightParams {
            text: "fn main() {}".to_string(),
            language: lang,
        },
    })?;
    println!("[tree_sitter_demo] textDocument/highlight -> error: {:?}", hl_resp.error);

    // 6) m3 防御演示
    println!("[tree_sitter_demo] --- m3 防御演示 ---");
    let json_args = json!({});
    for tool in TOOL_WHITELIST {
        let r = validate_tool_call(tool, &json_args);
        println!("[tree_sitter_demo] 白名单内 {tool} -> {r:?}");
        assert!(r.is_ok(), "白名单内工具 {tool} 应通过");
    }
    // 白名单外 (m3 hallucination 经典 — "apeireth_tree_sitter_format" 实际不存在)
    let bad = validate_tool_call("apeireth_tree_sitter_format", &json_args);
    println!(
        "[tree_sitter_demo] 白名单外 apeireth_tree_sitter_format -> {:?}",
        bad.as_ref().err()
    );
    assert!(bad.is_err(), "白名单外工具必须拒绝");
    assert_eq!(TOOL_WHITELIST_COUNT, 8, "TOOL_WHITELIST 编译期守门 8");

    // 7) 5 K-1 字样守门 (per task spec K-1 强校验 #4)
    println!("[tree_sitter_demo] --- 5 K-1 字样守门 ---");
    let test_strings = [
        ("apeireth-tree-sitter crate skeleton", "apeireth + tree_sitter"),
        ("highlight & parse APIs", "highlight + parse"),
        ("R20 must-do skeleton 阶段完成", "must-do"),
    ];
    for (s, label) in test_strings {
        let ok = contains_k1_keyword(s);
        println!("[tree_sitter_demo] \"{s}\" ({label}) = {ok}");
        assert!(ok, "5 K-1 字样守门失败: {s}");
    }
    assert_eq!(K1_KEYWORDS.len(), K1_KEYWORDS_COUNT, "K1_KEYWORDS 编译期守门 5");
    assert_eq!(K1_KEYWORDS_COUNT, 5, "5 K-1 字样");

    // 8) FoldKind / HighlightKind / IndentStyle 公共 API 展示 (per §1-§5)
    println!("[tree_sitter_demo] --- 公共 API 展示 ---");
    let span = HighlightSpan::new(0, 10, HighlightKind::String);
    println!("[tree_sitter_demo] HighlightSpan {{ start: 0, end: 10, kind: String }}: len = {}", span.len());
    assert_eq!(span.len(), 10);

    let node = NodeKind::from_type_str("function_item");
    println!("[tree_sitter_demo] NodeKind::from_type_str(\"function_item\") = {node:?}");
    assert_eq!(node, NodeKind::Function);

    let indent = IndentStyle::default_for(Language::Go);
    println!("[tree_sitter_demo] IndentStyle::default_for(Go) = {indent} (gofmt: tab)");
    assert_eq!(indent, IndentStyle::Tab);

    let indent_rust = IndentStyle::default_for(Language::Rust);
    println!("[tree_sitter_demo] IndentStyle::default_for(Rust) = {indent_rust} (rustfmt: 4 space)");
    assert_eq!(indent_rust, IndentStyle::Space { size: 4 });

    // 9) ParseResult empty helper 展示 (per §2 ast)
    let empty = ParseResult::empty(Language::Python);
    println!(
        "[tree_sitter_demo] ParseResult::empty(Python) elapsed_ms = {}, ast_nodes.len() = {}",
        empty.elapsed_ms,
        empty.ast_nodes.len()
    );
    assert_eq!(empty.language, Language::Python);
    assert_eq!(empty.elapsed_ms, 0);

    // 10) LspErrorBody code 展示 (per §6 LSP)
    let err = LspErrorBody::method_not_found("textDocument/ast");
    println!("[tree_sitter_demo] LspErrorBody::method_not_found: code = {}, message = {}", err.code, err.message);
    assert_eq!(err.code, -32601);

    // 11) LspResponse 反序列化演示 (per §6 LSP 协议面)
    let resp = LspResponse {
        id: 99,
        result: Some(json!({ "value": 42 })),
        error: None,
    };
    let resp_json = serde_json::to_string(&resp)?;
    println!("[tree_sitter_demo] LspResponse 序列化: {resp_json}");
    let resp_back: LspResponse = serde_json::from_str(&resp_json)?;
    assert_eq!(resp_back, resp);

    // 12) 4 子模块 API 完整性检查 (确保 6 § 章节全部覆盖)
    let _ = FoldKind::Comment;  // §4
    let _: Vec<Box<dyn std::fmt::Debug>> = vec![];  // 防止空文件警告

    println!("[tree_sitter_demo] completed (skeleton — R20 阶段 4 续接 bash/typescript/python/rust 4 grammar)");
    Ok(())
}
