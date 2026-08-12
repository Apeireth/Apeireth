//! HTML → accessibility-tree extraction (Playwright-style ARIA snapshot).
//!
//! Hand-rolled tokenizer (no `scraper`/`html5ever` deps) that handles the
//! 95% case: standard tags + ARIA roles/names + common semantic HTML
//! (`<button>`, `<a>`, `<input>`, etc.). NOT a full HTML5 parser — may
//! mis-parse pathological HTML but that's documented honestly.
//!
//! **Why this matters** (per v2 plan §9.2):
//! - LLM can consume the tree without needing vision model
//! - Token-efficient vs raw HTML (typically 10-50x reduction)
//! - Same approach as Microsoft playwright-mcp's ARIA snapshot
//!
//! **Role mapping** (subset):
//! - `<a href>` → role=link, name=link text
//! - `<button>` → role=button, name=text
//! - `<h1>`-`<h6>` → role=heading (with level), name=text
//! - `<input>` → role=textbox/button/checkbox (by type), name=label
//! - `<img alt>` → role=image, name=alt
//! - `<nav>` → role=navigation
//! - `<main>` → role=main
//! - `role="..."` attr → use that role verbatim

use std::collections::HashMap;

/// ARIA role subset (full WAI-ARIA has ~80 roles; we cover the common 20).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeRole {
    Document,
    Heading(u8),
    Link,
    Button,
    Textbox,
    Checkbox,
    Radio,
    Combobox,
    List,
    ListItem,
    Navigation,
    Main,
    Banner,
    ContentInfo,
    Image,
    Paragraph,
    Form,
    Region,
    Generic,
    /// Raw role string (anything not in the subset)
    Other(String),
}

impl NodeRole {
    pub fn as_str(&self) -> &str {
        match self {
            NodeRole::Document => "document",
            NodeRole::Heading(_) => "heading",
            NodeRole::Link => "link",
            NodeRole::Button => "button",
            NodeRole::Textbox => "textbox",
            NodeRole::Checkbox => "checkbox",
            NodeRole::Radio => "radio",
            NodeRole::Combobox => "combobox",
            NodeRole::List => "list",
            NodeRole::ListItem => "listitem",
            NodeRole::Navigation => "navigation",
            NodeRole::Main => "main",
            NodeRole::Banner => "banner",
            NodeRole::ContentInfo => "contentinfo",
            NodeRole::Image => "image",
            NodeRole::Paragraph => "paragraph",
            NodeRole::Form => "form",
            NodeRole::Region => "region",
            NodeRole::Generic => "generic",
            NodeRole::Other(s) => s,
        }
    }
}

/// Single node in the accessibility tree.
#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    pub role: NodeRole,
    pub name: String,
    /// Optional ref id (used for click/type targeting). Format: `e1`, `e2`, ...
    pub ref_id: Option<String>,
    /// Children indices into the parent tree's vec
    pub children: Vec<usize>,
    /// Tag name (raw HTML tag)
    pub tag: String,
    /// Additional attributes (aria-* etc.) — debug only
    pub attrs: HashMap<String, String>,
}

/// Tree of accessibility nodes. Flat vec + parent/children indices for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct AccessibilityTree {
    pub nodes: Vec<AccessibilityNode>,
    /// Root index (0 if nodes is non-empty)
    pub root: usize,
    /// Counter for assigning ref ids
    pub next_ref_id: usize,
}

impl AccessibilityTree {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Count real (non-root) nodes in tree.
    pub fn len(&self) -> usize {
        if self.nodes.is_empty() { 0 } else { self.nodes.len() - 1 }
    }

    /// Is the tree empty (no real content nodes)?
    pub fn is_empty(&self) -> bool {
        // Empty if no root OR root has no children
        if self.nodes.is_empty() { return true; }
        self.nodes[self.root].children.is_empty()
    }

    /// Render as ARIA snapshot text (Playwright-compatible).
    /// Format: hierarchical with 2-space indent + `[ref=e1]` annotations.
    pub fn to_snapshot(&self) -> String {
        let mut out = String::new();
        if self.nodes.is_empty() {
            return String::new();
        }
        self.render_node(self.root, 0, &mut out);
        out
    }

    fn render_node(&self, idx: usize, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        let node = &self.nodes[idx];
        let name_part = if node.name.is_empty() {
            String::new()
        } else {
            format!(" \"{}\"", node.name)
        };
        let ref_part = match &node.ref_id {
            Some(r) => format!(" [ref={}]", r),
            None => String::new(),
        };
        out.push_str(&format!("{}- {}{}{}\n", indent, node.role.as_str(), name_part, ref_part));
        for child_idx in &node.children {
            self.render_node(*child_idx, depth + 1, out);
        }
    }

