//! Integration tests for apeireth-tool-browser (post-1.0.0)
//!
//! src/ 7 module 真实现 (browser/accessibility/cli/compat/enhanced/fetch/mcp).
//! 这里 (tests/) 加跨 API 集成 + 真实 HTML 提取 + boundary.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_tool_browser::{
    extract_tree, AccessibilityNode, AccessibilityTree, BrowserCommand, BrowserCompatRouter,
    BrowserMode, PageSnapshot, BROWSER_COMMAND_COUNT, R139_DELIVERABLES, UPGRADE_DIMENSIONS,
};

// =============================================================================
// Constants
// =============================================================================

#[test]
fn r139_deliverables_count() {
    assert_eq!(R139_DELIVERABLES, 7);
}

#[test]
fn upgrade_dimensions_count() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}

#[test]
fn browser_command_count() {
    assert_eq!(BROWSER_COMMAND_COUNT, 2);
}

// =============================================================================
// BrowserMode
// =============================================================================

#[test]
fn browser_mode_variants() {
    assert_eq!(BrowserMode::Fetch, BrowserMode::Fetch);
    assert_ne!(BrowserMode::Fetch, BrowserMode::Cdp);
    assert_ne!(BrowserMode::Cdp, BrowserMode::Auto);
    assert_ne!(BrowserMode::Fetch, BrowserMode::Auto);
}

#[test]
fn browser_mode_copy_eq() {
    let m = BrowserMode::Fetch;
    let m2 = m;
    assert_eq!(m, m2);
}

// =============================================================================
// PageSnapshot
// =============================================================================

#[test]
fn page_snapshot_construction() {
    let snap = PageSnapshot {
        url: "https://example.com".into(),
        title: "Example".into(),
        accessibility: AccessibilityTree::empty(),
        raw_html: None,
        status: 200,
        timestamp: "2026-08-12T00:00:00Z".into(),
    };
    assert_eq!(snap.status, 200);
    assert_eq!(snap.url, "https://example.com");
    assert!(snap.raw_html.is_none());
}

#[test]
fn page_snapshot_with_html() {
    let snap = PageSnapshot {
        url: "x".into(),
        title: "t".into(),
        accessibility: AccessibilityTree::empty(),
        raw_html: Some("<html></html>".into()),
        status: 200,
        timestamp: "t".into(),
    };
    assert_eq!(snap.raw_html.as_deref(), Some("<html></html>"));
}

#[test]
fn page_snapshot_clone() {
    let snap = PageSnapshot {
        url: "u".into(),
        title: "t".into(),
        accessibility: AccessibilityTree::empty(),
        raw_html: None,
        status: 200,
        timestamp: "t".into(),
    };
    let snap2 = snap.clone();
    assert_eq!(snap.url, snap2.url);
    assert_eq!(snap.status, snap2.status);
}

// =============================================================================
// NodeRole
// =============================================================================

#[test]
fn node_role_as_str_basic() {
    assert_eq!(apeireth_tool_browser::NodeRole::Link.as_str(), "link");
    assert_eq!(apeireth_tool_browser::NodeRole::Button.as_str(), "button");
    assert_eq!(
        apeireth_tool_browser::NodeRole::Document.as_str(),
        "document"
    );
}

#[test]
fn node_role_heading_as_str() {
    let h = apeireth_tool_browser::NodeRole::Heading(2);
    assert_eq!(h.as_str(), "heading", "heading level 数字 不暴露");
}

#[test]
fn node_role_other() {
    let r = apeireth_tool_browser::NodeRole::Other("tab".into());
    assert_eq!(r.as_str(), "tab");
}

// =============================================================================
// AccessibilityTree
// =============================================================================

#[test]
fn tree_empty() {
    let t = AccessibilityTree::empty();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
}

#[test]
fn tree_default() {
    let t = AccessibilityTree::default();
    assert!(t.is_empty());
}

#[test]
fn tree_to_snapshot_empty() {
    let t = AccessibilityTree::empty();
    assert_eq!(t.to_snapshot(), "");
}

#[test]
fn tree_find_by_ref_missing_none() {
    let t = AccessibilityTree::empty();
    assert!(t.find_by_ref("e0").is_none());
}

// =============================================================================
// extract_tree - basic HTML
// =============================================================================

#[test]
fn extract_empty_html_empty_tree() {
    let t = extract_tree("");
    assert!(t.is_empty());
}

#[test]
fn extract_simple_h1() {
    let h = "<h1>Hello</h1>";
    let t = extract_tree(h);
    assert_eq!(t.len(), 1, "1 个 h1 node");
    // 找 h1
    let h1 = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Heading(1)));
    assert!(h1.is_some());
    assert_eq!(h1.unwrap().name, "Hello");
}

