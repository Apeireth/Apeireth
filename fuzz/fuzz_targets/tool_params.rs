#![no_main]

//! fuzz: **能力级工具的参数面** (取参 / 校验 / 边界), 不是审批分档面。
//!
//! 覆盖的能力级工具 (仓内 `crates/capabilities/tools/src/`):
//! - `doc_ops.rs`        —— 文档操作 (`tool.doc_ops`)
//! - `sheet_ops.rs`      —— 表格操作 (`tool.sheet_ops`)
//! - `skill/distill.rs`  —— 技能沉淀 (`tool.skill_distill`)
//! - `voice/speak.rs`    —— 语音播报 (`tool.voice_speak`)
//! - `worktree_apply.rs` —— 工作树应用 (`tool.worktree_apply`)
//! - `device/device_control.rs` —— 设备受控执行 (`tool.device_control`)
//!   (只做 `device_actions` target **没覆盖**的部分: 三拍同计划绑定 +
//!    `params` 对象形状 + 白名单/词表边界; 动作参数闸 `validate_action_params`
//!    已由 `device_actions` 覆盖, 此处不重复)
//!
//! # 为什么这个面值得 fuzz
//!
//! 每个能力级工具都有一段「**从 JSON 参数里取字段**」的代码。那段代码历史上
//! 反复出真缺陷: 畸形值被静默回落成默认值 (fail-open) —— 「没给」与「给了
//! 畸形值」混为一谈, 于是本该拒绝的调用照常执行。本 target 的判据正是
//! 这条: **类型不符 / 越界 / 必填缺失 ⇒ 必须报错, 不得静默变成另一个值**。
//!
//! # 判据来源 (每条断言都能从被测函数自己的文档/注释推出来)
//!
//! 判据分两类:
//! 1. **类型面** (来自各工具自己写明的 JSON schema `declaration()` 与
//!    `#[serde(...)]` 字段类型): 字段类型不符 ⇒ 反序列化**必须失败**;
//!    未知字段 ⇒ 反序列化**必须失败** (`deny_unknown_fields`)。
//! 2. **值域/必填面** (来自各校验函数自己的 doc 注释):
//!    - `doc_ops::doc_ops_params_refusal` (doc_ops.rs:3490-3494)
//!    - `sheet_ops::sheet_ops_params_refusal` (sheet_ops.rs:2040-2045)
//!    - `skill::distill::source_request_refusal` (distill.rs:100-103)
//!    - `skill::distill::distill_request_refusal` (distill.rs:143-145)
//!    - `skill::card::card_draft_refusal` (card.rs:160-164)
//!    - `voice::speak::speak_request_refusal` (speak.rs:224-228)
//!    - `worktree_apply::prepare_apply` (worktree_apply.rs:261-262)
//!    - `worktree_plan::change_order_refusal` (worktree_plan.rs:224-228)
//!
//! # 判据写法纪律 (来自 `docs/02-guides/fuzz-target-authoring.md`)
//! - 断言一律用**可判定的后置条件** (「必须 Err」/「必须 Some」/「必须 None」),
//!   不用 `contains(某 token)` 判「有泄漏」;
//! - **默认值只有在契约写明允许回落时才算合法** (如 `voice_speak.rate` 缺省
//!   = 引擎默认, `sheet_ops.order` 缺省 `asc`, `doc_ops.max_chars` 可收紧);
//! - 不自造期望值: 每条期望都能指回上面那几行文档。
//!
//! # 本 target 抓到的两个真缺陷 (已修, 这里是常驻回归闸)
//! 1. `skill_distill` 的 `path` 只读通道只做**词法层**限域: 工作区内的符号链接
//!    指向工作区外的文件时, 内容被读进技能卡 (敏感面判定也只落在拼写上,
//!    看不见真实目标) —— 见 `probe_symlink_escape`。
//! 2. `doc_ops` 的参数矩阵文档承诺「每动作的必填/**禁填**字段矩阵」, 实现只做
//!    「必填」: 多给的字段被静默吞掉 —— 见通用对象面的禁填断言。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value, json};

use apeireth_tools_canonical::device::device_control::{
    DEVICE_CONTROL_BEATS, DeviceControlConfig, DeviceControlParams, DeviceControlRunner,
};
use apeireth_tools_canonical::doc_ops::{
    DOC_OPS_ACTIONS, DOC_OPS_MAX_SOURCE_CHARS, DOC_OPS_MAX_SOURCE_PARAGRAPHS, DocOpsConfig,
    DocOpsParams, doc_ops_params_refusal,
};
use apeireth_tools_canonical::sheet_ops::{
    AGGREGATE_FUNCTIONS, FILTER_PREDICATES, SHEET_OPS_ACTIONS, SORT_ORDERS, SheetOpsConfig,
    SheetOpsParams, sheet_ops_params_refusal,
};
use apeireth_tools_canonical::skill::card::{SkillCardDraft, card_draft_refusal};
use apeireth_tools_canonical::skill::distill::{
    SKILL_SOURCE_MAX_BYTES, SkillDistillParams, distill_request_refusal, source_request_refusal,
};
use apeireth_tools_canonical::voice::speak::{
    SpeakScope, VOICE_RATE_MAX, VOICE_RATE_MIN, VOICE_SPEAK_MAX_TEXT_CHARS, VOICE_VOLUME_MAX,
    VOICE_VOLUME_MIN, VoiceSpeakParams, speak_request_refusal,
};
use apeireth_tools_canonical::worktree_apply::{VerifyCommand, WorktreeApplyParams, prepare_apply};
use apeireth_tools_canonical::worktree_plan::{WorktreeChangeOrder, change_order_refusal};

// ---------------------------------------------------------------------------
// 工作区夹具 (一次性): 参数面要判「路径限域 / 输出只创建不覆盖」, 必须有真文件
// ---------------------------------------------------------------------------

/// fuzz 工作区根 (进程内唯一)。
fn workspace() -> &'static Path {
    static WS: OnceLock<PathBuf> = OnceLock::new();
    WS.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("apeireth-fuzz-tool-params-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("in.pdf"), b"%PDF-1.4\n% fixture\n");
        let _ = std::fs::write(dir.join("in.docx"), b"PK\x03\x04 fixture");
        let _ = std::fs::write(dir.join("table.csv"), b"a,b\n1,2\n");
        // 已存在的文件: 用来钉「输出只创建不覆盖」。
        let _ = std::fs::write(dir.join("existing.txt"), b"x");
        let _ = std::fs::create_dir_all(dir.join("out"));
        dir
    })
}

/// `doc_ops` 配置 (写总闸开: 这样「每动作必填矩阵」才是被测的那把尺)。
fn doc_config(gate_open: bool) -> DocOpsConfig {
    DocOpsConfig::new(workspace()).with_write_gate(gate_open)
}

/// `sheet_ops` 配置 (写总闸开: 同上)。
fn sheet_config(gate_open: bool) -> SheetOpsConfig {
    SheetOpsConfig::new(workspace()).with_write_gate(gate_open)
}

// ---------------------------------------------------------------------------
// 类型面表格 (逐字取自各工具自己写明的 JSON schema / serde 字段类型)
// ---------------------------------------------------------------------------

/// JSON 值形状。
#[derive(Clone, Copy, PartialEq, Debug)]
enum JType {
    Str,
    Bool,
    Arr,
    Obj,
    /// 非负整数 (u64 面)。
    UInt,
    /// i32 面 (可负)。
    I32,
    /// u32 面 (可负会越界 ⇒ 反序列化必须失败)。
    U32,
    /// 任意 JSON (字段类型就是 `serde_json::Value`)。
    Any,
}

/// 字段在场语义 (取自字段声明本身)。
#[derive(Clone, Copy, PartialEq, Debug)]
enum Presence {
    /// 必填 (`T` 且无 `#[serde(default)]`): 缺省 ⇒ 反序列化必须失败。
    Required,
    /// 可缺省 (`Option<T>`): 缺省 = 「没给」。
    Optional,
    /// 缺省走 `#[serde(default)]`: 缺省 = 该类型的默认值。
    Defaulted,
}

/// 一个字段的类型契约。
#[derive(Clone, Copy, Debug)]
struct FieldSpec {
    name: &'static str,
    ty: JType,
    presence: Presence,
    /// 闭合枚举值 (若有): 值不在集合内 ⇒ 反序列化必须失败。
    enums: &'static [&'static str],
}

const fn field(name: &'static str, ty: JType, presence: Presence) -> FieldSpec {
    FieldSpec {
        name,
        ty,
        presence,
        enums: &[],
    }
}

/// `voice_speak` 字段面 (speak.rs:539-576 的 `declaration()` + speak.rs:196-215)。
const VOICE_FIELDS: &[FieldSpec] = &[
    field("text", JType::Str, Presence::Required),
    FieldSpec {
        name: "scope",
        ty: JType::Str,
        presence: Presence::Optional,
        enums: &["care", "chat"],
    },
    field("rate", JType::I32, Presence::Optional),
    field("volume", JType::U32, Presence::Optional),
    field("voice", JType::Str, Presence::Optional),
];

/// `skill_distill` 字段面 (distill.rs:324-357 的 `declaration()` + distill.rs:74-89)。
const SKILL_FIELDS: &[FieldSpec] = &[
    field("card", JType::Obj, Presence::Required),
    field("text", JType::Str, Presence::Optional),
    field("path", JType::Str, Presence::Optional),
    field("update", JType::Bool, Presence::Defaulted),
];

