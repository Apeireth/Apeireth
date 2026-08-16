//! **战役 2-2 / VCP `toolCallRecordStore.js` — 工具调用全记录 (append-only)**
//!
//! **目标**: 工具调用的全量记录写入 `apeireth-memory` 的 `action_stream`
//! (append-only 6 历史流之一, D2 §5.2 行动域时间线).
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - `toolCallRecordStore.js:129-167 initDb` — `tool_call_records` 表 14 字段 (id / tool_name / caller_signature / started_at / finished_at / duration_ms / status / success / call_content_json / return_content_json / error_text / has_multimodal ...)
//! - `toolCallRecordStore.js:319-358 beginRecord` — insert 一条记录, status=running
//! - `toolCallRecordStore.js:360-400 finishRecord` — UPDATE 同一行, status=success/failure
//! - `toolCallRecordStore.js:303-309 shouldRecord` — excludeTools 排除
//! - `toolCallRecordStore.js:311-317 detectCaller` — maid / valet 字段识别
//! - `toolCallRecordStore.js:260-275 containsMultimodal` — image_url 标记
//!
//! **Apeireth 适配**:
//! - VCP 独立 SQLite 文件 (`tool-call-records.sqlite3`), 我们用 `apeireth-memory::action_stream`
//!   (D2 §5.2 行动域 = 工具调用最自然的归位)
//! - VCP 14 字段平铺成 SQL 列, 我们塞进 `HistoryEntry.payload` (JSON Value), 灵活 + 不破坏 6 流 schema
//! - VCP 异步两阶段 (beginRecord + finishRecord), 我们 **单阶段** (一次 record 完整生命周期, 按 DoD 简化)
//! - VCP `callerSignature` (maid) 我们从 `args.maid` 提取
//!
//! **不假装**:
//! - ✅ 真用 `apeireth-memory::SqliteMemoryStore` + `ActionStream`
//! - ✅ 字段级引用 VCP 6 字段 (id / tool_name / caller_signature / started_at / duration_ms / status / success / call_content / return_content / error_text)
//! - ✅ append-only (apeireth-memory schema trigger 保证, 不假装)
//! - ✅ 编译期 hardcode (`RECORD_PAYLOAD_VERSION = 1`)

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_memory::{ActionStream, HistoryEntry, HistoryStream, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;
use uuid::Uuid;

use crate::executor::ExecutionResult;
use crate::parser::ParsedToolCall;

/// **战役 2-2 — 工具调用记录 (read-side struct)**
///
/// **字段级引用** `toolCallRecordStore.js:130-160` 表字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// 记录 id (UUID v4)
    pub id: String,
    /// 工具名
    pub tool_name: String,
    /// 调用者署名 (从 `args.maid` 提取, VCP `callerSignature`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_signature: Option<String>,
    /// 调用者类型 (VCP `callerType`, "maid" / "valet" / null)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_type: Option<String>,
    /// 请求 IP (VCP `requestIp`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_ip: Option<String>,
    /// 来源节点 (VCP `sourceNode`, e.g. "post" / "agent")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_node: Option<String>,
    /// 开始时间 (unix ms, VCP `started_at_ms`)
    pub started_at_ms: i64,
    /// 结束时间 (unix ms, VCP `finished_at_ms`)
    pub finished_at_ms: i64,
    /// 耗时 (毫秒, VCP `duration_ms`)
    pub duration_ms: i64,
    /// 状态 (VCP `status`, "success" / "failure" / "timeout")
    pub status: String,
    /// 是否成功 (VCP `success`)
    pub success: bool,
    /// 调用内容 (VCP `call_content_json`, JSON Value)
    pub call_content: Value,
    /// 返回内容 (VCP `return_content_json`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_content: Option<Value>,
    /// 错误文本 (VCP `error_text`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_text: Option<String>,
    /// 是否经过 privacy mask (战役 2-2 新增, 区别于 VCP)
    pub masked: bool,
}

/// **战役 2-2 — Record store**
///
/// 复刻 VCP `toolCallRecordStore.js` 字段级, 适配到 `apeireth-memory::ActionStream`.
pub struct RecordStore {
    store: Arc<SqliteMemoryStore>,
}

