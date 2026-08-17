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
// TP20-N20: bridge 类型从 apeireth-team-lead 导入 (wire format 协议), 用别名避免
// 与本地 `ApprovalRequest` (本地 SQLite 存储, 含 id/rev/status/updated_at) 重名.
use apeireth_team_lead::{
    ApprovalBridge, ApprovalRequest as WireApprovalRequest,
    ApprovalResponse as WireApprovalResponse,
};
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
///
/// `bridge` 可选: 若提供, 同步通过 ApprovalBridge 通知 orchestrator (失败 eprintln
/// 不阻塞主路径, 不假装"已透传").
pub fn record_request(
    store: &Arc<SqliteMemoryStore>,
    tool: &str,
    args: &Value,
    reason: &str,
    bridge: Option<&Arc<dyn ApprovalBridge>>,
) {
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

    // TP20-N20: 同步通过 bridge 通知 orchestrator, 失败 eprintln 不阻塞主路径.
    if let Some(b) = bridge {
        let wire = WireApprovalRequest {
            chain: req.chain.clone(),
            tool: req.tool.clone(),
            args_preview: req.args_preview.clone(),
            reason: req.reason.clone(),
            created_at: req.created_at,
            extra: Default::default(),
        };
        match b.dispatch_request(wire) {
            Ok(resp) => {
                debug_assert_eq!(resp.chain, req.chain);
                // 响应写回本地 store (append-only): 新 id + 同 chain + rev+1
                apply_wire_response(store, &req.chain, resp);
            }
            Err(e) => {
                eprintln!("[apreq] bridge.dispatch_request 失败 (chain={}): {e}", req.chain);
                // 0 装 PASS: 不阻塞主路径, 不假装"已透传"
            }
        }
    }
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
///
/// `bridge` 可选: 若提供, 同步通过 ApprovalBridge 把响应推回 orchestrator (双向同步).
pub fn mark_approved(
    store: &Arc<SqliteMemoryStore>,
    chain_or_id: &str,
    bridge: Option<&Arc<dyn ApprovalBridge>>,
) -> Result<(), String> {
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

    // TP20-N20: 状态变更推回 orchestrator (双向同步)
    if let Some(b) = bridge {
        let wire = WireApprovalResponse {
            chain: r.chain.clone(),
            decision: "approved".into(),
            decided_at: r.updated_at,
            note: String::new(),
            extra: Default::default(),
        };
        if let Err(e) = b.dispatch_response(wire) {
            eprintln!("[apreq] bridge.dispatch_response 失败 (chain={}): {e}", r.chain);
        }
    }
    Ok(())
}

/// 把 orchestrator 通过 bridge 回传的响应写回本地 store (append-only, 同 chain + rev+1).
///
/// **0 装 PASS**: 响应决策非法 / chain 不存在都 eprintln 不阻塞主路径.
fn apply_wire_response(store: &Arc<SqliteMemoryStore>, chain: &str, resp: WireApprovalResponse) {
    let list = list(store, None);
    let Some(mut existing) = list.into_iter().find(|r| r.chain == chain) else {
        eprintln!("[apreq] bridge response 但 chain 不存在: {chain}");
        return;
    };
    match resp.decision.as_str() {
        "approved" => existing.status = "approved".into(),
        "rejected" => existing.status = "rejected".into(),
        "pending" => return, // orchestrator 暂挂, 不改本地状态
        _ => {
            eprintln!(
                "[apreq] bridge response 未知决策: {} (chain={})",
                resp.decision, chain
            );
            return;
        }
    }
    existing.updated_at = if resp.decided_at > 0 {
        resp.decided_at
    } else {
        chrono::Utc::now().timestamp()
    };
    existing.rev += 1;
    existing.id = format!("apreq-{}", uuid::Uuid::new_v4());
    save(store, &existing);
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
    use apeireth_team_lead::InProcessBridge;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn record_dedupe_and_approve() {
        let s = store();
        record_request(&s, "FileOperator", &json!({"op": "write", "path": "x"}), "需要主人批准", None);
        record_request(&s, "FileOperator", &json!({"op": "write", "path": "x"}), "需要主人批准", None);
        // 同工具同摘要去重 → 1 条
        assert_eq!(list(&s, Some("pending")).len(), 1);
        // 不同摘要 → 2 条
        record_request(&s, "FileOperator", &json!({"op": "write", "path": "y"}), "需要主人批准", None);
        assert_eq!(list(&s, Some("pending")).len(), 2);
        // 批准第一条 → pending 1 条
        let first = list(&s, Some("pending"))[0].clone();
        mark_approved(&s, &first.chain, None).unwrap();
        assert_eq!(list(&s, Some("pending")).len(), 1);
        assert_eq!(list(&s, Some("approved")).len(), 1);
        // 重复批准报错 (最新已是 approved)
        assert!(mark_approved(&s, &first.chain, None).is_err());
    }

    #[test]
    fn pending_json_shape() {
        let s = store();
        record_request(&s, "ShellExec", &json!({"cmd": "dir"}), "需要主人批准", None);
        let j = pending_json(&s);
        assert_eq!(j["count"], json!(1));
        assert_eq!(j["requests"][0]["tool"], json!("ShellExec"));
        assert!(j["note"].as_str().is_some());
    }

    // ===== TP20-N20 bridge 集成测试 =====

    // t11: bridge.send_request 把请求透传给 orchestrator, 无回调默认不写回 (record_log)
    #[test]
    fn t11_bridge_send_no_callback_default_rejects() {
        let s = store();
        let bridge = Arc::new(InProcessBridge::new());
        let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();
        record_request(
            &s,
            "FileOperator",
            &json!({"op": "rm", "path": "/tmp/x"}),
            "需要主人批准",
            Some(&bridge_ref),
        );
        // bridge 收到 1 个请求
        let received = bridge.received_requests();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].tool, "FileOperator");
        assert_eq!(received[0].args_preview, r#"{"op":"rm","path":"/tmp/x"}"#);
        // 无回调 → 默认 reject, apply_wire_response 把 status 改成 rejected
        // 0 装 PASS: 这反映了真实状态 (orchestrator 默认拒绝), 不是"假装已批准"
        let rejected = list(&s, Some("rejected"));
        assert_eq!(rejected.len(), 1);
    }

    // t12: bridge.on_request 注册回调后, record_request 自动批准
    #[test]
    fn t12_bridge_callback_approves_via_record_request() {
        let s = store();
        let bridge = Arc::new(InProcessBridge::new());
        // 注册回调: 自动批准 (模拟 owner auto-approve)
        bridge.on_request(|req| WireApprovalResponse {
            chain: req.chain.clone(),
            decision: "approved".into(),
            decided_at: 1_700_000_999,
            note: "auto-approve".into(),
            extra: Default::default(),
        });
        let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();
        record_request(
            &s,
            "FileOperator",
            &json!({"op": "write", "path": "ok.txt"}),
            "x",
            Some(&bridge_ref),
        );
        // 因为 bridge 返回 approved, apply_wire_response 写回 approved 状态
        let approved = list(&s, Some("approved"));
        assert_eq!(approved.len(), 1);
        assert!(approved[0].updated_at >= 1_700_000_999);
        // bridge 收到 1 个请求 + 1 个响应 (回调自动 dispatch)
        assert_eq!(bridge.received_requests().len(), 1);
        assert_eq!(bridge.received_responses().len(), 1);
    }

    // t13: 状态双向同步 — companion mark_approved → bridge → orchestrator 收到
    #[test]
    fn t13_two_way_sync_mark_approved_dispatches_response() {
        let s = store();
        let bridge = Arc::new(InProcessBridge::new());
        // 回调返回 pending (orchestrator 暂挂): apply_wire_response 不改本地状态,
        // 记录保持 pending, mark_approved 路径才可达 (无回调默认 rejected, 见 t11).
        bridge.on_request(|req| WireApprovalResponse {
            chain: req.chain.clone(),
            decision: "pending".into(),
            decided_at: 0,
            note: "hold".into(),
            extra: Default::default(),
        });
        let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();
        record_request(
            &s,
            "ShellExec",
            &json!({"cmd": "ls"}),
            "需要批准",
            Some(&bridge_ref),
        );
        let pending = list(&s, Some("pending"));
        assert_eq!(pending.len(), 1, "pending 回调下记录应保持 pending");
        let chain = pending[0].chain.clone();

        // mark_approved 应触发 bridge.dispatch_response
        mark_approved(&s, &chain, Some(&bridge_ref)).unwrap();

        let responses = bridge.received_responses();
        let mark_resp = responses
            .iter()
            .find(|r| r.chain == chain && r.decision == "approved");
        assert!(mark_resp.is_some(), "mark_approved 必须 dispatch 1 个 approved 响应");
    }

    // t14: bridge 传 None 时, 主路径不受影响 (向后兼容老调用点)
    #[test]
    fn t14_bridge_none_does_not_break_local_storage() {
        let s = store();
        record_request(&s, "FileOperator", &json!({"op": "rm"}), "x", None);
        assert_eq!(list(&s, Some("pending")).len(), 1);

        let first = list(&s, Some("pending"))[0].clone();
        mark_approved(&s, &first.chain, None).unwrap();
        assert_eq!(list(&s, Some("approved")).len(), 1);
    }

    // t15: apply_wire_response 未知 chain 不 panic
    #[test]
    fn t15_apply_wire_response_unknown_chain_logs_not_panics() {
        let s = store();
        apply_wire_response(
            &s,
            "nonexistent-chain",
            WireApprovalResponse {
                chain: "nonexistent-chain".into(),
                decision: "approved".into(),
                decided_at: 1,
                note: "x".into(),
                extra: Default::default(),
            },
        );
        assert_eq!(list(&s, None).len(), 0);
    }
}
