//! # apeireth-ui-macro — 过程宏（`#[component]` + `view!`）
//!
//! 引擎的最底层 crate：**零内部依赖、零外部依赖**（只用 std 的 `proc-macro`）。
//! 职责边界（设计文档 §3.3.3）：只做「**语法 → 语法**」的翻译，**不产生运行时
//! 逻辑**；宏展开后需要的运行时辅助代码一律住在 `ui-core` / `ui-dom`。
//!
//! ## ✅ 已实现（P3）
//!
//! ### `#[component]` —— 「函数即组件」
//!
//! ```ignore
//! #[component]
//! fn Card(
//!     /// 待签文书
//!     request: ApprovalRequest,
//!     /// 紧凑排版
//!     #[prop(default = false)]
//!     compact: bool,
//!     /// 子内容插槽
//!     children: Children,
//! ) -> VNode {
//!     view! { div(class = "approval-card") { children.render() } }
//! }
//! ```
//!
//! 展开为「props 结构体 + 组件类型 + 构造器 + 构造器（按名）+ `UiComponent`
//! 实现」。两种构造形态：
//!
//! | 形态 | 用途 | 缺参数时 |
//! |---|---|---|
//! | `Name::new(必填…)` | 位置构造 | 编译期缺参数（少传就编译不过） |
//! | `Name::builder().字段(值)….build()` | **按名**构造（`view!` 用） | 运行期 `expect` 报错（消息带参数名） |
//!
//! ### `view!` —— 虚拟树构造宏
//!
//! 文法见 [`view`] 模块的文档（设计文档 §6.5）。宏展开成**运行时路径**上的
//! 构造调用（`::apeireth_ui_dom::VNode::element(...)` 等）⇒ 展开物里**没有**
//! 任何运行逻辑被复制。
//!
//! ## 🔵 未接线
//!
//! - **双向绑定 `bind:value` / `bind:checked`**：编译期**显式报错**（不是静默
//!   忽略）—— 渲染阶段的 property 回写尚未实现（设计文档 §5.4.3 的写回路径）。
//! - **泛型 props**（设计 §6.1：泛型 props 不支持，避免类型推断地狱）。
//! - **具名插槽**（设计 §6.2：本期不实现，用普通泛型函数参数表达）。
//!
//! ## 依赖纪律
//!
//! **零内部依赖、零外部依赖**：手写 token 解析（**不引 syn / quote** —— 会带入
//! proc-macro2 / unicode-ident 传递依赖）。
//!
//! ## 运行时路径约定
//!
//! 展开物引用 `::apeireth_ui_core::…` 与 `::apeireth_ui_dom::…`（**绝对路径**）
//! ⇒ 使用这些宏的 crate 须把 `apeireth-ui-core` / `apeireth-ui-dom` 列为直接
//! 依赖。本仓 `ui-kit` / `ui-demo` 均已满足。
//!
//! ## 分层
//!
//! 语法层（[`grammar`] / [`view`]）是**纯文本函数** ⇒ 可单元测试；本文件只做
//! 「记号流 ↔ 语法层」的接线。

#![forbid(unsafe_code)]

mod grammar;
mod view;

use proc_macro::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree};

use grammar::Param;

/// 组件声明宏：把「函数即组件」的写法翻译成「props 结构体 + 组件类型」。
///
/// 输入：
/// ```ignore
/// #[component]
/// fn Card(title: String) -> VNode { /* ... */ }
/// ```
///
/// 输出（等价）：
/// ```ignore
/// pub struct CardProps { pub title: String }
/// pub struct Card { pub props: CardProps }
/// pub struct CardBuilder { title: Option<String> }
/// impl Card { pub fn new(title: String) -> Self; pub fn builder() -> CardBuilder; }
/// impl UiComponent for Card { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return compile_error("#[component] 不接受参数（只支持 `#[component]`）");
    }
    match expand(item) {
        Ok(code) => code,
        Err(message) => compile_error(&message),
    }
}