#[test]
fn extract_link_with_aria_label() {
    let h = r#"<a href="/x" aria-label="Go to X">X</a>"#;
    let t = extract_tree(h);
    let link = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Link));
    assert!(link.is_some());
    assert_eq!(link.unwrap().name, "Go to X");
    assert!(link.unwrap().ref_id.is_some());
}

#[test]
fn extract_button_with_text() {
    let h = r#"<button>Click me</button>"#;
    let t = extract_tree(h);
    let btn = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Button));
    assert!(btn.is_some());
    assert_eq!(btn.unwrap().name, "Click me");
    assert!(btn.unwrap().ref_id.is_some());
}

#[test]
fn extract_input_checkbox_role() {
    let h = r#"<input type="checkbox">"#;
    let t = extract_tree(h);
    let cb = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Checkbox));
    assert!(cb.is_some());
}

#[test]
fn extract_input_text_role() {
    let h = r#"<input type="text" placeholder="Name">"#;
    let t = extract_tree(h);
    let tb = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Textbox));
    assert!(tb.is_some());
    assert_eq!(tb.unwrap().name, "Name");
}

#[test]
fn extract_nav_role() {
    let h = "<nav>menu</nav>";
    let t = extract_tree(h);
    let nav = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Navigation));
    assert!(nav.is_some());
}

#[test]
fn extract_main_role() {
    let h = "<main>content</main>";
    let t = extract_tree(h);
    let main = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Main));
    assert!(main.is_some());
}

#[test]
fn extract_img_with_alt() {
    let h = r#"<img src="x.png" alt="Logo">"#;
    let t = extract_tree(h);
    let img = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Image));
    assert!(img.is_some());
    assert_eq!(img.unwrap().name, "Logo");
}

#[test]
fn extract_h2_role() {
    let h = "<h2>Subtitle</h2>";
    let t = extract_tree(h);
    let h2 = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Heading(2)));
    assert!(h2.is_some());
    assert_eq!(h2.unwrap().name, "Subtitle");
}

#[test]
fn extract_h6_role() {
    let h = "<h6>tiny</h6>";
    let t = extract_tree(h);
    let h6 = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Heading(6)));
    assert!(h6.is_some());
}

#[test]
fn extract_custom_role_attr() {
    let h = r#"<div role="tab">Tab 1</div>"#;
    let t = extract_tree(h);
    let tab = t
        .nodes
        .iter()
        .find(|n| matches!(n.role, apeireth_tool_browser::NodeRole::Other(_)));
    assert!(tab.is_some());
    if let apeireth_tool_browser::NodeRole::Other(s) = &tab.unwrap().role {
        assert_eq!(s, "tab");
    }
}

#[test]
fn extract_skip_script_content() {
    let h = r#"<h1>Title</h1><script>var x = "<h2>fake</h2>";</script><p>Real</p>"#;
    let t = extract_tree(h);
    let roles: Vec<_> = t.nodes.iter().map(|n| n.role.clone()).collect();
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Heading(1)));
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Paragraph));
    assert!(
        !roles.contains(&apeireth_tool_browser::NodeRole::Heading(2)),
        "h2 inside script 应被剥离"
    );
}

#[test]
fn extract_skip_style_content() {
    let h = "<h1>Title</h1><style>body { color: red; }</style><p>Real</p>";
    let t = extract_tree(h);
    // style 不应进入 nodes
    let names: Vec<&str> = t.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(!names.iter().any(|n| n.contains("color")));
}

#[test]
fn extract_void_elements_no_panic() {
    let h = r#"<br><hr><img alt="logo"><input type="text" placeholder="Name">"#;
    let t = extract_tree(h);
    assert!(t.len() >= 3);
}

#[test]
fn extract_nested_dom() {
    let h = r#"
        <html><body>
            <nav><a href="/">Home</a></nav>
            <main>
                <h1>Title</h1>
                <button>OK</button>
            </main>
        </body></html>
    "#;
    let t = extract_tree(h);
    assert!(t.len() >= 5);
    // Find link ref
    let link = t.find_by_ref("e0");
    assert!(link.is_some());
    assert_eq!(link.unwrap().role, apeireth_tool_browser::NodeRole::Link);
}

// =============================================================================
// AccessibilityTree to_snapshot
// =============================================================================

#[test]
fn tree_to_snapshot_includes_refs() {
    let h = r#"<button>OK</button><a href="/">Home</a>"#;
    let t = extract_tree(h);
    let s = t.to_snapshot();
    assert!(s.contains("button"));
    assert!(s.contains("OK"));
    assert!(s.contains("link"));
    assert!(s.contains("Home"));
    assert!(s.contains("[ref="));
}

