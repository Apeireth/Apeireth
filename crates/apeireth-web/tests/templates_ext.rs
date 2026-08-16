//! Integration tests for apeireth-web (HTML template helpers — extended XSS / Unicode cases)
//!
//! **R18 路线图 Stage 2 续**: 在 `tests/templates.rs` 12 个基线测试基础上,
//! 加 10 个边界 case 覆盖 XSS 攻击向量 / Unicode 字符 / 大字符串 / 嵌套 escape / 多特诊 escape.
//!
//! **不假装**: 测试是真跑 (`cargo nextest run -p apeireth-web --test templates_ext`).

use apeireth_web::templates::{html_escape, render_error_page};

// =====================================================================
// XSS 攻击向量 (per OWASP 4 类) — 5 边界
// =====================================================================

#[test]
fn xss_script_tag_with_src() {
    // <script src="evil.js"> 注入
    let s = r#"<script src="evil.js"></script>"#;
    let escaped = html_escape(s);
    assert!(
        !escaped.contains("<script"),
        "<script 必须 escape, 实际: {escaped}"
    );
    assert!(escaped.contains("&lt;script"));
}

#[test]
fn xss_img_onerror_handler() {
    // <img src=x onerror=alert(1)> 经典 XSS
    let s = r#"<img src=x onerror=alert(1)>"#;
    let escaped = html_escape(s);
    assert!(!escaped.contains("<img"));
    assert!(escaped.contains("&lt;img"));
}

#[test]
fn xss_javascript_url_in_href() {
    // <a href="javascript:alert(1)">click</a>
    let s = r#"<a href="javascript:alert(1)">click</a>"#;
    let escaped = html_escape(s);
    // html_escape 不解析 protocol, 只 escape 字符
    // javascript: URL 仍存在, 但 < > 已 escape
    assert!(!escaped.contains("<a"));
    assert!(escaped.contains("&lt;a"));
    // src 用 &quot; 不是 &#34; (per apeireth_web::templates::html_escape 实现)
    assert!(
        escaped.contains("href=&quot;javascript"),
        "引号应 escape 成 &quot;, 实际: {escaped}"
    );
}

#[test]
fn xss_svg_with_script() {
    // <svg><script>alert(1)</script></svg>
    let s = "<svg><script>alert(1)</script></svg>";
    let escaped = html_escape(s);
    assert!(!escaped.contains("<svg"));
    assert!(escaped.contains("&lt;svg"));
}

#[test]
fn xss_unicode_fullwidth_less_than() {
    // 全角 < (%EF%BC%9C) 不应被 escape (html_escape 只匹配 ASCII <)
    // 这是一个 known limitation — 测试记录行为
    let s = "＜script＞alert(1)＜/script＞"; // 全角 < >
    let escaped = html_escape(s);
    // 全角字符不被识别为 < 或 >, 保持不变
    // 这是预期的 (html_escape 不处理 unicode bypass)
    assert_eq!(escaped, s, "全角字符应保留原样, 实际: {escaped}");
}

// =====================================================================
// Unicode + 特殊字符 (中文 / emoji / 组合字符)
// =====================================================================

#[test]
fn unicode_emoji_passthrough() {
    // emoji 不应被 escape
    let s = "Hello 🦀 world";
    let escaped = html_escape(s);
    assert_eq!(escaped, s, "emoji 应原样保留, 实际: {escaped}");
}

#[test]
fn unicode_cjk_html_entities_preserved() {
    // 中文字符不被 escape
    let s = "你好世界";
    let escaped = html_escape(s);
    assert_eq!(escaped, s);
}

#[test]
fn render_error_page_with_emoji() {
    // render_error_page 应能处理 emoji
    let html = render_error_page("服务出错 🚨");
    assert!(html.contains("服务出错"));
    assert!(html.contains("🚨"), "emoji 应在 error message 中: {html}");
}

// =====================================================================
// 大字符串 + 边界
// =====================================================================

#[test]
fn html_escape_very_long_string() {
    // 100K 字符的字符串应正常 escape, 不 panic
    let long: String = "a&b".repeat(50_000); // 150K chars (3 chars × 50K)
    let escaped = html_escape(&long);
    // escape 后 "a&b" → "a&amp;b" (5 chars), 应比 original 长
    assert!(
        escaped.len() > long.len(),
        "escape 后应更长, got len={} (orig {})",
        escaped.len(),
        long.len()
    );
    assert!(escaped.contains("&amp;"));
    // 原始字符计数
    let original_amp_count = long.matches('&').count();
    let escaped_amp_count = escaped.matches("&amp;").count();
    assert_eq!(
        original_amp_count, escaped_amp_count,
        "每个 & 应 escape 成 &amp;"
    );
}

#[test]
fn render_error_page_idempotent_structure() {
    // render_error_page 多次调用结构一致 (DOCTYPE / html / body / 返回首页 都在)
    let html1 = render_error_page("err1");
    let html2 = render_error_page("err2");
    // 关键骨架字段
    for html in [&html1, &html2] {
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains(r#"lang="zh-CN""#));
        assert!(html.contains(r#"charset="utf-8""#));
        assert!(html.contains(r#"href="/""#));
        assert!(html.contains("返回首页"));
        assert!(html.contains("<h1>"));
        assert!(html.contains("</html>"));
    }
}
