//! `view!` 宏的**语法层**：文本 → 记号 → 语法树 → 生成的 Rust 源码。
//!
//! ## 分层理由（与 `grammar.rs` 同源）
//!
//! 过程宏 crate 的单元测试**不能**调用 proc-macro API（实测：单元测试里用
//! `TokenStream` 会 panic「procedural macro API is used outside of a
//! procedural macro」）。因此这里把整个语法层做成**纯文本函数**：
//! proc-macro 层只做一件事 —— 把记号流 `to_string()` 成文本，然后交给本文件。
//!
//! 好处：测试可以直接喂**用户写的源码文本**（`view! { div(class = "a") { "x" } }`），
//! 断言生成的 Rust 源码 —— 既测行为又测得细。
//!
//! ## 文法（设计文档 §6.5；R1 提案由施工线落地）
//!
//! ```text
//! view! {
//!     div
//!     div(class = "a b")
//!     div(class:open = is_open)          // class token 条件增删
//!     input(type = "text", disabled = is_disabled)
//!     button(onclick = move |_| { … })    // 事件
//!     canvas(bind:this = canvas_ref)      // 元素引用绑定
//!     li(key = item.id)                   // 列表身份
//!     div { "文本" }                      // 字面量文本
//!     div { expr }                        // 动态文本（Display）
//!     div { { expr } }                    // 动态子节点
//!     div { Child(arg = v) }              // 子组件
//!     div { Child(arg = v) { "内容" } }    // 子组件 + 子内容
//!     if cond { … } else { … }
//!     for item in items { … }
//! }
//! ```
//!
//! ## 表达式怎么取
//!
//! **按源码片段取**（不是把记号重新拼一遍）：记号携带源区间 ⇒ 表达式取
//! 「首记号起点 → 末记号终点」的**原文**。这样
//! - 字符串字面量的转义原样保留（由 Rust 编译器处理）；
//! - 生命周期（`'static`）、泛型、闭包体一律逐字保留；
//! - 生成物与用户写的东西**逐字一致**（可复核）。

use std::fmt::Write as _;

/// 记号种类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokKind {
    /// 标识符（含 `r#` 原始标识符）。
    Ident(String),
    /// 标点（单字符）。
    Punct(char),
    /// 字面量（字符串 / 字符；**含引号的原文**）。
    Literal(String),
    /// 分组（圆括号 / 方括号 / 花括号）。
    Group {
        /// 左括号字符。
        open: char,
        /// 右括号字符。
        close: char,
        /// 组内记号。
        tokens: Vec<Tok>,
    },
}

/// 记号（带源区间 ⇒ 表达式可按原文取）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tok {
    /// 种类。
    pub(crate) kind: TokKind,
    /// 源文本起点（字节下标）。
    pub(crate) start: usize,
    /// 源文本终点（字节下标，**不含**）。
    pub(crate) end: usize,
}

impl Tok {
    /// 标识符文本。
    fn ident(&self) -> Option<&str> {
        match &self.kind {
            TokKind::Ident(text) => Some(text),
            _ => None,
        }
    }
}