/// 虚拟树构造宏：从 Rust 字面量构造 `VNode` 树。
///
/// 文法（设计文档 §6.5）：
/// ```ignore
/// view! {
///     div(class = "a b", class:open = is_open) {
///         span { "文本" }
///         button(onclick = move |_| { … })
///     }
/// }
/// ```
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let text = input.to_string();
    match view::parse(&text).and_then(|nodes| view::generate(&nodes)) {
        Ok(code) => code
            .parse::<TokenStream>()
            .unwrap_or_else(|error| compile_error(&format!("view! 生成物无法解析：{error}"))),
        Err(message) => compile_error(&message),
    }
}

/// 产出编译错误。
fn compile_error(message: &str) -> TokenStream {
    // 注：`Group::stream()` 返回**拷贝**，不能就地 extend —— 必须先造内容再包组。
    let mut inner = TokenStream::new();
    inner.extend([TokenTree::Literal(Literal::string(message))]);
    let group = Group::new(Delimiter::Parenthesis, inner);

    let mut stream = TokenStream::new();
    stream.extend([
        TokenTree::Ident(Ident::new("compile_error", Span::call_site())),
        TokenTree::Group(group),
    ]);
    stream
}

/// 解析后的组件函数。
struct ParsedItem {
    /// 组件名。
    name: String,
    /// 前导属性（doc 注释等）⇒ 转到生成的组件类型上。
    leading_attrs: Vec<String>,
    /// 参数列表。
    params: Vec<Param>,
    /// 返回类型（记号文本）。
    ret: String,
    /// 函数体（记号文本）。
    body: String,
}

/// 组件 props 结构体名（`Card` → `CardProps`）。
fn props_struct_name(component: &str) -> String {
    format!("{component}Props")
}

/// 组件构造器名（`Card` → `CardBuilder`）。
fn builder_struct_name(component: &str) -> String {
    format!("{component}Builder")
}

/// 宏生成的原函数体名（`Card` → `__apeireth_ui_component_body_Card`）。
fn body_fn_name(component: &str) -> String {
    format!("__apeireth_ui_component_body_{component}")
}

