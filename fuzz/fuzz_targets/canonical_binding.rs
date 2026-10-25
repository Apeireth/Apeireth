#![no_main]
//! fuzz: **决定性绑定 / 规范化 JSON 面** —— `canonical_json_bytes` + `ActionBinding`
//! + 协议规范化面（`ToolCall` / `ToolOutcome` / `ToolResult` / `NormalizedTool`）。
//!
//! ## 这个面为什么最值得 fuzz
//!
//! 审批与执行之间靠**规范化摘要**对齐：批准时算一次摘要，随审批记录落库；派发前对
//! **将执行的活动作**重算摘要并比对，不一致即拒绝（`approval.binding_mismatch`）。
//! ⇒ **规范化不一致 ⇒ 审批看到的参数 ≠ 执行的参数** ⇒「审批的是 A，执行的是 B」——
//! 这是审批体系最致命的失效形态，且波及**每一个**工具。
//!
//! ## 被 fuzz 的 API
//!
//! * `apeireth_governance::action_binding::canonical_json_bytes(&Value) -> Vec<u8>`
//! * `apeireth_governance::action_binding::ActionBinding`
//!   （`new` / `with_derived_target` / `canonical_value` / `canonical_bytes` /
//!   `digest` / `short_digest` / `derive_target`）
//! * `apeireth_governance::action_binding::verify_action_binding`
//! * `apeireth_governance::action_binding::classify_binding_drift`
//! * `apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult, NormalizedTool}`
//! * `apeireth_protocol::adapters::{OpenAiChatAdapter, OpenAiResponsesAdapter}` 的
//!   **tool call 参数归一**（字符串 → `Value`）——参数进执行面之前的最后一次改写。
//!
//! ## 契约来源（断言逐条指回下表 —— **不自行发明期望值**）
//!
//! | # | 契约 | 出处（文件:行 + 原文片段） |
//! |---|---|---|
//! | C1 | 对象键按 **UTF-8 字节序**升序；数组**元素顺序原样保留**；字符串最小转义；数字取解析值的确定性十进制文本；**任何位置零空白** | `crates/foundation/governance/src/action_binding.rs:22-29`「\| 对象 \| 键按 **UTF-8 字节序**升序；无空白；键按 JSON 字符串规则转义 \| \| 数组 \| **元素顺序原样保留**（顺序是语义的一部分，改顺序 = 改动作） \| \| 字符串 \| 最小转义：`\"` `\\` `\b` `\t` `\n` `\f` `\r`，其余 C0 控制符 `\u00xx`（小写 hex）；非 ASCII 原样 UTF-8 \| \| 数字 \| 按解析值的确定性十进制文本（整数与浮点表示视为不同值，`1` ≠ `1.0`，确定性优先） \| \| 字面量 \| `true` / `false` / `null` \| \| 空白 \| 任何位置零空白 \|」 |
//! | C2 | **同一语义值恒得同一字节串（键顺序无关）；不同语义值（含数组换序）必然不同** | `action_binding.rs:63-64`「对象键按 UTF-8 字节序排序、数组保序、字符串最小转义、数字取解析值的确定性十进制文本、零空白。**同一语义值恒得同一字节串（键顺序无关），不同语义值（含数组换序）必然不同。**」 |
//! | C3 | 摘要 = 规范化字节的 SHA-256（小写 hex）；短摘要 = hex 前 12 位 | `action_binding.rs:31-32`「摘要 = 规范化字节的 SHA-256（小写 hex）；短摘要 = hex 前 12 位」+ `action_binding.rs:210-214` |
//! | C4 | **执行边界重算比对**：批准动作与活动作的摘要一致 → `Ok`；不一致 → 拒绝即帧的因（带漂移归因） | `action_binding.rs:314-330`「执行边界重算比对：批准动作与活动作的摘要一致 → `Ok`；不一致 → 拒绝即帧的因 [`BindingMismatch`]`（带漂移归因）。」 |
//! | C5 | 漂移归因（确定性）：**按字段序找第一个不同面**；参数面值不同但数组多重集相同 → `ArgumentOrder`，否则 `Parameters` | `action_binding.rs:276-277`「漂移归因（确定性）：按字段序找第一个不同面；参数面值不同但数组多重集相同 → [`BindingDrift::ArgumentOrder`]，否则 [`BindingDrift::Parameters`]。」+ 字段序 = `action_binding.rs:279-293` 的 if 链（operation → agent → target → tool_schema_version → parameters） |
//! | C6 | 漂移词表冻结（六面） | `action_binding.rs:236-244`「全部漂移面（词表冻结面；新增漂移面必须显式入表并补测试）。」+ `action_binding.rs:231-232`「[`BindingDrift::ArgumentOrder`]：结构化 argv 的**顺序**变了（值集合相同、顺序不同）。」 |
//! | C7 | 目标派生规则（确定性）：参数里的 `command` **字符串** → 命令文本；否则 `path` **字符串** → 路径；都没有 → 空串 | `action_binding.rs:164-166`「目标派生规则（确定性）：参数里的 `command` 字符串 → 命令文本；否则 `path` 字符串 → 路径；都没有 → 空串（非命令/非路径面）。」 |
//! | C8 | 摘要随记录落库（跨重启持久）⇒ **序列化 round-trip 后摘要不变** | `action_binding.rs:582-585`「// 序列化 round-trip 后摘要不变（摘要随记录落库、跨重启重算比对的前提）。」 |
//! | C9 | 渲染面：JSON 字符串**解包**而非重新加引号（工具返回 `"42"` 应到模型处成 `42`） | `crates/foundation/protocol/src/canonical/tool_result.rs:113-125`「/// Render the outcome as the text a model will read. /// /// JSON strings are unwrapped rather than re-quoted: a tool returning /// `"42"` should reach the model as `42`, not as `"42"`.」 |
//! | C10 | 结果必须**与调用保持关联**（`tool_call_id` 不丢） | `tool_result.rs:127-131`「/// Convert into the [`MessageRole::Tool`] message appended to the transcript.」+ `tool_result.rs:164-173` 测试「conversion_to_a_message_preserves_the_correlation_id」 |
//! | C11 | 错误消息必须到达模型（不得被吞掉）；可重试性二值 | `tool_result.rs:24-31`「/// Human-readable failure description. Goes back to the model, so /// it should say what went wrong, not merely that something did. /// /// Whether an identical retry could plausibly succeed. Transport /// timeouts are retryable; a malformed argument is not.」 |
//! | C12 | **畸形 arguments 不得静默丢参数**：arguments 置 `Null`，原文记入 `raw_metadata.malformed_tool_arguments` 供排障 | `crates/foundation/protocol/src/adapters/openai_chat.rs:225-227`「// L4 修复 (2026-09-24 审计): 修复前把畸形 arguments JSON 静默吞为 /// `{}` —— 模型输出半个 JSON 时调用方无从得知参数已丢失。现在 arguments /// 置 Null, 原文记入 raw_metadata.malformed_tool_arguments 供排障。」+ 2026-10-25 fuzz 面复核后写入口径表（两个 OpenAI 形状适配器同表，见两文件注释） |
//! | C13 | `arguments` 的**规范形态 = JSON 字符串**（两个 OpenAI 形状协议一致）；非字符串形态 ⇒ fail-loud（`Null` + 排障面），**不得**静默吞成空对象、**不得**透传非 object 形态 | `crates/foundation/protocol/src/adapters/openai_responses.rs:266`「// OpenAI Responses 的 arguments 是 JSON string, 跟 OpenAI Chat 一致」+ `crates/foundation/protocol/src/normalized.rs:340`「/// 参数 (JSON object 风格, 内部统一 object, 序列化时按协议决定 string vs object)」 |
//!
//! ## harness 侧的诚实边界（**不自行发明期望值**）
//!
//! * **数字形态**：`1` / `1.0` / `1e0` 的字节口径由 C1 明写「整数与浮点表示视为不同值」
//!   ⇒ **不断言**「`1` 与 `1.0` 必须同字节」，也**不断言**「必须不同字节」（避免自造期望）。
//!   只断言 C1 承诺的那一半：数字文本必须是**解析值的确定性十进制文本**（回读同一数值）。
//! * **`-0.0` vs `0.0`**：serde_json 的 `Value` 相等性把两者判为相等，而确定性文本不同
//!   （C1 的「确定性优先」）⇒ 不断言「相等值必同字节」。
//! * **Unicode 代理对 / 孤立代理 / BOM**：非法 UTF-8 与孤立代理在 JSON 层就是非法的
//!   ⇒ 解析失败即**放弃断言**（不是失败）。
//! * **C11 的 `contains` 判据**：只用于「错误消息必须原样到达模型」这一条，且消息体是
//!   **本次运行独有的标记串**（渲染结果里只可能出现一次）⇒ 不会在别的上下文里误命中。
//!
//! ## 已知**不**断言的边界
//!
//! * **serde_json 的 f64 解析精度（上游限制，非本仓库缺陷面）**：serde_json 不保证
//!   f64 解析**正确舍入** —— 实测（2026-10-25，serde_json 1.0.x）15–25 位有效数字的
//!   十进制文本里约 **13.8%**（2484/18006 随机样本）被解析成与正确舍入**相差 1 ULP**
//!   的 f64 ⇒ 同一个数值写成「纯数字」与「指数」两种形态会被解析成**两个不同的 f64**
//!   （例：`222222222222222222222226` → `2.2222222222222222e23`，而
//!   `2.2222222222222222e23` → `2.2222222222222225e23`）。
//!   ⇒ 由它引起的**往返漂移**（规范化幂等性 / 摘要 round-trip 稳定性 / `ToolCall`
//!   serde 往返）在该情形下**不断言**（判据见 [`has_inexact_serde_float`]：
//!   同一十进制文本交给 std `f64::from_str`（正确舍入）与 serde_json，两者不等即命中）。
//!   **本仓库的规范化函数本身仍是确定性的**：同一 `Value` ⇒ 同一字节串（恒成立）。
//!   影响方向是 **fail-closed**（漂移 ⇒ 摘要不符 ⇒ 拒绝即帧），**不是审批绕过**。
//! * `Value::Number` 的 serde_json 内部表示（`PosInt` / `NegInt` / `Float`）——文档只承诺
//!   「解析值的确定性十进制文本」，故模型侧直接用同一表示，不另造期望。
//! * Gemini 侧 `ToolCall.id` 的具体形状（`gemini_call_<name>` vs 名字本身）：文档
//!   `normalized.rs:336`「Gemini 无, 用 name 替代」措辞两可 ⇒ **不断言**。

