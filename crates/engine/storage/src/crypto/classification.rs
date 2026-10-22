//! Field classification: 落盘列的加密定级单一真源 (B6-DEEP, 2026-10-21 拍板).
//!
//! ## 定级表 (代码即真源, 测试逐行钉)
//!
//! | 档 | 语义 | 处置 | 拍板依据 |
//! |---|---|---|---|
//! | [`FieldClass::SealedContent`] | 内容列 (对话/笔记/记忆/轨迹/知识正文) | **P0 进加密**: 分区 DEK 封缄 | 隐患 #6 P0 清单 |
//! | [`FieldClass::SealedFingerprint`] | 内容指纹哈希列 (`content_hash`/`fingerprint`) | **P0 进加密** | 哈希是内容指纹, 明文等价内容可被字典反推 (2026-10-21 拍板: 哈希列从「倾向明文」改判进 P0) |
//! | [`FieldClass::PlaintextVector`] | 向量检索基元 (`vector_json`) | **保明** | 检索基元, 加密毁检索且违「记忆摆在明面」产品宪法 |
//! | [`FieldClass::PlaintextMetadata`] | 聚合/排序/时间元数据 | **保明** | 可读性优先 (importance/access_count 等, 2026-10-21 拍板) |
//! | [`FieldClass::PlaintextSearchKey`] | 检索键/身份/结构列 (id 族/角色/状态) | **保明** | 查询语义, 加密即不可查 |
//! | [`FieldClass::Unrated`] | 未勘/边界列 | **显式待定 (fail-closed)** | 不默认明文; 新列先落这里, 拍板后归档 |
//!
//! 判级入口 [`classify_column`]: 先走显式 (表, 列) 表, 再走列名规则;
//! **未识别 = [`FieldClass::Unrated`]** (fail-closed, 不猜不默认)。
//!
//! ## 定级理由 (产品宪法 × 分区隔离墙)
//!
//! - 「记忆摆在明面: 可以读、可以导出、也可以清掉」(创世标尺 §5) →
//!   检索基元与元数据保明, 密钥主权在用户, 导出/清掉语义不受损;
//! - 分区隔离墙 (跨分区披露 R4 零容忍) 由**分区 DEK + AAD 绑分区**保障
//!   (见 [`crate::crypto::partition`]), 与列级定级正交: 进加密档的列
//!   都过分区封缄, 保明档列按查询语义留明文。
//!
//! ## 残余如实登记 (不假装无残余)
//!
//! 1. **相似度推理残余**: 同机同用户读库者可对**明向量** + 密文长度/结构
//!    元数据做相似度推理 (向量保明的代价, 拍板接受: 检索基元不可密);
//! 2. **哈希列改判的检索代价**: `content_hash`/`fingerprint` 进加密档后,
//!    去重/溯源查询须先解封或改建盲索引 (HMAC 化索引列) —— 接线时由各
//!    存储 owner 选型, 本批只定级不接;
//! 3. **边界列待定**: [`FieldClass::Unrated`] 桶 (tags/sources_json/scope_json
//!    等 JSON 附属列、query/sender 等) 含内容语义但未拍板, **现按待定处理
//!    (未加密)**, 归口主代理逐列核。

use serde::{Deserialize, Serialize};

/// 落盘列的加密定级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldClass {
    /// P0 内容列: 分区 DEK 封缄 ([`crate::crypto::SealingLayer`])。
    SealedContent,
    /// P0 内容指纹哈希列: 分区 DEK 封缄 (防字典反推); 查重/溯源检索走
    /// 盲索引 [`crate::crypto::blind_index`] (索引面与内容面分离)。
    SealedFingerprint,
    /// 检索基元 (向量): 保明。
    PlaintextVector,
    /// 聚合/排序/时间元数据: 保明。
    PlaintextMetadata,
    /// 检索键/身份/结构列: 保明。
    PlaintextSearchKey,
    /// 未勘/边界列: 显式待定 (fail-closed, 不默认明文)。
    Unrated,
}

impl FieldClass {
    /// 是否必须封缄 (P0 两档)。SealingLayer 只接受这两档的列值。
    pub fn must_seal(self) -> bool {
        matches!(self, Self::SealedContent | Self::SealedFingerprint)
    }
}

