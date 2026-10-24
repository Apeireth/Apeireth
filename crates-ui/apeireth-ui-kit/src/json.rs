//! JSON 值类型与序列化器（**零依赖**手写实现）。
//!
//! ## 为什么要自己实现
//!
//! 卡面上的参数 / 结果文本由**脚本侧**的 `JSON.stringify` 产出 ⇒ 引擎侧必须
//! 产出**逐字节相同**的文本 ⇒ 需要一个与脚本侧语义对齐的序列化器。
//!
//! - 通用库（serde 等）会把 `f64` 的**键序**交给 `HashMap`（乱序）⇒ 与脚本侧的
//!   **插入序**不一致 ⇒ 这里用 `Vec<(String, JsonValue)>` 保序。
//! - P0 纪律：**0 外部依赖** ⇒ 手写。
//!
//! ## 对齐了哪些语义
//!
//! | 维度 | 语义 |
//! |---|---|
//! | 对象键序 | **插入序**（与脚本侧一致；不是哈希序、不是字典序） |
//! | 缩进 | 每层 `indent` 个**空格**（脚本侧的 `JSON.stringify(v, null, 2)` 用 2 空格） |
//! | 空容器 | `{}` / `[]`（**不换行**） |
//! | 字符串转义 | `"` `\` ⇒ `\"` `\\`；`\b \f \n \r \t` 用短写法；其余控制字符用 `\u00xx`（小写十六进制）；**非 ASCII 原样输出** |
//! | 数值 | 走 [`crate::num::number_to_string`]（十进制 / 指数记法与脚本侧一致） |
//! | 非有限数值 | 写成 `null`（脚本侧语义：`JSON.stringify(NaN) === "null"`） |
//!
//! ## 「缺省」与「空值」是两件事
//!
//! 脚本侧的 `undefined` 在对象里**整个键被省略**，`null` 则保留为 `null` ⇒
//! 本类型用 `Option<JsonValue>` 表达这层差别：**`None` ⇒ 键被省略**，
//! `Some(JsonValue::Null)` ⇒ 键保留、值为 `null`。

use crate::num::number_to_string;

/// JSON 值。
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// 空值（脚本侧的 `null`）。
    Null,
    /// 布尔。
    Bool(bool),
    /// 数值（脚本侧的 `number`；非有限值按脚本侧语义写成 `null`）。
    Number(f64),
    /// 字符串。
    String(String),
    /// 数组（**保序**）。
    Array(Vec<JsonValue>),
    /// 对象（**保插入序** ⇒ 键序与脚本侧一致）。
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// 紧凑序列化（**无缩进**）⇒ 与脚本侧的 `JSON.stringify(v)` 逐字节相同。
    pub fn to_string_compact(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    /// 缩进序列化（每层 2 个空格）⇒ 与脚本侧的 `JSON.stringify(v, null, 2)`
    /// 逐字节相同。
    pub fn to_string_pretty(&self) -> String {
        self.to_string_pretty_with(2)
    }

    /// 缩进序列化（每层 `indent` 个空格）。
    pub fn to_string_pretty_with(&self, indent: usize) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, indent, 0);
        out
    }

    /// **真值**判定（脚本侧的 `!!value`）：空值 / 假 / 零 / 空串 / 空数组 ⇒ 假。
    ///
    /// ⚠️ 空对象 / 空数组是**真值**（与脚本侧一致）。
    pub fn is_truthy(&self) -> bool {
        match self {
            JsonValue::Null => false,
            JsonValue::Bool(flag) => *flag,
            JsonValue::Number(value) => *value != 0.0 && !value.is_nan(),
            JsonValue::String(text) => !text.is_empty(),
            JsonValue::Array(_) | JsonValue::Object(_) => true,
        }
    }

    /// 取对象的键值（**非对象** ⇒ `None`）。
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// 值的类型名（脚本侧的类型名口径）。
    pub fn type_name(&self) -> &'static str {
        match self {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        }
    }

    /// 紧凑写盘。
    fn write_compact(&self, out: &mut String) {
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(true) => out.push_str("true"),
            JsonValue::Bool(false) => out.push_str("false"),
            JsonValue::Number(value) => {
                if value.is_finite() {
                    out.push_str(&number_to_string(*value));
                } else {
                    out.push_str("null");
                }
            }
            JsonValue::String(text) => write_quoted(text, out),
            JsonValue::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_quoted(key, out);
                    out.push(':');
                    value.write_compact(out);
                }
                out.push('}');
            }
        }
    }

    /// 缩进写盘（`depth` = 当前层级）。
    fn write_pretty(&self, out: &mut String, indent: usize, depth: usize) {
        match self {
            JsonValue::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    out.push_str(&" ".repeat(indent * (depth + 1)));
                    item.write_pretty(out, indent, depth + 1);
                }
                out.push('\n');
                out.push_str(&" ".repeat(indent * depth));
                out.push(']');
            }
            JsonValue::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    out.push_str(&" ".repeat(indent * (depth + 1)));
                    write_quoted(key, out);
                    out.push_str(": ");
                    value.write_pretty(out, indent, depth + 1);
                }
                out.push('\n');
                out.push_str(&" ".repeat(indent * depth));
                out.push('}');
            }
            other => other.write_compact(out),
        }
    }
}