use std::str::FromStr;

use apeireth_governance::action_binding::{
    ActionBinding, BindingDrift, SHORT_DIGEST_CHARS, canonical_json_bytes,
    classify_binding_drift, verify_action_binding,
};
use apeireth_protocol::ProtocolAdapter;
use apeireth_protocol::adapters::{OpenAiChatAdapter, OpenAiResponsesAdapter};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolOutcome, ToolResult};
use apeireth_protocol::normalized::{ContentPart, MessageRole};
use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value, json};

// ─────────────────────────────────────────────────────────────────────────────
// 固定面（fuzz 只扰动参数面，不扰动身份面）
// ─────────────────────────────────────────────────────────────────────────────

/// 操作名（绑定五面之一）。
const OP: &str = "tool.shell";
/// 执行主体（绑定五面之一）。
const AGENT: &str = "sess-fuzz";
/// 动作目标（绑定五面之一；目标派生另有专项检查）。
const TARGET: &str = "target-fuzz";
/// 工具面版本（绑定五面之一）。
const VERSION: &str = "tool-schema-fuzz";

// ─────────────────────────────────────────────────────────────────────────────
// 文档口径模型（C1 / C2 的独立重写 —— 与实现对照 = 规范化面自洽检查）
// ─────────────────────────────────────────────────────────────────────────────

