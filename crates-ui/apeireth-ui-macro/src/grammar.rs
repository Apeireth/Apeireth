//! `#[component]` 的**语法层**：纯文本解析（不碰 proc-macro API ⇒ 可单元测试）。
//!
//! 分层理由：过程宏 crate 的单元测试**不能**调用 proc-macro API（实测：单元测试
//! 里用 `TokenStream` 会 panic「procedural macro API is used outside of a
//! procedural macro」）。因此把「参数切分 / 参数解析 / 错误文案」这层做成纯
//! 字符串函数，在本文件内直接钉住；proc-macro 层只负责把记号流接上这层。

/// 解析后的参数（同时是 props 字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Param {
    /// 参数名（同时是 props 字段名）。
    pub(crate) name: String,
    /// 参数类型（记号文本，原样搬运）。
    pub(crate) ty: String,
    /// 参数上的文档注释（`doc = "..."` 的内层文本），无则由宏生成默认文档。
    pub(crate) doc: Option<String>,
    /// 默认值表达式（`#[prop(default = ...)]`）。
    ///
    /// `None` ⇒ 该参数**必填**（构造器 `new` 要求传）。
    pub(crate) default: Option<String>,
    /// 该参数是不是子内容插槽（参数名固定为 `children`）。
    ///
    /// 插槽**永远可选**（缺省为空子树），且带 `.children(v)` 设置器
    /// （设计文档 §6.2）。
    pub(crate) is_children: bool,
}

/// 顶层切分：按**顶层逗号**切分（尖括号与箭头内的逗号不切）。
///
/// 例：`a: HashMap<String, u32>, b: Box<dyn Fn(i32) -> u32>`
/// ⇒ 两段（尖括号内与箭头后的逗号都不切）。
pub(crate) fn split_top_level(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut angle_depth: i32 = 0;

    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '-' => {
                current.push(ch);
                // `->` 整体吞掉第二个字符（避免把箭头的 '>' 当成泛型收尾）。
                if chars.peek() == Some(&'>') {
                    current.push(chars.next().expect("peek 已确认有字符"));
                }
            }
            '<' => {
                angle_depth += 1;
                current.push(ch);
            }
            '>' => {
                angle_depth -= 1;
                current.push(ch);
            }
            ',' if angle_depth == 0 => {
                let chunk = current.trim().to_owned();
                if !chunk.is_empty() {
                    chunks.push(chunk);
                }
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        chunks.push(tail.to_owned());
    }
    chunks
}

/// 解析参数列表（纯文本）。
pub(crate) fn parse_params(text: &str) -> Result<Vec<Param>, String> {
    split_top_level(text)
        .iter()
        .map(|chunk| parse_param(chunk))
        .collect()
}

/// 解析单个参数（`名字: 类型`）。
pub(crate) fn parse_param(chunk: &str) -> Result<Param, String> {
    let chunk = chunk.trim();
    let mut rest = chunk;
    let mut doc: Option<String> = None;
    let mut default: Option<String> = None;

    // 前置属性：`#[doc = "..."]` / `/// 文档` 与 `#[prop(default = ...)]`（顺序不限）。
    //
    // 注：记号流的**美化打印**会把 `#[doc = "..."]` 还原成 `/// 文档` ⇒ 两种
    // 形态都要认（文档注释在参数位置被宏吃掉后，rustc 看不到它 ⇒ 合法）。
    loop {
        if rest.starts_with("#[") {
            let end = find_attr_end(rest)?;
            let attr = rest[..=end].trim();
            let inner = attr
                .trim_start_matches('#')
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_owned();
            if inner.starts_with("doc") {
                doc = Some(inner);
            } else if inner.starts_with("prop") {
                let body = inner
                    .trim_start_matches("prop")
                    .trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .trim()
                    .to_owned();
                let (key, value) = body.split_once('=').ok_or_else(|| {
                    "#[component] `#[prop(...)]` 只支持 `default = 表达式`".to_owned()
                })?;
                if key.trim() != "default" {
                    return Err(format!(
                        "#[component] `#[prop(...)]` 只支持 `default = 表达式`（读到 `{}`）",
                        key.trim()
                    ));
                }
                default = Some(value.trim().to_owned());
            } else {
                return Err(format!(
                    "#[component] 不支持的参数属性 `{inner}`（支持 `#[doc = \"...\"]` 与 \
                     `#[prop(default = ...)]`）"
                ));
            }
            rest = rest[end + 1..].trim();
            continue;
        }
        if rest.starts_with("///") {
            let end = rest.find('\n').unwrap_or(rest.len());
            let line = rest[3..end].trim();
            doc = Some(format!("doc = {}", string_literal(line)));
            rest = rest[end..].trim_start();
            continue;
        }
        break;
    }

    let (name, ty) = rest
        .split_once(':')
        .ok_or_else(|| "#[component] 参数必须写成 `名字: 类型`".to_owned())?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
        return Err(format!("#[component] 参数名非法：`{name}`"));
    }

    let ty = ty.trim();
    if ty.is_empty() {
        return Err(format!("#[component] 参数 `{name}` 缺少类型"));
    }
    if ty.contains('\'') && !ty.contains("'static") {
        return Err(format!(
            "#[component] 参数 `{name}` 的类型含生命周期；只支持 owned 类型或 'static"
        ));
    }

    // 插槽参数（`children`）：永远可选，缺省为空子树。
    let is_children = name == "children";
    if is_children && default.is_none() {
        default = Some(String::from("::std::default::Default::default()"));
    }

    Ok(Param {
        name: name.to_owned(),
        ty: ty.to_owned(),
        doc,
        default,
        is_children,
    })
}