/// 写一个带引号并转义的字符串。
fn write_quoted(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    //! 序列化语义的断言（**行为**，不是源码镜像）。

    use super::*;

    /// 建一个对象（保序）。
    fn object(entries: Vec<(&str, JsonValue)>) -> JsonValue {
        JsonValue::Object(
            entries
                .into_iter()
                .map(|(key, value)| (String::from(key), value))
                .collect(),
        )
    }

    #[test]
    fn scalars_are_written_like_the_script_side() {
        assert_eq!(JsonValue::Null.to_string_compact(), "null");
        assert_eq!(JsonValue::Bool(true).to_string_compact(), "true");
        assert_eq!(JsonValue::Bool(false).to_string_compact(), "false");
        assert_eq!(JsonValue::Number(2.0).to_string_compact(), "2");
        assert_eq!(JsonValue::Number(1.5).to_string_compact(), "1.5");
        assert_eq!(
            JsonValue::String(String::from("卷宗")).to_string_compact(),
            "\"卷宗\""
        );
    }

    #[test]
    fn object_keys_keep_the_insertion_order() {
        let value = object(vec![
            ("zebra", JsonValue::Number(1.0)),
            ("alpha", JsonValue::Number(2.0)),
            ("middle", JsonValue::Number(3.0)),
        ]);
        assert_eq!(
            value.to_string_compact(),
            "{\"zebra\":1,\"alpha\":2,\"middle\":3}"
        );
    }

    #[test]
    fn strings_are_escaped_like_the_script_side() {
        let value = JsonValue::String(String::from("a\"b\\c\nd\te\u{1}"));
        assert_eq!(value.to_string_compact(), "\"a\\\"b\\\\c\\nd\\te\\u0001\"");
    }

    #[test]
    fn non_ascii_characters_are_not_escaped() {
        let value = JsonValue::String(String::from("卷宗/工作区"));
        assert_eq!(value.to_string_compact(), "\"卷宗/工作区\"");
    }

    #[test]
    fn empty_containers_stay_on_one_line() {
        assert_eq!(JsonValue::Array(Vec::new()).to_string_pretty(), "[]");
        assert_eq!(JsonValue::Object(Vec::new()).to_string_pretty(), "{}");
    }

    #[test]
    fn pretty_output_uses_two_spaces_per_level() {
        let value = object(vec![
            ("a", JsonValue::Number(1.0)),
            (
                "b",
                JsonValue::Array(vec![JsonValue::Number(1.0), JsonValue::Number(2.0)]),
            ),
        ]);
        assert_eq!(
            value.to_string_pretty(),
            "{\n  \"a\": 1,\n  \"b\": [\n    1,\n    2\n  ]\n}"
        );
    }

    #[test]
    fn pretty_output_matches_the_script_side_layout() {
        // 脚本侧：`JSON.stringify({args:{path:"/x",depth:2}}, null, 2)`
        let value = object(vec![(
            "args",
            object(vec![
                ("path", JsonValue::String(String::from("/x"))),
                ("depth", JsonValue::Number(2.0)),
            ]),
        )]);
        assert_eq!(
            value.to_string_pretty(),
            "{\n  \"args\": {\n    \"path\": \"/x\",\n    \"depth\": 2\n  }\n}"
        );
    }

    #[test]
    fn truthiness_follows_the_script_side() {
        assert!(!JsonValue::Null.is_truthy());
        assert!(!JsonValue::Bool(false).is_truthy());
        assert!(!JsonValue::Number(0.0).is_truthy());
        assert!(!JsonValue::Number(f64::NAN).is_truthy());
        assert!(!JsonValue::String(String::new()).is_truthy());
        // 空对象 / 空数组是**真值**。
        assert!(JsonValue::Object(Vec::new()).is_truthy());
        assert!(JsonValue::Array(Vec::new()).is_truthy());
        assert!(JsonValue::Number(1.0).is_truthy());
        assert!(JsonValue::String(String::from("x")).is_truthy());
    }

    #[test]
    fn object_lookup_finds_the_value() {
        let value = object(vec![("a", JsonValue::Number(1.0))]);
        assert_eq!(value.get("a"), Some(&JsonValue::Number(1.0)));
        assert_eq!(value.get("b"), None);
        assert_eq!(JsonValue::Null.get("a"), None);
    }

    #[test]
    fn non_finite_numbers_are_written_as_null() {
        assert_eq!(JsonValue::Number(f64::NAN).to_string_compact(), "null");
        assert_eq!(JsonValue::Number(f64::INFINITY).to_string_compact(), "null");
    }

    #[test]
    fn nested_structures_round_trip_through_both_writers() {
        let value = object(vec![
            (
                "list",
                JsonValue::Array(vec![
                    object(vec![("k", JsonValue::String(String::from("v")))]),
                    JsonValue::Null,
                ]),
            ),
            ("flag", JsonValue::Bool(true)),
        ]);
        let compact = value.to_string_compact();
        assert_eq!(compact, "{\"list\":[{\"k\":\"v\"},null],\"flag\":true}");
        let pretty = value.to_string_pretty();
        assert!(
            pretty.contains("\n  \"list\": [\n    {\n      \"k\": \"v\"\n    },\n    null\n  ]")
        );
    }
}
