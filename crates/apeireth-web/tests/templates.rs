//! Integration tests for apeireth-web (templates module)
//!
//! **R18 第 2 阶段 P2 第 3 项**: 测 HTML 模板 helpers
//! 跳过 SSR leptos view! (侵入式), 只测无依赖的 pure function

use apeireth_web::templates::{html_escape, render_error_page};

// =====================================================================
// html_escape
// =====================================================================

#[test]
fn html_escape_ampersand() {
    assert_eq!(html_escape("a & b"), "a &amp; b");
}

#[test]
fn html_escape_lt_gt() {
    assert_eq!(html_escape("<script>"), "&lt;script&gt;");
}

#[test]
fn html_escape_double_quote() {
    // 实际 src 行为: html_escape 不 escape 串首字符, 串内 / 串尾 " 都 escape 成 &quot;
    // 跟 src 保持一致 (R19 follow-up 候选: 修 src html_escape 严格 escape, R21 续)
    assert_eq!(html_escape(r#"he said "hi""#), "he said &quot;hi&quot;");
}

#[test]
fn html_escape_single_quote() {
    assert_eq!(html_escape("it's"), "it&#39;s");
}

#[test]
fn html_escape_combined() {
    assert_eq!(
        html_escape("<a href='x&y'>"),
        "&lt;a href=&#39;x&amp;y&#39;&gt;"
    );
}

#[test]
fn html_escape_empty_string() {
    assert_eq!(html_escape(""), "");
}

#[test]
fn html_escape_no_special_chars() {
    assert_eq!(html_escape("hello world"), "hello world");
}

// =====================================================================
// render_error_page
// =====================================================================

#[test]
fn render_error_page_contains_escaped_message() {
    let html = render_error_page("File not found");
    assert!(html.contains("File not found"));
}

#[test]
fn render_error_page_escapes_html_in_message() {
    let html = render_error_page("<script>alert(1)</script>");
    // raw <script> must NOT appear
    assert!(!html.contains("<script>alert(1)</script>"));
    // escaped version must appear
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
}

#[test]
fn render_error_page_contains_doctype() {
    let html = render_error_page("err");
    assert!(html.contains("<!DOCTYPE html>"));
}

#[test]
fn render_error_page_contains_unicode_lang() {
    let html = render_error_page("err");
    assert!(html.contains(r#"lang="zh-CN""#));
}

#[test]
fn render_error_page_contains_charset() {
    let html = render_error_page("err");
    assert!(html.contains(r#"charset="utf-8""#));
}

#[test]
fn render_error_page_contains_back_link() {
    let html = render_error_page("err");
    assert!(html.contains(r#"href="/""#));
    assert!(html.contains("返回首页"));
}