    /// Find a node by ref id (e.g. "e5").
    pub fn find_by_ref(&self, ref_id: &str) -> Option<&AccessibilityNode> {
        self.nodes.iter().find(|n| n.ref_id.as_deref() == Some(ref_id))
    }

    /// Extract all interactive node refs (links/buttons/textboxes/etc.)
    /// in document order. Useful for `apeireth browser click e3`.
    pub fn interactive_refs(&self) -> Vec<(String, NodeRole, String)> {
        let mut out = Vec::new();
        for node in &self.nodes {
            match &node.role {
                NodeRole::Link | NodeRole::Button | NodeRole::Textbox
                | NodeRole::Checkbox | NodeRole::Radio | NodeRole::Combobox => {
                    if let Some(r) = &node.ref_id {
                        out.push((r.clone(), node.role.clone(), node.name.clone()));
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// Extract accessibility tree from raw HTML string.
pub fn extract_tree(html: &str) -> AccessibilityTree {
    let mut tree = AccessibilityTree::default();
    // Synthesise a document root so un-parented top-level elements
    // (multiple roots like `<button/><a/>`) all share a parent.
    tree.nodes.push(AccessibilityNode {
        role: crate::accessibility::NodeRole::Document,
        name: String::new(),
        ref_id: None,
        children: Vec::new(),
        tag: "#document".to_string(),
        attrs: std::collections::HashMap::new(),
    });
    tree.root = 0;
    let mut stack: Vec<usize> = vec![0];
    let mut skip_until: Option<String> = None; // skip until </tag>
    let mut buf = String::new(); // text buffer for current node

    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let remaining = &html[i..];

        // Skip script/style/noscript content
        if let Some(close_tag) = &skip_until {
            if let Some(end) = remaining.find(&format!("</{}", close_tag)) {
                i += end + 2 + close_tag.len() + 1; // consume up to and past `</tag`
                skip_until = None;
                continue;
            } else {
                break;
            }
        }

        // Look for `<`
        if let Some(tag_start) = remaining.find('<') {
            // Flush text before tag
            if tag_start > 0 {
                let text = &remaining[..tag_start];
                if !text.trim().is_empty() {
                    buf.push_str(text);
                }
            }

            // Find `>`
            if let Some(tag_end) = remaining[tag_start..].find('>') {
                let raw_tag = &remaining[tag_start + 1..tag_start + tag_end];
                i += tag_start + tag_end + 1;

                if raw_tag.starts_with('/') {
                    // Closing tag — pop stack, flush text into popped node
                    // Only if name is empty (otherwise it came from aria-label/title/alt)
                    let tag_name = raw_tag[1..].trim().to_lowercase();
                    if let Some(node_idx) = stack.pop() {
                        if tree.nodes[node_idx].name.is_empty() && !buf.trim().is_empty() {
                            tree.nodes[node_idx].name.push_str(buf.trim());
                        }
                        buf.clear();
                    }
                    let _ = tag_name;
                } else if raw_tag.ends_with('/') {
                    // Self-closing tag (br, hr, img, input, meta, link)
                    let tag_content = &raw_tag[..raw_tag.len() - 1];
                    let (tag_name, attrs) = parse_tag_parts(tag_content);
                    let tag_lower = tag_name.to_lowercase();
                    let interactive = matches!(
                        tag_lower.as_str(),
                        "br" | "hr" | "img" | "input" | "meta" | "link"
                    );
                    if interactive {
                        if let Some(parent_idx) = stack.last().copied() {
                            let role = role_for_tag(&tag_lower, &attrs);
                            let name = name_from_attrs(&attrs);
                            let ref_id = assign_ref_id(&mut tree);
                            tree.nodes.push(AccessibilityNode {
                                role,
                                name,
                                ref_id: Some(ref_id),
                                children: Vec::new(),
                                tag: tag_lower,
                                attrs,
                            });
                            let new_idx = tree.nodes.len() - 1;
                            tree.nodes[parent_idx].children.push(new_idx);
                        }
                    }
                } else {
                    // Opening tag
                    let (tag_name, attrs) = parse_tag_parts(raw_tag);
                    let tag_lower = tag_name.to_lowercase();

                    // Handle special skip tags
                    if matches!(tag_lower.as_str(), "script" | "style" | "noscript") {
                        skip_until = Some(tag_lower);
                        continue;
                    }

                    // Flush buf into the *current parent*'s text buffer (don't pop yet,
                    // because this node's open tag may also receive text).
                    if !buf.trim().is_empty() {
                        if let Some(parent_idx) = stack.last().copied() {
                            if !tree.nodes[parent_idx].name.is_empty() {
                                tree.nodes[parent_idx].name.push(' ');
                            }
                            tree.nodes[parent_idx].name.push_str(buf.trim());
                        }
                        buf.clear();
                    }

                    let role = role_for_tag(&tag_lower, &attrs);
                    let name = name_from_attrs(&attrs);
                    let ref_id = if is_interactive(&role) {
                        Some(assign_ref_id(&mut tree))
                    } else {
                        None
                    };
                    tree.nodes.push(AccessibilityNode {
                        role,
                        name,
                        ref_id,
                        children: Vec::new(),
                        tag: tag_lower.clone(),
                        attrs,
                    });
                    let new_idx = tree.nodes.len() - 1;
                    // stack always contains at least the synthetic document root,
                    // so this branch is always taken
                    let parent_idx = stack.last().copied().expect("synthetic root");
                    tree.nodes[parent_idx].children.push(new_idx);
                    // Void elements — don't push onto stack
                    if !is_void(&tag_lower) {
                        stack.push(new_idx);
                    }
                }
            } else {
                break;
            }
        } else {
            // No more tags — rest is text
            if !remaining.trim().is_empty() {
                buf.push_str(remaining);
            }
            break;
        }
    }

    tree
}

fn parse_tag_parts(raw: &str) -> (String, HashMap<String, String>) {
    let trimmed = raw.trim();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let tag_name = trimmed[..i].to_string();
    let mut attrs: HashMap<String, String> = HashMap::new();
    let rest = &trimmed[i..];
    let mut chars = rest.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c.is_whitespace() { continue; }
        let mut name = String::new();
        name.push(c);
        while let Some(&(_, nc)) = chars.peek() {
            if nc == '=' || nc.is_whitespace() { break; }
            name.push(nc);
            chars.next();
        }
        if let Some(&(_, '=')) = chars.peek() {
            chars.next();
            match chars.peek().copied() {
                Some((_, q)) if q == '"' || q == '\'' => {
                    let quote = q;
                    chars.next();
                    let mut value = String::new();
                    while let Some((_, vc)) = chars.next() {
                        if vc == quote { break; }
                        value.push(vc);
                    }
                    attrs.insert(name.to_lowercase(), value);
                }
                _ => {
                    let mut value = String::new();
                    while let Some(&(_, vc)) = chars.peek() {
                        if vc.is_whitespace() { break; }
                        value.push(vc);
                        chars.next();
                    }
                    attrs.insert(name.to_lowercase(), value);
                }
            }
        } else {
            attrs.insert(name.to_lowercase(), String::new());
        }
    }
    (tag_name, attrs)
}

fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "br" | "hr" | "img" | "input" | "meta" | "link" | "area" | "base" | "col"
            | "embed" | "param" | "source" | "track" | "wbr"
    )
}

fn is_interactive(role: &NodeRole) -> bool {
    matches!(
        role,
        NodeRole::Link | NodeRole::Button | NodeRole::Textbox
            | NodeRole::Checkbox | NodeRole::Radio | NodeRole::Combobox
    )
}

fn assign_ref_id(tree: &mut AccessibilityTree) -> String {
    let id = tree.next_ref_id;
    tree.next_ref_id += 1;
    format!("e{}", id)
}

fn role_for_tag(tag: &str, attrs: &HashMap<String, String>) -> NodeRole {
    // role="..." attr wins
    if let Some(role) = attrs.get("role") {
        return NodeRole::Other(role.clone());
    }
    match tag {
        "a" => NodeRole::Link,
        "button" => NodeRole::Button,
        "nav" => NodeRole::Navigation,
        "main" => NodeRole::Main,
        "header" => NodeRole::Banner,
        "footer" => NodeRole::ContentInfo,
        "ul" | "ol" => NodeRole::List,
        "li" => NodeRole::ListItem,
        "p" => NodeRole::Paragraph,
        "form" => NodeRole::Form,
        "img" => NodeRole::Image,
        "h1" => NodeRole::Heading(1),
        "h2" => NodeRole::Heading(2),
        "h3" => NodeRole::Heading(3),
        "h4" => NodeRole::Heading(4),
        "h5" => NodeRole::Heading(5),
        "h6" => NodeRole::Heading(6),
        "input" => match attrs.get("type").map(|s| s.as_str()) {
            Some("checkbox") => NodeRole::Checkbox,
            Some("radio") => NodeRole::Radio,
            Some("button") | Some("submit") | Some("reset") => NodeRole::Button,
            _ => NodeRole::Textbox,
        },
        "select" => NodeRole::Combobox,
        "textarea" => NodeRole::Textbox,
        "section" | "article" | "aside" => {
            if attrs.contains_key("aria-label") || attrs.contains_key("aria-labelledby") {
                NodeRole::Region
            } else {
                NodeRole::Generic
            }
        }
        "html" | "body" | "head" => NodeRole::Document,
        _ => NodeRole::Generic,
    }
}

fn name_from_attrs(attrs: &HashMap<String, String>) -> String {
    // Priority: aria-label > aria-labelledby > title > alt > placeholder > value
    if let Some(v) = attrs.get("aria-label") {
        return v.clone();
    }
    if let Some(v) = attrs.get("title") {
        return v.clone();
    }
    if let Some(v) = attrs.get("alt") {
        return v.clone();
    }
    if let Some(v) = attrs.get("placeholder") {
        return v.clone();
    }
    if let Some(v) = attrs.get("value") {
        return v.clone();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_empty_html() {
        let tree = extract_tree("");
        assert!(tree.is_empty());
    }

    #[test]
    fn extract_simple_heading() {
        let html = "<h1>Hello World</h1>";
        let tree = extract_tree(html);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.nodes[1].role, NodeRole::Heading(1));
        assert_eq!(tree.nodes[1].name, "Hello World");
    }

    #[test]
    fn extract_link_with_aria_label() {
        let html = r#"<a href="/x" aria-label="Go to X">X</a>"#;
        let tree = extract_tree(html);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.nodes[1].role, NodeRole::Link);
        assert_eq!(tree.nodes[1].name, "Go to X");
        assert_eq!(tree.nodes[1].ref_id.as_deref(), Some("e0"));
    }

    #[test]
    fn extract_button() {
        let html = r#"<button>Click me</button>"#;
        let tree = extract_tree(html);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.nodes[1].role, NodeRole::Button);
        assert_eq!(tree.nodes[1].name, "Click me");
        assert!(tree.nodes[1].ref_id.is_some());
    }

