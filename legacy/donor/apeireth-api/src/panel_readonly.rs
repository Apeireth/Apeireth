//! B1 Web 面板 v2 — 只读面板数据端点 (team-work-doc §4 B1, 2026-08-17).
//!
//! **定位**: 给静态 Web 面板 (companion assets/panel/) 供数的 7 个只读 GET 端点.
//! 数据源全部是 `SqliteMemoryStore` 真实持久层 — 会话表 / 6 历史流 / episodes
//! (含 factg-* 图事实、link-* 图链接、apreq-* 授权请求) / action_stream 审计留痕.
//!
//! **只加不改**: 不新增写路径, 不改任何已有 API 语义; 授权批准走已有
//! `POST /v1/apeireth/grant` (master token) 机制, 本模块不新建安全口.
//!
//! **升级点 (如实标注)**:
//! - 会话管理: 现挂 `SessionStore` (apeireth-memory); backlog N2 OneRing 统一账本
//!   就绪后, 前端换接口即可 (JSON 形状保持 list+count).
//! - 图: 现直读 factg-*/link-* episodes (跟 companion SqliteGraphBackend 同一持久形态);
//!   图后端换 Kùzu/结构化 GraphQuery 后换 `/panel/graph` 实现即可.
//! - 授权请求: 直读 apreq-* episodes, 去重语义对齐
//!   `apeireth_companion::approval_requests::list` (chain 取最新 rev);
//!   若未来 api 可依赖 companion, 换直接调用.

use std::sync::Arc;

use apeireth_memory::{
    history_streams::StreamDepth, ActionStream, EpisodeQuery, EpisodeStore, HistoryStream,
    SessionStore, SqliteMemoryStore,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// 默认主体 id — 与 companion `APEIRETH_CONTINUITY_ID` 默认值一致.
pub const DEFAULT_SUBJECT: &str = "companion-main";

/// limit 上限 (防一次拉爆; 面板展示无需更大).
const MAX_LIMIT: usize = 200;

fn clamp_limit(n: Option<usize>, default: usize) -> usize {
    n.unwrap_or(default).min(MAX_LIMIT)
}

/// 面板只读路由 — 挂到 `/v1/panel` 下 (nest 由调用方决定).
///
/// 7 端点:
/// - `GET /sessions`                    会话列表 (含每会话 episode 数)
/// - `GET /sessions/:id/timeline`       会话时间线 (episodes)
/// - `GET /memory/streams`              6 历史流查询 (memory streams)
/// - `GET /memory/episodes`             记忆条目列表 + 子串搜索
/// - `GET /graph`                       图谱事实/链接 (factg-*/link-*)
/// - `GET /approvals`                   授权请求 (apreq-*, chain 最新 rev)
/// - `GET /audit`                       审计留痕 (action_stream / 按工具过滤)
pub fn panel_router(store: Arc<SqliteMemoryStore>) -> Router {
    Router::new()
        .route("/sessions", get(panel_sessions))
        .route("/sessions/:id/timeline", get(panel_session_timeline))
        .route("/memory/streams", get(panel_memory_streams))
        .route("/memory/episodes", get(panel_memory_episodes))
        .route("/graph", get(panel_graph))
        .route("/approvals", get(panel_approvals))
        .route("/audit", get(panel_audit))
        .with_state(store)
}

// ============================================================
// 1. 会话管理 (N2 升级点: OneRing 统一账本就绪后换源)
// ============================================================

async fn panel_sessions(State(store): State<Arc<SqliteMemoryStore>>) -> impl IntoResponse {
    let sessions = match store.list_all_sessions() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("list sessions: {e}")})),
            )
        }
    };
    let mut rows = Vec::with_capacity(sessions.len());
    for s in sessions {
        let episode_count = store.count_by_session(&s.id).unwrap_or(0);
        rows.push(json!({
            "id": s.id,
            "started_at": s.started_at,
            "last_active_at": s.last_active_at,
            "closed_at": s.closed_at,
            "episode_count": episode_count,
        }));
    }
    rows.sort_by(|a, b| {
        b["last_active_at"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["last_active_at"].as_i64().unwrap_or(0))
    });
    (
        StatusCode::OK,
        Json(json!({"count": rows.len(), "sessions": rows})),
    )
}