/// 把源文本切成记号。
pub(crate) fn lex(text: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = text.chars().collect();
    // 字符下标 ↔ 字节下标 的映射（表达式按字节切，故需要）。
    let mut byte_offsets: Vec<usize> = Vec::with_capacity(chars.len() + 1);
    let mut offset = 0usize;
    for ch in &chars {
        byte_offsets.push(offset);
        offset += ch.len_utf8();
    }
    byte_offsets.push(offset);

    let mut pos = 0usize;
    let mut stack: Vec<(char, Vec<Tok>, usize)> = Vec::new();
    let mut top: Vec<Tok> = Vec::new();

    while pos < chars.len() {
        let ch = chars[pos];
        if ch.is_whitespace() {
            pos += 1;
            continue;
        }
        let start = pos;
        match ch {
            '"' => {
                pos += 1;
                while pos < chars.len() {
                    if chars[pos] == '\\' {
                        pos += 2;
                        continue;
                    }
                    if chars[pos] == '"' {
                        pos += 1;
                        break;
                    }
                    pos += 1;
                }
                if pos > chars.len() {
                    return Err("字符串字面量未闭合".to_owned());
                }
                let literal: String = chars[start..pos].iter().collect();
                top.push(Tok {
                    kind: TokKind::Literal(literal),
                    start: byte_offsets[start],
                    end: byte_offsets[pos],
                });
            }
            '\'' => {
                // 字符字面量（`'a'` / `'\n'`）与生命周期（`'static`）要分清：
                // 形如 `'X'`（正好三个字符）的是字符字面量，其余按标点处理。
                let is_char_literal = if pos + 2 < chars.len() && chars[pos + 1] == '\\' {
                    let mut scan = pos + 2;
                    while scan < chars.len() && chars[scan] != '\'' {
                        scan += 1;
                    }
                    scan < chars.len() && scan > pos + 1
                } else {
                    pos + 2 < chars.len() && chars[pos + 2] == '\''
                };
                if is_char_literal {
                    let mut scan = pos + 1;
                    while scan < chars.len() && chars[scan] != '\'' {
                        scan += 1;
                    }
                    scan += 1;
                    let literal: String = chars[pos..scan].iter().collect();
                    top.push(Tok {
                        kind: TokKind::Literal(literal),
                        start: byte_offsets[pos],
                        end: byte_offsets[scan],
                    });
                    pos = scan;
                } else {
                    top.push(Tok {
                        kind: TokKind::Punct('\''),
                        start: byte_offsets[pos],
                        end: byte_offsets[pos + 1],
                    });
                    pos += 1;
                }
            }
            '(' | '[' | '{' => {
                stack.push((ch, std::mem::take(&mut top), start));
                pos += 1;
            }
            ')' | ']' | '}' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                let Some((open, outer, open_at)) = stack.pop() else {
                    return Err(format!("多余的右括号 `{ch}`"));
                };
                if open != expected {
                    return Err(format!("括号不匹配：`{open}` 对上了 `{ch}`"));
                }
                // 组内记号 = 打开括号**之后**累积的那些；`outer` 是组外的记号
                // （打开括号之前累积的）⇒ 还原回去。
                let inner = std::mem::take(&mut top);
                top = outer;
                top.push(Tok {
                    kind: TokKind::Group {
                        open,
                        close: ch,
                        tokens: inner,
                    },
                    start: byte_offsets[open_at],
                    end: byte_offsets[pos + 1],
                });
                pos += 1;
            }
            _ if ch.is_alphanumeric() || ch == '_' => {
                if ch == 'r' && pos + 1 < chars.len() && chars[pos + 1] == '#' {
                    pos += 2;
                }
                while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                    pos += 1;
                }
                let ident: String = chars[start..pos].iter().collect();
                top.push(Tok {
                    kind: TokKind::Ident(ident),
                    start: byte_offsets[start],
                    end: byte_offsets[pos],
                });
            }
            _ => {
                top.push(Tok {
                    kind: TokKind::Punct(ch),
                    start: byte_offsets[start],
                    end: byte_offsets[pos + 1],
                });
                pos += 1;
            }
        }
    }
    if !stack.is_empty() {
        return Err("括号未闭合".to_owned());
    }
    Ok(top)
}

/// 语法树节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Node {
    /// 元素。
    Element {
        /// 标签名。
        tag: String,
        /// 属性。
        attrs: Vec<Attr>,
        /// 子节点。
        children: Vec<Node>,
    },
    /// 子组件（首字母大写的标识符）。
    Component {
        /// 组件名。
        name: String,
        /// 构造参数（按名赋值）。
        args: Vec<Attr>,
        /// 子内容（`None` ⇒ 没写子内容块）。
        children: Option<Vec<Node>>,
    },
    /// 字面量文本（含引号的原文）。
    Text {
        /// 原文。
        literal: String,
    },
    /// 动态文本（`Display`）。
    DynText {
        /// 表达式原文。
        expr: String,
    },
    /// 动态子节点（产物是虚拟节点）。
    DynChild {
        /// 表达式原文。
        expr: String,
    },
    /// 条件块。
    If {
        /// 条件表达式原文。
        cond: String,
        /// 命中分支。
        then_branch: Vec<Node>,
        /// 未命中分支（`None` ⇒ 没写 `else`）。
        else_branch: Option<Vec<Node>>,
    },
    /// 列表块。
    For {
        /// 绑定模式原文。
        pattern: String,
        /// 被迭代表达式原文。
        iter: String,
        /// 循环体。
        body: Vec<Node>,
    },
}

/// 属性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Attr {
    /// 普通属性。
    Plain {
        /// 属性名。
        name: String,
        /// 值表达式原文。
        value: String,
    },
    /// class 整体赋值。
    Class {
        /// 值表达式原文（字符串字面量）。
        value: String,
    },
    /// 条件 class token。
    ClassCond {
        /// token 名。
        token: String,
        /// 条件表达式原文。
        value: String,
    },
    /// 列表身份。
    Key {
        /// 值表达式原文。
        value: String,
    },
    /// 事件处理器。
    Handler {
        /// 事件类型（Rust 枚举变体名）。
        kind: &'static str,
        /// 处理器表达式原文。
        value: String,
    },
    /// 元素引用绑定（`bind:this`）。
    This {
        /// 值表达式原文。
        value: String,
    },
    /// **property** 赋值（`prop:<名>` / `bind:value` / `bind:checked`）。
    ///
    /// ⚠️ property 与属性是**两条独立通道**：写 attribute 对 `input.value` 无效。
    Prop {
        /// property 名。
        name: String,
        /// 值表达式原文。
        value: String,
    },
}