/// `doc_ops` 字段面 (doc_ops.rs:3464-3488 的 `DocOpsParams`)。
const DOC_FIELDS: &[FieldSpec] = &[
    field("action", JType::Str, Presence::Optional),
    field("input", JType::Str, Presence::Optional),
    field("inputs", JType::Arr, Presence::Optional),
    field("output", JType::Str, Presence::Optional),
    field("output_dir", JType::Str, Presence::Optional),
    field("pages", JType::Arr, Presence::Optional),
    field("watermark_text", JType::Str, Presence::Optional),
    field("max_chars", JType::UInt, Presence::Optional),
    field("text", JType::Str, Presence::Optional),
];

/// `sheet_ops` 字段面 (sheet_ops.rs:1987-2017 的 `SheetOpsParams`)。
const SHEET_FIELDS: &[FieldSpec] = &[
    field("action", JType::Str, Presence::Optional),
    field("source", JType::Str, Presence::Optional),
    field("output", JType::Str, Presence::Optional),
    field("column", JType::Str, Presence::Optional),
    field("predicate", JType::Str, Presence::Optional),
    field("value", JType::Str, Presence::Optional),
    field("order", JType::Str, Presence::Optional),
    field("function", JType::Str, Presence::Optional),
    field("expression", JType::Str, Presence::Optional),
    field("new_column", JType::Str, Presence::Optional),
    field("offset", JType::UInt, Presence::Optional),
    field("limit", JType::UInt, Presence::Optional),
];

/// `worktree_apply` 字段面 (worktree_apply.rs:586-644 的 `declaration()` +
/// worktree_apply.rs:133-144 的 `WorktreeApplyParams`)。
const WORKTREE_FIELDS: &[FieldSpec] = &[
    field("plan", JType::Obj, Presence::Required),
    field("patch", JType::Str, Presence::Required),
    field("verify", JType::Arr, Presence::Defaulted),
];

/// `device_control` 字段面 (device_control.rs:1984-2030 的 `declaration()` +
/// device_control.rs:1910-1929 的 `DeviceControlParams`)。
const DEVICE_FIELDS: &[FieldSpec] = &[
    field("action", JType::Str, Presence::Required),
    field("device", JType::Str, Presence::Required),
    field("device_action", JType::Str, Presence::Required),
    field("params", JType::Any, Presence::Defaulted),
    field("plan_id", JType::Str, Presence::Optional),
    field("approved_by", JType::Str, Presence::Optional),
    field("im_card_id", JType::Str, Presence::Optional),
];

/// 面的编号 (与 `fuzz_target` 里的分派一致)。
const SURFACE_VOICE: usize = 0;
const SURFACE_SKILL: usize = 1;
const SURFACE_DOC: usize = 2;
const SURFACE_SHEET: usize = 3;
const SURFACE_WORKTREE: usize = 4;
const SURFACE_DEVICE: usize = 5;

/// 取某个面的字段表。
fn fields_of(surface: usize) -> &'static [FieldSpec] {
    match surface {
        SURFACE_VOICE => VOICE_FIELDS,
        SURFACE_SKILL => SKILL_FIELDS,
        SURFACE_DOC => DOC_FIELDS,
        SURFACE_SHEET => SHEET_FIELDS,
        SURFACE_WORKTREE => WORKTREE_FIELDS,
        _ => DEVICE_FIELDS,
    }
}

/// 标量类型是否吻合 (`Some(true)` = 吻合; `Some(false)` = 是整数但越界;
/// `None` = 类型就不对)。三种情形都 ⇒ 反序列化必须失败 (除 `Some(true)`)。
fn scalar_fits(value: &Value, spec: &FieldSpec) -> Option<bool> {
    match spec.ty {
        JType::Any => Some(true),
        JType::Str => {
            if !value.is_string() {
                return None;
            }
            let text = value.as_str().unwrap_or_default();
            if spec.enums.is_empty() || spec.enums.contains(&text) {
                Some(true)
            } else {
                Some(false)
            }
        }
        JType::Bool => {
            if value.is_boolean() {
                Some(true)
            } else {
                None
            }
        }
        JType::Arr => {
            if value.is_array() {
                Some(true)
            } else {
                None
            }
        }
        JType::Obj => {
            if value.is_object() {
                Some(true)
            } else {
                None
            }
        }
        JType::UInt => match value.as_u64() {
            Some(_) => Some(true),
            None => {
                if value.is_i64() || value.is_u64() {
                    Some(false)
                } else {
                    None
                }
            }
        },
        JType::I32 => match value.as_i64() {
            Some(number) => {
                if i32::try_from(number).is_ok() {
                    Some(true)
                } else {
                    Some(false)
                }
            }
            None => {
                if value.is_i64() || value.is_u64() {
                    Some(false)
                } else {
                    None
                }
            }
        },
        JType::U32 => match value.as_i64() {
            Some(number) => {
                if (0..=i64::from(u32::MAX)).contains(&number) {
                    Some(true)
                } else {
                    Some(false)
                }
            }
            None => {
                if value.is_i64() || value.is_u64() {
                    Some(false)
                } else {
                    None
                }
            }
        },
    }
}

/// 字段取值是否符合它自己声明的类型 (含嵌套结构)。
///
/// `Some(true)` = 符合; `Some(false)` = 不符合 (⇒ 整个参数反序列化必须失败);
/// `None` = 该字段无类型约束。
fn field_fits(surface: usize, field: &str, value: &Value) -> Option<bool> {
    let spec = fields_of(surface)
        .iter()
        .find(|spec| spec.name == field)
        .expect("字段在类型表里");
    if value.is_null() {
        return match spec.presence {
            Presence::Optional => Some(true),
            // 必填字段给 `null` ⇒ 反序列化必须失败。
            Presence::Required => Some(false),
            // `#[serde(default)]` 字段给 `null` ⇒ 也失败 (default 只管「缺省」);
            // 例外: 字段类型就是 `serde_json::Value` (`null` 是合法取值)。
            Presence::Defaulted => match spec.ty {
                JType::Any => Some(true),
                _ => Some(false),
            },
        };
    }
    match (surface, field) {
        // 结构体字段: 「符合」= 该结构体自身能反序列化。
        (SURFACE_SKILL, "card") => {
            Some(serde_json::from_value::<SkillCardDraft>(value.clone()).is_ok())
        }
        (SURFACE_WORKTREE, "plan") => {
            Some(serde_json::from_value::<WorktreeChangeOrder>(value.clone()).is_ok())
        }
        (SURFACE_WORKTREE, "verify") => {
            Some(serde_json::from_value::<Vec<VerifyCommand>>(value.clone()).is_ok())
        }
        // 数组字段: 元素类型也属于契约 (元素类型不符 ⇒ 反序列化必须失败)。
        (SURFACE_DOC, "inputs") => Some(
            value
                .as_array()
                .is_some_and(|items| items.iter().all(|item| item.is_string())),
        ),
        (SURFACE_DOC, "pages") => Some(
            value
                .as_array()
                .is_some_and(|items| items.iter().all(|item| item.as_u64().is_some())),
        ),
        _ => scalar_fits(value, spec),
    }
}

// ---------------------------------------------------------------------------
// 结构化输入: 从 fuzz 字节造一个「字段槽」
// ---------------------------------------------------------------------------

/// fuzz 字节 → 文本 (任意字节 ⇒ 合法 `str`)。
fn text_of(payload: &[u8]) -> String {
    String::from_utf8_lossy(payload).into_owned()
}

/// fuzz 字节 → 整数 (含负数 / 大数)。
fn int_of(payload: &[u8]) -> i64 {
    let mut bytes = [0u8; 8];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = payload.get(index).copied().unwrap_or(0);
    }
    let raw = i64::from_le_bytes(bytes);
    // 折叠到一个小范围, 让 fuzzer 更容易命中边界两侧 (±64)。
    (raw % 129) - 64
}

/// 一个「字段槽」: 字段名 + 值 (`None` = 该字段缺省)。
fn slot_value(shape: usize, payload: &[u8]) -> Option<Value> {
    match shape % 11 {
        0 => None,
        1 => Some(Value::Null),
        2 => Some(Value::String(String::new())),
        3 => Some(Value::String(text_of(payload))),
        4 => Some(Value::from(int_of(payload))),
        5 => Some(Value::from(int_of(payload) as f64 / 2.0)),
        6 => Some(Value::Bool(
            payload.first().is_some_and(|byte| byte & 1 == 1),
        )),
        7 => Some(Value::Array(Vec::new())),
        8 => Some(Value::Object(Map::new())),
        9 => Some(Value::Array(vec![Value::from(int_of(payload))])),
        _ => Some(Value::Object(Map::from_iter([(
            "k".to_string(),
            Value::from(int_of(payload)),
        )]))),
    }
}

// ---------------------------------------------------------------------------
// 期望
// ---------------------------------------------------------------------------

/// 一个字段在给定取值下的期望后置条件。
#[derive(Clone, Copy)]
enum Expect {
    /// 反序列化必须失败 (类型不符 / 越界 / 枚举外 / 必填缺失)。
    DeserErr,
    /// 反序列化必须成功, 且校验面必须拒绝。
    Refuse,
    /// 反序列化必须成功, 且校验面必须放行。
    Accept,
    /// 反序列化必须成功; 拒绝与否由取值决定 (`Some(true)` = 必须拒绝,
    /// `Some(false)` = 必须放行, `None` = 不判)。
    Decided(Option<bool>),
}