/// 展开入口。
fn expand(item: TokenStream) -> Result<TokenStream, String> {
    let parsed = parse_item(item)?;
    let name = parsed.name;
    let props_name = props_struct_name(&name);
    let builder_name = builder_struct_name(&name);
    let body_name = body_fn_name(&name);

    let args: Vec<String> = parsed
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, param.ty))
        .collect();
    let inits: Vec<String> = parsed
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect();
    let passes: Vec<String> = parsed
        .params
        .iter()
        .map(|param| format!("owned.{}", param.name))
        .collect();

    // 必填参数（`new` 的位置参数）。
    let required: Vec<&Param> = parsed
        .params
        .iter()
        .filter(|param| param.default.is_none())
        .collect();
    let required_args: Vec<String> = required
        .iter()
        .map(|param| format!("{}: {}", param.name, param.ty))
        .collect();

    let mut code = String::new();

    // props 结构体：字段名 = 参数名，字段类型 = 参数类型。
    code.push_str(&format!(
        "/// `{name}` 的 props：组件参数即 props（编译期类型检查，杜绝手写类型漂移）。\n"
    ));
    code.push_str("#[derive(Clone, Debug)]\n");
    code.push_str(&format!("pub struct {props_name} {{\n"));
    for param in &parsed.params {
        match &param.doc {
            Some(doc) => code.push_str(&format!("#[{doc}]\n")),
            None => code.push_str(&format!("/// 组件参数 `{}`。\n", param.name)),
        }
        code.push_str(&format!("pub {}: {},\n", param.name, param.ty));
    }
    code.push_str("}\n\n");

    // 组件类型（前导 doc 注释转到组件类型上）。
    for attr in &parsed.leading_attrs {
        code.push_str(&format!("#[{attr}]\n"));
    }
    code.push_str(&format!(
        "/// `{name}` 组件（由 `#[component]` 宏生成）。\n"
    ));
    code.push_str("#[derive(Clone, Debug)]\n");
    code.push_str(&format!("pub struct {name} {{\n"));
    code.push_str("/// 组件 props。\n");
    code.push_str(&format!("pub props: {props_name},\n"));
    code.push_str("}\n\n");

    // 构造器（按名）：`view!` 用它。
    code.push_str(&format!(
        "/// `{name}` 的构造器（按名赋值；缺省的可选参数取默认值）。\n"
    ));
    code.push_str("#[derive(Clone, Debug, Default)]\n");
    code.push_str(&format!("pub struct {builder_name} {{\n"));
    for param in &parsed.params {
        code.push_str(&format!("{}: Option<{}>,\n", param.name, param.ty));
    }
    code.push_str("}\n\n");

    code.push_str(&format!("impl {builder_name} {{\n"));
    for param in &parsed.params {
        code.push_str(&format!(
            "/// 设置 `{}`。\npub fn {}(mut self, value: {}) -> Self {{\nself.{0} = Some(value);\nself\n}}\n",
            param.name, param.name, param.ty
        ));
    }
    code.push_str("/// 产出组件实例（必填参数缺失 ⇒ 显式报错，消息带参数名）。\n");
    code.push_str(&format!("pub fn build(self) -> {name} {{\n"));
    for param in &parsed.params {
        match &param.default {
            None => {
                code.push_str(&format!(
                    "let {0} = self.{0}.expect(\"组件 `{name}` 的必填参数 `{0}` 缺失\");\n",
                    param.name
                ));
            }
            Some(default) => {
                code.push_str(&format!(
                    "let {0} = self.{0}.unwrap_or_else(|| {default});\n",
                    param.name
                ));
            }
        }
    }
    code.push_str(&format!("{name} {{ props: {props_name} {{ "));
    code.push_str(
        &parsed
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>()
            .join(", "),
    );
    code.push_str(" } }\n}\n}\n\n");

    // 组件类型上的方法。
    code.push_str(&format!("impl {name} {{\n"));
    code.push_str("/// 以全部**必填** props 构造组件实例（可选 props 取默认值）。\n");
    code.push_str(&format!(
        "pub fn new({}) -> Self {{\n",
        required_args.join(", ")
    ));
    for param in &parsed.params {
        if let Some(default) = &param.default {
            code.push_str(&format!("let {} = {default};\n", param.name));
        }
    }
    code.push_str(&format!("Self {{ props: {props_name} {{ "));
    code.push_str(
        &parsed
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>()
            .join(", "),
    );
    code.push_str(" } }\n}\n");
    code.push_str("/// 按名构造（`view!` 用）。\n");
    code.push_str(&format!(
        "pub fn builder() -> {builder_name} {{\n{builder_name}::default()\n}}\n"
    ));
    code.push_str("/// 读 props。\n");
    code.push_str(&format!(
        "pub fn props(&self) -> &{props_name} {{\n&self.props\n}}\n"
    ));
    code.push_str("}\n\n");

    // 原函数体（改名）：宏不改变函数体语义。
    code.push_str(&format!(
        "fn {body_name}({}) -> {} {{\n",
        args.join(", "),
        parsed.ret
    ));
    code.push_str(&format!("{}\n", parsed.body));
    code.push_str("}\n\n");

    // 组件 trait 实现：渲染是纯函数（props ⇒ 视图树）。
    code.push_str(
        "impl ::apeireth_ui_core::UiComponent for "
            .to_owned()
            .as_str(),
    );
    code.push_str(&format!("{name} {{\n"));
    code.push_str(&format!("type Props = {props_name};\n"));
    code.push_str(&format!("type Output = {};\n", parsed.ret));
    code.push_str("fn render(props: &Self::Props) -> Self::Output {\n");
    code.push_str("let owned = (*props).clone();\n");
    code.push_str(&format!("{body_name}({})\n", passes.join(", ")));
    code.push_str("}\n}\n");

    let _ = inits;
    code.parse::<TokenStream>()
        .map_err(|err| format!("内部错误：宏生成的代码无法解析（{err}）"))
}

