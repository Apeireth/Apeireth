//! HTML → accessibility-tree extraction (Playwright-style ARIA snapshot).
//!
//! Ported from legacy `apeireth-tool-browser::accessibility` (R139). A
//! hand-rolled tokenizer (no `scraper`/`html5ever` dependency) that handles
//! the common case: standard tags + ARIA roles/names + semantic HTML
//! (`<button>`, `<a>`, `<input>`, headings, landmarks). Not a full HTML5
//! parser — pathological markup may be mis-parsed, by documented design.
//!
//! Why this matters: an LLM can consume the rendered snapshot without a
//! vision model, and the snapshot is typically 10-50x smaller than the raw
//! HTML it came from (same approach as playwright-mcp's ARIA snapshot).

use std::collections::HashMap;

/// ARIA role subset (full WAI-ARIA has ~80 roles; the common 20 are covered).
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
    /// Verbatim `role="..."` attribute value not in the subset above.
    Other(String),
}

impl NodeRole {
    /// The role name as it appears in a snapshot.
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

/// One node in the accessibility tree.
#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    pub role: NodeRole,
    pub name: String,
    /// Ref id for interactive targeting (`e1`, `e2`, ...).
    pub ref_id: Option<String>,
    /// Indices of child nodes into the owning tree's `nodes` vec.
    pub children: Vec<usize>,
    /// The raw HTML tag this node came from.
    pub tag: String,
    /// Additional attributes (`aria-*`, `type`, ...).
    pub attrs: HashMap<String, String>,
}

/// Flat vec + parent/children indices for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct AccessibilityTree {
    pub nodes: Vec<AccessibilityNode>,
    /// Root index (0 when nodes is non-empty).
    pub root: usize,
    /// Counter for assigning ref ids.
    pub next_ref_id: usize,
}

impl AccessibilityTree {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Count of real (non-root) nodes.
    pub fn len(&self) -> usize {
        if self.nodes.is_empty() {
            0
        } else {
            self.nodes.len() - 1
        }
    }

    /// True when there are no real content nodes.
    pub fn is_empty(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }
        self.nodes[self.root].children.is_empty()
    }

    /// Render as Playwright-style ARIA snapshot text: hierarchical 2-space
    /// indent, `- role "name" [ref=eN]` lines.
    pub fn to_snapshot(&self) -> String {
        let mut out = String::new();
        if self.nodes.is_empty() {
            return out;
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
            Some(r) => format!(" [ref={r}]"),
            None => String::new(),
        };
        out.push_str(&format!(
            "{}- {}{}{}\n",
            indent,
            node.role.as_str(),
            name_part,
            ref_part
        ));
        for child_idx in &node.children {
            self.render_node(*child_idx, depth + 1, out);
        }
    }

    /// Find a node by ref id (e.g. `e5`).
    pub fn find_by_ref(&self, ref_id: &str) -> Option<&AccessibilityNode> {
        self.nodes
            .iter()
            .find(|n| n.ref_id.as_deref() == Some(ref_id))
    }

    /// All interactive node refs (links/buttons/textboxes/checkboxes/radios/
    /// comboboxes) in document order.
    pub fn interactive_refs(&self) -> Vec<(String, NodeRole, String)> {
        let mut out = Vec::new();
        for node in &self.nodes {
            if matches!(
                node.role,
                NodeRole::Link
                    | NodeRole::Button
                    | NodeRole::Textbox
                    | NodeRole::Checkbox
                    | NodeRole::Radio
                    | NodeRole::Combobox
            ) {
                if let Some(r) = &node.ref_id {
                    out.push((r.clone(), node.role.clone(), node.name.clone()));
                }
            }
        }
        out
    }
}

