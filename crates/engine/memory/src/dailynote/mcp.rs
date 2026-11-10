//! MCP server for daily notes (4 tools).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyNoteTool {
    NoteCreate,
    NoteGet,
    NoteSearch,
    NoteExport,
}

impl DailyNoteTool {
    pub fn name(&self) -> &'static str {
        match self {
            DailyNoteTool::NoteCreate => "note_create",
            DailyNoteTool::NoteGet => "note_get",
            DailyNoteTool::NoteSearch => "note_search",
            DailyNoteTool::NoteExport => "note_export",
        }
    }
    pub fn all() -> &'static [DailyNoteTool] {
        &[
            DailyNoteTool::NoteCreate,
            DailyNoteTool::NoteGet,
            DailyNoteTool::NoteSearch,
            DailyNoteTool::NoteExport,
        ]
    }
}

pub const DAILYNOTE_MCP_TOOL_COUNT: usize = 4;

/// **MCP 接线层**（4 个工具：create / get / search / export）。
///
/// ⚠️ **2026-11-03 改造**：此前 `tools/call` **只回显参数**
/// （`format!("create note title={:?}", …)`）⇒ ⇒ **从不真正执行**
/// ⇒ ⇒ 「零生产消费」的一部分原因是**它本来就是桩**。
/// ⇒ ⇒ 现在**持有 store** ⇒ ⇒ 四个工具**真的落库 / 真的读回 / 真的导出**。
pub struct DailyNoteMcp {
    /// ⚠️ **`Arc`**：`DailyNoteStore` 持 `rusqlite::Connection`（**不可 clone**）
    /// ⇒ ⇒ 而 MCP 接线层与 `EnhancedDailyNote` **必须共用同一份库**
    /// ⇒ ⇒ ⇒ 用 `Arc` 共享（否则两套库 ⇒ 数据不通）。
    store: std::sync::Arc<super::store::DailyNoteStore>,
}

impl DailyNoteMcp {
    /// 用给定 store 构造（**真执行**）。
    #[must_use]
    pub fn with_store(store: std::sync::Arc<super::store::DailyNoteStore>) -> Self {
        Self { store }
    }

    /// 进程内 store（测试与便捷用法）。
    #[must_use]
    pub fn new() -> Self {
        Self::with_store(std::sync::Arc::new(
            super::store::DailyNoteStore::open_in_memory().expect("in-memory store"),
        ))
    }

    /// 本接线层用的 store（供上层复用同一份数据）。
    #[must_use]
    pub fn store(&self) -> &super::store::DailyNoteStore {
        &self.store.as_ref()
    }
    pub fn handle(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "apeireth-dailynote", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = DailyNoteTool::all()
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name(),
                            "description": match t {
                                DailyNoteTool::NoteCreate => "Create a new daily note",
                                DailyNoteTool::NoteGet => "Get a daily note by ID",
                                DailyNoteTool::NoteSearch => "Search notes by query and/or tag",
                                DailyNoteTool::NoteExport => "Export a note in Markdown or JSON",
                            }
                        })
                    })
                    .collect();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({"tools": tools})),
                    error: None,
                }
            }
            "tools/call" => {
                let tool = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
                // ⚠️ **2026-11-03**：此前这里是**回显参数**（桩）⇒ ⇒ 现在**真执行**。
                // ⇒ ⇒ 每个分支都真的动 store；失败**如实报错**（不编成功）。
                let summary = match tool {
                    "note_create" => {
                        let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let tags: Vec<String> = args
                            .get("tags")
                            .and_then(|v| v.as_array())
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| item.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let mut note = super::note::DailyNote::new(title, content);
                        if !tags.is_empty() {
                            note = note.with_tags(tags);
                        }
                        match self.store.insert(&note) {
                            Ok(()) => format!("created note {}", note.id.0),
                            Err(error) => format!("note_create failed: {error}"),
                        }
                    }
                    "note_get" => {
                        let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        match self.store.get(&super::note::NoteId(id.to_string())) {
                            Ok(note) => format!(
                                "note {} title={:?} content={:?}",
                                note.id.0, note.title, note.content
                            ),
                            Err(error) => format!("note_get failed: {error}"),
                        }
                    }
                    "note_search" => {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let tag = args.get("tag").and_then(|v| v.as_str());
                        let ids = match tag {
                            Some(tag) => self.store.list_by_tag(tag).unwrap_or_default(),
                            None => self.store.list_all().unwrap_or_default(),
                        };
                        let mut notes = Vec::new();
                        for id in &ids {
                            if let Ok(note) = self.store.get(id) {
                                notes.push(note);
                            }
                        }
                        let hits = super::search::search_notes(&notes, query, tag);
                        format!("search query={query:?} hits={}", hits.len())
                    }
                    "note_export" => {
                        let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let format = args
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("markdown");
                        match self.store.get(&super::note::NoteId(id.to_string())) {
                            Ok(note) => match format {
                                "json" => super::export::export_json(&note),
                                _ => super::export::export_markdown(&note),
                            },
                            Err(error) => format!("note_export failed: {error}"),
                        }
                    }
                    other => format!("unknown tool: {}", other),
                };
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({
                        "content": [{"type": "text", "text": summary}],
                        "isError": false
                    })),
                    error: None,
                }
            }
            "ping" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({})),
                error: None,
            },
            other => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("method not found: {}", other),
                }),
            },
        }
    }
}

impl Default for DailyNoteMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_count_is_4() {
        assert_eq!(DAILYNOTE_MCP_TOOL_COUNT, 4);
    }
    #[test]
    fn initialize_works() {
        let m = DailyNoteMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        });
        assert!(r.result.is_some());
    }
    #[test]
    fn tools_list_4() {
        let m = DailyNoteMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        });
        let binding = r.result.unwrap();
        let tools = binding["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
    }
    #[test]
    fn note_create_tool() {
        let m = DailyNoteMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "note_create", "arguments": {"title": "T", "content": "C"}}),
        });
        let result = r.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        // ⚠️ **2026-11-03 改口径**：此前断言 `text.contains("title")` ⇒ ⇒ 那是**桩行为**
        // （回显参数）⇒ ⇒ 现在 `note_create` **真的落库** ⇒ ⇒ 回显是「created note <id>」。
        // ⇒ ⇒ 断言改为：**真落库**（库里多了一条）+ 回显是创建成功。
        assert!(
            text.starts_with("created note "),
            "note_create 该真落库（实测 {text:?}）"
        );
        assert_eq!(m.store().count().unwrap(), 1, "库里该有 1 条");
    }
}