/// 文档口径的 JSON 字符串最小转义（C1「字符串」行）。
///
/// 出处：`action_binding.rs:26`「最小转义：`\"` `\\` `\b` `\t` `\n` `\f` `\r`，
/// 其余 C0 控制符 `\u00xx`（小写 hex）；非 ASCII 原样 UTF-8」。
fn model_json_string(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// 文档口径的规范化序列化（C1 全表 + C2）。
///
/// 与 `canonical_json_bytes` **独立重写**：结构（键排序 / 数组保序 / 结构分隔）由本
/// 模型给出；数字沿用 serde_json 的解析值文本（C1 明写「按解析值的确定性十进制文本」，
/// 数字文本本身由 serde_json 决定，文档未另立口径）。
fn model_canonical(value: &Value) -> Vec<u8> {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            let mut out = Vec::with_capacity(64);
            out.push(b'{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                model_json_string(&mut out, key);
                out.push(b':');
                out.extend_from_slice(&model_canonical(item));
            }
            out.push(b'}');
            out
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(64);
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                out.extend_from_slice(&model_canonical(item));
            }
            out.push(b']');
            out
        }
        Value::String(s) => {
            let mut out = Vec::with_capacity(s.len() + 2);
            model_json_string(&mut out, s);
            out
        }
        Value::Null => b"null".to_vec(),
        Value::Bool(flag) => if *flag { "true" } else { "false" }.as_bytes().to_vec(),
        Value::Number(number) => number.to_string().into_bytes(),
    }
}

/// 单个数字：serde_json 对它的十进制文本解析**是否不精确**（上游精度限制）。
///
/// 判据（独立于本 target 的断言）：同一个十进制文本分别交给
/// * `f64::from_str`（std，**正确舍入**）—— 必然回到原值（最短表示的自反性）；
/// * `serde_json` 解析；
///
/// 两者不等 ⇒ serde_json 在这个文本上**不精确** ⇒ 由它引起的往返漂移归上游，不归本仓库。
///
/// 实测（2026-10-25，serde_json 1.0.x）：15–25 位有效数字的十进制文本里，约 **13.8%**
/// （2484/18006 随机样本）被 serde_json 解析成与正确舍入**相差 1 ULP** 的 f64 ⇒ 同一个
/// 数值写成「纯数字」与「指数」两种形态会被解析成**两个不同的 f64**。
fn serde_number_parse_is_inexact(number: &serde_json::Number) -> bool {
    let Some(float) = number.as_f64() else {
        return false;
    };
    let text = number.to_string();
    match f64::from_str(&text) {
        // std 正确舍入 ⇒ 回到原值（最短表示的自反性）；不等说明文本不可信。
        Ok(std_parsed) if std_parsed == float => {
            serde_json::from_str::<f64>(&text).ok().is_some_and(|parsed| parsed != float)
        }
        _ => true,
    }
}

/// 该值里是否含「serde_json 解析不精确」的 f64（**上游**精度限制，不是本仓库的缺陷面）。
fn has_inexact_serde_float(value: &Value) -> bool {
    match value {
        Value::Number(number) => serde_number_parse_is_inexact(number),
        Value::Array(items) => items.iter().any(has_inexact_serde_float),
        Value::Object(map) => map.values().any(has_inexact_serde_float),
        _ => false,
    }
}

/// 规范化输出里**字符串之外**不得有空白（C1「空白 | 任何位置零空白」）。
fn no_whitespace_outside_strings(bytes: &[u8]) -> bool {
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        if byte == b'"' {
            in_string = true;
            continue;
        }
        if byte.is_ascii_whitespace() {
            return false;
        }
    }
    true
}