/// 期望「缺省」时该怎么判 (逐字段, 取自各契约的必填面)。
fn absent_rule(surface: usize, field: &str, baseline: &Map<String, Value>) -> Expect {
    match surface {
        SURFACE_VOICE => match field {
            // `text` 必填 ⇒ 缺省即反序列化失败 (已由 Required 覆盖)。
            "text" => Expect::DeserErr,
            // 其余字段缺省 = 引擎默认 (契约写明允许回落)。
            _ => Expect::Accept,
        },
        SURFACE_SKILL => match field {
            "card" => Expect::DeserErr,
            "update" => Expect::Accept,
            // 内容输入二选一: 另一个来源还在 ⇒ 放行; 不在 ⇒ 拒绝。
            "text" => {
                if baseline
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| !path.trim().is_empty())
                {
                    Expect::Accept
                } else {
                    Expect::Refuse
                }
            }
            "path" => {
                if baseline
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    Expect::Accept
                } else {
                    Expect::Refuse
                }
            }
            _ => Expect::Accept,
        },
        SURFACE_DOC => match field {
            // `max_chars` 是「参数可收紧」的预算, 缺省 = 配置上限 (契约允许)。
            "max_chars" => Expect::Accept,
            // 其余字段在该动作的必填面里 ⇒ 缺省必须拒绝。
            _ => Expect::Refuse,
        },
        SURFACE_SHEET => match field {
            // `order` / `offset` / `limit` 缺省有契约写明的默认值。
            "order" | "offset" | "limit" => Expect::Accept,
            _ => Expect::Refuse,
        },
        SURFACE_WORKTREE => match field {
            "verify" => Expect::Accept,
            _ => Expect::DeserErr,
        },
        _ => match field {
            "action" | "device" | "device_action" => Expect::DeserErr,
            // `params` 缺省 = 空参数 (契约写明)。
            "params" => Expect::Accept,
            _ => Expect::Accept,
        },
    }
}

// ---------------------------------------------------------------------------
// 各能力级工具的基线 (每个字段一套完整合法参数)
// ---------------------------------------------------------------------------

/// `voice_speak` 合法基线 (speak.rs:196-215 的字段集)。
fn voice_baseline(field: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("text".into(), json!("一句"));
    map.insert("scope".into(), json!("care"));
    match field {
        "rate" => {
            map.insert("rate".into(), json!(0));
        }
        "volume" => {
            map.insert("volume".into(), json!(50));
        }
        "voice" => {
            map.insert("voice".into(), json!("嗓"));
        }
        _ => {}
    }
    map
}

/// `skill_distill` 合法基线 (distill.rs:74-89 的字段集)。
fn skill_baseline(field: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("card".into(), valid_card());
    match field {
        "path" => {
            map.insert("path".into(), json!("in.md"));
        }
        _ => {
            map.insert("text".into(), json!("正文"));
        }
    }
    map
}

/// 合法技能卡草稿 (card.rs:73-84 的字段集)。
fn valid_card() -> Value {
    json!({
        "name": "样例",
        "summary": "摘要",
        "triggers": ["触发"],
        "body": "正文",
        "source": { "kind": "document", "title": "标题" }
    })
}

/// `doc_ops` 合法基线: 按字段给对应动作的完整合法参数集。
fn doc_baseline(field: &str) -> Map<String, Value> {
    let mut map = Map::new();
    match field {
        "action" => {
            map.insert("action".into(), json!("pdf_info"));
            map.insert("input".into(), json!("in.pdf"));
        }
        "input" => {
            map.insert("action".into(), json!("pdf_info"));
            map.insert("input".into(), json!("in.pdf"));
        }
        "output" => {
            map.insert("action".into(), json!("pdf_merge"));
            map.insert("inputs".into(), json!(["in.pdf", "in.docx"]));
            map.insert("output".into(), json!("out/merged.pdf"));
        }
        "output_dir" => {
            map.insert("action".into(), json!("pdf_split"));
            map.insert("input".into(), json!("in.pdf"));
            map.insert("output_dir".into(), json!("out"));
        }
        "pages" => {
            map.insert("action".into(), json!("pdf_extract_pages"));
            map.insert("input".into(), json!("in.pdf"));
            map.insert("output".into(), json!("out/pages.pdf"));
            map.insert("pages".into(), json!([1, 2]));
        }
        "watermark_text" => {
            map.insert("action".into(), json!("pdf_watermark_text"));
            map.insert("input".into(), json!("in.pdf"));
            map.insert("output".into(), json!("out/wm.pdf"));
            map.insert("watermark_text".into(), json!("WM"));
        }
        "inputs" => {
            map.insert("action".into(), json!("pdf_merge"));
            map.insert("inputs".into(), json!(["in.pdf", "in.docx"]));
            map.insert("output".into(), json!("out/merged.pdf"));
        }
        "max_chars" => {
            map.insert("action".into(), json!("pdf_extract_text"));
            map.insert("input".into(), json!("in.pdf"));
            map.insert("max_chars".into(), json!(100));
        }
        _ => {
            // `text` (源文面: docx_from_markdown / docx_append)
            map.insert("action".into(), json!("docx_from_markdown"));
            map.insert("text".into(), json!("# 标题\n\n正文"));
            map.insert("output".into(), json!("out/doc.docx"));
        }
    }
    map
}

/// `sheet_ops` 合法基线: 按字段给对应动作的完整合法参数集。
fn sheet_baseline(field: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("source".into(), json!("table.csv"));
    match field {
        "action" => {
            map.insert("action".into(), json!("read"));
        }
        "offset" => {
            map.insert("action".into(), json!("read"));
            map.insert("offset".into(), json!(0));
        }
        "limit" => {
            map.insert("action".into(), json!("read"));
            map.insert("limit".into(), json!(10));
        }
        "column" => {
            map.insert("action".into(), json!("filter"));
            map.insert("column".into(), json!("a"));
            map.insert("predicate".into(), json!("eq"));
            map.insert("value".into(), json!("1"));
        }
        "predicate" => {
            map.insert("action".into(), json!("filter"));
            map.insert("column".into(), json!("a"));
            map.insert("predicate".into(), json!("eq"));
            map.insert("value".into(), json!("1"));
        }
        "value" => {
            map.insert("action".into(), json!("filter"));
            map.insert("column".into(), json!("a"));
            map.insert("predicate".into(), json!("eq"));
            map.insert("value".into(), json!("1"));
        }
        "order" => {
            map.insert("action".into(), json!("sort"));
            map.insert("column".into(), json!("a"));
            map.insert("order".into(), json!("asc"));
        }
        "function" => {
            map.insert("action".into(), json!("aggregate"));
            map.insert("column".into(), json!("a"));
            map.insert("function".into(), json!("sum"));
        }
        "expression" => {
            map.insert("action".into(), json!("formula"));
            map.insert("expression".into(), json!("a+1"));
            map.insert("new_column".into(), json!("c"));
        }
        "new_column" => {
            map.insert("action".into(), json!("formula"));
            map.insert("expression".into(), json!("a+1"));
            map.insert("new_column".into(), json!("c"));
        }
        "output" => {
            map.insert("action".into(), json!("export_csv"));
            map.insert("output".into(), json!("out/table.csv"));
        }
        _ => {
            map.insert("action".into(), json!("summary"));
        }
    }
    map
}

/// `worktree_apply` 合法基线 (worktree_apply.rs:586-644 的字段集)。
fn worktree_baseline(field: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("plan".into(), valid_plan());
    map.insert("patch".into(), json!(valid_patch()));
    if field == "verify" {
        map.insert("verify".into(), json!([{ "program": "true" }]));
    }
    map
}

/// 合法改单 (worktree_plan.rs:94-104 的字段集)。
fn valid_plan() -> Value {
    json!({
        "goal": "加一个文件",
        "files": [{ "path": "new.txt", "action": "add", "expect": "新建" }]
    })
}

/// 与 `valid_plan` 逐项相符的补丁 (worktree_apply.rs:197-249 的交叉一致口径)。
fn valid_patch() -> String {
    "*** Begin Patch\n*** Add File: new.txt\n+hello\n*** End Patch".to_string()
}

/// `device_control` 合法基线 (device_control.rs:1984-2030 的字段集)。
fn device_baseline(field: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("action".into(), json!("plan"));
    map.insert("device".into(), json!("lamp"));
    map.insert("device_action".into(), json!("power_on"));
    if field == "params" {
        map.insert("params".into(), json!({}));
    }
    map
}

// ---------------------------------------------------------------------------
// 值域规则 (逐条来自被测函数自己的文档)
// ---------------------------------------------------------------------------

/// `voice_speak::speak_request_refusal` (speak.rs:224-228):
/// ① 文本去空白后非空且限长; ② 语速在 `[-10, 10]` (越界**拒绝**, 不钳制);
/// ③ 音量在 `[0, 100]`; ④ 给了嗓就必须给非空嗓名。
fn voice_rule(field: &str, value: &Value) -> Option<bool> {
    match (field, value) {
        ("text", Value::String(text)) => {
            if text.trim().is_empty() {
                Some(true)
            } else {
                Some(text.chars().count() > VOICE_SPEAK_MAX_TEXT_CHARS)
            }
        }
        ("rate", Value::Number(number)) => {
            let rate = number.as_i64()?;
            Some(!(i64::from(VOICE_RATE_MIN)..=i64::from(VOICE_RATE_MAX)).contains(&rate))
        }
        ("volume", Value::Number(number)) => {
            let volume = number.as_i64()?;
            if volume < 0 {
                return None;
            }
            Some(!(i64::from(VOICE_VOLUME_MIN)..=i64::from(VOICE_VOLUME_MAX)).contains(&volume))
        }
        ("voice", Value::String(name)) => Some(name.trim().is_empty()),
        _ => None,
    }
}