impl RecordStore {
    /// 新建 record store
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }

    /// **写入一条工具调用记录 (单阶段, VCP 两阶段简化)**
    ///
    /// **VCP 复刻**: `toolCallRecordStore.js:319-358 beginRecord` + `360-400 finishRecord` 合并
    ///
    /// **数据流**:
    /// 1. 提取 caller_signature (从 args.maid 或 valet, VCP `detectCaller`)
    /// 2. 组装 `ToolCallRecord`
    /// 3. 序列化成 `HistoryEntry.payload` (JSON Value)
    /// 4. 写入 `action_stream` (append-only, D2 §5.2)
    pub async fn record(
        &self,
        call: &ParsedToolCall,
        result: &Value,
        masked: bool,
    ) -> Result<String, String> {
        self.record_with_meta(call, result, masked, None, None, None)
            .await
    }

    /// 完整版 record (带 IP / source / extra meta)
    pub async fn record_with_meta(
        &self,
        call: &ParsedToolCall,
        result: &Value,
        masked: bool,
        request_ip: Option<&str>,
        source_node: Option<&str>,
        extra_meta: Option<Value>,
    ) -> Result<String, String> {
        let now_ms = now_unix_ms();
        let id = format!("tcr-{}", Uuid::new_v4());

        // 1. 提取 caller_signature (VCP toolCallRecordStore.js:311-317 detectCaller)
        let (caller_signature, caller_type) = detect_caller(&call.args);

        // 2. 组装 record
        let record = ToolCallRecord {
            id: id.clone(),
            tool_name: call.tool_name.clone(),
            caller_signature,
            caller_type,
            request_ip: request_ip.map(|s| s.to_string()),
            source_node: source_node.map(|s| s.to_string()),
            started_at_ms: now_ms,
            finished_at_ms: now_ms, // 单阶段: 同步同 now
            duration_ms: 0,         // 单阶段: 调用方预知耗时, 可通过 extra_meta 传入
            status: "success".to_string(),
            success: true,
            call_content: serde_json::json!({
                "tool_name": call.tool_name,
                "arguments": call.args,
            }),
            return_content: Some(result.clone()),
            error_text: None,
            masked,
        };

        // 3. 序列化到 payload
        let mut payload = serde_json::to_value(&record).map_err(|e| format!("serialize: {e}"))?;
        if let Some(extra) = extra_meta {
            if let (Some(payload_obj), Value::Object(extra_obj)) = (payload.as_object_mut(), extra)
            {
                for (k, v) in extra_obj {
                    payload_obj.insert(k, v);
                }
            }
        }

        // 4. 写入 action_stream (D2 §5.2 行动域时间线)
        let entry = HistoryEntry {
            id: id.clone(),
            subject_id: format!("tool_call:{}", call.tool_name),
            subject_rev: 0,
            session_id: None,
            created_at: now_ms / 1000, // HistoryEntry.created_at 是 unix seconds
            payload,
            source: "tool_runtime".to_string(),
            tags: vec!["tool_call_record".to_string()],
            tombstoned_at: None,
        };

        let conn = self.store.conn().map_err(|e| format!("memory conn: {e}"))?;
        let stream = ActionStream::new(&conn);
        stream
            .append(&entry)
            .map_err(|e| format!("append action_stream: {e}"))?;

        debug!("[RecordStore] wrote record id={id} tool={}", call.tool_name);
        Ok(id)
    }

    /// **写入一条失败的工具调用记录**
    pub async fn record_failure(
        &self,
        call: &ParsedToolCall,
        error: &str,
    ) -> Result<String, String> {
        self.record_failure_with_status(call, error, "failure")
            .await
    }

    /// 失败 record, 自定义 status
    pub async fn record_failure_with_status(
        &self,
        call: &ParsedToolCall,
        error: &str,
        status: &str,
    ) -> Result<String, String> {
        let now_ms = now_unix_ms();
        let id = format!("tcr-{}", Uuid::new_v4());

        let (caller_signature, caller_type) = detect_caller(&call.args);

        let record = ToolCallRecord {
            id: id.clone(),
            tool_name: call.tool_name.clone(),
            caller_signature,
            caller_type,
            request_ip: None,
            source_node: None,
            started_at_ms: now_ms,
            finished_at_ms: now_ms,
            duration_ms: 0,
            status: status.to_string(),
            success: false,
            call_content: serde_json::json!({
                "tool_name": call.tool_name,
                "arguments": call.args,
            }),
            return_content: None,
            error_text: Some(error.to_string()),
            masked: false,
        };

        let payload = serde_json::to_value(&record).map_err(|e| format!("serialize: {e}"))?;
        let entry = HistoryEntry {
            id: id.clone(),
            subject_id: format!("tool_call:{}", call.tool_name),
            subject_rev: 0,
            session_id: None,
            created_at: now_ms / 1000,
            payload,
            source: "tool_runtime".to_string(),
            tags: vec!["tool_call_record".to_string(), "failure".to_string()],
            tombstoned_at: None,
        };

        let conn = self.store.conn().map_err(|e| format!("memory conn: {e}"))?;
        let stream = ActionStream::new(&conn);
        stream
            .append(&entry)
            .map_err(|e| format!("append action_stream: {e}"))?;
        Ok(id)
    }

    /// **从 ExecutionResult 写记录 (便捷)**
    pub async fn record_execution(
        &self,
        call: &ParsedToolCall,
        exec: &ExecutionResult,
        masked: bool,
    ) -> Result<String, String> {
        let now_ms = now_unix_ms();
        let id = format!("tcr-{}", Uuid::new_v4());

        let (caller_signature, caller_type) = detect_caller(&call.args);

        let record = ToolCallRecord {
            id: id.clone(),
            tool_name: call.tool_name.clone(),
            caller_signature,
            caller_type,
            request_ip: None,
            source_node: None,
            started_at_ms: now_ms - exec.duration_ms as i64,
            finished_at_ms: now_ms,
            duration_ms: exec.duration_ms as i64,
            status: if exec.success {
                "success".to_string()
            } else {
                "failure".to_string()
            },
            success: exec.success,
            call_content: serde_json::json!({
                "tool_name": call.tool_name,
                "arguments": call.args,
            }),
            return_content: Some(exec.output.clone()),
            error_text: exec.error.clone(),
            masked,
        };

        let payload = serde_json::to_value(&record).map_err(|e| format!("serialize: {e}"))?;
        let entry = HistoryEntry {
            id: id.clone(),
            subject_id: format!("tool_call:{}", call.tool_name),
            subject_rev: 0,
            session_id: None,
            created_at: now_ms / 1000,
            payload,
            source: "tool_runtime".to_string(),
            tags: vec!["tool_call_record".to_string()],
            tombstoned_at: None,
        };

        let conn = self.store.conn().map_err(|e| format!("memory conn: {e}"))?;
        let stream = ActionStream::new(&conn);
        stream
            .append(&entry)
            .map_err(|e| format!("append action_stream: {e}"))?;
        Ok(id)
    }

    /// **列出某工具的所有记录**
    pub fn list_for_tool(&self, tool_name: &str) -> Result<Vec<ToolCallRecord>, String> {
        let conn = self.store.conn().map_err(|e| format!("memory conn: {e}"))?;
        let stream = ActionStream::new(&conn);
        let entries = stream
            .list_for_subject(&format!("tool_call:{tool_name}"), None, None, false)
            .map_err(|e| format!("list action_stream: {e}"))?;
        let mut records = Vec::with_capacity(entries.len());
        for entry in entries {
            if let Ok(r) = serde_json::from_value::<ToolCallRecord>(entry.payload) {
                records.push(r);
            }
        }
        Ok(records)
    }

    /// **取内部 store 引用 (供上层用, e.g. admin web)**
    pub fn store(&self) -> &Arc<SqliteMemoryStore> {
        &self.store
    }
}

