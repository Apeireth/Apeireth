//! 证据信封「四面遮形」编排 —— `recon` / `external` 两族共用的**一份**实现。
//!
//! # 覆盖面（四面）
//!
//! | 面 | 内容 | 走的扫描 |
//! |---|---|---|
//! | ① `details` | **递归逐字符串叶子**（键名 / 数字 / 布尔 / null / 结构一律不动） | JSON 叶子面 |
//! | ② 顶层字段 | `claim` / `tool` / `tool_version` / `target` / `scope` / `observed_at` | 文本面 |
//! | ③ `evidence[]` | `kind` / `source` / `digest` / `note` | 文本面 |
//! | ④ `limitations[]` | 每条局限说明 | 文本面 |
//!
//! 两族**同构**（同一套证据信封 + 同一组面）⇒ ⇒ 四面遮形的**编排**只留这一份
//! ⇒ ⇒ 两族不再各写一份逐段对应的编排。
//!
//! # 职责切分（扫描 vs 挂侧车）
//!
//! * **本模块（共享层）**：只**扫** —— 知道信封的**形状**（哪四面、哪些字段、什么
//!   层序、什么顺序）；**不挂侧车**、**不碰族的特有面**；
//! * **族层**（`recon/tripwire.rs` / `external/tripwire.rs`）：挂 `credential_tripwire`
//!   侧车 + 族特有面（`external` 的 stderr 回显面 = 第五面）。
//!
//! # 层序
//!
//! **形状层 (M1) → 字段值层 (M4)**（批次 1 裁决，沿用不改序）：形状层先在**明文**上
//! 判定 ⇒ 命中是真的（不是对已遮内容的误命中）；字段层后接管**字段值** ⇒ 可见标签
//! 与接线前一致。层序由本模块**统一**编排 ⇒ ⇒ 两族不会各自漂移。
//!
//! `details` 的 JSON 叶子走**形状层**口径（孤立叶子没有键上下文 ⇒ 字段层对它是
//! 空操作）。
//!
//! # 为什么两个扫描入口由族层注入
//!
//! 接线口径**按族登记**（接线守卫按**文件所在族**枚举接线方 ⇒ 期望集恰为九族）
//! ⇒ ⇒ 两个扫描入口**由族文件绑进来** ⇒ ⇒ 本模块**不自己认**内核助手的名字
//! ⇒ ⇒ 「一族一份接线」的口径留在族文件里，本模块只留**四面编排**这一份。
//!
//! # 纪律
//!
//! * **零命中** ⇒ 信封与入参**逐字节等价**，且**不挂任何侧车**（干净答案的形状与
//!   字节都不变）；
//! * **命中** ⇒ 内容换成脱敏文本，命中族清单交给族层挂成可观测的
//!   `credential_tripwire` 侧车。

use serde_json::Value;

use crate::guardrail::{LeakedCredentialKind, ResultFaceScan};
use crate::recon::evidence::Evidence;

/// 文本面扫描入口：扫一个**字符串面**（顶层字段 / 证据项 / 局限说明）。
///
/// 入口交回**形状层**的扫描结果 ⇒ 字段值层（层序的另一半）由本模块统一接上
/// ⇒ ⇒ 两族不会各自漂移层序。
pub(crate) type ScanTextFace = fn(&str) -> ResultFaceScan;

/// JSON 面扫描入口：递归扫一个 JSON 值的**字符串叶子**（键名 / 数字 / 布尔 /
/// null / 数组与对象的结构一律不动）。
pub(crate) type ScanJsonLeaves = fn(&Value) -> (Value, Vec<LeakedCredentialKind>);

/// 证据信封「四面遮形」编排器 —— `recon` / `external` 两族共用一份。
#[derive(Clone, Copy)]
pub(crate) struct EvidenceFaceScanner {
    scan_text: ScanTextFace,
    scan_json: ScanJsonLeaves,
}

impl EvidenceFaceScanner {
    /// 构造：绑定两族的扫描入口（形状层文本面 + JSON 叶子面）。
    pub(crate) const fn new(scan_text: ScanTextFace, scan_json: ScanJsonLeaves) -> Self {
        Self {
            scan_text,
            scan_json,
        }
    }

    /// 扫一个字符串面：**形状层 (M1) → 字段值层 (M4)**。
    ///
    /// 零命中 ⇒ 文本**逐字节**原样返回（形状层扫描的契约）。
    fn scan_text_face(&self, text: &str) -> (String, Vec<LeakedCredentialKind>) {
        let scan = (self.scan_text)(text).with_field_layer();
        (scan.text, scan.leaked_kinds)
    }

    /// 扫一个字符串面并就地写回：**命中才写回**（零命中 ⇒ 字节不变）。
    fn scan_face(&self, text: &mut String, kinds: &mut Vec<LeakedCredentialKind>) {
        let (masked, hit) = self.scan_text_face(text);
        if !hit.is_empty() {
            *text = masked;
            kinds.extend(hit);
        }
    }

    /// 证据信封**四面遮形**（**不改结构**）。
    ///
    /// 返回 (遮形后的信封, 命中族清单)。**零命中** ⇒ 信封与入参**逐字节等价**
    /// （且不挂任何侧车 —— 侧车由族层挂）。
    pub(crate) fn redact_evidence_faces(
        &self,
        evidence: &Evidence,
    ) -> (Evidence, Vec<LeakedCredentialKind>) {
        let mut masked = evidence.clone();
        let mut kinds: Vec<LeakedCredentialKind> = Vec::new();

        // 面 ② ：顶层六个字符串字段。
        self.scan_face(&mut masked.claim, &mut kinds);
        self.scan_face(&mut masked.tool, &mut kinds);
        self.scan_face(&mut masked.tool_version, &mut kinds);
        self.scan_face(&mut masked.target, &mut kinds);
        self.scan_face(&mut masked.scope, &mut kinds);
        self.scan_face(&mut masked.observed_at, &mut kinds);

        // 面 ① ：`details` 的递归字符串叶子（键名 / 数字 / 布尔 / null / 结构不动）。
        let (scanned, hit) = (self.scan_json)(&masked.details);
        masked.details = scanned;
        kinds.extend(hit);

        // 面 ③ ：`evidence[]` 的字符串项。
        for item in &mut masked.evidence {
            self.scan_face(&mut item.kind, &mut kinds);
            self.scan_face(&mut item.source, &mut kinds);
            if let Some(digest) = item.digest.as_mut() {
                self.scan_face(digest, &mut kinds);
            }
            if let Some(note) = item.note.as_mut() {
                self.scan_face(note, &mut kinds);
            }
        }

        // 面 ④ ：`limitations[]`。
        for limitation in &mut masked.limitations {
            self.scan_face(limitation, &mut kinds);
        }

        (masked, dedupe_kinds(kinds))
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
