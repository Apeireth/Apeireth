// apeireth-web shared HTML template helpers (R18).
//
// Extracted into the lib crate so multiple handlers (Council in main.rs,
// Memory in memory.rs) can reuse the same error page + escape logic.

/// HTML entity escape. Returns a new String with `&`, `<`, `>`, `"`, `'`
/// replaced by their corresponding entities.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Render the shared error page. Any handler can return this on failure.
pub fn render_error_page(msg: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth 错误</title>
</head>
<body>
    <main class="apeireth-app">
        <div class="apeireth-error-page">
            <h1>错误</h1>
            <p>{}</p>
            <a class="apeireth-button-link" href="/">← 返回首页</a>
        </div>
    </main>
</body>
</html>"#,
        html_escape(msg)
    )
}