/// `skill_distill::source_request_refusal` (distill.rs:100-103):
/// ① `text`/`path` 恰给一个且非空; ② 内联正文限字节; ③ 文件路径词法限域
/// (禁绝对路径 / 盘符 UNC / `..` 组件)。
fn skill_rule(field: &str, value: &Value, baseline: &Map<String, Value>) -> Option<bool> {
    match field {
        "text" => match value {
            Value::String(text) => {
                if text.trim().is_empty() {
                    return Some(true);
                }
                // 字节上限 (distill.rs:54: 「内容输入的字节上限」)。
                Some(text.len() > SKILL_SOURCE_MAX_BYTES)
            }
            _ => None,
        },
        "path" => match value {
            Value::String(path) => {
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    return Some(true);
                }
                let target = Path::new(trimmed);
                if target.is_absolute() {
                    return Some(true);
                }
                if has_parent_dir(trimmed) {
                    return Some(true);
                }
                // 与 `text` 同时在场 ⇒ 二选一违约 (distill.rs:113-115)。
                let text_present = baseline
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.trim().is_empty());
                if text_present {
                    return Some(true);
                }
                // 路径是否真的可读 / 是否落在敏感面, 交给库判。
                None
            }
            _ => None,
        },
        _ => None,
    }
}

/// `doc_ops::doc_ops_params_refusal` (doc_ops.rs:3490-3494):
/// ① 动作在闭合词表; ② 每动作的必填/禁填字段矩阵; ③ 选页合法
/// (≥1 / 无重复 / 不超上限); ④ 路径工作区限域 + 敏感面拒; ⑤ 输入大小上限;
/// ⑥ 输出文件只创建不覆盖。
fn doc_rule(field: &str, value: &Value, baseline: &Map<String, Value>) -> Option<bool> {
    let config = doc_config(true);
    match field {
        "action" => match value {
            Value::String(action) => {
                if !DOC_OPS_ACTIONS.contains(&action.as_str()) {
                    return Some(true);
                }
                // 词表内的动作: 基线只满足 `pdf_info` 一族的必填面, 其余动作
                // 必填字段缺失 ⇒ 必须拒绝 (每动作必填矩阵)。
                let read_family = matches!(
                    action.as_str(),
                    "pdf_info" | "pdf_extract_text" | "docx_extract_text" | "docx_outline"
                );
                Some(!read_family)
            }
            _ => None,
        },
        "input" => match value {
            Value::String(path) => path_must_refuse(path),
            _ => None,
        },
        "output" => match value {
            Value::String(path) => {
                // 「输出文件只创建不覆盖」(doc_ops.rs:3683-3691): 目标已存在
                // ⇒ 必须拒绝。
                if path.trim().is_empty() {
                    return Some(true);
                }
                if escapes_workspace(path) {
                    return Some(true);
                }
                if workspace().join(path).exists() {
                    return Some(true);
                }
                // 别的取值不判: 路径是否落在敏感面 / 是否能解析, 交给库判。
                None
            }
            _ => None,
        },
        "output_dir" => match value {
            Value::String(path) => path_must_refuse(path),
            _ => None,
        },
        "inputs" => match value {
            Value::Array(items) => {
                // 「pdf_merge 必须给 2-32 个 inputs」(doc_ops.rs:3527-3544)。
                let baseline_action = baseline
                    .get("action")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if baseline_action != "pdf_merge" {
                    return None;
                }
                if !(2..=32).contains(&items.len()) {
                    return Some(true);
                }
                let empty = items
                    .iter()
                    .any(|item| item.as_str().is_none_or(|text| text.trim().is_empty()));
                if empty {
                    return Some(true);
                }
                // 输入路径是否真的存在 / 是否落在敏感面, 交给库判。
                None
            }
            _ => None,
        },
        "pages" => match value {
            Value::Array(items) => {
                // 「1 起页码列表, 非空」+「0 非法」+「无重复」+「不超上限」
                // (doc_ops.rs:3554-3586)。
                if items.is_empty() {
                    return Some(true);
                }
                let mut seen = std::collections::BTreeSet::new();
                for item in items {
                    let Some(page) = item.as_u64() else {
                        return None;
                    };
                    if page == 0 {
                        return Some(true);
                    }
                    if !seen.insert(page) {
                        return Some(true);
                    }
                }
                Some(items.len() as u64 > config.max_pages)
            }
            _ => None,
        },
        "watermark_text" => match value {
            Value::String(text) => Some(text.trim().is_empty()),
            _ => None,
        },
        "max_chars" => None,
        "text" => match value {
            Value::String(text) => {
                let chars = text.chars().count();
                if chars > DOC_OPS_MAX_SOURCE_CHARS {
                    return Some(true);
                }
                let paragraphs = text.lines().filter(|line| !line.trim().is_empty()).count();
                if paragraphs > DOC_OPS_MAX_SOURCE_PARAGRAPHS {
                    return Some(true);
                }
                // XML 非法控制字符拒绝即帧 (doc_ops.rs:3642-3646)。
                if text.chars().any(is_xml_illegal) {
                    return Some(true);
                }
                Some(text.trim().is_empty())
            }
            _ => None,
        },
        _ => None,
    }
}

/// 路径类字段的「必须拒绝」判定 (doc_ops / sheet_ops 口径):
/// 空路径 / **逃出工作区根**。
///
/// 其余取值 (存在性 / 敏感面 / 能否解析) **不判** —— 那些是库自己的判定,
/// harness 在这里复刻敏感词表即等于自造期望。
///
/// 注: 这两个面的契约是「Paths must stay inside the workspace root」
/// (doc_ops.rs:207 / sheet_ops.rs:236), **不是**「必须相对路径」——
/// 工作区根**之内**的绝对路径库是放行的, 所以不能把「绝对路径」写成拒绝面。
fn path_must_refuse(path: &str) -> Option<bool> {
    if path.trim().is_empty() {
        return Some(true);
    }
    if escapes_workspace(path) {
        return Some(true);
    }
    None
}

/// 该路径是否逃出工作区根 (doc_ops / sheet_ops 的 `resolve_contained` 口径)。
///
/// 拼写层: 拼写里出现 `..` + 分隔符 (`../` / `..\`) ⇒ 路径穿越拒
/// (库: `ToolGuardrail::verify_path_access` 的 `PathTraversal`)。
/// 现实层: 绝对路径若解析后不在工作区根之下 ⇒ 越界拒。
fn escapes_workspace(requested: &str) -> bool {
    let root = workspace();
    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        root.join(requested)
    };
    let spelling = candidate.to_string_lossy().into_owned();
    if spelling.contains("../") || spelling.contains("..\\") {
        return true;
    }
    if !Path::new(requested).is_absolute() {
        return false;
    }
    let canonical_root = std::fs::canonicalize(root).ok();
    let resolved = std::fs::canonicalize(&candidate).ok().or_else(|| {
        candidate
            .parent()
            .and_then(|parent| std::fs::canonicalize(parent).ok())
    });
    match (canonical_root, resolved) {
        (Some(canonical_root), Some(resolved)) => !resolved.starts_with(&canonical_root),
        // 解析不出来 ⇒ 库必拒 (输入不存在 / 工作区根不可访问 / 越界)。
        _ => true,
    }
}

/// 路径含 `..` 组件? (skill_distill / worktree_plan 的口径: **组件级**拒绝)
fn has_parent_dir(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}

/// XML 1.0 非法控制字符 (与 `doc_ops` 内部同一把尺)。
fn is_xml_illegal(character: char) -> bool {
    let point = character as u32;
    point < 0x20 && !matches!(character, '\t' | '\n' | '\r')
}

/// `sheet_ops::sheet_ops_params_refusal` (sheet_ops.rs:2040-2045):
/// ① 动作在闭合词表; ② 每动作的必填/禁填字段矩阵; ③ 谓词/函数/顺序
/// 在闭合词表; ④ 写面受写总闸约束; ⑤ 路径限域 + 敏感面拒; ⑥ 输出只创建不覆盖。
fn sheet_rule(field: &str, value: &Value) -> Option<bool> {
    let config = sheet_config(true);
    match field {
        "action" => match value {
            Value::String(candidate) => {
                if !SHEET_OPS_ACTIONS.contains(&candidate.as_str()) {
                    return Some(true);
                }
                // 基线是 `action=read` + `source`: 只有 read / summary 两个动作
                // 的必填面被满足 ⇒ 其余动作必填缺失 ⇒ 必须拒绝。
                Some(!matches!(candidate.as_str(), "read" | "summary"))
            }
            _ => None,
        },
        "source" => match value {
            Value::String(path) => path_must_refuse(path),
            _ => None,
        },
        "output" => match value {
            Value::String(path) => {
                if path.trim().is_empty() {
                    return Some(true);
                }
                if escapes_workspace(path) {
                    return Some(true);
                }
                // 「输出只创建不覆盖」(sheet_ops.rs:2237-2251)。
                if workspace().join(path).exists() {
                    return Some(true);
                }
                None
            }
            _ => None,
        },
        "column" => match value {
            Value::String(name) => Some(name.trim().is_empty()),
            _ => None,
        },
        "predicate" => match value {
            Value::String(name) => Some(!FILTER_PREDICATES.contains(&name.as_str())),
            _ => None,
        },
        "value" => match value {
            Value::String(text) => Some(text.is_empty()),
            _ => None,
        },
        "order" => match value {
            Value::String(name) => Some(!SORT_ORDERS.contains(&name.as_str())),
            _ => None,
        },
        "function" => match value {
            Value::String(name) => Some(!AGGREGATE_FUNCTIONS.contains(&name.as_str())),
            _ => None,
        },
        "expression" => match value {
            Value::String(text) => Some(text.trim().is_empty()),
            _ => None,
        },
        "new_column" => match value {
            Value::String(name) => {
                if name.trim().is_empty() {
                    return Some(true);
                }
                // 「formula: new_column 首尾不得含空白」(sheet_ops.rs:2193-2199)。
                Some(name.trim() != name)
            }
            _ => None,
        },
        "offset" => None,
        "limit" => match value {
            Value::Number(number) => {
                let limit = number.as_u64()?;
                // 「read: limit 必须在 1..={max_preview_rows}」(sheet_ops.rs:2115-2123)。
                Some(limit == 0 || limit as usize > config.max_preview_rows)
            }
            _ => None,
        },
        _ => None,
    }
}