/// HTML 标签白名单（判定「裸标识符是元素还是表达式」用）。
///
/// 只收**常用**标签：白名单越大，动态文本的变量名被误认成标签的概率越高。
const HTML_TAGS: &[&str] = &[
    "a",
    "article",
    "aside",
    "b",
    "blockquote",
    "br",
    "button",
    "canvas",
    "caption",
    "code",
    "col",
    "colgroup",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "img",
    "input",
    "kbd",
    "label",
    "legend",
    "li",
    "main",
    "mark",
    "nav",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "path",
    "pre",
    "progress",
    "q",
    "rect",
    "s",
    "samp",
    "section",
    "select",
    "small",
    "span",
    "strong",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

/// 判定一个名字是不是 HTML 标签（白名单）。
fn is_html_tag(name: &str) -> bool {
    HTML_TAGS.contains(&name)
}

/// 支持的事件类型（DOM 事件名 → Rust 枚举变体）。
const EVENT_KINDS: &[(&str, &str)] = &[
    ("onclick", "Click"),
    ("onkeydown", "KeyDown"),
    ("oninput", "Input"),
    ("onchange", "Change"),
    ("onfocusout", "FocusOut"),
];

/// 解析整个 `view!` 体（文本 → 语法树）。
pub(crate) fn parse(text: &str) -> Result<Vec<Node>, String> {
    let tokens = lex(text)?;
    let mut cursor = Cursor {
        source: text,
        tokens: &tokens,
        pos: 0,
    };
    let nodes = cursor.parse_nodes(&[])?;
    if !cursor.at_end() {
        return Err(format!(
            "多余的记号：`{}`",
            cursor.source_slice(cursor.pos..cursor.tokens.len())
        ));
    }
    Ok(nodes)
}

/// 记号游标。
struct Cursor<'a> {
    /// 源文本（表达式按**原文**取，不重新拼记号）。
    source: &'a str,
    /// 记号。
    tokens: &'a [Tok],
    /// 当前位置。
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// 源文本片段（按记号下标区间；**逐字**取自源文本）。
    fn slice(&self, from: usize, to: usize) -> String {
        if from >= to || to > self.tokens.len() {
            return String::new();
        }
        let start = self.tokens[from].start;
        let end = self.tokens[to - 1].end;
        self.source[start..end].to_owned()
    }

    /// 分组内部的源文本（去掉两侧的括号）。
    fn group_inner(&self, group: &Tok) -> String {
        let TokKind::Group { open, close, .. } = &group.kind else {
            return String::new();
        };
        let open_len = open.len_utf8();
        let close_len = close.len_utf8();
        self.source[group.start + open_len..group.end - close_len].to_owned()
    }
}