#[derive(Debug, Deserialize)]
pub struct TimelineParams {
    pub limit: Option<usize>,
}

async fn panel_session_timeline(
    State(store): State<Arc<SqliteMemoryStore>>,
    Path(id): Path<String>,
    Query(p): Query<TimelineParams>,
) -> impl IntoResponse {
    let limit = clamp_limit(p.limit, 100);
    // EpisodeQuery 按 session 过滤 (时间倒序由存储层保证同 recent_episodes 序)
    let q = EpisodeQuery::new().for_session(&id).limit(limit);
    match store.query(&q) {
        Ok(eps) => {
            let rows: Vec<Value> = eps
                .iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "timestamp": e.timestamp,
                        "role": e.role,
                        "content": e.content,
                        "session_id": e.session_id,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({"session_id": id, "count": rows.len(), "episodes": rows})),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("timeline: {e}")})),
        ),
    }
}

// ============================================================
// 2. 记忆浏览 (memory streams + episodes 搜索)
// ============================================================

#[derive(Debug, Deserialize)]
pub struct StreamParams {
    /// thought / proposal / action / relation / evolution / reflection
    pub kind: Option<String>,
    /// 主体 id (默认 companion-main)
    pub subject: Option<String>,
    pub limit: Option<usize>,
    /// 起始时间 (unix seconds)
    pub since: Option<i64>,
}

async fn panel_memory_streams(
    State(store): State<Arc<SqliteMemoryStore>>,
    Query(p): Query<StreamParams>,
) -> impl IntoResponse {
    let kind = p.kind.unwrap_or_else(|| "action".to_string());
    let subject = p.subject.unwrap_or_else(|| DEFAULT_SUBJECT.to_string());
    let limit = clamp_limit(p.limit, 50);
    match StreamDepth::query_by_name(&store, &kind, &subject, limit, p.since) {
        Ok(entries) => {
            let rows: Vec<Value> = entries
                .iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "subject_id": e.subject_id,
                        "session_id": e.session_id,
                        "created_at": e.created_at,
                        "payload": e.payload,
                        "source": e.source,
                        "tags": e.tags,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({
                    "kind": kind,
                    "subject": subject,
                    "count": rows.len(),
                    "entries": rows,
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("streams: {e}")})),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct EpisodeParams {
    pub session: Option<String>,
    /// 内容子串过滤 (大小写不敏感)
    pub q: Option<String>,
    pub role: Option<String>,
    pub limit: Option<usize>,
}

async fn panel_memory_episodes(
    State(store): State<Arc<SqliteMemoryStore>>,
    Query(p): Query<EpisodeParams>,
) -> impl IntoResponse {
    let limit = clamp_limit(p.limit, 50);
    // 带搜索时放大拉取窗口再过滤 (ponytail: 简单可靠, 上限 MAX_LIMIT*5)
    let fetch = if p.q.is_some() {
        (limit * 5).min(MAX_LIMIT * 5)
    } else {
        limit
    };
    let mut q = EpisodeQuery::new().limit(fetch);
    if let Some(s) = &p.session {
        q = q.for_session(s);
    }
    if let Some(r) = &p.role {
        q = q.with_role(r);
    }
    let eps = match store.query(&q) {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("episodes: {e}")})),
            )
        }
    };
    let needle = p.q.map(|s| s.to_lowercase());
    let rows: Vec<Value> = eps
        .iter()
        .filter(|e| {
            needle
                .as_ref()
                .map_or(true, |n| e.content.to_lowercase().contains(n))
        })
        .take(limit)
        .map(|e| {
            json!({
                "id": e.id,
                "timestamp": e.timestamp,
                "role": e.role,
                "content": e.content,
                "session_id": e.session_id,
            })
        })
        .collect();
    (
        StatusCode::OK,
        Json(json!({"count": rows.len(), "episodes": rows})),
    )
}

// ============================================================
// 3. 图谱可视化 (factg-*/link-* 直读; 升级点: GraphBackend 结构化查询)
// ============================================================

#[derive(Debug, Deserialize)]
pub struct GraphParams {
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub object: Option<String>,
    pub limit: Option<usize>,
}