/// 显式 (表, 列) 表: 拍板点名的列 (表名/列名均小写蛇形)。
fn explicit_class(table: &str, column: &str) -> Option<FieldClass> {
    match (table, column) {
        // ---- P0 内容列 (隐患 #6 P0 清单逐条点名) ----
        ("episodes", "content")
        | ("episodes", "data")
        | ("notes", "content")
        | ("notes", "data")
        | ("notes", "title")
        | ("sessions", "data")
        | ("sessions", "title")
        | ("memory_items", "data")
        | ("agent_traces", "summary")
        | ("agent_traces", "data")
        | ("facts", "data")
        | ("links", "data")
        | ("topic_groups", "data")
        | ("provenance", "data")
        | ("context_ledger_messages", "content")
        | ("commitments", "statement")
        | ("persona_memory_entries", "text")
        | ("wiki_entries", "summary")
        | ("wiki_entries", "body")
        | ("wiki_entries", "topic")
        | ("kg_facts", "predicate") => Some(FieldClass::SealedContent),
        // 六流表内容列 (真列名 payload, 不是 content)
        (
            "thought_stream" | "proposal_stream" | "action_stream" | "relation_stream"
            | "evolution_stream" | "reflection_stream",
            "payload",
        ) => Some(FieldClass::SealedContent),
        // ---- P0 内容指纹哈希列 (2026-10-21 拍板: 进加密) ----
        ("dedup_fingerprints", "fingerprint") | ("source_hash", "content_hash") => {
            Some(FieldClass::SealedFingerprint)
        }
        // ---- 保明拍板项 ----
        ("memory_vectors", "vector_json") => Some(FieldClass::PlaintextVector),
        ("memory_items", "importance") | ("memory_items", "access_count") => {
            Some(FieldClass::PlaintextMetadata)
        }
        _ => None,
    }
}

/// 列名规则 (显式表未命中时的兜底; 命中即定级)。
fn class_by_column_name(column: &str) -> Option<FieldClass> {
    match column {
        // 内容列名规则 (与显式表同语义, 覆盖未逐条点名的同类列)
        "content" | "data" | "payload" | "summary" | "body" | "statement" | "text" | "title"
        | "predicate" => Some(FieldClass::SealedContent),
        // 内容指纹哈希列名规则
        "content_hash" | "fingerprint" => Some(FieldClass::SealedFingerprint),
        // 向量检索基元
        "vector_json" | "embedding" => Some(FieldClass::PlaintextVector),
        // 时间/聚合元数据
        "timestamp"
        | "created_at"
        | "updated_at"
        | "accessed_at_ms"
        | "ts"
        | "started_at"
        | "ended_at"
        | "last_active_at"
        | "closed_at"
        | "extracted_at"
        | "believed_at_ms"
        | "due_at_ms"
        | "retracted_at_ms"
        | "completed_at_ms"
        | "valid_from"
        | "valid_until"
        | "valid_from_ms"
        | "valid_until_ms"
        | "first_seen_ms"
        | "last_seen_ms"
        | "last_activated"
        | "tombstoned_at"
        | "birth_time"
        | "importance"
        | "access_count"
        | "access_times"
        | "confidence"
        | "rank"
        | "score"
        | "weight"
        | "revision"
        | "subject_rev"
        | "ingest_count"
        | "skip_count"
        | "co_occurrence_count"
        | "strength"
        | "stability"
        | "is_tombstone"
        | "dimension"
        | "updated_at_ms" => Some(FieldClass::PlaintextMetadata),
        // 检索键/身份/结构列
        "id" | "namespace" | "source" | "kind" | "role" | "status" | "state" | "actor"
        | "model_id" | "wing" | "frontend" | "access_kind" | "subject_kind" | "object_kind"
        | "relation_key" | "from_entity" | "to_entity" | "entity_a" | "entity_b" | "object_id"
        | "protection" | "polarity" | "scope" | "project_id" => {
            Some(FieldClass::PlaintextSearchKey)
        }
        // `*_id` 族 = 引用键
        _ if column.ends_with("_id") => Some(FieldClass::PlaintextSearchKey),
        // 未识别 = 显式待定 (fail-closed)
        _ => None,
    }
}