/// `worktree_plan::change_order_refusal` (worktree_plan.rs:224-228):
/// ① 任务目标非空; ② 至少一条文件声明; ③ 每条: 路径非空 + 词法包含
/// (禁绝对路径 / `..` 组件) + 预期非空; ④ 同一目标不得重复声明。
fn worktree_plan_rule(value: &Value) -> Option<bool> {
    let Value::Object(plan) = value else {
        return None;
    };
    let goal = plan.get("goal").and_then(Value::as_str).unwrap_or_default();
    if goal.trim().is_empty() {
        return Some(true);
    }
    let Some(files) = plan.get("files").and_then(Value::as_array) else {
        return Some(true);
    };
    if files.is_empty() {
        return Some(true);
    }
    let mut seen = std::collections::BTreeSet::new();
    for file in files {
        let path = file.get("path").and_then(Value::as_str).unwrap_or_default();
        if path.trim().is_empty() {
            return Some(true);
        }
        let target = Path::new(path.trim());
        if target.is_absolute() {
            return Some(true);
        }
        if target
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Some(true);
        }
        let expect = file
            .get("expect")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if expect.trim().is_empty() {
            return Some(true);
        }
        if !seen.insert(path.trim().replace('\\', "/")) {
            return Some(true);
        }
    }
    Some(false)
}

/// `worktree_apply::prepare_apply` (worktree_apply.rs:251-259, 261-262):
/// 校验命令程序名非空; 参数解析 → 改单校验 → 补丁解析 → 交叉一致 → 校验命令合法。
fn worktree_verify_rule(value: &Value) -> Option<bool> {
    let Value::Array(items) = value else {
        return None;
    };
    let mut empty = false;
    for item in items {
        let program = item.get("program").and_then(Value::as_str);
        match program {
            Some(program) if !program.trim().is_empty() => {}
            _ => empty = true,
        }
    }
    Some(empty)
}

/// `device_control` 三拍绑定 (device_control.rs:1932-1943 的工具描述 +
/// device_control.rs:1422-1519 的 runner 语义)。
fn device_rule(field: &str, value: &Value) -> Option<bool> {
    match (field, value) {
        ("action", Value::String(beat)) => Some(!DEVICE_CONTROL_BEATS.contains(&beat.as_str())),
        // 白名单面 (device_control.rs:521-530): 未登记设备拒绝即帧。
        ("device", Value::String(id)) => Some(!DEVICE_MOCK_IDS.contains(&id.as_str())),
        // 每设备动作词表闭合 (device_control.rs:1431-1436)。
        ("device_action", Value::String(action)) => {
            Some(!DEVICE_LAMP_ACTIONS.contains(&action.as_str()))
        }
        // 「params 必须是 JSON 对象」(device_control.rs:778-787): `null` = 空对象,
        // 其余非对象一律拒绝即帧; 基线动作 `power_on` 声明零参数 ⇒ 对象必须为空。
        ("params", Value::Null) => Some(false),
        ("params", Value::Object(map)) => Some(!map.is_empty()),
        ("params", _) => Some(true),
        _ => None,
    }
}

/// mock 白名单里的设备 id (device_control.rs:627-666 的 `mock_fixture_entries`)。
const DEVICE_MOCK_IDS: &[&str] = &["lamp", "air_conditioner", "microwave"];

/// 灯 (`lamp`) 登记的动作词表 (device_control.rs:634-638)。
const DEVICE_LAMP_ACTIONS: &[&str] = &["power_on", "power_off", "set_brightness"];

/// 该槽位是「显式 null」且字段可缺省 ⇒ 语义等同「没给」(`Option<T>` 吃 null)。
fn null_means_absent(surface: usize, field: &str, value: &Value) -> bool {
    if !value.is_null() {
        return false;
    }
    fields_of(surface)
        .iter()
        .find(|spec| spec.name == field)
        .map(|spec| spec.presence == Presence::Optional)
        .unwrap_or(false)
}

/// 槽位期望: 缺省 / 显式 null / 取值不符 三条路。
fn slot_expectation(
    surface: usize,
    field: &str,
    value: Option<Value>,
    baseline: &Map<String, Value>,
    rule: impl Fn(&Value) -> Option<bool>,
) -> Expect {
    match value.clone() {
        None => absent_rule(surface, field, baseline),
        Some(filled) => {
            if null_means_absent(surface, field, &filled) {
                return absent_rule(surface, field, baseline);
            }
            match field_fits(surface, field, &filled) {
                Some(true) => match rule(&filled) {
                    Some(want) => Expect::Decided(Some(want)),
                    // 规则给不出判定 (取值本身合法, 但结果还依赖别的面) ⇒ 不判。
                    None => Expect::Decided(None),
                },
                _ => Expect::DeserErr,
            }
        }
    }
}

/// 枚举式改单构造器: 路径拼写从**安全集合**里选 (避开敏感面词表), 结构由 fuzz
/// 字节选 —— 这样能覆盖改单契约的每一条拒绝面, 又不会把「敏感面命中」误当成
/// 契约违约。
fn plan_value(payload: &[u8]) -> Value {
    let goal = match payload.first().copied().unwrap_or(0) % 3 {
        0 => "目标".to_string(),
        1 => String::new(),
        _ => "   ".to_string(),
    };
    let path = match payload.get(1).copied().unwrap_or(0) % 6 {
        0 => "new.txt".to_string(),
        1 => String::new(),
        2 => "/abs/new.txt".to_string(),
        3 => "../escape.txt".to_string(),
        4 => "./new.txt".to_string(),
        _ => "a/../b.txt".to_string(),
    };
    let action = match payload.get(2).copied().unwrap_or(0) % 3 {
        0 => "add".to_string(),
        1 => "update".to_string(),
        _ => "delete".to_string(),
    };
    let expect = match payload.get(3).copied().unwrap_or(0) % 2 {
        0 => "预期".to_string(),
        _ => String::new(),
    };
    let files = if payload.get(4).copied().unwrap_or(0) % 5 == 0 {
        // 重复声明同一目标 (改单唯一性, worktree_plan.rs:230-231)。
        json!([
            { "path": path, "action": action, "expect": expect },
            { "path": path, "action": action, "expect": expect }
        ])
    } else if payload.get(5).copied().unwrap_or(0) % 7 == 0 {
        // 零条文件声明 (worktree_plan.rs:232-234)。
        json!([])
    } else {
        json!([{ "path": path, "action": action, "expect": expect }])
    };
    json!({ "goal": goal, "files": files })
}

/// 枚举式校验命令构造器 (程序名空 / 非空两档)。
fn verify_value(payload: &[u8]) -> Value {
    let program = match payload.first().copied().unwrap_or(0) % 3 {
        0 => "true".to_string(),
        1 => String::new(),
        _ => "   ".to_string(),
    };
    json!([{ "program": program }])
}

/// 基线自检: harness 自己的「合法参数集」必须被放行 —— 否则后面的断言全是空的。
fn check_baselines() {
    for spec in VOICE_FIELDS {
        let args = voice_baseline(spec.name);
        let params: VoiceSpeakParams =
            serde_json::from_value(Value::Object(args)).expect("voice 基线可解析");
        assert!(
            speak_request_refusal(&params).is_none(),
            "voice_speak 基线 ({}) 竟被拒绝",
            spec.name
        );
    }
    for spec in SKILL_FIELDS {
        let args = skill_baseline(spec.name);
        let params: SkillDistillParams =
            serde_json::from_value(Value::Object(args)).expect("skill 基线可解析");
        assert!(
            distill_request_refusal(&params).is_none(),
            "skill_distill 基线 ({}) 竟被拒绝",
            spec.name
        );
    }
    for spec in DOC_FIELDS {
        let args = doc_baseline(spec.name);
        let params: DocOpsParams =
            serde_json::from_value(Value::Object(args)).expect("doc 基线可解析");
        assert!(
            doc_ops_params_refusal(&doc_config(true), &params).is_none(),
            "doc_ops 基线 ({}) 竟被拒绝",
            spec.name
        );
    }
    for spec in SHEET_FIELDS {
        let args = sheet_baseline(spec.name);
        let params: SheetOpsParams =
            serde_json::from_value(Value::Object(args)).expect("sheet 基线可解析");
        assert!(
            sheet_ops_params_refusal(&sheet_config(true), &params).is_none(),
            "sheet_ops 基线 ({}) 竟被拒绝",
            spec.name
        );
    }
    {
        let args = worktree_baseline("patch");
        assert!(
            prepare_apply(workspace(), &Value::Object(args)).is_ok(),
            "worktree_apply 基线竟被拒绝"
        );
    }
    {
        let args = device_baseline("params");
        let params: DeviceControlParams =
            serde_json::from_value(Value::Object(args)).expect("device 基线可解析");
        let runner = DeviceControlRunner::mock(DeviceControlConfig::new(workspace()));
        assert!(
            runner
                .plan(&params.device, &params.device_action, &json!({}))
                .is_ok(),
            "device_control 基线竟被拒绝"
        );
    }
}

// ---------------------------------------------------------------------------
// 各面执行
// ---------------------------------------------------------------------------

/// 把一个字段槽写进基线 (缺省 = 删除该字段)。
fn apply_slot(baseline: &mut Map<String, Value>, field: &str, value: Option<Value>) {
    match value {
        Some(value) => {
            baseline.insert(field.to_string(), value);
        }
        None => {
            baseline.remove(field);
        }
    }
}

