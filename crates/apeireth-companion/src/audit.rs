//! `apeireth-companion::audit` — 审计能力包内容 (audit_log 工具).
//!
//! 审计链组成 (docs/release-plan.md audit-pack):
//! - **留痕**: RecordStore → apeireth-memory action_stream (append-only, 6 流之一)
//! - **脱敏**: 调用/返回经隐私 mask (masked 标记) + 出站 guard
//! - **溢出**: 超大工具结果 spill 到会话私有文件 (messages 只留定位)
//! - **可读**: 本工具 `audit_log` — 查询工具调用留痕 (只读, 供主人/AI 复盘)
//!
//! 0 假装: 只读查询; masked 记录的参数**不还原** (输出时替换为占位符);
//! 不提供删除/篡改接口 (append-only 由 memory 层保证).

use std::sync::Arc;

use apeireth_memory::HistoryStream;
use apeireth_memory::SqliteMemoryStore;
use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use serde_json::{json, Value};

/// audit_log 工具: 查询工具调用留痕 (按工具名过滤, 最近 N 条).
pub struct AuditLogTool {
    store: Arc<SqliteMemoryStore>,
}

impl AuditLogTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl Tool for AuditLogTool {
    fn name(&self) -> &str {
        "audit_log"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let tool_name = args
            .get("tool_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as usize;
        let records = apeireth_tool_runtime::record::RecordStore::new(Arc::clone(&self.store));
        let list = match tool_name {
            Some(t) => records.list_for_tool(t)?,
            None => {
                // 无过滤: 取 action_stream 最近 limit 条 (list_recent, 时间升序)
                let conn = self.store.conn().map_err(|e| format!("memory conn: {e}"))?;
                let stream = apeireth_memory::ActionStream::new(&conn);
                let entries = stream
                    .list_recent(limit, false)
                    .map_err(|e| format!("list action_stream: {e}"))?;
                let mut out = Vec::new();
                for entry in entries {
                    if let Ok(r) = serde_json::from_value::<
                        apeireth_tool_runtime::record::ToolCallRecord,
                    >(entry.payload)
                    {
                        out.push(r);
                    }
                }
                out
            }
        };
        // 取最近 limit 条 (append-only, 末尾最新; list_for_tool 是升序)
        let tail: Vec<_> = list.iter().rev().take(limit).collect();
        let rows: Vec<Value> = tail
            .iter()
            .map(|r| {
                // 脱敏: masked 记录不还原参数; 0 假装: 参数已在前端 mask, 此处不再展开
                let call_view = if r.masked {
                    json!({"tool_name": r.tool_name, "arguments": "[masked by audit] (隐私已脱敏)"})
                } else {
                    r.call_content.clone()
                };
                json!({
                    "id": r.id,
                    "tool_name": r.tool_name,
                    "started_at_ms": r.started_at_ms,
                    "duration_ms": r.duration_ms,
                    "status": r.status,
                    "success": r.success,
                    "masked": r.masked,
                    "call": call_view,
                    "error": r.error_text,
                })
            })
            .collect();
        Ok(json!({
            "count": rows.len(),
            "total_for_filter": list.len(),
            "records": rows,
            "note": "append-only 留痕 (action_stream); masked 记录参数已脱敏, 不还原"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_runtime::parser::ParsedToolCall;
    use apeireth_tool_runtime::record::RecordStore;

    #[tokio::test]
    async fn audit_log_lists_recent_records() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let records = RecordStore::new(Arc::clone(&store));
        for i in 0..5 {
            let call = ParsedToolCall {
                tool_name: "WebSearch".into(),
                args: json!({"query": format!("搜索 {i}")}),
                raw_marker: String::new(),
                archery: false,
                archery_no_reply: false,
            };
            records
                .record(&call, &json!({"ok": true}), false)
                .await
                .unwrap();
        }
        let tool = AuditLogTool::new(store);
        // 按工具过滤
        let v = tool
            .call(json!({"tool_name": "WebSearch", "limit": 3}))
            .await
            .unwrap();
        assert_eq!(v["count"], json!(3), "取最近 3 条");
        assert_eq!(v["total_for_filter"], json!(5));
        // 无过滤
        let v2 = tool.call(json!({"limit": 10})).await.unwrap();
        assert_eq!(v2["count"], json!(5));
        // 未知工具 → 0 条, 不报错
        let v3 = tool.call(json!({"tool_name": "Nope"})).await.unwrap();
        assert_eq!(v3["count"], json!(0));
    }

    #[tokio::test]
    async fn audit_log_masks_private_arguments() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let records = RecordStore::new(Arc::clone(&store));
        let call = ParsedToolCall {
            tool_name: "WebFetch".into(),
            args: json!({"url": "https://secret.example/api?key=abc123"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        records
            .record(&call, &json!({"ok": true}), true)
            .await
            .unwrap();
        let tool = AuditLogTool::new(store);
        let v = tool.call(json!({"tool_name": "WebFetch"})).await.unwrap();
        assert_eq!(v["records"][0]["masked"], json!(true));
        let args = &v["records"][0]["call"]["arguments"];
        assert_eq!(
            args,
            &json!("[masked by audit] (隐私已脱敏)"),
            "敏感参数不还原"
        );
        assert!(
            !serde_json::to_string(&v).unwrap().contains("abc123"),
            "密钥不得出现在审计输出"
        );
    }
}