/// 判级: 显式 (表, 列) 表 → 列名规则 → [`FieldClass::Unrated`] (fail-closed)。
pub fn classify_column(table: &str, column: &str) -> FieldClass {
    explicit_class(table, column)
        .or_else(|| class_by_column_name(column))
        .unwrap_or(FieldClass::Unrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p0_content_columns_are_sealed() {
        let sealed_content = [
            ("episodes", "content"),
            ("episodes", "data"),
            ("notes", "content"),
            ("sessions", "data"),
            ("sessions", "title"),
            ("memory_items", "data"),
            ("agent_traces", "summary"),
            ("facts", "data"),
            ("context_ledger_messages", "content"),
            ("commitments", "statement"),
            ("persona_memory_entries", "text"),
            ("wiki_entries", "body"),
            ("wiki_entries", "topic"),
            ("kg_facts", "predicate"),
            ("thought_stream", "payload"),
            ("reflection_stream", "payload"),
        ];
        for (table, column) in sealed_content {
            let class = classify_column(table, column);
            assert_eq!(
                class,
                FieldClass::SealedContent,
                "{table}.{column} 必须进 P0 加密档"
            );
            assert!(class.must_seal(), "{table}.{column} 必须封缄");
        }
    }

    #[test]
    fn fingerprint_hash_columns_are_sealed_not_plaintext() {
        // 2026-10-21 拍板: 内容指纹哈希列从「倾向明文」改判进 P0 加密
        // (明文哈希等价内容, 可被字典反推)。
        for (table, column) in [
            ("dedup_fingerprints", "fingerprint"),
            ("source_hash", "content_hash"),
            ("memory_vectors", "content_hash"),
        ] {
            let class = classify_column(table, column);
            assert_eq!(
                class,
                FieldClass::SealedFingerprint,
                "{table}.{column} 必须进指纹加密档"
            );
            assert!(class.must_seal());
        }
    }

    #[test]
    fn vector_and_aggregate_metadata_stay_plaintext() {
        // 2026-10-21 拍板: 向量 = 检索基元保明; 聚合元数据保明 (可读性优先)。
        assert_eq!(
            classify_column("memory_vectors", "vector_json"),
            FieldClass::PlaintextVector
        );
        assert!(!classify_column("memory_vectors", "vector_json").must_seal());
        for column in ["importance", "access_count"] {
            assert_eq!(
                classify_column("memory_items", column),
                FieldClass::PlaintextMetadata,
                "memory_items.{column} 必须保明 (元数据)"
            );
        }
    }

    #[test]
    fn search_keys_and_identity_columns_stay_plaintext() {
        assert_eq!(
            classify_column("episodes", "id"),
            FieldClass::PlaintextSearchKey
        );
        assert_eq!(
            classify_column("episodes", "session_id"),
            FieldClass::PlaintextSearchKey
        );
        assert_eq!(
            classify_column("episodes", "continuity_id"),
            FieldClass::PlaintextSearchKey
        );
        assert_eq!(
            classify_column("episodes", "role"),
            FieldClass::PlaintextSearchKey
        );
        assert_eq!(
            classify_column("episodes", "timestamp"),
            FieldClass::PlaintextMetadata
        );
    }

    #[test]
    fn unknown_and_borderline_columns_fail_closed_as_unrated() {
        // fail-closed: 未勘/边界列显式待定, 不默认明文 (改判需拍板 + 同步本测试)。
        for (table, column) in [
            ("notes", "tags_json"),
            ("persona_memory_entries", "sources_json"),
            ("temporal_graph_facts", "provenance_json"),
            ("memory_access_events", "query"),
            ("context_ledger_messages", "sender"),
            ("some_future_table", "some_future_column"),
        ] {
            assert_eq!(
                classify_column(table, column),
                FieldClass::Unrated,
                "{table}.{column} 未拍板, 必须显式待定 (fail-closed)"
            );
        }
        assert!(!FieldClass::Unrated.must_seal());
    }
}
