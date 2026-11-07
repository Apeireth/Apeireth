//! 结果面凭据形状绊线 —— `re.*` 族（recon 族）的接线。
//!
//! # 为什么这一族要单独一道
//!
//! 本族的结果面是**证据信封** [`Evidence`](super::evidence::Evidence)，而其中的
//! `details` 是**任意 JSON** ⇒ 凭据可能藏在任意深度的字符串叶子里。典型：
//! `re.pack` 的 `details.strings` 是二进制**可打印字符串的全量提取** ⇒ 目标里
//! 硬编码的密钥会**原样回显**。
//!
//! # 覆盖面（一次接线覆盖本族 9 个工具）
//!
//! | 面 | 内容 | 用的助手 |
//! |---|---|---|
//! | `details` | **递归逐字符串叶子**（键名 / 数字 / 布尔 / null / 结构一律不动）| `guardrail::scan_json_string_leaves` |
//! | 顶层字段 | `claim` / `tool` / `tool_version` / `target` / `scope` / `observed_at` | `guardrail::scan_result_face` |
//! | `evidence[]` | `kind` / `source` / `digest` / `note` | 同上 |
//! | `limitations[]` | 每条局限说明 | 同上 |
//!
//! # 层序
//!
//! **形状层 (M1) → 字段值层 (M4)**（批次 1 裁决，沿用不改序）：形状层先在**明文**
//! 上判定 ⇒ 命中是真的（不是对已遮内容的误命中）；字段层后接管**字段值** ⇒ 可见
//! 标签与接线前一致。
//!
//! `details` 的 JSON 叶子走共享助手的**形状层**口径（孤立叶子没有键上下文 ⇒ 字段
//! 层对它是空操作；见 `guardrail::scan_json_string_leaves` 的文档）。
//!
//! # 纪律
//!
//! * **零命中** ⇒ 内容**逐字节**不变，且**不挂任何侧车**（干净答案的形状与字节
//!   都不变）；
//! * **命中** ⇒ 内容换成脱敏文本，并挂上可观测的 `credential_tripwire` 侧车
//!   （`{"triggered": true, "leaked_kinds": [...]}`，字段口径对齐 shell 结果面）。

use serde_json::Value;

use crate::guardrail::{
    attach_credential_tripwire, scan_json_string_leaves, scan_result_face, LeakedCredentialKind,
};

use super::evidence::Evidence;

/// 文本面清扫：形状层 (M1) → 字段值层 (M4)。
///
/// 零命中 ⇒ 文本**逐字节**原样返回（[`scan_result_face`] 的契约）。
fn scan_text_face(text: &str) -> (String, Vec<LeakedCredentialKind>) {
    let scan = scan_result_face(text).with_field_layer();
    (scan.text, scan.leaked_kinds)
}

/// 扫一个字符串面并就地写回；**命中才写回**（零命中 ⇒ 字节不变）。
fn scan_face(text: &mut String, kinds: &mut Vec<LeakedCredentialKind>) {
    let (masked, hit) = scan_text_face(text.as_str());
    if !hit.is_empty() {
        *text = masked;
        kinds.extend(hit);
    }
}

/// 命中族清单去重（保持首次出现顺序）：侧车只报**族**，不报命中次数。
fn dedupe_kinds(kinds: Vec<LeakedCredentialKind>) -> Vec<LeakedCredentialKind> {
    let mut out: Vec<LeakedCredentialKind> = Vec::new();
    for kind in kinds {
        if !out.contains(&kind) {
            out.push(kind);
        }
    }
    out
}

/// 把本族结果面上的凭据形状遮掉（**不改结构**）。
///
/// 返回 (遮形后的信封, 命中族清单)。**零命中** ⇒ 信封与入参**逐字节等价**
/// （且不挂任何侧车）。
pub fn redact_credential_faces(evidence: &Evidence) -> (Evidence, Vec<LeakedCredentialKind>) {
    let mut masked = evidence.clone();
    let mut kinds: Vec<LeakedCredentialKind> = Vec::new();

    // 顶层字符串字段。
    scan_face(&mut masked.claim, &mut kinds);
    scan_face(&mut masked.tool, &mut kinds);
    scan_face(&mut masked.tool_version, &mut kinds);
    scan_face(&mut masked.target, &mut kinds);
    scan_face(&mut masked.scope, &mut kinds);
    scan_face(&mut masked.observed_at, &mut kinds);

    // `details`：共享助手递归扫**字符串叶子**（键名 / 数字 / 布尔 / null / 结构不动）。
    let (scanned, hit) = scan_json_string_leaves(&masked.details);
    masked.details = scanned;
    kinds.extend(hit);

    // `evidence[]` 的字符串项。
    for item in &mut masked.evidence {
        scan_face(&mut item.kind, &mut kinds);
        scan_face(&mut item.source, &mut kinds);
        if let Some(digest) = item.digest.as_mut() {
            scan_face(digest, &mut kinds);
        }
        if let Some(note) = item.note.as_mut() {
            scan_face(note, &mut kinds);
        }
    }

    // `limitations[]`。
    for limitation in &mut masked.limitations {
        scan_face(limitation, &mut kinds);
    }

    (masked, dedupe_kinds(kinds))
}

/// 本族结果面的**最终 JSON 值**：遮形后的信封 +（**仅命中时**）`credential_tripwire`
/// 侧车。
///
/// 零命中 ⇒ 与 `serde_json::to_value(evidence)` **逐字节相同**（不挂侧车）。
pub fn sanitized_evidence_value(evidence: &Evidence) -> Value {
    let (masked, kinds) = redact_credential_faces(evidence);
    let mut value = serde_json::to_value(&masked).unwrap_or(Value::Null);
    attach_credential_tripwire(&mut value, &kinds);
    value
}
