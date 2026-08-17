//! `apeireth-tools::schema` — 手写最小 JSON schema 校验 (TP12)
//!
//! **设计动机** (per `docs/team-work-doc.md` §11 TP12):
//! - 幻觉传播源头之一 = 工具输出无 schema 校验 → 模型盲目重放
//! - VCP `vcpLoop` 三件套给了范式 (toolExecutor 校验 + 协议层兜底)
//! - 本模块把"递归 enum schema + 手写校验器"做成纯函数库, 工具作者按需标注输出结构
//!
//! **为什么不引入 jsonschema crate** (per `§1.3` 禁止新框架):
//! - jsonschema / schemars / valico 都是 JS-style 巨型校验器, 100k+ 行代码
//! - 我们只关心 5 种基础类型 + Object/Array/Optional, 手写 200 行就够
//! - 校验器是纯函数, 无堆栈溢出风险 (递归深度受 schema 嵌套控制, 实战 ≤ 5)
//!
//! **向后兼容契约** (per `§1.2 0 装 PASS`):
//! - `SchemaNode` 是 0 装默认值 = 工具声明 `output_schema: None` = 不校验
//! - 已有工具 (`web_search.rs` / `file_ops.rs` 等) 0 改动即可继续工作
//! - 注册中心 (`apeireth-tool-registry`) 不修改 — 本模块是 sidecar
//!
//! **字段级引用 VCP** (`research/source/vcptoolbox/vcpLoop/toolExecutor.js`):
//! - `_createErrorResult` (toolExecutor.js:475-482) 错误结果格式 → `ValidationError`
//! - `_validateArgs` (toolExecutor.js:163-191) args schema 校验思路 → `validate()` 递归 walk

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// **递归 schema 类型节点**
///
/// **字段**:
/// - `String` — JSON 字符串 (单字符任意 unicode)
/// - `Number` — JSON 数字 (i64 / u64 / f64 全收)
/// - `Bool`   — JSON true / false
/// - `Null`   — JSON null (单独存在场景: 返回值是 null)
/// - `Object` — JSON object, `fields` 列出每个键的子 schema (未列出键 = 任意)
/// - `Array`  — JSON array, `item` 是元素 schema (整个数组元素同构)
/// - `Optional` — 内层 schema, 额外接受 `null` (per VCP `toolExecutor.js:_createErrorResult`
///   中 "可选字段可 null" 的字段级语义)
///
/// **0 装 PASS**: `None` (SchemaNode) = 不校验 (向后兼容老工具).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchemaNode {
    /// JSON 字符串
    String,
    /// JSON 数字 (整数 + 浮点)
    Number,
    /// JSON 布尔
    Bool,
    /// JSON null
    Null,
    /// JSON object, `fields` 列出每个键的子 schema
    Object {
        /// 字段名 → 子 schema (BTreeMap 保持确定性输出)
        fields: BTreeMap<String, SchemaNode>,
    },
    /// JSON array, `item` 是元素 schema (整个数组元素同构)
    Array {
        /// 数组元素 schema
        item: Box<SchemaNode>,
    },
    /// Optional<inner> = inner OR null (VCP 字段级引用 toolExecutor.js:163-191 可选字段)
    Optional {
        /// 内层 schema
        inner: Box<SchemaNode>,
    },
}

/// **校验失败时的结构化错误**
///
/// **字段** (per VCP `_createErrorResult` 字段级):
/// - `path` — JSON pointer 形式 (e.g. `$.items[3].name`), 模型可定位
/// - `expected` — 期望的 schema 类型字符串 (e.g. `"string"` / `"array<string>"`)
/// - `actual` — 实际值类型 (`"string"` / `"number"` / `"null"` / `"array"` / `"object"` / `"bool"`)
/// - `hint` — 可行动提示 (人类 + 模型都能读)
///
/// **结构化**: 故意不带 `Display` 自动格式化, 让消费者 (tool_bridge.rs) 显式构造
/// 注入到 tool message 的内容, 避免模型把结构化字段当 string panic.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidationError {
    /// JSON pointer 路径, 根 = `"$"`, 字段 = `"$.field"`, 数组 = `"$[3]"`
    pub path: String,
    /// 期望类型 (human-readable, e.g. `"object<name:string, age:number>"`)
    pub expected: String,
    /// 实际类型 (one of `type_name(Value)`)
    pub actual: String,
    /// 可行动提示 (e.g. "fix string→number at $.items[3].count")
    pub hint: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[validation] at {}: expected {}, got {} ({})",
            self.path, self.expected, self.actual, self.hint
        )
    }
}