/// 把一段文本变成 Rust 字符串字面量（含引号）。
fn string_literal(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// 找属性 `#[...]` 的右方括号位置（**括号深度感知**：默认值里可能有 `]`）。
///
/// 返回**字节**下标（切分用的是字节下标 ⇒ 非 ASCII 内容也不会切坏）。
fn find_attr_end(text: &str) -> Result<usize, String> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'[' | b'(' | b'{' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            b')' | b'}' => depth -= 1,
            _ => {}
        }
    }
    Err("#[component] 参数属性缺少右方括号".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_commas_split_parameters() {
        let chunks = split_top_level("a: String , b: Vec < u32 >");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "a: String");
    }

    #[test]
    fn generic_parameters_are_not_split() {
        let chunks = split_top_level("a: HashMap < String , u32 > , b: u8");
        assert_eq!(chunks.len(), 2, "尖括号内的逗号不得切分");
    }

    #[test]
    fn function_pointer_types_are_not_split() {
        let chunks = split_top_level("a: Box < dyn Fn ( i32 ) -> u32 > , b: u8");
        assert_eq!(chunks.len(), 2, "箭头后的类型不得被切分");
    }

    #[test]
    fn parameter_is_parsed_into_name_and_type() {
        let param = parse_param("title: String").expect("解析成功");
        assert_eq!(param.name, "title");
        assert_eq!(param.ty, "String");
        assert!(param.doc.is_none());
    }

    #[test]
    fn doc_attribute_is_captured() {
        let param = parse_param("#[doc = \"标题\"] title: String").expect("解析成功");
        assert_eq!(param.doc.as_deref(), Some("doc = \"标题\""));
        assert_eq!(param.name, "title");
        assert!(param.default.is_none(), "无 `#[prop]` ⇒ 必填");
    }

    #[test]
    fn doc_comment_form_is_captured() {
        // 记号流的美化打印会把 `#[doc = "..."]` 还原成 `/// ...` ⇒ 两种形态都要认。
        let param = parse_param("/// 标题\ntitle: String").expect("解析成功");
        assert_eq!(param.doc.as_deref(), Some("doc = \"标题\""));
        assert_eq!(param.name, "title");
    }

    #[test]
    fn doc_comment_and_prop_default_can_appear_together() {
        let param =
            parse_param("/// 紧凑\n#[prop(default = false)]\ncompact: bool").expect("解析成功");
        assert_eq!(param.doc.as_deref(), Some("doc = \"紧凑\""));
        assert_eq!(param.default.as_deref(), Some("false"));
        assert_eq!(param.name, "compact");
    }

    #[test]
    fn prop_default_makes_a_parameter_optional() {
        let param = parse_param("#[prop(default = false)] flag: bool").expect("解析成功");
        assert_eq!(param.default.as_deref(), Some("false"));
        assert_eq!(param.name, "flag");
    }

    #[test]
    fn prop_default_with_brackets_inside_is_parsed() {
        let param = parse_param("#[prop(default = vec![1, 2])] items: Vec<u8>").expect("解析成功");
        assert_eq!(param.default.as_deref(), Some("vec![1, 2]"));
    }

    #[test]
    fn children_slot_is_always_optional() {
        let param = parse_param("children: Children").expect("解析成功");
        assert!(param.is_children);
        assert_eq!(
            param.default.as_deref(),
            Some("::std::default::Default::default()")
        );
    }

    #[test]
    fn unsupported_helper_attribute_is_rejected() {
        let err = parse_param("#[whatever(1)] flag: bool").expect_err("必须报错");
        assert!(err.contains("不支持的参数属性"), "实测错误文案：{err}");
    }

    #[test]
    fn prop_attribute_with_unknown_key_is_rejected() {
        let err = parse_param("#[prop(required = true)] flag: bool").expect_err("必须报错");
        assert!(err.contains("只支持 `default"), "实测错误文案：{err}");
    }

    #[test]
    fn missing_type_is_rejected() {
        let err = parse_param("title:").expect_err("必须报错");
        assert!(err.contains("缺少类型"), "实测错误文案：{err}");
    }

    #[test]
    fn non_static_lifetime_is_rejected() {
        let err = parse_param("title: & 'a str").expect_err("必须报错");
        assert!(err.contains("生命周期"), "实测错误文案：{err}");
    }

    #[test]
    fn static_lifetime_is_allowed() {
        let param = parse_param("title: & 'static str").expect("解析成功");
        assert_eq!(param.ty, "& 'static str");
    }
}
