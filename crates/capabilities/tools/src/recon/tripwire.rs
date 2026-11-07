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
//! 四面（`details` 的递归字符串叶子 / 顶层六个字符串字段 / `evidence[]` /
//! `limitations[]`）的**编排**在共享层 `crate::evidence_faces` ⇒ ⇒ 本族只**接线**、
//! 不再自己组织四面。
//!
//! # 职责切分（扫描 vs 挂侧车）
//!
//! * **共享层**：四面遮形（扫描）—— 知道信封的形状、面序与层序；
//! * **族层（本文件）**：挂 `credential_tripwire` 侧车 + 族特有面（本族无额外面）。
//!
//! # 与 `ext.*` 族的关系
//!
//! 两族**同构**（同一套证据信封 + 同一组面）⇒ ⇒ 四面遮形**共用一份**实现 ⇒ ⇒
//! 两族不再各写一份逐段对应的编排；**口径仍然同源**（两族绑的是同一批内核助手，
//! 层序由共享层统一编排 ⇒ 不会各自漂移）。
//!
//! # 纪律
//!
//! * **零命中** ⇒ 内容**逐字节**不变，且**不挂任何侧车**（干净答案的形状与字节
//!   都不变）；
//! * **命中** ⇒ 内容换成脱敏文本，并挂上可观测的 `credential_tripwire` 侧车
//!   （`{"triggered": true, "leaked_kinds": [...]}`，字段口径对齐 shell 结果面）。

use serde_json::Value;

use crate::evidence_faces::EvidenceFaceScanner;
use crate::guardrail::{
    attach_credential_tripwire, scan_json_string_leaves, scan_result_face, LeakedCredentialKind,
};

use super::evidence::Evidence;

/// 本族的四面扫描器：把本族的两条扫描入口（形状层文本面 + JSON 叶子面）绑到
/// **共享的四面编排**上。
///
/// ⚠ 入口在**族层**绑（不藏进共享模块）⇒ ⇒ 接线口径**按族登记**仍成立（接线守卫
/// 按文件所在族枚举接线方 ⇒ 本族仍被枚举）⇒ ⇒ 共享层只留「四面编排」这一份。
const SCANNER: EvidenceFaceScanner =
    EvidenceFaceScanner::new(scan_result_face, scan_json_string_leaves);

/// 把本族结果面上的凭据形状遮掉（**不改结构**）。
///
/// 返回 (遮形后的信封, 命中族清单)。**零命中** ⇒ 信封与入参**逐字节等价**
/// （且不挂任何侧车）。
pub fn redact_credential_faces(evidence: &Evidence) -> (Evidence, Vec<LeakedCredentialKind>) {
    SCANNER.redact_evidence_faces(evidence)
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