/// Extract an accessibility tree from a raw HTML string.
pub fn extract_tree(html: &str) -> AccessibilityTree {
    let mut tree = AccessibilityTree::default();
    // Synthetic document root so un-parented top-level elements share a parent.
    tree.nodes.push(AccessibilityNode {
        role: NodeRole::Document,
        name: String::new(),
        ref_id: None,
        children: Vec::new(),
        tag: "#document".to_string(),
        attrs: HashMap::new(),
    });
    tree.root = 0;
    let mut stack: Vec<usize> = vec![0];
    let mut skip_until: Option<String> = None;
    let mut buf = String::new();

    let bytes = html.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let remaining = &html[i..];

        // Skip script/style/noscript content entirely.
        if let Some(close_tag) = &skip_until {
            if let Some(end) = remaining.find(&format!("</{close_tag}")) {
                i += end + 2 + close_tag.len() + 1;
                skip_until = None;
                continue;
            }
            break;
        }

        // Look for `<`.
        if let Some(tag_start) = remaining.find('<') {
            if tag_start > 0 {
                let text = &remaining[..tag_start];
                if !text.trim().is_empty() {
                    buf.push_str(text);
                }
            }

            let Some(tag_end) = remaining[tag_start..].find('>') else {
                break;
            };
            let raw_tag = &remaining[tag_start + 1..tag_start + tag_end];
            i += tag_start + tag_end + 1;

            if raw_tag.starts_with('/') {
                // Closing tag: pop one node and flush buffered text into it.
                if let Some(node_idx) = stack.pop() {
                    if tree.nodes[node_idx].name.is_empty() && !buf.trim().is_empty() {
                        tree.nodes[node_idx].name.push_str(buf.trim());
                    }
                    buf.clear();
                }
            } else if raw_tag.ends_with('/') {
                // Self-closing tag (br, hr, img, input, meta, link, ...).
                let tag_content = raw_tag.strip_suffix('/').unwrap_or(raw_tag);
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
                // Opening tag.
                let (tag_name, attrs) = parse_tag_parts(raw_tag);
                let tag_lower = tag_name.to_lowercase();

                if matches!(tag_lower.as_str(), "script" | "style" | "noscript") {
                    skip_until = Some(tag_lower);
                    continue;
                }

                // Flush pending text into the current parent's name buffer.
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
                let parent_idx = stack
                    .last()
                    .copied()
                    .expect("synthetic document root always present");
                tree.nodes[parent_idx].children.push(new_idx);
                if !is_void(&tag_lower) {
                    stack.push(new_idx);
                }
            }
        } else {
            // No more tags; the rest is text.
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
        if c.is_whitespace() {
            continue;
        }
        let mut name = String::new();
        name.push(c);
        while let Some(&(_, nc)) = chars.peek() {
            if nc == '=' || nc.is_whitespace() {
                break;
            }
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
                        if vc == quote {
                            break;
                        }
                        value.push(vc);
                    }
                    attrs.insert(name.to_lowercase(), value);
                }
                _ => {
                    let mut value = String::new();
                    while let Some((_, vc)) = chars.peek().copied() {
                        if vc.is_whitespace() {
                            break;
                        }
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
        "br" | "hr"
            | "img"
            | "input"
            | "meta"
            | "link"
            | "area"
            | "base"
            | "col"
            | "embed"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_interactive(role: &NodeRole) -> bool {
    matches!(
        role,
        NodeRole::Link
            | NodeRole::Button
            | NodeRole::Textbox
            | NodeRole::Checkbox
            | NodeRole::Radio
            | NodeRole::Combobox
    )
}

fn assign_ref_id(tree: &mut AccessibilityTree) -> String {
    let id = tree.next_ref_id;
    tree.next_ref_id += 1;
    format!("e{id}")
}

fn role_for_tag(tag: &str, attrs: &HashMap<String, String>) -> NodeRole {
    // An explicit role="..." attribute always wins.
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
    // Priority: aria-label > title > alt > placeholder > value.
    for key in ["aria-label", "title", "alt", "placeholder", "value"] {
        if let Some(v) = attrs.get(key) {
            return v.clone();
        }
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
    fn extract_nested_landmarks() {
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
        let link = tree.find_by_ref("e0").expect("e0 should be a link");
        assert_eq!(link.role, NodeRole::Link);
    }

    #[test]
    fn skip_script_content() {
        let html = r#"<h1>Title</h1><script>var x = "<h2>fake</h2>";</script><p>Real</p>"#;
        let tree = extract_tree(html);
        let roles: Vec<_> = tree.nodes.iter().map(|n| n.role.clone()).collect();
        assert!(roles.contains(&NodeRole::Heading(1)));
        assert!(roles.contains(&NodeRole::Paragraph));
        assert!(
            !roles.contains(&NodeRole::Heading(2)),
            "heading inside script content must be skipped"
        );
    }

    #[test]
    fn snapshot_rendering() {
        let html = r#"<button>OK</button><a href="/">Home</a>"#;
        let tree = extract_tree(html);
        let snap = tree.to_snapshot();
        assert!(snap.contains("button"), "{snap}");
        assert!(snap.contains("OK"), "{snap}");
        assert!(snap.contains("link"), "{snap}");
        assert!(snap.contains("Home"), "{snap}");
        assert!(snap.contains("[ref="), "{snap}");
    }

    #[test]
    fn interactive_refs() {
        let html = r#"<button>OK</button><a href="/">Home</a><h1>Title</h1>"#;
        let tree = extract_tree(html);
        let refs = tree.interactive_refs();
        assert_eq!(
            refs.len(),
            2,
            "button and link are interactive, heading is not"
        );
    }

    #[test]
    fn custom_role_attr_wins() {
        let html = r#"<div role="tab">Tab 1</div>"#;
        let tree = extract_tree(html);
        assert_eq!(tree.nodes[1].role, NodeRole::Other("tab".to_string()));
    }

    #[test]
    fn input_checkbox_role() {
        let html = r#"<input type="checkbox">Accept</input>"#;
        let tree = extract_tree(html);
        assert_eq!(tree.nodes[1].role, NodeRole::Checkbox);
    }

    #[test]
    fn void_elements_do_not_panic() {
        let html = r#"<br><hr><img alt="logo"><input type="text" placeholder="Name">"#;
        let tree = extract_tree(html);
        assert!(tree.len() >= 3);
    }

    #[test]
    fn token_efficient_snapshot() {
        // Realistic pages carry scripts, styles, and class/id noise that the
        // snapshot drops. Compact tag-only HTML is *not* a fair comparison.
        let html = format!(
            "<html><head><style>{}</style><script>{}</script></head><body class=\"page wrap\">{}<h1 class=\"title\">Doc</h1>{}</body></html>",
            "body{color:red;margin:0;padding:0;} ".repeat(80),
            "function noise(){console.log('x');} ".repeat(80),
            "<p class=\"lorem ipsum dolor sit\" data-id=\"n\" id=\"p\">Lorem ipsum.</p>".repeat(20),
            "<a class=\"btn primary\" href=\"/x\" data-track=\"click\">link</a>".repeat(10)
        );
        let tree = extract_tree(&html);
        let snap = tree.to_snapshot();
        assert!(
            snap.len() < html.len() / 5,
            "snapshot {} vs html {}",
            snap.len(),
            html.len()
        );
    }
}
