//! `apeireth-companion::approval_requests` — 授权请求机制 (权限洋葱的真实载体).
//!
//! 背景 (2026-08-16 主人反馈): AI 曾虚构「弹窗批准」交互 — serve 没有弹窗,
//! 被拒工具没有真实载体让主人批准 → 前端「网络错误」。
//! 修复: 工具被 RequireApproval 拒绝时产生一条**待批请求** (append-only, apreq-*),
//! 前端轮询展示, 主人一键批准 (复用 /v1/apeireth/grant 的 PermissionPack 授权)。
//!
//! 0 假装: 批准 = 追加同 chain 的 approved 版本 (append-only, 查询取最新);
//! 过期/手动忽略 = 追加 expired 版本。请求本身不自动重试 (主人批准后由对话继续驱动)。

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 授权请求条目 (episodes content JSON, id 前缀 apreq-).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    /// 逻辑链标识 (同工具+同摘要的请求; 批准/过期 = 新 id + 同 chain).
    pub chain: String,
    /// 链内单调版本号 (同 experience/principles 的确定性去重).
    pub rev: u64,
    /// 请求的工具名.
    pub tool: String,
    /// 参数摘要 (截断, 展示用).
    pub args_preview: String,
    /// 请求理由 (工具返回的拒绝信息).
    pub reason: String,
    /// pending / approved / expired.
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 记录一条待批请求 (被 RequireApproval 拒绝时调用; 同工具同摘要去重).
pub fn record_request(store: &Arc<SqliteMemoryStore>, tool: &str, args: &Value, reason: &str) {
    let preview: String = serde_json::to_string(args)
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect();
    let list = list(store, Some("pending"));
    // 同工具同摘要已有 pending → 不重复记录 (防刷屏)
    if list.iter().any(|r| r.tool == tool && r.args_preview == preview) {
        return;
    }
    let now = chrono::Utc::now().timestamp();
    let id = format!("apreq-{}", uuid::Uuid::new_v4());
    let req = ApprovalRequest {
        id: id.clone(),
        chain: id,
        rev: 1,
        tool: tool.to_string(),
        args_preview: preview,
        reason: reason.to_string(),
        status: "pending".into(),
        created_at: now,
        updated_at: now,
    };
    save(store, &req);
}

/// 保存 (append-only; 变更 = 新 id + 同 chain + rev+1).
fn save(store: &Arc<SqliteMemoryStore>, req: &ApprovalRequest) {
    let content = match serde_json::to_string(req) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[apreq] 序列化失败: {e}");
            return;
        }
    };
    let ep = CoreEpisode {
        id: req.id.clone(),
        timestamp: req.created_at,
        role: "assistant".into(),
        content,
        session_id: "me".into(),
    };
    if let Err(e) = store.put_episode(&ep) {
        eprintln!("[apreq] 写入失败: {e}");
    }
}

/// 列出请求 (先按 chain 去重取最新版, 再按 status 过滤 — 顺序关键: 过滤前置会让旧版漏进).
pub fn list(store: &Arc<SqliteMemoryStore>, status: Option<&str>) -> Vec<ApprovalRequest> {
    let eps = store.recent_episodes("me", 500).unwrap_or_default();
    let mut by_chain: std::collections::HashMap<String, ApprovalRequest> = std::collections::HashMap::new();
    for e in eps
        .iter()
        .filter(|e| e.id.starts_with("apreq-"))
        .filter_map(|e| serde_json::from_str::<ApprovalRequest>(&e.content).ok())
    {
        match by_chain.get(&e.chain) {
            Some(existing) if existing.rev > e.rev => {}
            _ => {
                by_chain.insert(e.chain.clone(), e);
            }
        }
    }
    let mut out: Vec<ApprovalRequest> = by_chain
        .into_values()
        .filter(|r| status.map_or(true, |s| r.status == s))
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

/// 标记已批准 (主人批准后调用; 同 chain 追加 approved 版本).
pub fn mark_approved(store: &Arc<SqliteMemoryStore>, chain_or_id: &str) -> Result<(), String> {
    let mut list = list(store, None);
    let idx = list
        .iter()
        .position(|r| r.chain == chain_or_id || r.id == chain_or_id)
        .ok_or_else(|| format!("授权请求不存在: {chain_or_id}"))?;
    let mut r = list.swap_remove(idx);
    if r.status != "pending" {
        return Err(format!("授权请求状态为 {}, 仅 pending 可批准", r.status));
    }
    r.status = "approved".into();
    r.updated_at = chrono::Utc::now().timestamp();
    r.rev += 1;
    r.id = format!("apreq-{}", uuid::Uuid::new_v4());
    save(store, &r);
    Ok(())
}

/// 待批请求 → 前端展示 JSON.
pub fn pending_json(store: &Arc<SqliteMemoryStore>) -> Value {
    let pending = list(store, Some("pending"));
    json!({
        "count": pending.len(),
        "requests": pending.iter().map(|r| json!({
            "id": r.id,
            "tool": r.tool,
            "args_preview": r.args_preview,
            "reason": r.reason,
            "created_at": r.created_at,
        })).collect::<Vec<_>>(),
        "note": "主人批准后, 对话里让本座重试即可"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn record_dedupe_and_approve() {
        let s = store();
        record_request(&s, "FileOperator", &json!({"op": "write", "path": "x"}), "需要主人批准");
        record_request(&s, "FileOperator", &json!({"op": "write", "path": "x"}), "需要主人批准");
        // 同工具同摘要去重 → 1 条
        assert_eq!(list(&s, Some("pending")).len(), 1);
        // 不同摘要 → 2 条
        record_request(&s, "FileOperator", &json!({"op": "write", "path": "y"}), "需要主人批准");
        assert_eq!(list(&s, Some("pending")).len(), 2);
        // 批准第一条 → pending 1 条
        let first = list(&s, Some("pending"))[0].clone();
        mark_approved(&s, &first.chain).unwrap();
        assert_eq!(list(&s, Some("pending")).len(), 1);
        assert_eq!(list(&s, Some("approved")).len(), 1);
        // 重复批准报错 (最新已是 approved)
        assert!(mark_approved(&s, &first.chain).is_err());
    }

    #[test]
    fn pending_json_shape() {
        let s = store();
        record_request(&s, "ShellExec", &json!({"cmd": "dir"}), "需要主人批准");
        let j = pending_json(&s);
        assert_eq!(j["count"], json!(1));
        assert_eq!(j["requests"][0]["tool"], json!("ShellExec"));
        assert!(j["note"].as_str().is_some());
    }
}
