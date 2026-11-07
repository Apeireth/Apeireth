//! EnhancedDailyNote composed entry.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use super::export::{export_json, export_markdown};
use super::mcp::{DailyNoteMcp, McpRequest, McpResponse};
use super::note::DailyNote;
use super::search::search_notes;
use super::store::DailyNoteStore;

pub struct EnhancedDailyNote {
    /// ⚠️ **`Arc`**：与 MCP 接线层**共用同一份库**（`DailyNoteStore` 不可 clone）。
    store: std::sync::Arc<DailyNoteStore>,
    mcp: DailyNoteMcp,
}

impl EnhancedDailyNote {
    pub fn new_in_memory() -> Self {
        let store = std::sync::Arc::new(DailyNoteStore::open_in_memory().expect("in-memory store"));
        // ⚠️ **2026-11-03 修**：`mcp` 此前**自己另开一个 store**
        // ⇒ ⇒ `dispatch_mcp` 看到的库与 `insert`/`search` 看到的库**不是同一个**
        // ⇒ ⇒ **经 MCP 建的笔记在 `search` 里查不到** ⇒ ⇒ 数据不通。
        // ⇒ ⇒ 现在**共用同一个 store**（`Arc` ⇒ 同一份库）。
        let mcp = DailyNoteMcp::with_store(std::sync::Arc::clone(&store));
        Self { store, mcp }
    }

    /// 用给定 store 构造（MCP 与本体**共用**它 ⇒ 数据一致）。
    #[must_use]
    pub fn with_store(store: std::sync::Arc<DailyNoteStore>) -> Self {
        let mcp = DailyNoteMcp::with_store(std::sync::Arc::clone(&store));
        Self { store, mcp }
    }

    /// 本实例的 store（供上层直接访问同一份数据）。
    #[must_use]
    pub fn store(&self) -> &DailyNoteStore {
        &self.store.as_ref()
    }
    pub fn insert(&self, note: &DailyNote) -> Result<(), super::store::DailyNoteError> {
        self.store.insert(note)
    }
    pub fn search(
        &self,
        query: &str,
        tag: Option<&str>,
    ) -> Result<Vec<super::note::DailyNote>, super::store::DailyNoteError> {
        let mut notes = Vec::new();
        // ⚠️ **2026-11-03 自查发现并修掉的假实现**：此前**无 tag 过滤**时
        // 直接 `return Ok(Vec::new())`（注释写着 "Simplified: just return empty"）
        // ⇒ ⇒ **不传 tag 的搜索永远搜不到东西** ⇒ ⇒ 而这恰是**最常见的用法**。
        // ⇒ ⇒ 现在两条路径都真取候选：
        // · **有 tag** ⇒ 按 tag 索引取；
        // · **无 tag** ⇒ 按 `list_all()` 取全量（**新在前**）。
        let ids = match tag {
            Some(tag) => self.store.list_by_tag(tag).unwrap_or_default(),
            None => self.store.list_all().unwrap_or_default(),
        };
        for id in &ids {
            if let Ok(note) = self.store.get(id) {
                notes.push(note);
            }
        }
        let hits = search_notes(&notes, query, tag);
        let mut out = Vec::new();
        for h in hits {
            if let Ok(n) = self.store.get(&h.note_id) {
                out.push(n);
            }
        }
        Ok(out)
    }
    pub fn export_md(&self, note: &DailyNote) -> String {
        export_markdown(note)
    }
    pub fn export_json(&self, note: &DailyNote) -> String {
        export_json(note)
    }
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        self.mcp.handle(req)
    }
}

impl Default for EnhancedDailyNote {
    fn default() -> Self {
        Self::new_in_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **2026-11-03**：证明 MCP 的 4 个工具**真执行**（此前只是回显参数的桩），
    /// 且 **MCP 与本体共用同一份库**（此前是两个库 ⇒ 数据不通）。
    #[test]
    fn mcp_tools_really_execute_and_share_the_same_store() {
        let enhanced = EnhancedDailyNote::new_in_memory();

        let text_of = |response: &McpResponse| -> String {
            response
                .result
                .as_ref()
                .and_then(|value| value.get("content"))
                .and_then(|items| items.as_array())
                .and_then(|items| items.first())
                .and_then(|item| item.get("text"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string()
        };

        // ① note_create ⇒ **真的落库**（不是回显 "create note title=..."）
        let created = enhanced.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "note_create",
                "arguments": {"title": "会议纪要", "content": "讨论了记忆引擎", "tags": ["work"]}
            }),
        });
        let text = text_of(&created);
        assert!(
            text.starts_with("created note "),
            "note_create 该真落库（实测 {text:?}）"
        );
        let note_id = text.trim_start_matches("created note ").to_string();
        assert_eq!(enhanced.store().count().unwrap(), 1, "库里该有 1 条");

        // ② note_get ⇒ 真读回
        let got = enhanced.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "note_get",
                "arguments": {"id": note_id}
            }),
        });
        let text = text_of(&got);
        assert!(
            text.contains("会议纪要"),
            "note_get 该真读回内容（实测 {text:?}）"
        );

        // ③ note_search ⇒ **无 tag 过滤**也能搜到（**这正是此前假实现的坑**）
        let searched = enhanced.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "note_search",
                "arguments": {"query": "记忆引擎"}
            }),
        });
        let text = text_of(&searched);
        assert!(
            text.contains("hits=1"),
            "无 tag 的搜索该搜得到（实测 {text:?}）"
        );

        // ④ 本体的 search 也该看到 **MCP 建的那条**（**同一个库**）
        let found = enhanced.search("记忆引擎", None).unwrap();
        assert_eq!(found.len(), 1, "MCP 建的笔记本体也该搜得到");
    }

    #[test]
    fn insert_and_search() {
        let e = EnhancedDailyNote::new_in_memory();
        let n = DailyNote::new("Rust", "ownership").with_tags(vec!["rust".into()]);
        e.insert(&n).unwrap();
        let results = e.search("rust", Some("rust")).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn export_md() {
        let e = EnhancedDailyNote::new_in_memory();
        let n = DailyNote::new("T", "C");
        let md = e.export_md(&n);
        assert!(md.contains("# T"));
    }

    #[test]
    fn dispatch_mcp() {
        let e = EnhancedDailyNote::new_in_memory();
        let r = e.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        });
        assert!(r.result.is_some());
    }
}