/// 数字文本必须回读成**同一个数值**（C1「按解析值的确定性十进制文本」）。
///
/// 这是**可判定的后置条件**（不是字符串存在性判据）：规范化出的数字文本重新解析后
/// 必须与原值逐值相等 ⇒ 规范化没有改变数值语义。
///
/// ⚠️ 上游精度豁免：serde_json 的 f64 解析**不保证正确舍入**（见
/// [`serde_number_parse_is_inexact`]）⇒ 该情形下回读会漂 1 ULP（**上游**限制，非本
/// 仓库缺陷）⇒ 不断言；其余数字照常断言。
fn check_number_roundtrip(value: &Value) {
    match value {
        Value::Number(number) => {
            if serde_number_parse_is_inexact(number) {
                return;
            }
            let text = number.to_string();
            let reparsed: Value = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("数字文本回读失败: {text:?} ({error})"));
            assert_eq!(
                reparsed, *value,
                "数字文本回读后数值变了: {text:?}（C1: 解析值的确定性十进制文本）"
            );
        }
        Value::Array(items) => {
            for item in items {
                check_number_roundtrip(item);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                check_number_roundtrip(item);
            }
        }
        _ => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 面 1: 规范化字节面（`canonical_json_bytes`）
// ─────────────────────────────────────────────────────────────────────────────

/// 规范化字节面检查（C1 / C2）。
fn check_canonical_bytes(value: &Value) {
    let bytes = canonical_json_bytes(value);

    // C1：与文档口径模型**逐字节**一致（键序 / 数组保序 / 最小转义 / 零空白）。
    assert_eq!(
        bytes,
        model_canonical(value),
        "规范化字节与文档口径不一致（C1/C2）: value={value}"
    );

    // C1「空白 | 任何位置零空白」：字符串之外不得有空白。
    assert!(
        no_whitespace_outside_strings(&bytes),
        "规范化输出含空白（C1 零空白）: {}",
        String::from_utf8_lossy(&bytes)
    );

    // C2「同一语义值恒得同一字节串」：规范化输出必须是合法 JSON，且**再规范化幂等**
    // （可判定后置条件：规范化后再规范化 ⇒ 同一字节）。
    //
    // ⚠️ 上游精度豁免：serde_json 的 f64 解析**不保证正确舍入**（见
    // [`has_inexact_serde_float`]）⇒ 含这类 f64 的值，回读会漂 1 ULP，幂等性由上游
    // 破掉（**不是本仓库的规范化函数的缺陷**）⇒ 该情形不断言幂等。
    if !has_inexact_serde_float(value) {
        let reparsed: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("规范化输出不是合法 JSON ({error}): {bytes:?}"));
        assert_eq!(
            canonical_json_bytes(&reparsed),
            bytes,
            "规范化不幂等（C2 同一语义值恒得同一字节串）: value={value}"
        );
    } else {
        // 豁免情形仍须保证：**输出是合法 JSON**（可判定后置条件）。
        serde_json::from_slice::<Value>(&bytes)
            .unwrap_or_else(|error| panic!("规范化输出不是合法 JSON ({error}): {bytes:?}"));
    }

    // C1「数字 | 按解析值的确定性十进制文本」：数字文本回读同一数值。
    check_number_roundtrip(value);

    // C2：同一输入两次规范化必须逐字节相同（确定性）。
    assert_eq!(
        canonical_json_bytes(value),
        bytes,
        "同一输入两次规范化不一致（确定性）: value={value}"
    );
}

/// 面 1 入口：把任意字节变成一个 JSON 值再规范化。
fn canonical_bytes_face(mode: u8, payload: &[u8]) {
    let value = match mode % 4 {
        0 => match serde_json::from_slice::<Value>(payload) {
            Ok(value) => value,
            Err(_) => return,
        },
        // 任意字节 ⇒ 合法 str（含畸形 UTF-8 ⇒ U+FFFD）：打字符串转义面。
        1 => Value::String(String::from_utf8_lossy(payload).into_owned()),
        2 => match serde_json::from_slice::<Value>(payload) {
            Ok(value) => value,
            Err(_) => Value::String(String::from_utf8_lossy(payload).into_owned()),
        },
        // 字节数组：打数组面 + 非 ASCII 元素面。
        _ => Value::Array(payload.iter().map(|byte| Value::from(*byte)).collect()),
    };
    check_canonical_bytes(&value);
}

// ─────────────────────────────────────────────────────────────────────────────
// 面 2: 绑定面（`ActionBinding` / `verify_action_binding`）
// ─────────────────────────────────────────────────────────────────────────────

/// 找第一个**字符串叶子**并改掉它的内容（内容必变 ⇒ 规范化字节必变）。
fn mutate_first_string(value: &Value) -> Option<Value> {
    match value {
        Value::String(text) => Some(Value::String(format!("{text}\u{1}"))),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            let mut changed = false;
            for item in items {
                if !changed {
                    if let Some(mutated) = mutate_first_string(item) {
                        out.push(mutated);
                        changed = true;
                        continue;
                    }
                }
                out.push(item.clone());
            }
            changed.then_some(Value::Array(out))
        }
        Value::Object(map) => {
            let mut out = Map::new();
            let mut changed = false;
            for (key, item) in map {
                if !changed {
                    if let Some(mutated) = mutate_first_string(item) {
                        out.insert(key.clone(), mutated);
                        changed = true;
                        continue;
                    }
                }
                out.insert(key.clone(), item.clone());
            }
            changed.then_some(Value::Object(out))
        }
        _ => None,
    }
}