impl<'a> Cursor<'a> {
    /// 是否已到末尾。
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// 当前记号。
    fn peek(&self) -> Option<&'a Tok> {
        self.tokens.get(self.pos)
    }

    /// 源文本片段（按字节区间）。
    fn source_slice(&self, range: std::ops::Range<usize>) -> String {
        // 记号自带区间，但本函数只用于「把剩余记号拼回去给人看」⇒
        // 用记号的 kind 还原成可读文本。
        let mut out = String::new();
        for token in &self.tokens[range] {
            match &token.kind {
                TokKind::Ident(text) => {
                    let _ = write!(out, "{text} ");
                }
                TokKind::Punct(ch) => {
                    let _ = write!(out, "{ch} ");
                }
                TokKind::Literal(text) => {
                    let _ = write!(out, "{text} ");
                }
                TokKind::Group { open, close, .. } => {
                    let _ = write!(out, "{open} … {close} ");
                }
            }
        }
        out.trim().to_owned()
    }

    /// 解析节点序列（到 `stop` 为止；`stop` 为空 ⇒ 到末尾）。
    fn parse_nodes(&mut self, stop: &[&str]) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        while let Some(token) = self.peek() {
            if let Some(ident) = token.ident() {
                if stop.contains(&ident) {
                    return Ok(nodes);
                }
            }
            let node = self.parse_node()?;
            nodes.push(node);
        }
        Ok(nodes)
    }

    /// 解析一个节点。
    ///
    /// ## 元素 vs 表达式（歧义的判定规则）
    ///
    /// 子内容位置上，一个**裸标识符**既可能是元素标签，也可能是动态文本
    /// （设计文档 §6.5：`div { expr }` 是动态文本）。判定规则（**确定、可复核**）：
    ///
    /// | 情形 | 判定 |
    /// |---|---|
    /// 首字母大写 | **组件** |
    /// 后面紧跟 `(` 或 `{` | **元素 / 组件**（有属性或子内容） |
    /// 在 [`HTML_TAGS`] 白名单里 | **元素** |
    /// 其余 | **动态文本**（表达式） |
    ///
    /// 白名单之外的名字一律当表达式 ⇒ 动态文本里的变量名不会被误认成标签；
    /// 代价是「想用白名单外的自定义标签」不行（写 `div(class = "...")` 即可）。
    fn parse_node(&mut self) -> Result<Node, String> {
        let Some(token) = self.peek() else {
            return Err("节点缺失".to_owned());
        };
        match &token.kind {
            TokKind::Ident(ident) => {
                let ident = ident.clone();
                match ident.as_str() {
                    "if" => self.parse_if(),
                    "for" => self.parse_for(),
                    _ => {
                        let component = is_component_name(&ident);
                        let next_kind = self.tokens.get(self.pos + 1).map(|token| &token.kind);
                        let followed_by_brace = matches!(
                            next_kind,
                            Some(TokKind::Group { open, .. }) if *open == '{'
                        );
                        let followed_by_paren = matches!(
                            next_kind,
                            Some(TokKind::Group { open, .. }) if *open == '('
                        );
                        let known_tag = is_html_tag(&ident);
                        if !component
                            && !followed_by_brace
                            && !(followed_by_paren && known_tag)
                            && !known_tag
                        {
                            // 裸标识符且不是标签 ⇒ 动态文本表达式（**整段**表达式
                            // 都要吃掉：`a.b().c()` 是一个表达式，不是三个节点）。
                            let expr = self.consume_expression()?;
                            return Ok(Node::DynText { expr });
                        }
                        self.pos += 1;
                        let attrs = self.parse_optional_attrs()?;
                        let children = self.parse_optional_children()?;
                        if component {
                            Ok(Node::Component {
                                name: ident,
                                args: attrs,
                                children,
                            })
                        } else {
                            Ok(Node::Element {
                                tag: ident,
                                attrs,
                                children: children.unwrap_or_default(),
                            })
                        }
                    }
                }
            }
            TokKind::Literal(literal) => {
                let literal = literal.clone();
                self.pos += 1;
                Ok(Node::Text { literal })
            }
            TokKind::Group { open, tokens, .. } if *open == '{' => {
                // `{ expr }` ⇒ 动态**子节点**（产物是虚拟节点）；动态**文本**
                // 写成裸表达式（见上面的判定规则）。
                let expr = self.group_inner(token);
                self.pos += 1;
                if expr.is_empty() {
                    return Err("动态子节点缺表达式".to_owned());
                }
                Ok(Node::DynChild { expr })
            }
            TokKind::Punct(ch) => Err(format!("意外的标点 `{ch}`")),
            TokKind::Group { open, .. } => Err(format!("意外的分组（`{open}`）")),
        }
    }

    /// 解析可选的属性列表（`(...)`）。
    fn parse_optional_attrs(&mut self) -> Result<Vec<Attr>, String> {
        let Some(token) = self.peek() else {
            return Ok(Vec::new());
        };
        let TokKind::Group { open, tokens, .. } = &token.kind else {
            return Ok(Vec::new());
        };
        if *open != '(' {
            return Ok(Vec::new());
        }
        let tokens = tokens.clone();
        self.pos += 1;
        parse_attrs(&tokens, self.source)
    }

    /// 解析可选的子内容块（`{ ... }`）。
    fn parse_optional_children(&mut self) -> Result<Option<Vec<Node>>, String> {
        let Some(token) = self.peek() else {
            return Ok(None);
        };
        let TokKind::Group { open, tokens, .. } = &token.kind else {
            return Ok(None);
        };
        if *open != '{' {
            return Ok(None);
        }
        let tokens = tokens.clone();
        self.pos += 1;
        let mut cursor = Cursor {
            source: self.source,
            tokens: &tokens,
            pos: 0,
        };
        let nodes = cursor.parse_nodes(&[])?;
        Ok(Some(nodes))
    }

    /// 解析条件块。
    fn parse_if(&mut self) -> Result<Node, String> {
        self.pos += 1; // `if`
        let cond = self.consume_expression()?;
        let then_branch = self.parse_optional_children()?.unwrap_or_default();
        let mut else_branch = None;
        if let Some(token) = self.peek() {
            if token.ident() == Some("else") {
                self.pos += 1;
                else_branch = Some(self.parse_optional_children()?.unwrap_or_default());
            }
        }
        Ok(Node::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    /// 解析列表块。
    fn parse_for(&mut self) -> Result<Node, String> {
        self.pos += 1; // `for`
        let pattern = self.consume_expression()?;
        if self.peek().and_then(|token| token.ident()) != Some("in") {
            return Err("`for` 语法：`<模式> in <可迭代对象> { … }`".to_owned());
        }
        self.pos += 1; // `in`
        let iter = self.consume_expression()?;
        let body = self.parse_optional_children()?.unwrap_or_default();
        Ok(Node::For {
            pattern,
            iter,
            body,
        })
    }

    /// 吃掉**一整段表达式**（返回其源文本）。
    ///
    /// 支持的形态（够用且可判定）：
    /// - 标识符：`text`
    /// - 括号组：`(key, value)`（**元组模式** / 括号表达式）⇒ 原文透传
    /// - 方法链 / 字段链：`a.b.c`、`a.b()`、`a.b()?.c()`
    /// - 直接调用：`format!("…")`
    /// - 前置 `!` / `-`：`!flag`
    ///
    /// 遇到**其它**形态（二元运算符、闭包字面量等）⇒ 停止并把剩下的记号留给
    /// 调用方 ⇒ 调用方会给出明确错误（不静默吞掉）。
    fn consume_expression(&mut self) -> Result<String, String> {
        let start = self.pos;
        // 前置一元运算符。
        while let Some(token) = self.peek() {
            match token.kind {
                TokKind::Punct('!') | TokKind::Punct('-') => self.pos += 1,
                _ => break,
            }
        }
        // 主体：**括号组**（元组模式 / 括号表达式）或标识符。
        if let Some(TokKind::Group { open, .. }) = self.tokens.get(self.pos).map(|t| &t.kind) {
            if *open == '(' {
                // 整组透传（`(key, value)` ⇒ 模式原文；`(expr)` ⇒ 表达式原文）。
                self.pos += 1;
                return Ok(self.slice(start, self.pos));
            }
        }
        match self.peek().map(|token| token.ident()) {
            Some(Some(_)) => self.pos += 1,
            _ => return Err("表达式缺失（期望一个标识符或括号组）".to_owned()),
        }
        // 直接调用：`name(...)`。
        if let Some(TokKind::Group { open, .. }) = self.tokens.get(self.pos).map(|t| &t.kind) {
            if *open == '(' {
                self.pos += 1;
                return Ok(self.slice(start, self.pos));
            }
        }
        // 链式：` .name` / `.name(...)` / `.name(...)?`
        loop {
            let is_dot = matches!(
                self.tokens.get(self.pos).map(|token| &token.kind),
                Some(TokKind::Punct('.'))
            );
            if !is_dot {
                break;
            }
            let is_ident = matches!(
                self.tokens.get(self.pos + 1).map(|token| token.ident()),
                Some(Some(_))
            );
            if !is_ident {
                break;
            }
            self.pos += 2;
            if let Some(TokKind::Group { open, .. }) = self.tokens.get(self.pos).map(|t| &t.kind) {
                if *open == '(' {
                    self.pos += 1;
                }
            }
            if matches!(
                self.tokens.get(self.pos).map(|token| &token.kind),
                Some(TokKind::Punct('?'))
            ) {
                self.pos += 1;
            }
        }
        Ok(self.slice(start, self.pos))
    }
}

/// 组件名判定（首字母大写 ⇒ 组件）。
fn is_component_name(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_uppercase())
        .unwrap_or(false)
}