/// 通用断言尾巴: 校验面必须拒绝 / 必须放行 / 取值相关。
fn assert_expectation(tool: &str, field: &str, effective: &Value, expect: Expect, refused: bool) {
    match expect {
        Expect::Refuse => assert!(
            refused,
            "{tool}: 字段 {field} 取值 {effective:?} 违反契约, 校验面却放行了"
        ),
        Expect::Accept => assert!(
            !refused,
            "{tool}: 字段 {field} 取值 {effective:?} 合法, 校验面却拒绝了"
        ),
        Expect::Decided(decision) => {
            if let Some(want) = decision {
                assert_eq!(
                    refused, want,
                    "{tool}: 字段 {field} 取值 {effective:?} 的拒绝判定与契约不符"
                );
            }
        }
        Expect::DeserErr => {}
    }
}

/// `voice_speak` 面。
fn check_voice(field: &str, value: Option<Value>) {
    let mut args = voice_baseline(field);
    apply_slot(&mut args, field, value.clone());
    let effective = value.clone().unwrap_or(Value::Null);
    let expect = slot_expectation(SURFACE_VOICE, field, value.clone(), &args, |filled| {
        voice_rule(field, filled)
    });
    let parsed: Result<VoiceSpeakParams, _> = serde_json::from_value(Value::Object(args.clone()));
    if matches!(expect, Expect::DeserErr) {
        assert!(
            parsed.is_err(),
            "voice_speak: 字段 {field} 的取值 {effective:?} 类型不符/必填缺失, 反序列化却成功了 \
             (畸形值被静默回落成默认值?)"
        );
        return;
    }
    let params = parsed.expect("voice_speak: 期望解析成功");
    let refused = speak_request_refusal(&params).is_some();
    assert_expectation("voice_speak", field, &effective, expect, refused);
}

/// `skill_distill` 面。
fn check_skill(field: &str, value: Option<Value>) {
    let mut args = skill_baseline(field);
    apply_slot(&mut args, field, value.clone());
    let effective = value.clone().unwrap_or(Value::Null);
    let expect = match value.clone() {
        None => absent_rule(SURFACE_SKILL, field, &args),
        Some(filled) => {
            if null_means_absent(SURFACE_SKILL, field, &filled) {
                absent_rule(SURFACE_SKILL, field, &args)
            } else {
                match field_fits(SURFACE_SKILL, field, &filled) {
                    Some(true) => {
                        if field == "card" {
                            // 坏卡拒 (card.rs:160-164): 能解析但内容坏 ⇒ 卡面拒绝。
                            let draft: SkillCardDraft =
                                serde_json::from_value(filled.clone()).expect("已判定为可解析");
                            if card_draft_refusal(&draft).is_some() {
                                Expect::Refuse
                            } else {
                                Expect::Accept
                            }
                        } else {
                            match skill_rule(field, &filled, &args) {
                                Some(want) => Expect::Decided(Some(want)),
                                None => Expect::Accept,
                            }
                        }
                    }
                    _ => Expect::DeserErr,
                }
            }
        }
    };
    let parsed: Result<SkillDistillParams, _> = serde_json::from_value(Value::Object(args.clone()));
    if matches!(expect, Expect::DeserErr) {
        assert!(
            parsed.is_err(),
            "skill_distill: 字段 {field} 的取值 {effective:?} 类型不符/必填缺失, 反序列化却成功了 \
             (畸形值被静默回落成默认值?)"
        );
        return;
    }
    let params = parsed.expect("skill_distill: 期望解析成功");
    let refused = distill_request_refusal(&params).is_some();
    assert_expectation("skill_distill", field, &effective, expect, refused);
}

/// `doc_ops` 面。
fn check_doc(field: &str, value: Option<Value>) {
    let mut args = doc_baseline(field);
    apply_slot(&mut args, field, value.clone());
    let effective = value.clone().unwrap_or(Value::Null);
    let expect = slot_expectation(SURFACE_DOC, field, value.clone(), &args, |filled| {
        doc_rule(field, filled, &args)
    });
    let parsed: Result<DocOpsParams, _> = serde_json::from_value(Value::Object(args.clone()));
    if matches!(expect, Expect::DeserErr) {
        assert!(
            parsed.is_err(),
            "doc_ops: 字段 {field} 的取值 {effective:?} 类型不符/必填缺失, 反序列化却成功了 \
             (畸形值被静默回落成默认值?)"
        );
        return;
    }
    let params = parsed.expect("doc_ops: 期望解析成功");
    let refused = doc_ops_params_refusal(&doc_config(true), &params).is_some();
    assert_expectation("doc_ops", field, &effective, expect, refused);
}

/// `sheet_ops` 面。
fn check_sheet(field: &str, value: Option<Value>) {
    let mut args = sheet_baseline(field);
    apply_slot(&mut args, field, value.clone());
    let effective = value.clone().unwrap_or(Value::Null);
    let expect = slot_expectation(SURFACE_SHEET, field, value.clone(), &args, |filled| {
        sheet_rule(field, filled)
    });
    let parsed: Result<SheetOpsParams, _> = serde_json::from_value(Value::Object(args.clone()));
    if matches!(expect, Expect::DeserErr) {
        assert!(
            parsed.is_err(),
            "sheet_ops: 字段 {field} 的取值 {effective:?} 类型不符/必填缺失, 反序列化却成功了 \
             (畸形值被静默回落成默认值?)"
        );
        return;
    }
    let params = parsed.expect("sheet_ops: 期望解析成功");
    let refused = sheet_ops_params_refusal(&sheet_config(true), &params).is_some();
    assert_expectation("sheet_ops", field, &effective, expect, refused);
}

/// `worktree_apply` 面。
fn check_worktree(field: &str, value: Option<Value>) {
    let mut args = worktree_baseline(field);
    apply_slot(&mut args, field, value.clone());
    let effective = value.clone().unwrap_or(Value::Null);
    let expect = match value.clone() {
        None => absent_rule(SURFACE_WORKTREE, field, &args),
        Some(filled) if null_means_absent(SURFACE_WORKTREE, field, &filled) => {
            absent_rule(SURFACE_WORKTREE, field, &args)
        }
        Some(filled) => match field_fits(SURFACE_WORKTREE, field, &filled) {
            Some(true) => match field {
                // 改单自身的合法性用改单校验面判 (补丁交叉一致是另一把尺,
                // 混在一起会把「改单合法但补丁漂移」误判成契约违约)。
                "plan" => match worktree_plan_rule(&filled) {
                    Some(want) => Expect::Decided(Some(want)),
                    None => Expect::Accept,
                },
                "verify" => match worktree_verify_rule(&filled) {
                    Some(want) => Expect::Decided(Some(want)),
                    None => Expect::Accept,
                },
                // 补丁文本: 只钉「空补丁必须拒」这一条可判定方向; 补丁内容与
                // 改单的交叉一致另由 `check_worktree_binding` 覆盖。
                _ => {
                    if filled.as_str().is_some_and(|text| text.trim().is_empty()) {
                        Expect::Refuse
                    } else {
                        Expect::Decided(None)
                    }
                }
            },
            _ => Expect::DeserErr,
        },
    };
    let parsed: Result<WorktreeApplyParams, _> =
        serde_json::from_value(Value::Object(args.clone()));
    if matches!(expect, Expect::DeserErr) {
        assert!(
            parsed.is_err(),
            "worktree_apply: 字段 {field} 的取值 {effective:?} 类型不符/必填缺失, 反序列化却成功了 \
             (畸形值被静默回落成默认值?)"
        );
        return;
    }
    let params = parsed.expect("worktree_apply: 期望解析成功");
    let refused = match field {
        "plan" => change_order_refusal(workspace(), &params.plan).is_some(),
        _ => prepare_apply(workspace(), &Value::Object(args.clone())).is_err(),
    };
    assert_expectation("worktree_apply", field, &effective, expect, refused);
}

/// `device_control` 面 (只做 `device_actions` 没覆盖的部分)。
fn check_device(field: &str, value: Option<Value>) {
    let mut args = device_baseline(field);
    apply_slot(&mut args, field, value.clone());
    let effective = value.clone().unwrap_or(Value::Null);
    let expect = slot_expectation(SURFACE_DEVICE, field, value.clone(), &args, |filled| {
        device_rule(field, filled)
    });
    let parsed: Result<DeviceControlParams, _> =
        serde_json::from_value(Value::Object(args.clone()));
    if matches!(expect, Expect::DeserErr) {
        assert!(
            parsed.is_err(),
            "device_control: 字段 {field} 的取值 {effective:?} 类型不符/必填缺失, 反序列化却成功了 \
             (畸形值被静默回落成默认值?)"
        );
        return;
    }
    let params = parsed.expect("device_control: 期望解析成功");
    let runner = DeviceControlRunner::mock(DeviceControlConfig::new(workspace()));
    let action_params = if params.params.is_null() {
        json!({})
    } else {
        params.params.clone()
    };
    // 拍闭合词表: 不在 plan/preview/execute 里 ⇒ 拒绝即帧。
    let refused = if !DEVICE_CONTROL_BEATS.contains(&params.action.as_str()) {
        true
    } else {
        runner
            .plan(&params.device, &params.device_action, &action_params)
            .is_err()
    };
    assert_expectation("device_control", field, &effective, expect, refused);
}