/// 盲区突变矩阵：把可执行字段**藏进**规范化的各个角落（C4「绑定不可绕过」）。
///
/// 覆盖：多余的键 / 嵌套里藏一个可执行字段 / 数组里塞对象 / 字符串里嵌 JSON /
/// 字符串叶子内容变化 / 值类型变化（对象 → 数组）。
fn blind_spot_mutations(parameters: &Value) -> Vec<(&'static str, Value)> {
    let mut out: Vec<(&'static str, Value)> = Vec::new();

    // ① 多余的顶层键。
    let mut with_extra = parameters.clone();
    match &mut with_extra {
        Value::Object(map) => {
            map.insert("zz_unused".to_string(), Value::Null);
        }
        other => {
            *other = json!({ "zz_unused": Value::Null, "payload": other.clone() });
        }
    }
    out.push(("多余的顶层键", with_extra));

    // ② 嵌套里藏一个可执行字段。
    let mut nested = parameters.clone();
    match &mut nested {
        Value::Object(map) => {
            map.insert(
                "zz_nested".to_string(),
                json!({ "command": "rm -rf /", "hidden": { "argv": ["sh"] } }),
            );
        }
        other => {
            *other = json!({
                "zz_nested": { "command": "rm -rf /" },
                "payload": other.clone(),
            });
        }
    }
    out.push(("嵌套藏可执行字段", nested));

    // ③ 字符串里嵌 JSON（整个载荷被塞进一个字符串）。
    let embedded = Value::String(serde_json::to_string(parameters).unwrap_or_default());
    out.push(("字符串里嵌 JSON", json!({ "payload": embedded })));

    // ④ 数组里塞对象。
    out.push((
        "数组里塞对象",
        json!({ "payload": [parameters.clone()] }),
    ));

    // ⑤ 字符串叶子内容变化。
    if let Some(mutated) = mutate_first_string(parameters) {
        out.push(("字符串叶子变化", mutated));
    }

    // ⑥ 值类型变化（对象 → 数组）。
    out.push((
        "值类型变化",
        json!({ "payload": [parameters.clone()] }),
    ));

    out
}

/// 目标派生口径检查（C7）。
fn check_derive_target(parameters: &Value) {
    let derived = ActionBinding::with_derived_target(OP, AGENT, VERSION, parameters.clone());
    let expected = if let Some(command) = parameters.get("command").and_then(Value::as_str) {
        command.to_string()
    } else if let Some(path) = parameters.get("path").and_then(Value::as_str) {
        path.to_string()
    } else {
        String::new()
    };
    assert_eq!(
        derived.target, expected,
        "目标派生口径不一致（C7）: parameters={parameters}"
    );
}

/// 面 2 入口：绑定摘要 + 执行边界比对。
fn binding_face(mode: u8, payload: &[u8]) {
    let value = match serde_json::from_slice::<Value>(payload) {
        Ok(value) => value,
        Err(_) => Value::String(String::from_utf8_lossy(payload).into_owned()),
    };
    let parameters = match mode % 3 {
        0 => value.clone(),
        1 => json!({ "arguments": value.clone() }),
        _ => json!({ "argv": [value.clone()], "cwd": "/tmp" }),
    };

    let binding = ActionBinding::new(OP, AGENT, TARGET, VERSION, parameters.clone());

    // C3：摘要确定；短摘要 = hex 前 12 位。
    let digest = binding.digest();
    assert_eq!(binding.digest(), digest, "摘要不确定");
    assert_eq!(binding.short_digest().len(), SHORT_DIGEST_CHARS);
    assert_eq!(binding.short_digest(), digest[..SHORT_DIGEST_CHARS]);

    // 绑定的规范化字节 = 绑定值的规范化字节（C1 在绑定面上的落点）。
    assert_eq!(
        binding.canonical_bytes(),
        canonical_json_bytes(&binding.canonical_value()),
        "绑定的规范化字节口径不一致"
    );

    // C8：序列化 round-trip 后摘要不变（摘要随记录落库、跨重启重算比对的前提）。
    //
    // ⚠️ 上游精度豁免（同 [`has_inexact_serde_float`]）：serde_json 的 f64 解析不保证
    // 正确舍入 ⇒ 含这类 f64 的载荷，回读会漂 1 ULP ⇒ 摘要漂移**归上游**，不断言。
    if !has_inexact_serde_float(&parameters) {
        let encoded = serde_json::to_string(&binding).expect("绑定可序列化");
        let restored: ActionBinding = serde_json::from_str(&encoded)
            .unwrap_or_else(|error| panic!("绑定可反序列化: {error}"));
        assert_eq!(
            restored.digest(),
            digest,
            "序列化 round-trip 后摘要变了（C8）: encoded={encoded}"
        );

        // C4：同摘要必须通过。
        assert!(
            verify_action_binding(&binding, &restored).is_ok(),
            "同摘要竟被拒（C4）"
        );
    }

    // C2 / C4：**任何**载荷改动都必须改摘要并被拒（绑定不可绕过）。
    for (label, mutated) in blind_spot_mutations(&parameters) {
        if mutated == parameters {
            // 值相等（改写没真改）⇒ 不构成「不同语义值」，跳过（避免自造期望）。
            continue;
        }
        let other = ActionBinding::new(OP, AGENT, TARGET, VERSION, mutated);
        assert_ne!(
            other.digest(),
            digest,
            "载荷改动后摘要未变（C2/C4 绑定不可绕过）: {label} parameters={parameters}"
        );
        let mismatch = verify_action_binding(&binding, &other)
            .expect_err("载荷改动后必须拒绝即帧（C4）");
        assert_eq!(mismatch.expected_digest, digest, "拒绝帧的批准摘要不对");
        assert_eq!(mismatch.actual_digest, other.digest(), "拒绝帧的活摘要不对");
    }

    // C7：目标派生口径。
    check_derive_target(&parameters);
}

// ─────────────────────────────────────────────────────────────────────────────
// 面 3: 漂移归因面（`classify_binding_drift`）
// ─────────────────────────────────────────────────────────────────────────────

/// 面 3 入口：漂移归因（字段序 + 顺序面 / 值面）。
fn drift_face(mode: u8, payload: &[u8]) {
    let base = String::from_utf8_lossy(payload).into_owned();
    let second = format!("{base}\u{1}");
    let third = format!("{base}\u{2}");

    let approved = ActionBinding::new(
        OP,
        AGENT,
        TARGET,
        VERSION,
        json!({ "argv": [base.clone(), second.clone()] }),
    );

    // ① 值集合相同、顺序不同 ⇒ `ArgumentOrder`（C5 / C6）。
    let reordered = ActionBinding::new(
        OP,
        AGENT,
        TARGET,
        VERSION,
        json!({ "argv": [second.clone(), base.clone()] }),
    );
    assert_ne!(
        approved.digest(),
        reordered.digest(),
        "argv 顺序是动作语义的一部分（C2）"
    );
    let mismatch = verify_action_binding(&approved, &reordered).expect_err("顺序变了必须拒");
    assert_eq!(
        mismatch.drift,
        BindingDrift::ArgumentOrder,
        "顺序漂移归因不对（C5/C6）"
    );

    // ② 值变了（数组多重集不同）⇒ `Parameters`（C5）。
    let changed = ActionBinding::new(
        OP,
        AGENT,
        TARGET,
        VERSION,
        json!({ "argv": [base.clone(), third.clone()] }),
    );
    let mismatch = verify_action_binding(&approved, &changed).expect_err("值变了必须拒");
    assert_eq!(
        mismatch.drift,
        BindingDrift::Parameters,
        "值漂移归因不对（C5）"
    );

    // ③ 字段序：operation → agent → target → tool_schema_version（C5「按字段序找第一个
    // 不同面」）。每个变体都**同时**改了参数面，确保归因不是被参数面抢走。
    let mut other_operation = changed.clone();
    other_operation.operation = "tool.other".to_string();
    assert_eq!(
        classify_binding_drift(&approved, &other_operation),
        BindingDrift::Operation,
        "operation 漂移归因不对（C5 字段序）"
    );

    let mut other_agent = changed.clone();
    other_agent.agent = "sess-other".to_string();
    assert_eq!(
        classify_binding_drift(&approved, &other_agent),
        BindingDrift::Agent,
        "agent 漂移归因不对（C5 字段序）"
    );

    let mut other_target = changed.clone();
    other_target.target = "target-other".to_string();
    assert_eq!(
        classify_binding_drift(&approved, &other_target),
        BindingDrift::Target,
        "target 漂移归因不对（C5 字段序）"
    );

    let mut other_version = changed.clone();
    other_version.tool_schema_version = "tool-schema-other".to_string();
    assert_eq!(
        classify_binding_drift(&approved, &other_version),
        BindingDrift::ToolSchemaVersion,
        "tool_schema_version 漂移归因不对（C5 字段序）"
    );

    // C6：漂移词表冻结（六面齐全）。
    assert_eq!(
        BindingDrift::ALL.len(),
        6,
        "漂移词表漂移（C6 冻结面）: {:?}",
        BindingDrift::ALL
    );

    // 模式位：留一面给「绑定五面全同、只改参数」的最简形状。
    if mode % 2 == 1 {
        let same = ActionBinding::new(
            OP,
            AGENT,
            TARGET,
            VERSION,
            json!({ "argv": [base.clone()] }),
        );
        assert!(
            verify_action_binding(&approved, &same).is_err(),
            "参数面变了竟通过（C4）"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 面 4: 协议规范化面（`ToolCall` / `ToolOutcome` / `ToolResult` / `NormalizedTool`）
// ─────────────────────────────────────────────────────────────────────────────

/// 面 4 入口：协议规范化面契约。
fn protocol_type_face(mode: u8, payload: &[u8]) {
    let value = match serde_json::from_slice::<Value>(payload) {
        Ok(value) => value,
        Err(_) => Value::String(String::from_utf8_lossy(payload).into_owned()),
    };

    // `ToolCall` 的 serde 往返保真（参数面 = `arguments` 原样进 `Value`）。
    //
    // ⚠️ 上游精度豁免（同 [`has_inexact_serde_float`]）：serde_json 的 f64 解析不保证
    // 正确舍入 ⇒ 含这类 f64 的参数回读会漂 1 ULP（**上游**限制，非本仓库缺陷）⇒ 不断言。
    let call = ToolCall {
        id: "call_fuzz".to_string(),
        name: "tool_fuzz".to_string(),
        arguments: value.clone(),
    };
    let encoded = serde_json::to_string(&call).expect("ToolCall 可序列化");
    let restored: ToolCall =
        serde_json::from_str(&encoded).unwrap_or_else(|error| panic!("ToolCall 可反序列化: {error}"));
    if !has_inexact_serde_float(&value) {
        assert_eq!(restored, call, "ToolCall 序列化往返失真");
    } else {
        // 豁免情形仍须保证：往返**不得改变参数的类型面**（数值 / 字符串 / 对象 / 数组）。
        assert_eq!(
            std::mem::discriminant(&restored.arguments),
            std::mem::discriminant(&call.arguments),
            "ToolCall 往返改变了参数的类型面"
        );
    }

    // C9：渲染面（字符串解包 / 其余按值文本）。
    let result = ToolResult::ok("call_fuzz", value.clone());
    let expected = match &value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    assert_eq!(result.render(), expected, "渲染口径不一致（C9）");
    assert!(result.is_ok(), "Ok 结果不得判成失败（C11）");
    assert!(
        !result.outcome.is_retryable(),
        "Ok 结果不得判成可重试（C11）"
    );

    // C10：结果必须与调用保持关联（`tool_call_id` / `name` 不丢）。
    let message = result.clone().with_name("tool_fuzz").into_message();
    assert_eq!(message.role, MessageRole::Tool, "结果消息角色不对（C10）");
    assert_eq!(
        message.tool_call_id.as_deref(),
        Some("call_fuzz"),
        "结果消息丢了调用关联 id（C10）"
    );
    assert_eq!(
        message.name.as_deref(),
        Some("tool_fuzz"),
        "结果消息丢了工具名（C10）"
    );
    assert!(message.tool_calls.is_empty(), "结果消息不得携带调用（C10）");
    match message.content.as_slice() {
        [ContentPart::Text { text }] => {
            assert_eq!(text, &expected, "结果消息正文与渲染不一致（C9/C10）");
        }
        other => panic!("结果消息正文形状不对: {other:?}"),
    }

    // C11：错误消息必须原样到达模型（标记串本次运行独有 ⇒ 判据可判定）。
    let marker = format!("\u{2}fuzz-marker-{}\u{2}", payload.len());
    let permanent = ToolResult::permanent_error("call_fuzz", marker.clone());
    assert!(!permanent.is_ok(), "失败结果不得判成成功（C11）");
    assert!(
        !permanent.outcome.is_retryable(),
        "永久失败不得判成可重试（C11）"
    );
    assert!(
        permanent.render().contains(&marker),
        "错误消息被吞掉（C11）: rendered={:?}",
        permanent.render()
    );
    let retryable = ToolResult::retryable_error("call_fuzz", marker.clone());
    assert!(
        retryable.outcome.is_retryable(),
        "可重试失败未判成可重试（C11）"
    );
    assert!(
        retryable.render().contains(&marker),
        "可重试失败的消息被吞掉（C11）"
    );

    // `ToolOutcome` 的 serde 往返（两种形状都是 `status` 标签）。
    //
    // ⚠️ 上游精度豁免（同 [`has_inexact_serde_float`]）：serde_json 的 f64 解析不保证
    // 正确舍入 ⇒ 含这类 f64 的载荷回读会漂 1 ULP（**上游**限制，非本仓库缺陷）⇒ 不断言。
    let outcome = ToolOutcome::Ok {
        value: value.clone(),
    };
    let encoded = serde_json::to_string(&outcome).expect("ToolOutcome 可序列化");
    let restored: ToolOutcome = serde_json::from_str(&encoded)
        .unwrap_or_else(|error| panic!("ToolOutcome 可反序列化: {error}"));
    if !has_inexact_serde_float(&value) {
        assert_eq!(restored, outcome, "ToolOutcome 序列化往返失真");
    }

    // `NormalizedTool` 的 serde 形状：空参数集 / `strict=false` / 无描述**不落键**。
    let tool = NormalizedTool::new("tool_fuzz");
    let encoded = serde_json::to_value(&tool).expect("NormalizedTool 可序列化");
    assert_eq!(encoded["name"], "tool_fuzz");
    assert!(
        encoded.get("parameters").is_none(),
        "空参数集不得落键: {encoded}"
    );
    assert!(
        encoded.get("strict").is_none(),
        "strict=false 不得落键: {encoded}"
    );
    assert!(
        encoded.get("description").is_none(),
        "无描述不得落键: {encoded}"
    );

    // 模式位：留一面给「带参数的工具」形状（参数集必须落键且保真）。
    if mode % 2 == 1 {
        let mut parameters = Map::new();
        parameters.insert("path".to_string(), value.clone());
        let tool = NormalizedTool::new("tool_fuzz").with_parameters(parameters);
        let encoded = serde_json::to_value(&tool).expect("NormalizedTool 可序列化");
        assert_eq!(encoded["parameters"]["path"], value, "参数集往返失真");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 面 5: 协议参数归一面（tool call `arguments` 字符串 → `Value`）
// ─────────────────────────────────────────────────────────────────────────────

/// OpenAI Chat 形状的原始响应（把 fuzz 值放进 `function.arguments`）。
fn openai_chat_raw(arguments: &Value) -> Value {
    json!({
        "id": "chatcmpl-fuzz",
        "model": "fuzz-model",
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_fuzz",
                    "type": "function",
                    "function": { "name": "tool_fuzz", "arguments": arguments.clone() }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    })
}

/// OpenAI Responses 形状的原始响应。
fn openai_responses_raw(arguments: &Value) -> Value {
    json!({
        "id": "resp_fuzz",
        "model": "fuzz-model",
        "output": [{
            "type": "function_call",
            "call_id": "call_fuzz",
            "name": "tool_fuzz",
            "arguments": arguments.clone()
        }]
    })
}

/// 面 5 入口：两个 OpenAI 形状协议的参数归一口径。
fn decode_face(mode: u8, payload: &[u8]) {
    let wire = match serde_json::from_slice::<Value>(payload) {
        Ok(value) => value,
        Err(_) => Value::String(String::from_utf8_lossy(payload).into_owned()),
    };

    let (decoded, raw_metadata) = match mode % 2 {
        0 => {
            let response = OpenAiChatAdapter::new()
                .adapt_response(&openai_chat_raw(&wire))
                .expect("OpenAI Chat 回程可解码");
            (
                response.tool_calls.first().map(|call| call.arguments.clone()),
                response.raw_metadata,
            )
        }
        _ => {
            let response = OpenAiResponsesAdapter::new()
                .adapt_response(&openai_responses_raw(&wire))
                .expect("OpenAI Responses 回程可解码");
            (
                response.tool_calls.first().map(|call| call.arguments.clone()),
                response.raw_metadata,
            )
        }
    };

    let Some(arguments) = decoded else {
        return;
    };

    match &wire {
        // C12：畸形 arguments（字符串但不是合法 JSON）⇒ `Null` + 原文记入
        // `raw_metadata.malformed_tool_arguments`（**参数丢失必须可见**）。
        Value::String(text) => match serde_json::from_str::<Value>(text) {
            Ok(parsed) => {
                assert_eq!(
                    arguments, parsed,
                    "合法 arguments 字符串未被原样归一（C12/C13）"
                );
            }
            Err(_) => {
                assert_eq!(
                    arguments,
                    Value::Null,
                    "畸形 arguments 字符串必须置 Null（C12）"
                );
                let recorded = raw_metadata
                    .get("malformed_tool_arguments")
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| {
                        panic!(
                            "畸形 arguments 必须记入 raw_metadata.malformed_tool_arguments（C12）: metadata={raw_metadata:?}"
                        )
                    });
                assert!(
                    recorded.iter().any(|item| item.as_str() == Some(text.as_str())),
                    "畸形 arguments 原文未记入排障面（C12）: recorded={recorded:?}"
                );
            }
        },
        // C12 / C13：**非字符串** arguments（载荷里带真参数）不得被**静默吞成空对象**
        // ——「参数丢失必须可见」是 L4 修复写明的口径；两个 OpenAI 形状协议口径必须一致。
        //
        // 后置条件（可判定、且**不挑修法**）：解码结果要么**原样保留**载荷，要么
        // **fail-loud**（`Null`）；**唯一禁止的形状是空对象** —— 那正是「载荷无声
        // 消失、调用方无从得知」的静默吞并。
        Value::Object(map) if !map.is_empty() => {
            assert!(
                !arguments.as_object().is_some_and(|decoded| decoded.is_empty()),
                "非空对象形态的 arguments 被静默吞成空对象（C12/C13）: wire={wire} decoded={arguments}"
            );
            assert!(
                arguments == Value::Null || arguments == wire,
                "非字符串 arguments 必须原样保留或 fail-loud（Null），不得换形（C12/C13）: wire={wire} decoded={arguments}"
            );
        }
        _ => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 面 6: 同义输入面（同一语义值的不同书写 ⇒ 同一规范化字节）
// ─────────────────────────────────────────────────────────────────────────────

/// 把 JSON 文本里的非 ASCII 字符改写成 `\uXXXX`（仍是同一语义值的另一种书写）。
fn escape_non_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if (ch as u32) > 0x7f {
            out.push_str(&format!("\\u{:04x}", ch as u32));
        } else {
            out.push(ch);
        }
    }
    out
}

/// 同义输入面：同一语义值的不同书写 ⇒ **逐字节相同**的规范化结果（C2「同一语义值恒得
/// 同一字节串（键顺序无关）」）。
///
/// 覆盖的书写差异：空白 / 重复键 / Unicode 转义 vs 原字符 / 键序 / 尾随空白。
/// 对照项：**大小写不同**的键是**不同**的语义值 ⇒ 规范化字节必须不同。
fn synonym_face(payload: &[u8]) {
    let Ok(value) = serde_json::from_slice::<Value>(payload) else {
        return;
    };
    // ⚠️ 上游精度豁免：本面全部断言都建立在「文本 → serde 解析 → 同一语义值」之上；
    // serde_json 的 f64 解析不保证正确舍入（见 [`has_inexact_serde_float`]）⇒ 该情形下
    // 「同一语义的两种书写」会被解析成两个不同的 f64（**上游**限制，非本仓库缺陷）⇒ 跳过。
    if has_inexact_serde_float(&value) {
        return;
    }
    let Value::Object(map) = &value else {
        return;
    };
    let Some((key, item)) = map.iter().next() else {
        return;
    };
    let item_text = serde_json::to_string(item).expect("值可序列化");

    // ① 基线书写（紧凑）。
    let baseline = canonical_json_bytes(&value);

    // ② 加空白（同一语义值）。
    let spaced = format!("{{ {key:?} : {item_text} }}");
    let parsed: Value = serde_json::from_str(&spaced).expect("加空白的书写仍是合法 JSON");
    assert_eq!(parsed, value, "加空白改变了语义值");
    assert_eq!(
        canonical_json_bytes(&parsed),
        baseline,
        "空白差异不得改变规范化字节（C2 同一语义值恒得同一字节串）"
    );

    // ③ 重复键（同一语义值：后写覆盖前写 ⇒ 值不变）。
    let duplicated = format!("{{ {key:?} : {item_text} , {key:?} : {item_text} }}");
    let parsed: Value = serde_json::from_str(&duplicated).expect("重复键的书写仍是合法 JSON");
    assert_eq!(parsed, value, "重复键改变了语义值");
    assert_eq!(
        canonical_json_bytes(&parsed),
        baseline,
        "重复键不得改变规范化字节（C2）"
    );

    // ④ Unicode 转义 vs 原字符（同一语义值）。
    let canonical_text = String::from_utf8(baseline.clone()).expect("规范化输出是 UTF-8");
    let escaped_text = escape_non_ascii(&canonical_text);
    let parsed: Value = serde_json::from_str(&escaped_text).expect("转义书写仍是合法 JSON");
    assert_eq!(parsed, value, "Unicode 转义改变了语义值");
    assert_eq!(
        canonical_json_bytes(&parsed),
        baseline,
        "Unicode 转义 vs 原字符不得改变规范化字节（C2）"
    );

    // ⑤ 键序无关：把同一个对象按**逆序**键重新写出（语义值不变）。
    let mut reversed = Map::new();
    for (name, child) in map.iter().rev() {
        reversed.insert(name.clone(), child.clone());
    }
    let reversed: Value = Value::Object(reversed);
    assert_eq!(reversed, value, "键序改变了语义值");
    assert_eq!(
        canonical_json_bytes(&reversed),
        baseline,
        "键序不得改变规范化字节（C2 键顺序无关）"
    );

    // ⑥ 对照：**大小写不同**的键 = 不同的语义值 ⇒ 规范化字节必须不同。
    let mut cased = Map::new();
    for (name, child) in map.iter() {
        cased.insert(name.to_ascii_uppercase(), child.clone());
    }
    let cased: Value = Value::Object(cased);
    if cased != value {
        assert_ne!(
            canonical_json_bytes(&cased),
            baseline,
            "不同语义值（大小写不同的键）不得给出同一规范化字节（C2 不同语义值必然不同）"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 入口
// ─────────────────────────────────────────────────────────────────────────────

fuzz_target!(|data: &[u8]| {
    let Some((face, rest)) = data.split_first() else {
        return;
    };
    let Some((mode, payload)) = rest.split_first() else {
        return;
    };

    match face % 6 {
        0 => canonical_bytes_face(*mode, payload),
        1 => binding_face(*mode, payload),
        2 => drift_face(*mode, payload),
        3 => protocol_type_face(*mode, payload),
        4 => decode_face(*mode, payload),
        _ => synonym_face(payload),
    }
});