impl std::error::Error for ValidationError {}

/// **TP12 sidecar — per-tool 输出 schema 映射**
///
/// **为什么需要**:
/// - `apeireth-tool-registry::Tool` trait 在 N15 已锁定, 不能挂 `output_schema` 字段
/// - 工具作者按 tool_name 注入 schema, ToolExecutor 启动时传入
/// - 缺省 = 空 HashMap = 全部工具不校验 (向后兼容老工具)
///
/// **0 装 PASS**:
/// - `None` 注入 = 该工具不校验 (向后兼容老工具)
/// - 空 map = 全部不校验 (ToolExecutor 默认构造就这样)
/// - 校验触发 = 仅当 tool_name 在 map 中且 value 与 schema 不匹配
///
/// **字段**:
/// - `map: HashMap<String, SchemaNode>` — tool_name → 输出 schema
///
/// **API**: `insert(name, schema)` / `get(name) -> Option<&SchemaNode>` / `is_empty()`
#[derive(Debug, Default, Clone)]
pub struct SchemaMap {
    /// tool_name → 输出 schema
    map: HashMap<String, SchemaNode>,
}

impl SchemaMap {
    /// 新建空 SchemaMap (向后兼容默认)
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入某个工具的输出 schema
    pub fn insert(&mut self, tool_name: impl Into<String>, schema: SchemaNode) {
        self.map.insert(tool_name.into(), schema);
    }

    /// 查询某个工具的输出 schema (None = 该工具未声明, 不校验)
    pub fn get(&self, tool_name: &str) -> Option<&SchemaNode> {
        self.map.get(tool_name)
    }