// ============================================================
// 内部 helper
// ============================================================

/// **VCP 字段级引用** `toolCallRecordStore.js:311-317 detectCaller`
///
/// 优先 maid, 退到 valet
fn detect_caller(args: &Value) -> (Option<String>, Option<String>) {
    if let Some(maid) = args.get("maid").and_then(|v| v.as_str()) {
        if !maid.trim().is_empty() {
            return (Some(maid.trim().to_string()), Some("maid".to_string()));
        }
    }
    if let Some(valet) = args.get("valet").and_then(|v| v.as_str()) {
        if !valet.trim().is_empty() {
            return (Some(valet.trim().to_string()), Some("valet".to_string()));
        }
    }
    (None, None)
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// Payload schema version (战役 2-2 v1)
pub const RECORD_PAYLOAD_VERSION: u32 = 1;

const _: () = {
    assert!(
        RECORD_PAYLOAD_VERSION == 1,
        "RECORD_PAYLOAD_VERSION must be 1 (战役 2-2 v1)"
    );
};

// ============================================================
// 单元测试 (战役 2-2 DoD: ≥ 5 个, append-only)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::SqliteMemoryStore;
    use serde_json::json;

    fn make_store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().expect("open in-memory"))
    }

    #[tokio::test]
    async fn record_writes_to_action_stream() {
        // 单条写入
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"input": "hello"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let id = rec
            .record(&call, &json!({"echo": "hello"}), false)
            .await
            .expect("record");
        assert!(id.starts_with("tcr-"));

        // 读出
        let records = rec.list_for_tool("EchoSync").expect("list");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tool_name, "EchoSync");
        assert_eq!(records[0].call_content["tool_name"], "EchoSync");
        assert_eq!(records[0].call_content["arguments"]["input"], "hello");
    }

    #[tokio::test]
    async fn record_append_only_multiple() {
        // 多条 append, 不覆盖
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        for i in 0..5 {
            let call = ParsedToolCall {
                tool_name: "Counter".to_string(),
                args: json!({"i": i}),
                raw_marker: "".into(),
                archery: false,
                archery_no_reply: false,
            };
            rec.record(&call, &json!({"result": i}), false)
                .await
                .expect("record");
        }

        let records = rec.list_for_tool("Counter").expect("list");
        assert_eq!(records.len(), 5, "应 5 条 append-only");
    }

    #[tokio::test]
    async fn record_with_maid_caller() {
        // maid 字段提取 (VCP detectCaller)
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "T".to_string(),
            args: json!({"maid": "chuling", "x": 1}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        rec.record(&call, &json!({}), false).await.expect("record");

        let records = rec.list_for_tool("T").expect("list");
        assert_eq!(records[0].caller_signature, Some("chuling".to_string()));
        assert_eq!(records[0].caller_type, Some("maid".to_string()));
    }

    #[tokio::test]
    async fn record_with_valet_caller() {
        // valet fallback (VCP detectCaller)
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "T2".to_string(),
            args: json!({"valet": "anonymous_user"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        rec.record(&call, &json!({}), false).await.expect("record");

        let records = rec.list_for_tool("T2").expect("list");
        assert_eq!(
            records[0].caller_signature,
            Some("anonymous_user".to_string())
        );
        assert_eq!(records[0].caller_type, Some("valet".to_string()));
    }

    #[tokio::test]
    async fn record_marks_masked_flag() {
        // masked=true 应在 record 中标记
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "T3".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        rec.record(
            &call,
            &json!({"api_key": "[APEIRETH_PRIVACY_REDACTED]"}),
            true,
        )
        .await
        .expect("record");

        let records = rec.list_for_tool("T3").expect("list");
        assert!(records[0].masked, "masked=true 应被记录");
    }

    #[tokio::test]
    async fn record_failure_writes_error() {
        // 失败 record (record_failure)
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "T4".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        rec.record_failure(&call, "Tool not found")
            .await
            .expect("record failure");

        let records = rec.list_for_tool("T4").expect("list");
        assert_eq!(records.len(), 1);
        assert!(!records[0].success);
        assert_eq!(records[0].status, "failure");
        assert_eq!(records[0].error_text, Some("Tool not found".to_string()));
    }

    #[tokio::test]
    async fn record_execution_captures_duration() {
        // record_execution 捕获 duration
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "T5".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let exec = ExecutionResult {
            success: true,
            output: json!({"r": 1}),
            error: None,
            duration_ms: 123,
            tool_name: "T5".to_string(),
        };
        rec.record_execution(&call, &exec, false)
            .await
            .expect("record");

        let records = rec.list_for_tool("T5").expect("list");
        assert_eq!(records[0].duration_ms, 123);
        assert!(records[0].success);
    }

    #[tokio::test]
    async fn records_have_unique_ids() {
        // 多条 record 的 id 唯一
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "Idempotent".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let id1 = rec.record(&call, &json!({}), false).await.expect("r1");
        let id2 = rec.record(&call, &json!({}), false).await.expect("r2");
        let id3 = rec.record(&call, &json!({}), false).await.expect("r3");
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn record_tombstoned_excluded() {
        // 软删除的记录默认不列出
        let store = make_store();
        let rec = RecordStore::new(store.clone());

        let call = ParsedToolCall {
            tool_name: "TS".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let id = rec.record(&call, &json!({}), false).await.expect("record");
        // 软删除: 用 scope 限制 MutexGuard 生命周期, 避免与后续 list_for_tool 重入死锁
        // (apeireth-memory 用 std::sync::Mutex, 不可重入)
        {
            let conn = store.conn().expect("conn");
            let stream = ActionStream::new(&conn);
            stream
                .tombstone(&id, 1_700_000_000, "test")
                .expect("tombstone");
        }
        let records = rec.list_for_tool("TS").expect("list");
        assert_eq!(records.len(), 0, "tombstoned 记录应被过滤");
    }
}