    #[test]
    fn extract_nested() {
        let html = r#"
            <html><body>
                <nav><a href="/">Home</a></nav>
                <main>
                    <h1>Title</h1>
                    <button>OK</button>
                </main>
            </body></html>
        "#;
        let tree = extract_tree(html);
        assert!(tree.len() >= 5);
        // Find link ref
        let link = tree.find_by_ref("e0").expect("e0 should be link");
        assert_eq!(link.role, NodeRole::Link);
    }

    #[test]
    fn skip_script_content() {
        let html = r#"<h1>Title</h1><script>var x = "<h2>fake</h2>";</script><p>Real</p>"#;
        let tree = extract_tree(html);
        // Should have h1, paragraph — not h2 (which was inside script)
        let roles: Vec<_> = tree.nodes.iter().map(|n| n.role.clone()).collect();
        assert!(roles.contains(&NodeRole::Heading(1)));
        assert!(roles.contains(&NodeRole::Paragraph));
        assert!(!roles.contains(&NodeRole::Heading(2)), "h2 inside script should be skipped");
    }

    #[test]
    fn snapshot_rendering() {
        let html = r#"<button>OK</button><a href="/">Home</a>"#;
        let tree = extract_tree(html);
        let snap = tree.to_snapshot();
        assert!(snap.contains("button"));
        assert!(snap.contains("OK"));
        assert!(snap.contains("link"));
        assert!(snap.contains("Home"));
        assert!(snap.contains("[ref="));
    }

    #[test]
    fn interactive_refs() {
        let html = r#"<button>OK</button><a href="/">Home</a><h1>Title</h1>"#;
        let tree = extract_tree(html);
        let refs = tree.interactive_refs();
        // Button and link are interactive, h1 is not
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn custom_role_attr_wins() {
        let html = r#"<div role="tab">Tab 1</div>"#;
        let tree = extract_tree(html);
        match &tree.nodes[1].role {
            NodeRole::Other(s) => assert_eq!(s, "tab"),
            _ => panic!("expected Other(\"tab\")"),
        }
    }

    #[test]
    fn input_checkbox_role() {
        let html = r#"<input type="checkbox">Accept</input>"#;
        let tree = extract_tree(html);
        assert_eq!(tree.nodes[1].role, NodeRole::Checkbox);
    }

    #[test]
    fn void_elements_dont_panic() {
        let html = r#"<br><hr><img alt="logo"><input type="text" placeholder="Name">"#;
        let tree = extract_tree(html);
        assert!(tree.len() >= 3);
    }
}