#[test]
fn tree_to_snapshot_indents() {
    let h = "<main><h1>Title</h1><button>OK</button></main>";
    let t = extract_tree(h);
    let s = t.to_snapshot();
    // 缩进存在 (至少含 "  -")
    assert!(s.contains("  -"));
}

// =============================================================================
// interactive_refs
// =============================================================================

#[test]
fn interactive_refs_returns_only_interactive() {
    let h = r#"<button>OK</button><a href="/">Home</a><h1>Title</h1>"#;
    let t = extract_tree(h);
    let refs = t.interactive_refs();
    assert_eq!(refs.len(), 2, "button + link 互动, h1 不互动");
}

#[test]
fn interactive_refs_form_inputs() {
    let h = r#"
        <input type="checkbox" name="agree">
        <input type="radio" name="choice">
        <input type="text" placeholder="Name">
        <select><option>A</option></select>
    "#;
    let t = extract_tree(h);
    let refs = t.interactive_refs();
    assert_eq!(refs.len(), 4);
}

// =============================================================================
// BrowserCommand
// =============================================================================

#[test]
fn browser_command_from_str_2() {
    for s in ["BrowserNavigator", "WebReadFile"] {
        assert_ne!(BrowserCommand::from_str(s), BrowserCommand::Unknown);
    }
}

#[test]
fn browser_command_unknown_fallback() {
    assert_eq!(BrowserCommand::from_str("xyz"), BrowserCommand::Unknown);
    assert_eq!(BrowserCommand::from_str(""), BrowserCommand::Unknown);
}

#[test]
fn browser_command_eq_hash() {
    let a = BrowserCommand::BrowserNavigator;
    let b = BrowserCommand::BrowserNavigator;
    let c = BrowserCommand::WebReadFile;
    assert_eq!(a, b);
    assert_ne!(a, c);
    let mut set = std::collections::HashSet::new();
    set.insert(a);
    set.insert(b);
    set.insert(c);
    set.insert(BrowserCommand::Unknown);
    assert_eq!(set.len(), 3);
}

#[test]
fn browser_command_mode() {
    let nav = BrowserCommand::BrowserNavigator;
    let web = BrowserCommand::WebReadFile;
    // mode() 返 BrowserMode, 不强制 哪个 (per src comment: navigator=Auto, web=Auto 同样)
    let _ = nav.mode();
    let _ = web.mode();
}

// =============================================================================
// BrowserCompatRouter
// =============================================================================

#[test]
fn browser_router_count() {
    assert_eq!(BrowserCompatRouter::command_count(), 2);
}

#[test]
fn browser_router_default() {
    let _r = BrowserCompatRouter::default();
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_extract_then_snapshot() {
    let h = r#"
        <html>
        <head><title>Real Site</title></head>
        <body>
            <header>
                <h1>Welcome</h1>
            </header>
            <nav>
                <a href="/a">A</a>
                <a href="/b">B</a>
            </nav>
            <main>
                <button>Click</button>
                <input type="text" placeholder="Search">
            </main>
        </body>
        </html>
    "#;
    let t = extract_tree(h);
    let snap = t.to_snapshot();
    assert!(snap.contains("Welcome"));
    assert!(snap.contains("A"));
    assert!(snap.contains("B"));
    assert!(snap.contains("Click"));
    assert!(snap.contains("Search"));
}

#[test]
fn integration_interactive_refs_in_real_page() {
    let h = r#"
        <form>
            <input type="text" placeholder="Username">
            <input type="password" placeholder="Password">
            <button type="submit">Login</button>
            <a href="/register">Register</a>
        </form>
    "#;
    let t = extract_tree(h);
    let refs = t.interactive_refs();
    // 2 inputs + 1 button + 1 link = 4
    assert_eq!(refs.len(), 4);
}

#[test]
fn integration_snapshot_role_distribution() {
    let h = r#"
        <header><h1>Site</h1></header>
        <nav><a href="/">Home</a></nav>
        <main>
            <article>
                <h2>Article</h2>
                <p>Body text</p>
            </article>
            <button>Subscribe</button>
        </main>
        <footer>Copyright 2026</footer>
    "#;
    let t = extract_tree(h);
    let roles: std::collections::HashSet<_> = t.nodes.iter().map(|n| n.role.clone()).collect();
    // 验证 多种 role 都存在
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Banner));
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Navigation));
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Main));
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::ContentInfo));
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Heading(1)));
    assert!(roles.contains(&apeireth_tool_browser::NodeRole::Heading(2)));
}

#[test]
fn integration_empty_snapshot_safe() {
    let t = AccessibilityTree::empty();
    let snap = t.to_snapshot();
    assert_eq!(snap, "");
    assert!(t.find_by_ref("anything").is_none());
    assert_eq!(t.interactive_refs().len(), 0);
}