    /// 是否空 (空 map = 全部不校验, 等价 0 装)
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 包含的工具数
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// **递归 schema 校验**
///
/// **行为**:
/// - `schema` 是 `&SchemaNode` (调用方持所有权, 调用不消耗)
/// - `value` 是 `&Value` (任意 JSON)
/// - 匹配 → `Ok(())`; 不匹配 → `Err(ValidationError { path, expected, actual, hint })`
///
/// **0 装**: 校验路径出错 = `Err` (绝不返 `Ok` 假装通过).
///
/// **panic 安全**: 递归深度由 `schema` 嵌套深度限制, 实战嵌套 ≤ 5 (per `web_search.rs`
/// 输出结构). 极端深度 (≥ 10000) 由 serde_json 自身保护 (其解析栈限 ≤ 128 默认).
pub fn validate(schema: &SchemaNode, value: &Value) -> Result<(), ValidationError> {
    validate_at(schema, value, "$")
}

/// 内部递归: 在指定 path 下校验 value 与 schema
fn validate_at(schema: &SchemaNode, value: &Value, path: &str) -> Result<(), ValidationError> {
    match schema {
        SchemaNode::String => expect_type("string", value, path, schema),
        SchemaNode::Number => expect_type("number", value, path, schema),
        SchemaNode::Bool => expect_type("bool", value, path, schema),
        SchemaNode::Null => expect_type("null", value, path, schema),
        SchemaNode::Object { fields } => {
            // 期望 object
            if let Value::Object(map) = value {
                // 列出字段逐项
                for (k, child_schema) in fields {
                    let child_path = format!("{path}.{k}");
                    if let Some(child_value) = map.get(k) {
                        validate_at(child_schema, child_value, &child_path)?;
                    } else {
                        // 字段缺失 (非 Optional 视为缺失 = 错误)
                        if !matches!(child_schema, SchemaNode::Optional { .. }) {
                            return Err(ValidationError {
                                path: child_path,
                                expected: schema_label(child_schema),
                                actual: "missing".into(),
                                hint: format!("field `{k}` required by schema but absent"),
                            });
                        }
                    }
                }
                // 未列出键 = 任意 (向后兼容: 工具可加额外字段不被驳回)
                Ok(())
            } else {
                Err(ValidationError {
                    path: path.to_string(),
                    expected: schema_label(schema),
                    actual: type_name(value).into(),
                    hint: format!("expected JSON object at {path}"),
                })
            }
        }
        SchemaNode::Array { item } => {
            if let Value::Array(arr) = value {
                for (i, element) in arr.iter().enumerate() {
                    let child_path = format!("{path}[{i}]");
                    validate_at(item, element, &child_path)?;
                }
                Ok(())
            } else {
                Err(ValidationError {
                    path: path.to_string(),
                    expected: schema_label(schema),
                    actual: type_name(value).into(),
                    hint: format!("expected JSON array at {path}"),
                })
            }
        }
        SchemaNode::Optional { inner } => {
            // Optional 接受 null 或 inner
            if value.is_null() {
                Ok(())
            } else {
                validate_at(inner, value, path)
            }
        }
    }
}

/// 期望 value 是 type_str, 否则返 ValidationError
fn expect_type(
    type_str: &str,
    value: &Value,
    path: &str,
    schema: &SchemaNode,
) -> Result<(), ValidationError> {
    let actual = type_name(value);
    if actual == type_str {
        Ok(())
    } else {
        Err(ValidationError {
            path: path.to_string(),
            expected: schema_label(schema),
            actual: actual.into(),
            hint: format!("expected {type_str}, got {actual}"),
        })
    }
}

/// JSON Value 类型名 (单数字段, 用于 actual 字段)
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// SchemaNode → 人类可读标签 (用于 expected 字段)
fn schema_label(s: &SchemaNode) -> String {
    match s {
        SchemaNode::String => "string".into(),
        SchemaNode::Number => "number".into(),
        SchemaNode::Bool => "bool".into(),
        SchemaNode::Null => "null".into(),
        SchemaNode::Object { fields } => {
            let inner: Vec<String> =
                fields.iter().map(|(k, v)| format!("{k}:{}", schema_label(v))).collect();
            format!("object<{}>", inner.join(", "))
        }
        SchemaNode::Array { item } => format!("array<{}>", schema_label(item)),
        SchemaNode::Optional { inner } => format!("optional<{}>", schema_label(inner)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 标量 happy path
    #[test]
    fn validate_primitives() {
        assert!(validate(&SchemaNode::String, &json!("hello")).is_ok());
        assert!(validate(&SchemaNode::Number, &json!(42)).is_ok());
        assert!(validate(&SchemaNode::Number, &json!(3.14)).is_ok());
        assert!(validate(&SchemaNode::Bool, &json!(true)).is_ok());
        assert!(validate(&SchemaNode::Null, &json!(null)).is_ok());
    }

    /// 标量 type mismatch 必返 ValidationError (0 装: 不假装通过)
    #[test]
    fn validate_type_mismatch_returns_error() {
        let r = validate(&SchemaNode::String, &json!(42));
        assert!(r.is_err());
        let e = r.unwrap_err();
        assert_eq!(e.path, "$");
        assert_eq!(e.expected, "string");
        assert_eq!(e.actual, "number");
        assert!(e.hint.contains("string"));
    }

    /// Object 字段类型对齐
    #[test]
    fn validate_object_field_types() {
        let schema = SchemaNode::Object {
            fields: BTreeMap::from([
                ("name".into(), SchemaNode::String),
                ("count".into(), SchemaNode::Number),
            ]),
        };
        let v = json!({"name": "x", "count": 3});
        assert!(validate(&schema, &v).is_ok());

        // count 类型错
        let bad = json!({"name": "x", "count": "three"});
        let r = validate(&schema, &bad).unwrap_err();
        assert_eq!(r.path, "$.count");
        assert_eq!(r.expected, "number");
        assert_eq!(r.actual, "string");
    }

    /// Object 字段缺失 (非 Optional) 必报错
    #[test]
    fn validate_object_missing_required_field() {
        let schema = SchemaNode::Object {
            fields: BTreeMap::from([("name".into(), SchemaNode::String)]),
        };
        let v = json!({});
        let r = validate(&schema, &v).unwrap_err();
        assert_eq!(r.path, "$.name");
        assert_eq!(r.actual, "missing");
    }

    /// Object 额外字段 (未在 schema 列出) 不报错 (向后兼容)
    #[test]
    fn validate_object_extra_fields_allowed() {
        let schema = SchemaNode::Object {
            fields: BTreeMap::from([("name".into(), SchemaNode::String)]),
        };
        let v = json!({"name": "x", "extra": "ok"});
        assert!(validate(&schema, &v).is_ok());
    }

    /// Array 同构校验
    #[test]
    fn validate_array_homogeneous() {
        let schema = SchemaNode::Array {
            item: Box::new(SchemaNode::Number),
        };
        assert!(validate(&schema, &json!([1, 2, 3])).is_ok());

        let r = validate(&schema, &json!([1, "two", 3])).unwrap_err();
        assert_eq!(r.path, "$[1]");
        assert_eq!(r.expected, "number");
        assert_eq!(r.actual, "string");
    }

    /// Array 嵌套 Array
    #[test]
    fn validate_nested_array() {
        let schema = SchemaNode::Array {
            item: Box::new(SchemaNode::Array {
                item: Box::new(SchemaNode::Number),
            }),
        };
        assert!(validate(&schema, &json!([[1, 2], [3, 4]])).is_ok());

        let r = validate(&schema, &json!([[1, 2], [3, "bad"]])).unwrap_err();
        assert_eq!(r.path, "$[1][1]");
        assert_eq!(r.actual, "string");
    }

    /// Optional 接受 null + inner
    #[test]
    fn validate_optional_inner_or_null() {
        let schema = SchemaNode::Optional {
            inner: Box::new(SchemaNode::String),
        };
        assert!(validate(&schema, &json!(null)).is_ok());
        assert!(validate(&schema, &json!("hi")).is_ok());

        let r = validate(&schema, &json!(42)).unwrap_err();
        assert_eq!(r.actual, "number");
    }

    /// Object 中字段为 Optional, 缺失不算错
    #[test]
    fn validate_optional_field_in_object() {
        let schema = SchemaNode::Object {
            fields: BTreeMap::from([(
                "nickname".into(),
                SchemaNode::Optional {
                    inner: Box::new(SchemaNode::String),
                },
            )]),
        };
        let v = json!({});
        assert!(validate(&schema, &v).is_ok());
        assert!(validate(&schema, &json!({"nickname": "x"})).is_ok());
        assert!(validate(&schema, &json!({"nickname": null})).is_ok());

        // 类型错仍报错
        let r = validate(&schema, &json!({"nickname": 42})).unwrap_err();
        assert_eq!(r.path, "$.nickname");
    }

    /// 真实场景: WebSearch 输出 schema (per `web_search.rs` 字段级)
    #[test]
    fn validate_realistic_web_search_output() {
        let schema = SchemaNode::Object {
            fields: BTreeMap::from([
                ("results".into(), SchemaNode::Array {
                    item: Box::new(SchemaNode::Object {
                        fields: BTreeMap::from([
                            ("title".into(), SchemaNode::String),
                            ("url".into(), SchemaNode::String),
                            ("snippet".into(), SchemaNode::Optional {
                                inner: Box::new(SchemaNode::String),
                            }),
                        ]),
                    }),
                }),
                ("total".into(), SchemaNode::Number),
            ]),
        };

        // 合法
        let ok = json!({
            "results": [
                {"title": "A", "url": "https://x", "snippet": "..."},
                {"title": "B", "url": "https://y"},
            ],
            "total": 2
        });
        assert!(validate(&schema, &ok).is_ok());

        // 第一项缺 title → $.results[0].title
        let bad = json!({
            "results": [
                {"url": "https://x"},
            ],
            "total": 1
        });
        let r = validate(&schema, &bad).unwrap_err();
        assert_eq!(r.path, "$.results[0].title");
        assert_eq!(r.actual, "missing");
    }

    /// schema_label 格式化
    #[test]
    fn schema_label_formats() {
        assert_eq!(schema_label(&SchemaNode::String), "string");
        assert_eq!(
            schema_label(&SchemaNode::Array {
                item: Box::new(SchemaNode::Number),
            }),
            "array<number>"
        );
        assert_eq!(
            schema_label(&SchemaNode::Optional {
                inner: Box::new(SchemaNode::String),
            }),
            "optional<string>"
        );
    }

    /// ValidationError serde 序列化 (tool_bridge.rs 回灌需要)
    #[test]
    fn validation_error_serializes_to_structured_json() {
        let schema = SchemaNode::Object {
            fields: BTreeMap::from([("count".into(), SchemaNode::Number)]),
        };
        let err = validate(&schema, &json!({"count": "x"})).unwrap_err();
        let s = serde_json::to_string(&err).unwrap();
        // 结构化字段保留, 不是 Display 字符串
        assert!(s.contains("\"path\":\"$.count\""));
        assert!(s.contains("\"expected\":\"number\""));
        assert!(s.contains("\"actual\":\"string\""));
        assert!(s.contains("\"hint\""));
    }
}