/// `doc_ops` 逐动作字段集合 (禁填面; 取自各动作真正消费的参数,
/// 与 `doc_ops::allowed_fields` 同一口径)。
const DOC_ALLOWED_FIELDS: &[(&str, &[&str])] = &[
    ("pdf_info", &["input"]),
    ("pdf_extract_text", &["input", "max_chars"]),
    ("docx_extract_text", &["input", "max_chars"]),
    ("docx_outline", &["input"]),
    ("pdf_merge", &["inputs", "output"]),
    ("pdf_split", &["input", "output_dir"]),
    ("pdf_extract_pages", &["input", "output", "pages"]),
    ("pdf_watermark_text", &["input", "output", "watermark_text"]),
    ("docx_from_markdown", &["text", "output"]),
    ("docx_append", &["input", "text", "output"]),
];

/// `sheet_ops` 逐动作字段集合 (禁填面; 与 `sheet_ops::allowed_fields` 同一口径)。
const SHEET_ALLOWED_FIELDS: &[(&str, &[&str])] = &[
    ("read", &["offset", "limit"]),
    ("summary", &[]),
    ("filter", &["column", "predicate", "value", "output"]),
    ("sort", &["column", "order", "output"]),
    ("aggregate", &["column", "function"]),
    ("formula", &["expression", "new_column", "output"]),
    ("export_csv", &["output"]),
    ("export_xlsx", &["output"]),
];