/// 解析属性列表。
fn parse_attrs(tokens: &[Tok], source: &str) -> Result<Vec<Attr>, String> {
    let mut attrs = Vec::new();
    for chunk in split_top_level(tokens) {
        if chunk.is_empty() {
            continue;
        }
        attrs.push(parse_attr(&chunk, source)?);
    }
    Ok(attrs)
}

/// 按顶层逗号切分。
fn split_top_level(tokens: &[Tok]) -> Vec<Vec<Tok>> {
    let mut chunks: Vec<Vec<Tok>> = Vec::new();
    let mut current: Vec<Tok> = Vec::new();
    for token in tokens {
        if matches!(token.kind, TokKind::Punct(',')) {
            chunks.push(std::mem::take(&mut current));
            continue;
        }
        current.push(token.clone());
    }
    chunks.push(current);
    chunks
}

/// 解析单个属性。
fn parse_attr(tokens: &[Tok], source: &str) -> Result<Attr, String> {
    let Some(first) = tokens.first() else {
        return Err("属性为空".to_owned());
    };
    let mut name = match &first.kind {
        TokKind::Ident(text) => text.clone(),
        TokKind::Literal(text) => text.trim_matches('"').to_owned(),
        other => return Err(format!("属性名非法：{other:?}")),
    };
    let mut index = 1usize;
    // 连字符属性名（`aria-label` / `aria-expanded` / `data-*`）⇒ 逐段拼回。
    while let (
        Some(Tok {
            kind: TokKind::Punct('-'),
            ..
        }),
        Some(Tok {
            kind: TokKind::Ident(text),
            ..
        }),
    ) = (tokens.get(index), tokens.get(index + 1))
    {
        name.push('-');
        name.push_str(text);
        index += 2;
    }
    let mut sub: Option<String> = None;
    if let Some(Tok {
        kind: TokKind::Punct(':'),
        ..
    }) = tokens.get(index)
    {
        let Some(Tok {
            kind: TokKind::Ident(text),
            ..
        }) = tokens.get(index + 1)
        else {
            return Err(format!("属性 `{name}` 的 `:` 后面缺名字"));
        };
        sub = Some(text.clone());
        index += 2;
    }
    let mut value: Option<String> = None;
    if let Some(Tok {
        kind: TokKind::Punct('='),
        ..
    }) = tokens.get(index)
    {
        index += 1;
        if index >= tokens.len() {
            return Err(format!("属性 `{name}` 缺值"));
        }
        let start = tokens[index].start;
        let end = tokens[tokens.len() - 1].end;
        value = Some(source[start..end].to_owned());
    }

    match (name.as_str(), sub.as_deref()) {
        ("class", None) => {
            let value = value.ok_or_else(|| "属性 `class` 缺值".to_owned())?;
            Ok(Attr::Class { value })
        }
        ("class", Some(token)) => {
            let value = value.ok_or_else(|| "属性 `class:token` 缺值".to_owned())?;
            Ok(Attr::ClassCond {
                token: token.to_owned(),
                value,
            })
        }
        ("key", None) => {
            let value = value.ok_or_else(|| "属性 `key` 缺值".to_owned())?;
            Ok(Attr::Key { value })
        }
        ("bind", Some("this")) => {
            let value = value.ok_or_else(|| "属性 `bind:this` 缺值".to_owned())?;
            Ok(Attr::This { value })
        }
        // `bind:value` / `bind:checked` ⇒ **property 赋值**（Svelte 的同口径：
        // 对原生控件写 property，不写 attribute）。
        ("bind", Some(name @ ("value" | "checked"))) => {
            let value = value.ok_or_else(|| format!("属性 `bind:{name}` 缺值"))?;
            Ok(Attr::Prop {
                name: name.to_owned(),
                value,
            })
        }
        // 任意 property：`prop:value = expr` / `prop:checked = expr`。
        ("prop", Some(name)) => {
            let value = value.ok_or_else(|| format!("属性 `prop:{name}` 缺值"))?;
            Ok(Attr::Prop {
                name: name.to_owned(),
                value,
            })
        }
        ("bind", Some(other)) => Err(format!(
            "双向绑定 `bind:{other}` **尚未接线** —— 本批支持 `bind:this` / \
             `bind:value` / `bind:checked`（后两者写 property）；\
             其它 property 用 `prop:{other} = <表达式>`"
        )),
        (name, None) if name.starts_with("on") => {
            let value = value.ok_or_else(|| format!("事件属性 `{name}` 缺值"))?;
            let kind = EVENT_KINDS
                .iter()
                .find(|(event, _)| *event == name)
                .map(|(_, kind)| *kind)
                .ok_or_else(|| {
                    format!(
                        "不支持的事件 `{name}`；本批支持：{}",
                        EVENT_KINDS
                            .iter()
                            .map(|(event, _)| *event)
                            .collect::<Vec<_>>()
                            .join(" / ")
                    )
                })?;
            Ok(Attr::Handler { kind, value })
        }
        _ => {
            let value = value.ok_or_else(|| format!("属性 `{name}` 缺值"))?;
            Ok(Attr::Plain { name, value })
        }
    }
}

