//! 结果面凭据形状绊线 —— `ext.*` 族（外部工具适配族）的接线。
//!
//! # 为什么这一族要单独一道
//!
//! 本族的结果面是**证据信封** [`Evidence`]，而其中的 `details` 是**任意 JSON**
//! ⇒ 凭据可能藏在任意深度的字符串叶子里。本族有两种典型的**原样回显面**：
//!
//! * **规则扫描族**：把工具 stdout 的**前 8192 字原文**整段塞进 `details` 的一枚
//!   字符串叶子 ⇒ 被扫目标里硬编码的密钥**原样回显**；
//! * **元数据族 / 静态分析族**：把工具自己的 JSON 输出整份放进 `details` ⇒ 工具
//!   吐出的任意串（含凭据）同样**原样回显**。
//!
//! # 覆盖面（一次接线覆盖本族 10 个工具）
//!
//! | 面 | 内容 | 用的助手 |
//! |---|---|---|
//! | `details` | **递归逐字符串叶子**（键名 / 数字 / 布尔 / null / 结构一律不动） | `guardrail::scan_json_string_leaves` |
//! | 顶层字段 | `claim` / `tool` / `tool_version` / `target` / `scope` / `observed_at` | `guardrail::scan_result_face` |
//! | `evidence[]` | `kind` / `source` / `digest` / `note` | 同上 |
//! | `limitations[]` | 每条局限说明 | 同上 |
//!
//! # 原样回显面怎么覆盖
//!
//! 原样回显面**就是** `details` 里的一枚（可能很大的）字符串叶子 ⇒ JSON 叶子扫描
//! **递归**走到它 ⇒ **不做任何长度截断** ⇒ 8192 字量级的原文整块被扫（形状层
//! 对文本长度**无上限** ⇒ 没有「只扫前 N 字」的漏检口）。
//!
//! # 第五面：stderr 回显面（**失败帧**）
//!
//! 上面的四张面都是**成功面**（证据信封）。本族还有第五张面：**失败帧** ——
//! 外部工具的 **stderr** 被拼进**错误消息**交给模型 ⇒ 工具自己吐出的凭据形状
//! （目标里硬编码的密钥 / 工具配置里的令牌）会**原样回显**在错误帧里。
//!
//! 本族有两类 stderr 回显面（外部工具把 stderr 交给我们的那两条路）：
//!
//! | 面 | 谁走 | 长度 |
//! |---|---|---|
//! | 截断面 | 多数适配器（`ensure_success`） | 前 **400 字**（既有合同，先于本族接线） |
//! | 整段面 | 规则扫描族 / 元数据族 / 证书链族 | **不截断**（整段 stderr 原文） |
//!
//! **整段面是最凶的一面**（量级大、无上限、且规则扫描族那一路把**全部** stderr
//! 原样塞进帧）⇒ ⇒ 两条路都必须过同一道绊线。
//!
//! 口径与其它族的失败面一致（`git_write` / `worktree_apply` 的失败面）：
//! **形状层 (M1) 遮形 + 仅命中才在帧末尾挂一行 JSON 注解**（`credential_tripwire`
//! 侧车 —— 裸文本面挂不住结构化侧车）。
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
//! ⚠️ **stderr 回显面只过形状层**（不带字段值层）：stderr 是工具的**裸文本**，字段层
//! 按「键 = 值」遮形会连**未必是凭据**的 `token=` / `key=` 行一起改 ⇒ **干净**
//! stderr 的帧也会变 ⇒ 与「零命中 ⇒ 逐字节不变」冲突 ⇒ 失败面只用形状层（形状层
//! 只在**真凭据形状**上命中）。这与 `git_write` / `worktree_apply` 的失败面同一口径。
//!
//! # 与 `re.*` 族的关系（如实登记）
//!
//! 本族与 `re.*` 族**同构**（同一套证据信封 + 同一组字符串面）⇒ 本模块与
//! `crate::recon::tripwire` 是**两份同族型的接线**，口径一致（不是两套规则）。
//! 泛化收口（把族型助手上收到一处共享层，让两族各调一份）属**下一批**：
//! `recon/tripwire.rs` 不在本批写域内 ⇒ 本批不动它，也不动它的既有接线。
//!
//! # 纪律
//!
//! * **零命中** ⇒ 内容**逐字节**不变，且**不挂任何侧车**（干净答案的形状与字节
//!   都不变）；
//! * **命中** ⇒ 内容换成脱敏文本，并挂上可观测的 `credential_tripwire` 侧车
//!   （`{"triggered": true, "leaked_kinds": [...]}`，字段口径对齐结果面）。

use serde_json::Value;

use crate::guardrail::{
    append_credential_tripwire, attach_credential_tripwire, scan_json_string_leaves,
    scan_result_face, LeakedCredentialKind,
};
use crate::recon::evidence::Evidence;

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

/// stderr 回显面（**失败帧**）：外部工具的 stderr 被拼进错误消息的那一面。
///
/// # 口径
///
/// * **形状层 (M1) 遮形** + **仅命中才**在末尾挂一行 JSON 注解
///   （`credential_tripwire` 侧车）—— 裸文本面挂不住结构化侧车 ⇒ 与
///   `git_write` / `worktree_apply` 的失败面同一做法；
/// * **零命中 ⇒ 文本逐字节不变**（不挂注解）⇒ 干净 stderr 的错误帧与接线前
///   **完全相同**（形状与字节都不变）；
/// * **只接失败帧**：成功面走 [`sanitized_evidence_value`]，本函数**不动**它；
/// * **不接字段值层 (M4)**：stderr 是工具的**裸文本**，字段层按「键 = 值」遮形
///   （含 `token=` / `key=` 这类**未必是凭据**的行）⇒ 那会让**干净** stderr 的
///   帧也变 ⇒ 与「零命中 ⇒ 逐字节不变」冲突 ⇒ 本面只用形状层（形状层只在
///   **真凭据形状**上命中）。
///
/// # 输入是「进帧的那一段」
///
/// 调用方给什么就扫什么：截断面给**前 400 字**（既有合同），整段面给**整段
/// stderr** ⇒ 扫的正是**真的会进帧**的那些字节 ⇒ 不多扫、不少扫。
pub fn scan_stderr_frame(stderr: &str) -> String {
    let scan = scan_result_face(stderr);
    append_credential_tripwire(&scan.text, &scan.leaked_kinds)
}
