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
//! 四面（`details` 的递归字符串叶子 / 顶层六个字符串字段 / `evidence[]` /
//! `limitations[]`）的**编排**在共享层 `crate::evidence_faces` ⇒ ⇒ 本族只**接线**、
//! 不再自己组织四面。
//!
//! # 职责切分（扫描 vs 挂侧车）
//!
//! * **共享层**：四面遮形（扫描）—— 知道信封的形状、面序与层序；
//! * **族层（本文件）**：挂 `credential_tripwire` 侧车 + 族特有面（下面的第五面）。
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
//! ⚠️ **stderr 回显面只过形状层**（不带字段值层）：stderr 是工具的**裸文本**，字段层
//! 按「键 = 值」遮形会连**未必是凭据**的 `token=` / `key=` 行一起改 ⇒ **干净**
//! stderr 的帧也会变 ⇒ 与「零命中 ⇒ 逐字节不变」冲突 ⇒ 失败面只用形状层（形状层
//! 只在**真凭据形状**上命中）。这与 `git_write` / `worktree_apply` 的失败面同一口径。
//!
//! # 与 `re.*` 族的关系
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
//!   （`{"triggered": true, "leaked_kinds": [...]}`，字段口径对齐结果面）。

use serde_json::Value;

use crate::evidence_faces::EvidenceFaceScanner;
use crate::guardrail::{
    append_credential_tripwire, attach_credential_tripwire, scan_json_string_leaves,
    scan_result_face, LeakedCredentialKind,
};
use crate::recon::evidence::Evidence;

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