// ── 代码生成 ────────────────────────────────────────────────────────────────

/// 虚拟节点运行时的绝对路径前缀。
const VNODE: &str = "::apeireth_ui_dom::VNode";

/// 生成节点序列的 Rust 表达式。
pub(crate) fn generate(nodes: &[Node]) -> Result<String, String> {
    let mut parts = Vec::with_capacity(nodes.len());
    for node in nodes {
        parts.push(generate_node(node)?);
    }
    match parts.len() {
        0 => Ok(format!("{VNODE}::fragment(::std::vec::Vec::new())")),
        1 => Ok(parts.remove(0)),
        _ => Ok(format!(
            "{VNODE}::fragment(::std::vec![{}])",
            parts.join(", ")
        )),
    }
}

/// 生成单个节点的 Rust 表达式。
fn generate_node(node: &Node) -> Result<String, String> {
    match node {
        Node::Text { literal } => Ok(format!("{VNODE}::text({literal})")),
        Node::DynText { expr } => Ok(format!(
            "{VNODE}::text(::std::string::ToString::to_string(&({expr})))"
        )),
        Node::DynChild { expr } => Ok(format!("({expr})")),
        Node::Element {
            tag,
            attrs,
            children,
        } => generate_element(tag, attrs, children),
        Node::Component {
            name,
            args,
            children,
        } => generate_component(name, args, children.as_deref()),
        Node::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let then_code = generate(then_branch)?;
            let else_code = match else_branch {
                Some(branch) => generate(branch)?,
                None => format!("{VNODE}::fragment(::std::vec::Vec::new())"),
            };
            Ok(format!(
                "{VNODE}::fragment(::std::vec![if ({cond}) {{ {then_code} }} else {{ {else_code} }}])"
            ))
        }
        Node::For {
            pattern,
            iter,
            body,
        } => {
            let body_code = generate(body)?;
            Ok(format!(
                "{VNODE}::fragment(({iter}).into_iter().map(|{pattern}| {{ {body_code} }})\
                 .collect::<::std::vec::Vec<_>>())"
            ))
        }
    }
}