/// 从 episodes 读前缀 id 的 JSON 条目 (跟 companion SqliteGraphBackend 同持久形态).
fn episodes_by_prefix(store: &SqliteMemoryStore, prefix: &str) -> Vec<Value> {
    store
        .recent_episodes("me", 500)
        .unwrap_or_default()
        .iter()
        .filter(|e| e.id.starts_with(prefix))
        .filter_map(|e| serde_json::from_str::<Value>(&e.content).ok())
        .collect()
}

async fn panel_graph(
    State(store): State<Arc<SqliteMemoryStore>>,
    Query(p): Query<GraphParams>,
) -> impl IntoResponse {
    let limit = clamp_limit(p.limit, 100);
    let match_field = |v: &Value, field: &str, want: &Option<String>| -> bool {
        want.as_ref().map_or(true, |w| {
            v.get(field)
                .and_then(|x| x.as_str())
                .map_or(false, |s| s.contains(w.as_str()))
        })
    };
    let facts: Vec<Value> = episodes_by_prefix(&store, "factg-")
        .into_iter()
        .filter(|f| {
            match_field(f, "subject", &p.subject)
                && match_field(f, "predicate", &p.predicate)
                && match_field(f, "object", &p.object)
        })
        .take(limit)
        .collect();
    let links: Vec<Value> = episodes_by_prefix(&store, "link-")
        .into_iter()
        .take(limit)
        .collect();
    (
        StatusCode::OK,
        Json(json!({
            "facts_count": facts.len(),
            "links_count": links.len(),
            "facts": facts,
            "links": links,
        })),
    )
}

// ============================================================
// 4. 授权中心 (apreq-* 只读; 批准走已有 /v1/apeireth/grant)
// ============================================================

#[derive(Debug, Deserialize)]
pub struct ApprovalParams {
    /// pending / approved / expired; 缺省 = 全部
    pub status: Option<String>,
}

async fn panel_approvals(
    State(store): State<Arc<SqliteMemoryStore>>,
    Query(p): Query<ApprovalParams>,
) -> impl IntoResponse {
    // 去重语义对齐 companion::approval_requests::list: 按 chain 取最新 rev, 再状态过滤.
    let mut by_chain: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    for e in store
        .recent_episodes("me", 500)
        .unwrap_or_default()
        .iter()
        .filter(|e| e.id.starts_with("apreq-"))
        .filter_map(|e| serde_json::from_str::<Value>(&e.content).ok())
    {
        let chain = e
            .get("chain")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let rev = e.get("rev").and_then(|v| v.as_u64()).unwrap_or(0);
        match by_chain.get(&chain) {
            Some(existing) if existing.get("rev").and_then(|v| v.as_u64()).unwrap_or(0) > rev => {}
            _ => {
                by_chain.insert(chain, e);
            }
        }
    }
    let mut rows: Vec<Value> = by_chain
        .into_values()
        .filter(|r| {
            p.status.as_ref().map_or(true, |s| {
                r.get("status").and_then(|v| v.as_str()) == Some(s.as_str())
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        b["created_at"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["created_at"].as_i64().unwrap_or(0))
    });
    (
        StatusCode::OK,
        Json(json!({"count": rows.len(), "requests": rows})),
    )
}

// ============================================================
// 5. 审计视图 (action_stream 留痕 / 按工具过滤)
// ============================================================

#[derive(Debug, Deserialize)]
pub struct AuditParams {
    /// 按工具名过滤 (缺省 = 全量最近)
    pub tool: Option<String>,
    pub limit: Option<usize>,
}

async fn panel_audit(
    State(store): State<Arc<SqliteMemoryStore>>,
    Query(p): Query<AuditParams>,
) -> impl IntoResponse {
    let limit = clamp_limit(p.limit, 50);
    let records_store = apeireth_tool_runtime::record::RecordStore::new(Arc::clone(&store));
    let records: Vec<apeireth_tool_runtime::record::ToolCallRecord> = match &p.tool {
        Some(t) if !t.is_empty() => match records_store.list_for_tool(t) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("audit list_for_tool: {e}")})),
                )
            }
        },
        _ => {
            // 无过滤: action_stream 最近 limit 条 (与 companion audit_log 工具同路径)
            let conn = match store.conn() {
                Ok(c) => c,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("memory conn: {e}")})),
                    )
                }
            };
            let stream = ActionStream::new(&conn);
            match stream.list_recent(limit, false) {
                Ok(entries) => entries
                    .into_iter()
                    .filter_map(|e| {
                        serde_json::from_value::<apeireth_tool_runtime::record::ToolCallRecord>(
                            e.payload,
                        )
                        .ok()
                    })
                    .collect(),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("list action_stream: {e}")})),
                    )
                }
            }
        }
    };
    // 脱敏语义与 companion audit_log 一致: masked 记录不还原参数.
    let rows: Vec<Value> = records
        .iter()
        .rev()
        .take(limit)
        .map(|r| {
            let mut v = serde_json::to_value(r).unwrap_or_default();
            if r.masked {
                v["call_content"] = json!("[masked by audit] (隐私已脱敏)");
            }
            v
        })
        .collect();
    (
        StatusCode::OK,
        Json(json!({"count": rows.len(), "records": rows})),
    )
}