/// 某动作的禁填面: 返回该动作**不消费**却在场的字段名。
fn forbidden_fields(action: &str, table: &[(&str, &[&str])], present: &[&str]) -> Vec<String> {
    let Some((_, allowed)) = table.iter().find(|(name, _)| *name == action) else {
        return Vec::new();
    };
    present
        .iter()
        .filter(|field| !allowed.contains(field))
        .map(|field| (*field).to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// 通用对象面 (整个 fuzz 输入是一个 JSON 对象时的类型/未知字段契约)
// ---------------------------------------------------------------------------

/// 类型面通用断言: 只留该工具认识的字段, 然后逐字段核对类型契约。
///
/// 断言: ① 类型不符 / 整数越界 / 枚举外 ⇒ 反序列化**必须失败**;
/// ② 未知字段 ⇒ 反序列化**必须失败** (`deny_unknown_fields`)。
fn generic_type_asserts(object: &Map<String, Value>) {
    for surface in [
        SURFACE_VOICE,
        SURFACE_SKILL,
        SURFACE_DOC,
        SURFACE_SHEET,
        SURFACE_WORKTREE,
        SURFACE_DEVICE,
    ] {
        let specs = fields_of(surface);
        let filtered = filter_known(object, specs);
        for spec in specs {
            let Some(value) = filtered.get(spec.name) else {
                continue;
            };
            if matches!(field_fits(surface, spec.name, value), Some(true)) {
                continue;
            }
            assert!(
                deserialize_fails(surface, &filtered),
                "{}: 字段 {} 取值 {value:?} 类型不符, 反序列化却成功了 (畸形值被静默回落成默认值?)",
                tool_name(surface),
                spec.name
            );
        }
        // 禁填面: 该动作不消费的字段在场 ⇒ 校验面必须拒绝即帧
        // (doc_ops.rs:3492-3494 / sheet_ops.rs:2042-2045 的「必填/**禁填**字段矩阵」)。
        match surface {
            SURFACE_DOC => assert_forbidden_refused(
                surface,
                &filtered,
                DOC_ALLOWED_FIELDS,
                &specs.iter().map(|spec| spec.name).collect::<Vec<_>>(),
                "doc_ops",
            ),
            SURFACE_SHEET => assert_forbidden_refused(
                surface,
                &filtered,
                SHEET_ALLOWED_FIELDS,
                &specs.iter().map(|spec| spec.name).collect::<Vec<_>>(),
                "sheet_ops",
            ),
            _ => {}
        }
        // 未知字段 ⇒ 反序列化必须失败。
        let unknown: Vec<&str> = object
            .keys()
            .map(String::as_str)
            .filter(|key| !specs.iter().any(|spec| spec.name == *key))
            .collect();
        if !unknown.is_empty() {
            assert!(
                deserialize_fails(surface, &filtered),
                "{}: 未知字段 {unknown:?} 被静默接受 (deny_unknown_fields 未生效?)",
                tool_name(surface)
            );
        }
    }
}

/// 工具名 (报错文案用)。
fn tool_name(surface: usize) -> &'static str {
    match surface {
        SURFACE_VOICE => "voice_speak",
        SURFACE_SKILL => "skill_distill",
        SURFACE_DOC => "doc_ops",
        SURFACE_SHEET => "sheet_ops",
        SURFACE_WORKTREE => "worktree_apply",
        _ => "device_control",
    }
}

/// 只保留该工具认识的字段。
fn filter_known(object: &Map<String, Value>, specs: &[FieldSpec]) -> Map<String, Value> {
    object
        .iter()
        .filter(|(key, _)| specs.iter().any(|spec| spec.name == key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// 该工具的参数在给定 JSON 上反序列化必须失败吗。
fn deserialize_fails(surface: usize, filtered: &Map<String, Value>) -> bool {
    let effective = Value::Object(filtered.clone());
    match surface {
        SURFACE_VOICE => serde_json::from_value::<VoiceSpeakParams>(effective).is_err(),
        SURFACE_SKILL => serde_json::from_value::<SkillDistillParams>(effective).is_err(),
        SURFACE_DOC => serde_json::from_value::<DocOpsParams>(effective).is_err(),
        SURFACE_SHEET => serde_json::from_value::<SheetOpsParams>(effective).is_err(),
        SURFACE_WORKTREE => serde_json::from_value::<WorktreeApplyParams>(effective).is_err(),
        _ => serde_json::from_value::<DeviceControlParams>(effective).is_err(),
    }
}

/// 禁填面断言: 动作在闭合词表内、且带了该动作不消费的字段 ⇒ 校验面必须拒绝。
fn assert_forbidden_refused(
    surface: usize,
    filtered: &Map<String, Value>,
    table: &[(&str, &[&str])],
    known: &[&str],
    tool: &str,
) {
    let Some(action) = filtered.get("action").and_then(Value::as_str) else {
        return;
    };
    let present: Vec<&str> = known
        .iter()
        .copied()
        .filter(|name| filtered.contains_key(*name))
        .collect();
    let forbidden = forbidden_fields(action, table, &present);
    if forbidden.is_empty() {
        return;
    }
    let refused = match surface {
        SURFACE_DOC => {
            let params: DocOpsParams = serde_json::from_value(Value::Object(filtered.clone()))
                .expect("doc_ops 参数可解析");
            doc_ops_params_refusal(&doc_config(true), &params).is_some()
        }
        _ => {
            let params: SheetOpsParams = serde_json::from_value(Value::Object(filtered.clone()))
                .expect("sheet_ops 参数可解析");
            sheet_ops_params_refusal(&sheet_config(true), &params).is_some()
        }
    };
    assert!(
        refused,
        "{tool}: 动作 {action:?} 不消费字段 {forbidden:?}, 校验面却放行了 (禁填字段矩阵未生效?)"
    );
}

// ---------------------------------------------------------------------------
// 多字节边界面 (字符数 vs 字节数: 本仓已栽过「多字节让下标错位」)
// ---------------------------------------------------------------------------

/// 边界专用: 取一个贴近边界的长度 (base ± 8)。
fn boundary_delta(payload: &[u8]) -> i64 {
    let raw = payload.first().copied().unwrap_or(0) as i64;
    (raw % 17) - 8
}

/// 多字节边界断言: 上限是**字符**还是**字节**, 按各函数自己的文档判定。
fn check_multibyte_boundary(case: usize, payload: &[u8]) {
    match case % 4 {
        // voice_speak: 「at most 2000 characters」⇒ 计数单位是**字符**。
        0 => {
            let length = VOICE_SPEAK_MAX_TEXT_CHARS as i64 + boundary_delta(payload);
            if length < 0 {
                return;
            }
            let text = "字".repeat(length as usize);
            let params = VoiceSpeakParams {
                text,
                rate: None,
                volume: None,
                voice: None,
                scope: Some(SpeakScope::Care),
            };
            let refused = speak_request_refusal(&params).is_some();
            assert_eq!(
                refused,
                length > VOICE_SPEAK_MAX_TEXT_CHARS as i64,
                "voice_speak: {length} 个多字节字符 ({} 字节) 的拒绝判定与「限 2000 字符」契约不符",
                length * 3
            );
        }
        // skill_distill: 「内容输入的字节上限」⇒ 计数单位是**字节**。
        1 => {
            let length = SKILL_SOURCE_MAX_BYTES as i64 + boundary_delta(payload);
            if length < 0 {
                return;
            }
            let text = "a".repeat(length as usize);
            let params = SkillDistillParams {
                card: serde_json::from_value(valid_card()).expect("合法卡"),
                text: Some(text),
                path: None,
                update: false,
            };
            let refused = source_request_refusal(&params).is_some();
            assert_eq!(
                refused,
                length > SKILL_SOURCE_MAX_BYTES as i64,
                "skill_distill: {length} 字节正文的拒绝判定与「限 {} 字节」契约不符",
                SKILL_SOURCE_MAX_BYTES
            );
        }
        // doc_ops: 「源文 {chars} 字符超过 {DOC_OPS_MAX_SOURCE_CHARS} 字符上限」。
        2 => {
            let length = DOC_OPS_MAX_SOURCE_CHARS as i64 + boundary_delta(payload);
            if length < 0 {
                return;
            }
            let text = "字".repeat(length as usize);
            let params = DocOpsParams {
                action: Some("docx_from_markdown".to_string()),
                input: None,
                inputs: None,
                output: Some("out/boundary.docx".to_string()),
                output_dir: None,
                pages: None,
                watermark_text: None,
                max_chars: None,
                text: Some(text),
            };
            let refused = doc_ops_params_refusal(&doc_config(true), &params).is_some();
            assert_eq!(
                refused,
                length > DOC_OPS_MAX_SOURCE_CHARS as i64,
                "doc_ops: {length} 个多字节字符的源文拒绝判定与「限 {} 字符」契约不符",
                DOC_OPS_MAX_SOURCE_CHARS
            );
        }
        // doc_ops: 段落上限 (非空行 = 一段)。
        _ => {
            let length = DOC_OPS_MAX_SOURCE_PARAGRAPHS as i64 + boundary_delta(payload);
            if length < 0 {
                return;
            }
            let text = "段\n".repeat(length as usize);
            let params = DocOpsParams {
                action: Some("docx_from_markdown".to_string()),
                input: None,
                inputs: None,
                output: Some("out/boundary2.docx".to_string()),
                output_dir: None,
                pages: None,
                watermark_text: None,
                max_chars: None,
                text: Some(text),
            };
            let refused = doc_ops_params_refusal(&doc_config(true), &params).is_some();
            assert_eq!(
                refused,
                length > DOC_OPS_MAX_SOURCE_PARAGRAPHS as i64,
                "doc_ops: {length} 段源文的拒绝判定与「限 {} 段」契约不符",
                DOC_OPS_MAX_SOURCE_PARAGRAPHS
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 交叉一致 + device_control 三拍绑定
// ---------------------------------------------------------------------------

/// `worktree_apply` 的补丁↔改单交叉一致 (worktree_apply.rs:197-249):
/// 补丁动了改单之外的目标 / 改单有补丁未动的目标 / 同目标动作标签不一致 ⇒ 拒绝。
fn check_worktree_binding(payload: &[u8]) {
    // 基线: 改单与补丁逐项相符 ⇒ 必须放行。
    let base = Value::Object(worktree_baseline("patch"));
    assert!(
        prepare_apply(workspace(), &base).is_ok(),
        "worktree_apply: 与改单逐项相符的补丁竟被拒绝"
    );
    // 漂移: 补丁声明了改单之外的目标 ⇒ 拒绝即帧。
    let mut drifted = worktree_baseline("patch");
    drifted.insert(
        "patch".into(),
        json!("*** Begin Patch\n*** Add File: other.txt\n+x\n*** End Patch"),
    );
    assert!(
        prepare_apply(workspace(), &Value::Object(drifted)).is_err(),
        "worktree_apply: 动了改单之外目标的补丁竟被接受 (改单边界被绕过)"
    );
    // 动作标签不一致 ⇒ 拒绝即帧。
    let mut mismatched = worktree_baseline("patch");
    mismatched.insert(
        "patch".into(),
        json!("*** Begin Patch\n*** Update File: new.txt\n@@\n-x\n+y\n*** End Patch"),
    );
    assert!(
        prepare_apply(workspace(), &Value::Object(mismatched)).is_err(),
        "worktree_apply: 同目标动作标签不一致的补丁竟被接受 (交叉一致被绕过)"
    );
    // 改单里重复声明同一目标 ⇒ 拒绝即帧 (worktree_plan.rs:230-231)。
    let mut duplicated = worktree_baseline("plan");
    duplicated.insert(
        "plan".into(),
        json!({
            "goal": "重复目标",
            "files": [
                { "path": "new.txt", "action": "add", "expect": "a" },
                { "path": "new.txt", "action": "add", "expect": "b" }
            ]
        }),
    );
    assert!(
        prepare_apply(workspace(), &Value::Object(duplicated)).is_err(),
        "worktree_apply: 同一目标重复声明的改单竟被接受"
    );
    // 校验命令程序名为空 ⇒ 拒绝即帧 (worktree_apply.rs:251-259)。
    let mut empty_program = worktree_baseline("verify");
    empty_program.insert("verify".into(), json!([{ "program": "  " }]));
    assert!(
        prepare_apply(workspace(), &Value::Object(empty_program)).is_err(),
        "worktree_apply: 程序名为空的校验命令竟被接受"
    );
    // 任意字节 ⇒ 不得 panic (补丁解析面)。
    if let Ok(text) = std::str::from_utf8(payload) {
        let mut arbitrary = worktree_baseline("patch");
        arbitrary.insert("patch".into(), json!(text));
        let _ = prepare_apply(workspace(), &Value::Object(arbitrary));
    }
}

/// `device_control` 三拍绑定 (device_actions target 未覆盖的部分)。
fn check_device_binding(payload: &[u8]) {
    let runner = DeviceControlRunner::mock(DeviceControlConfig::new(workspace()));
    let plan = runner
        .plan("lamp", "power_on", &json!({}))
        .expect("灯 + power_on 是合法计划");
    // 正确 plan_id ⇒ preview 放行。
    assert!(
        runner
            .preview("lamp", "power_on", &json!({}), Some(plan.plan_id.as_str()))
            .is_ok(),
        "device_control: 计划自带的 plan_id 竟被 preview 拒绝"
    );
    // 漂移的 plan_id ⇒ preview 拒绝 (三拍同计划绑定)。
    let drifted = format!("{}x", plan.plan_id);
    assert!(
        runner
            .preview("lamp", "power_on", &json!({}), Some(drifted.as_str()))
            .is_err(),
        "device_control: 漂移的 plan_id 竟被 preview 接受 (计划绑定被绕过)"
    );
    // 参数漂移 ⇒ plan_id 变 ⇒ 旧 plan_id 必须失效。
    let drifted_params = runner
        .plan("lamp", "set_brightness", &json!({ "level": 10 }))
        .expect("合法动作参数");
    assert_ne!(
        drifted_params.plan_id, plan.plan_id,
        "device_control: 不同动作/参数的 plan_id 相同 (计划身份失去区分度)"
    );
    // 未登记设备 ⇒ 拒绝即帧 (白名单面)。
    let unknown_device = payload
        .iter()
        .copied()
        .take(16)
        .map(|byte| char::from(byte))
        .collect::<String>();
    assert!(
        runner
            .plan(&unknown_device, "power_on", &json!({}))
            .is_err(),
        "device_control: 未登记设备 {unknown_device:?} 竟被放行 (白名单被绕过)"
    );
    // 词表外动作 ⇒ 拒绝即帧 (闭合分级词表)。
    assert!(
        runner.plan("lamp", "self_destruct", &json!({})).is_err(),
        "device_control: 词表外动作竟被放行 (闭合词表被绕过)"
    );
}

// ---------------------------------------------------------------------------
// 临时探针: skill_distill 只读通道的符号链接现实层
// ---------------------------------------------------------------------------

/// 探针: 工作区内的符号链接指向工作区外的文件 ⇒ 只读通道必须拒绝。
#[cfg(unix)]
fn probe_symlink_escape() {
    use apeireth_tools_canonical::skill::distill::read_source;
    let root = workspace();
    let outside =
        std::env::temp_dir().join(format!("apeireth-fuzz-outside-{}.txt", std::process::id()));
    let _ = std::fs::write(&outside, b"OUTSIDE-OF-WORKSPACE");
    let link = root.join("fuzz-link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&outside, &link).expect("symlink");
    let params = SkillDistillParams {
        card: serde_json::from_value(valid_card()).expect("合法卡"),
        text: None,
        path: Some("fuzz-link".to_string()),
        update: false,
    };
    let outcome = read_source(root, &params);
    let _ = std::fs::remove_file(&link);
    let _ = std::fs::remove_file(&outside);
    assert!(
        outcome.is_err(),
        "skill_distill: 只读通道通过工作区内的符号链接读到了工作区外的文件: {:?}",
        outcome.map(|source| source.text)
    );
}

// ---------------------------------------------------------------------------
// 主入口
// ---------------------------------------------------------------------------

fuzz_target!(|data: &[u8]| {
    // ── 面 -1: 基线自检 (harness 自己的合法参数集必须被放行) ─────────────────
    check_baselines();
    #[cfg(unix)]
    probe_symlink_escape();

    // ── 面 0: 整个输入是一个 JSON 对象 ⇒ 通用类型/未知字段契约 ─────────────
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        if let Some(object) = value.as_object() {
            generic_type_asserts(object);
        }
    }

    // ── 面 1: 结构化槽位 (每个能力级工具逐字段) ────────────────────────────
    let surface = data.first().copied().unwrap_or(0) as usize % 6;
    let field_index = data.get(1).copied().unwrap_or(0) as usize;
    let shape = data.get(2).copied().unwrap_or(0) as usize;
    let payload = data.get(3..).unwrap_or(&[]);
    let value = slot_value(shape, payload);
    let field = fields_of(surface)[field_index % fields_of(surface).len()].name;

    match surface {
        SURFACE_VOICE => check_voice(field, value),
        SURFACE_SKILL => check_skill(field, value),
        SURFACE_DOC => check_doc(field, value),
        SURFACE_SHEET => check_sheet(field, value),
        SURFACE_WORKTREE => {
            // 改单 / 校验命令是结构化对象, 用枚举式构造器 (结构由 fuzz 字节选)。
            let value = match field {
                "plan" => Some(plan_value(payload)),
                "verify" => Some(verify_value(payload)),
                _ => value,
            };
            check_worktree(field, value);
        }
        _ => check_device(field, value),
    }

    // ── 面 2: 多字节边界 (字符上限 vs 字节上限) ────────────────────────────
    if data.first().copied().unwrap_or(0) % 11 == 10 {
        check_multibyte_boundary(data.get(1).copied().unwrap_or(0) as usize, payload);
    }

    // ── 面 3: 交叉一致 + device_control 三拍绑定 ───────────────────────────
    if data.first().copied().unwrap_or(0) % 13 == 12 {
        check_worktree_binding(payload);
        check_device_binding(payload);
    }
});