/// 生成元素节点的 Rust 表达式。
fn generate_element(tag: &str, attrs: &[Attr], children: &[Node]) -> Result<String, String> {
    let mut code = String::new();
    let _ = write!(code, "{{ let mut __n = {VNODE}::element({tag:?});");

    // class：多个来源合成一个 token 集合（保序去重由后端负责）。
    let class_literals: Vec<&str> = attrs
        .iter()
        .filter_map(|attr| match attr {
            Attr::Class { value } => Some(value.as_str()),
            _ => None,
        })
        .collect();
    let class_conds: Vec<(&str, &str)> = attrs
        .iter()
        .filter_map(|attr| match attr {
            Attr::ClassCond { token, value } => Some((token.as_str(), value.as_str())),
            _ => None,
        })
        .collect();
    if !class_literals.is_empty() || !class_conds.is_empty() {
        code.push_str(
            "__n = __n.attr(\"class\", { let mut __c: ::std::vec::Vec<&str> = ::std::vec::Vec::new();",
        );
        for literal in &class_literals {
            let _ = write!(code, " __c.push({literal});");
        }
        for (token, value) in &class_conds {
            let _ = write!(code, " if ({value}) {{ __c.push({token:?}); }}");
        }
        code.push_str(" __c.join(\" \") });");
    }

    for attr in attrs {
        match attr {
            Attr::Plain { name, value } => {
                let _ = write!(code, " __n = __n.attr({name:?}, {value});");
            }
            Attr::Prop { name, value } => {
                let _ = write!(code, " __n = __n.prop({name:?}, {value});");
            }
            Attr::Key { value } => {
                let _ = write!(code, " __n = __n.key({value});");
            }
            Attr::Handler { kind, value } => {
                let _ = write!(
                    code,
                    " __n = __n.handler(::apeireth_ui_dom::EventKind::{kind}, \
                     ::apeireth_ui_dom::ElementHandler::new({value}));"
                );
            }
            Attr::This { value } => {
                let _ = write!(code, " __n = __n.this({value});");
            }
            Attr::Class { .. } | Attr::ClassCond { .. } => {}
        }
    }

    for child in children {
        let child_code = generate_node(child)?;
        let _ = write!(code, " __n = __n.child({child_code});");
    }
    code.push_str(" __n.build() }");
    Ok(code)
}

/// 生成子组件节点的 Rust 表达式。
fn generate_component(
    name: &str,
    args: &[Attr],
    children: Option<&[Node]>,
) -> Result<String, String> {
    let mut setters = String::new();
    for arg in args {
        match arg {
            Attr::Plain { name: field, value } => {
                let _ = write!(setters, ".{field}({value})");
            }
            Attr::Prop { name: field, value } => {
                // property 不是属性 ⇒ props 结构体里用 `prop_<名>` 方法（不占属性名）。
                let _ = write!(setters, ".prop_{field}({value})");
            }
            Attr::Key { value } => {
                let _ = write!(setters, ".key({value})");
            }
            Attr::Handler { kind, value } => {
                let _ = write!(
                    setters,
                    ".handler(::apeireth_ui_dom::EventKind::{kind}, \
                     ::apeireth_ui_dom::ElementHandler::new({value}))"
                );
            }
            Attr::This { value } => {
                let _ = write!(setters, ".this({value})");
            }
            Attr::Class { value } => {
                let _ = write!(setters, ".class_name({value})");
            }
            Attr::ClassCond { token, value } => {
                let _ = write!(setters, ".class_cond({token:?}, {value})");
            }
        }
    }
    if let Some(children) = children {
        let children_code = generate(children)?;
        let _ = write!(
            setters,
            ".children(::apeireth_ui_dom::Children::new(move || {{ {children_code} }}))"
        );
    }
    Ok(format!(
        "{{ let __c = {name}::builder(){setters}.build(); \
         let __v: {VNODE} = <{name} as ::apeireth_ui_core::UiComponent>::render(__c.props()); \
         __v }}"
    ))
}

#[cfg(test)]
mod tests {
    //! 语法层的单元断言（**生成物快照**：比对宏自己生成的代码，属设计文档
    //! §10.2 的 E1 例外——对象是生成物，不是被测业务源码）。

    use super::*;

    /// 生成代码（解析 + 生成）。
    fn gen(text: &str) -> String {
        let nodes = parse(text).expect("解析必须成功");
        generate(&nodes).expect("生成必须成功")
    }

    #[test]
    fn plain_element_generates_an_element_constructor() {
        let code = gen("div");
        assert!(code.contains("VNode::element(\"div\")"), "实测：{code}");
        assert!(code.contains("__n.build()"), "实测：{code}");
    }

    #[test]
    fn class_sources_are_merged_into_one_token_set() {
        let code = gen("span(class = \"a b\", class:open = flag)");
        assert!(
            code.contains("__c.push(\"a b\")")
                && code.contains("if (flag) { __c.push(\"open\"); }"),
            "实测：{code}"
        );
        assert!(code.contains("__c.join(\" \")"), "实测：{code}");
    }

    #[test]
    fn event_attribute_maps_to_the_event_kind() {
        let code = gen("button(onclick = move |_| {})");
        assert!(
            code.contains("EventKind::Click") && code.contains("ElementHandler::new"),
            "实测：{code}"
        );
    }

    #[test]
    fn unknown_event_is_rejected_with_the_supported_list() {
        let err = parse("button(onhover = move |_| {})").expect_err("必须报错");
        assert!(err.contains("onclick"), "实测错误文案：{err}");
    }