/// 解析组件函数。
fn parse_item(item: TokenStream) -> Result<ParsedItem, String> {
    let tokens: Vec<TokenTree> = item.into_iter().collect();
    let mut index = 0usize;

    // 前导属性（doc 注释等）：跳过并保留（转到生成的组件类型上）。
    let mut leading_attrs: Vec<String> = Vec::new();
    while index + 1 < tokens.len() {
        let is_attr = matches!(
            (&tokens[index], &tokens[index + 1]),
            (TokenTree::Punct(punct), TokenTree::Group(group))
                if punct.as_char() == '#' && group.delimiter() == Delimiter::Bracket
        );
        if !is_attr {
            break;
        }
        let TokenTree::Group(group) = &tokens[index + 1] else {
            unreachable!("前一步已确认是属性组");
        };
        leading_attrs.push(group.stream().to_string());
        index += 2;
    }

    // 可选 `pub`。
    if let Some(TokenTree::Ident(ident)) = tokens.get(index) {
        if ident.to_string() == "pub" {
            index += 1;
        }
    }

    // `fn`。
    match tokens.get(index) {
        Some(TokenTree::Ident(ident)) if ident.to_string() == "fn" => index += 1,
        _ => return Err("#[component] 只支持 `fn` 形式的组件函数".to_owned()),
    }

    // 组件名。
    let name = match tokens.get(index) {
        Some(TokenTree::Ident(ident)) => {
            index += 1;
            ident.to_string()
        }
        _ => return Err("#[component] 需要一个组件函数名".to_owned()),
    };

    // 参数列表。
    let params_text = match tokens.get(index) {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            index += 1;
            group.stream().to_string()
        }
        _ => return Err("#[component] 需要 `fn Name(...)` 形式的参数列表".to_owned()),
    };

    // 可选 `-> 返回类型`。
    let mut ret_tokens: Vec<TokenTree> = Vec::new();
    if is_arrow(&tokens, index) {
        index += 2;
        while index < tokens.len() {
            if let TokenTree::Group(group) = &tokens[index] {
                if group.delimiter() == Delimiter::Brace {
                    break;
                }
            }
            ret_tokens.push(tokens[index].clone());
            index += 1;
        }
    }

    // 函数体。
    let body = match tokens.get(index) {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
            index += 1;
            group.stream().to_string()
        }
        _ => return Err("#[component] 需要一个函数体 `{ ... }`".to_owned()),
    };

    if index != tokens.len() {
        return Err("#[component] 不支持函数名之后的额外记号".to_owned());
    }

    // `where` 子句不在语法面内。
    if ret_tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Ident(ident) if ident.to_string() == "where"))
    {
        return Err("#[component] 不支持 `where` 子句".to_owned());
    }

    let ret = ret_tokens
        .iter()
        .map(|token| token.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    if ret.trim().is_empty() {
        return Err("#[component] 需要一个返回类型（例如 `-> VNode`）".to_owned());
    }

    let params = grammar::parse_params(&params_text)?;
    Ok(ParsedItem {
        name,
        leading_attrs,
        params,
        ret,
        body,
    })
}

/// 判定 `index` 处是否是 `->`。
fn is_arrow(tokens: &[TokenTree], index: usize) -> bool {
    let dash = matches!(
        tokens.get(index),
        Some(TokenTree::Punct(punct))
            if punct.as_char() == '-' && punct.spacing() == proc_macro::Spacing::Joint
    );
    let angle = matches!(
        tokens.get(index + 1),
        Some(TokenTree::Punct(punct)) if punct.as_char() == '>'
    );
    dash && angle
}

#[cfg(test)]
mod tests {
    //! crate 级断言：宏的**命名面**（纯字符串，不碰 proc-macro API —— 实测在
    //! 单元测试里调用 proc-macro API 会 panic「procedural macro API is used
    //! outside of a procedural macro」）。展开行为断言放在集成测试
    //! （tests/component_expansion.rs），语法面断言在 grammar.rs / view.rs 内。

    use super::*;

    #[test]
    fn generated_names_follow_the_documented_shape() {
        assert_eq!(props_struct_name("Card"), "CardProps");
        assert_eq!(builder_struct_name("Card"), "CardBuilder");
        assert_eq!(body_fn_name("Card"), "__apeireth_ui_component_body_Card");
    }

    #[test]
    fn generated_body_name_cannot_collide_with_user_names() {
        assert!(body_fn_name("Card").starts_with("__apeireth_ui_component_body_"));
        assert_ne!(body_fn_name("Card"), "Card");
    }

    #[test]
    fn builder_name_cannot_collide_with_props_name() {
        assert_ne!(builder_struct_name("Card"), props_struct_name("Card"));
    }
}