// ============================================================
// 测试 (in-memory store + tower oneshot)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::{Episode, Session};
    use apeireth_memory::StreamKind;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn seeded_store() -> Arc<SqliteMemoryStore> {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        // 会话 + episodes
        store
            .upsert_session(&Session {
                id: "s-1".into(),
                started_at: 100,
                last_active_at: 200,
            })
            .unwrap();
        store
            .put_episode(&Episode {
                id: "ep-1".into(),
                timestamp: 150,
                role: "user".into(),
                content: "你好, 记住我喜欢咖啡".into(),
                session_id: "s-1".into(),
            })
            .unwrap();
        store
            .put_episode(&Episode {
                id: "ep-2".into(),
                timestamp: 160,
                role: "assistant".into(),
                content: "好的, 已记住咖啡偏好".into(),
                session_id: "s-1".into(),
            })
            .unwrap();
        // 图事实 + 图链接 (companion SqliteGraphBackend 同形态)
        let fact = json!({"id":"factg-1","subject":"主人","predicate":"喜欢","object":"咖啡","importance":80});
        store
            .put_episode(&Episode {
                id: "factg-1".into(),
                timestamp: 170,
                role: "assistant".into(),
                content: fact.to_string(),
                session_id: "me".into(),
            })
            .unwrap();
        store
            .put_episode(&Episode {
                id: "link-1".into(),
                timestamp: 171,
                role: "assistant".into(),
                content: json!({"id":"link-1","from":"factg-1","to":"ep-1","weight":0.9})
                    .to_string(),
                session_id: "me".into(),
            })
            .unwrap();
        // 授权请求 v1 + v2 (同 chain 取最新 rev)
        store
            .put_episode(&Episode {
                id: "apreq-1".into(),
                timestamp: 180,
                role: "assistant".into(),
                content: json!({"id":"apreq-1","chain":"apreq-1","rev":1,"tool":"ShellExec","args_preview":"{\"cmd\":\"dir\"}","reason":"需要主人批准","status":"pending","created_at":180,"updated_at":180}).to_string(),
                session_id: "me".into(),
            })
            .unwrap();
        store
            .put_episode(&Episode {
                id: "apreq-2".into(),
                timestamp: 190,
                role: "assistant".into(),
                content: json!({"id":"apreq-2","chain":"apreq-1","rev":2,"tool":"ShellExec","args_preview":"{\"cmd\":\"dir\"}","reason":"需要主人批准","status":"approved","created_at":180,"updated_at":190}).to_string(),
                session_id: "me".into(),
            })
            .unwrap();
        // 审计: action_stream 一条 ToolCallRecord (subject_id 约定: tool_call:{工具名})
        let rec = json!({
            "id": "call-1", "tool_name": "WebSearch", "started_at_ms": 1000,
            "finished_at_ms": 1500, "duration_ms": 500, "status": "success",
            "success": true, "call_content": {"query": "rust axum"}, "masked": false
        });
        let entry = apeireth_memory::HistoryEntry {
            id: "act-1".into(),
            subject_id: "tool_call:WebSearch".into(),
            subject_rev: 1,
            session_id: None,
            created_at: 200,
            payload: rec,
            source: "ai_generated".into(),
            tags: vec![],
            tombstoned_at: None,
        };
        StreamDepth::insert(&store, StreamKind::Action, &entry).unwrap();
        store
    }

    async fn get_json(store: Arc<SqliteMemoryStore>, path: &str) -> (StatusCode, Value) {
        let app = panel_router(store);
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(path)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn panel_sessions_lists_with_counts() {
        let (status, j) = get_json(seeded_store(), "/sessions").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(j["count"], 1);
        assert_eq!(j["sessions"][0]["id"], "s-1");
        assert_eq!(j["sessions"][0]["episode_count"], 2);
    }

    #[tokio::test]
    async fn panel_timeline_returns_session_episodes() {
        let (status, j) = get_json(seeded_store(), "/sessions/s-1/timeline?limit=10").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(j["count"], 2);
    }

    #[tokio::test]
    async fn panel_memory_streams_queries_action_stream() {
        // action_stream 的审计条目 subject_id 约定为 tool_call:{工具名}
        let (status, j) = get_json(
            seeded_store(),
            "/memory/streams?kind=action&subject=tool_call:WebSearch",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(j["count"], 1);
        assert_eq!(j["entries"][0]["payload"]["tool_name"], "WebSearch");
    }

    #[tokio::test]
    async fn panel_memory_streams_bad_kind_400() {
        let (status, _) = get_json(seeded_store(), "/memory/streams?kind=nope").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn panel_memory_episodes_search_filters() {
        let (status, j) = get_json(seeded_store(), "/memory/episodes?q=咖啡&session=s-1").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(j["count"], 2);
        let (status2, j2) = get_json(seeded_store(), "/memory/episodes?q=不存在").await;
        assert_eq!(status2, StatusCode::OK);
        assert_eq!(j2["count"], 0);
    }

    #[tokio::test]
    async fn panel_graph_returns_facts_and_links() {
        let (status, j) = get_json(seeded_store(), "/graph").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(j["facts_count"], 1);
        assert_eq!(j["links_count"], 1);
        assert_eq!(j["facts"][0]["subject"], "主人");
        // subject 过滤生效
        let (_, j2) = get_json(seeded_store(), "/graph?subject=不存在").await;
        assert_eq!(j2["facts_count"], 0);
    }

    #[tokio::test]
    async fn panel_approvals_dedup_chain_latest_rev() {
        let (status, j) = get_json(seeded_store(), "/approvals").await;
        assert_eq!(status, StatusCode::OK);
        // 同 chain 只留最新 rev (rev2 = approved)
        assert_eq!(j["count"], 1);
        assert_eq!(j["requests"][0]["status"], "approved");
        // pending 过滤 → 0 条
        let (_, j2) = get_json(seeded_store(), "/approvals?status=pending").await;
        assert_eq!(j2["count"], 0);
    }

    #[tokio::test]
    async fn panel_audit_lists_action_records() {
        let (status, j) = get_json(seeded_store(), "/audit").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(j["count"], 1);
        assert_eq!(j["records"][0]["tool_name"], "WebSearch");
        // 按工具过滤: 命中 + 未命中
        let (_, j2) = get_json(seeded_store(), "/audit?tool=WebSearch").await;
        assert_eq!(j2["count"], 1);
        let (_, j3) = get_json(seeded_store(), "/audit?tool=Nope").await;
        assert_eq!(j3["count"], 0);
    }

    #[tokio::test]
    async fn panel_audit_masks_private_records() {
        let store = seeded_store();
        // 追加一条 masked 记录 (rev 覆盖 action_stream)
        let rec = json!({
            "id": "call-2", "tool_name": "FileOperator", "started_at_ms": 2000,
            "finished_at_ms": 2100, "duration_ms": 100, "status": "success",
            "success": true, "call_content": {"path": "secret.txt"}, "masked": true
        });
        let entry = apeireth_memory::HistoryEntry {
            id: "act-2".into(),
            subject_id: "tool_call:FileOperator".into(),
            subject_rev: 1,
            session_id: None,
            created_at: 210,
            payload: rec,
            source: "ai_generated".into(),
            tags: vec![],
            tombstoned_at: None,
        };
        StreamDepth::insert(&store, StreamKind::Action, &entry).unwrap();
        let (status, j) = get_json(store, "/audit").await;
        assert_eq!(status, StatusCode::OK);
        let masked = j["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "call-2")
            .unwrap()
            .clone();
        assert!(masked["call_content"]
            .as_str()
            .unwrap()
            .contains("masked by audit"));
    }
}