    #[test]
    fn bind_value_now_parses_as_a_property() {
        // ⚠️ 本测试原先断言「`bind:value` **必须报错**（property 回写未实现）」。
        // 渲染器的 property 通道接通后该约束**不再成立** ⇒ 反转成正向断言：
        // `bind:value` ⇒ **property**（不是属性）。
        let code = gen("input(bind:value = name)");
        assert!(
            code.contains(".prop(\"value\", name)"),
            "bind:value 必须生成 property 写入，实测：{code}"
        );
        assert!(
            !code.contains(".attr(\"value\""),
            "bind:value **不得**生成属性写入（写 attribute 对 input.value 无效）：{code}"
        );
        // `bind:checked` ⇒ 同样走 property（布尔由 `From<bool>` 转 PropValue）。
        let checked = gen("input(bind:checked = flag)");
        assert!(
            checked.contains(".prop(\"checked\", flag)"),
            "bind:checked 必须生成 property 写入，实测：{checked}"
        );
        // 任意 property：`prop:<名>`。
        let other = gen("input(prop:indeterminate = half)");
        assert!(
            other.contains(".prop(\"indeterminate\", half)"),
            "prop:<名> 必须生成 property 写入，实测：{other}"
        );
        // 不认识的 `bind:` 仍然报错（不静默放过）。
        let err = parse("input(bind:group = g)").expect_err("未知 bind 必须报错");
        assert!(err.contains("尚未接线"), "实测错误文案：{err}");
    }

    #[test]
    fn bind_this_is_supported() {
        let code = gen("canvas(bind:this = slot)");
        assert!(code.contains("__n.this(slot)"), "实测：{code}");
    }

    #[test]
    fn bare_identifier_is_dynamic_text_unless_it_is_a_tag() {
        let code = gen("span { text_value }");
        assert!(
            code.contains("ToString::to_string(&(text_value))"),
            "实测：{code}"
        );
        let element = gen("span { strong }");
        assert!(
            element.contains("VNode::element(\"strong\")"),
            "实测：{element}"
        );
    }

    #[test]
    fn braced_expression_is_a_dynamic_child_node() {
        let code = gen("div { { inner } }");
        // 表达式按**源文本原样**取（不重新拼记号）⇒ 空白与原文一致。
        assert!(code.contains("__n.child(( inner ))"), "实测：{code}");
    }

    #[test]
    fn expressions_are_taken_verbatim_from_the_source() {
        // `::` 与闭包体必须逐字保留（重新拼记号会把 `::` 拆成 `: :`）。
        let code = gen("span { value.clone().unwrap_or_default() }");
        assert!(
            code.contains("value.clone().unwrap_or_default()"),
            "实测：{code}"
        );
    }

    #[test]
    fn if_block_generates_a_fragment_with_both_branches() {
        let code = gen("div { if cond { span { \"a\" } } else { span { \"b\" } } }");
        assert!(code.contains("if (cond)"), "实测：{code}");
        assert!(code.contains("else"), "实测：{code}");
        assert!(code.contains("VNode::fragment"), "实测：{code}");
    }

    #[test]
    fn for_block_maps_over_the_iterator() {
        let code = gen("ul { for item in items { li { item } } }");
        assert!(
            code.contains("items).into_iter().map(|item|"),
            "实测：{code}"
        );
        assert!(
            code.contains("collect::<::std::vec::Vec<_>>()"),
            "实测：{code}"
        );
    }

    #[test]
    fn tuple_patterns_are_passed_through_verbatim() {
        let code = gen("ul { for (key, value) in entries { li { key } } }");
        assert!(
            code.contains("entries).into_iter().map(|(key, value)|"),
            "实测：{code}"
        );
    }

    #[test]
    fn hyphenated_attribute_names_are_joined_back() {
        let code = gen("div(aria-expanded = flag, aria-label = \"收起详情\") { \"x\" }");
        assert!(
            code.contains("__n.attr(\"aria-expanded\", flag)"),
            "实测：{code}"
        );
        assert!(
            code.contains("__n.attr(\"aria-label\", \"收起详情\")"),
            "实测：{code}"
        );
    }

    #[test]
    fn component_call_uses_the_builder() {
        let code = gen("div { Child(title = \"x\") }");
        assert!(code.contains("Child::builder()"), "实测：{code}");
        assert!(code.contains(".title(\"x\")"), "实测：{code}");
        assert!(
            code.contains("<Child as ::apeireth_ui_core::UiComponent>::render"),
            "实测：{code}"
        );
    }

    #[test]
    fn component_with_children_becomes_a_children_slot() {
        let code = gen("div { Child(title = \"x\") { span { \"y\" } } }");
        assert!(code.contains("Children::new(move ||"), "实测：{code}");
        assert!(code.contains(".children("), "实测：{code}");
    }

    #[test]
    fn unclosed_brace_is_reported() {
        let err = parse("div { span").expect_err("必须报错");
        assert!(err.contains("括号未闭合"), "实测错误文案：{err}");
    }

    #[test]
    fn mismatched_brackets_are_reported() {
        let err = parse("div(class = \"a\") ]").expect_err("必须报错");
        assert!(err.contains("括号"), "实测错误文案：{err}");
    }

    #[test]
    fn string_literal_with_braces_stays_intact() {
        let code = gen("span { \"a{b}c\" }");
        assert!(code.contains("\"a{b}c\""), "实测：{code}");
    }
}
