//! 联系人合同面的真实链路 (B3a 批): 真 `FileContactRegistry` 落盘 + 真宿主绑定档 +
//! 真 CLI 审计落盘 + 真 HTTP 路由。
//!
//! 这里钉五件事 (面形状在 gateway 侧 `panel_contacts_face.rs`, 本文件钉**真数据面**):
//! 1. **真落盘 + 重启可见** —— 建完联系人, 记录真躺在
//!    `<data_dir>/agents/contacts/<id>/contact.json`, 换一条链 (新装配) 读得到;
//! 2. **真乐观并发** —— 陈旧修订号 = 409 `contact_conflict`, 联系人逐字段不变;
//! 3. **归档是软删除** —— 归档后默认列表不含、`include_archived=true` 才含、单体仍可读
//!    且 `status=archived`; 再改 = 403 `contact_archived` (终态不可复活);
//! 4. **绑定只存 id + 修订号** —— 绑定档里没有工作区快照、没有根路径; 绑定声明必须
//!    过工作区事实校验 (不存在/已吊销/修订号不符 = 409, 字段零变化);
//! 5. **可检索审计** —— 建/改/归档/被拒都落 `gateway.contact_action` 审计行,
//!    `GET /v1/panel/audit` 查得到, 详情里没有绝对路径。
//!
//! 全程用临时目录, **不碰真实 `~/.apeireth`**。

use std::sync::{Arc, Mutex};

use apeireth_cli::contact_panel::CliContactPanel;
use apeireth_cli::gateway_panels::{bind_managed_workspace, CliPanelData, CliWorkspacePanel};
use apeireth_core::kernel::{HistoryEntry, StreamKind};
use apeireth_core::Episode;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, AuditQuery,
    ContactPanel, GatewayServices, GatewayTokenGate, WorkspacePanel,
};
use apeireth_governance::{Permission, PermissionPolicy};
use apeireth_plugin::memory_backend::{BackendKind, CapabilityResult, MemoryBackend};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// 内存记忆后端 (只为把真 `CliPanelData` 装起来: 审计落盘是真链路的一部分)。
struct FakeMemory {
    episodes: Mutex<Vec<Episode>>,
}

impl FakeMemory {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            episodes: Mutex::new(Vec::new()),
        })
    }
}

impl MemoryBackend for FakeMemory {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::InMemory
    }

    fn put_episode(&self, ep: &Episode) -> CapabilityResult<()> {
        self.episodes.lock().expect("锁").push(ep.clone());
        Ok(())
    }

    fn get_episode(&self, id: &str) -> CapabilityResult<Option<Episode>> {
        Ok(self
            .episodes
            .lock()
            .expect("锁")
            .iter()
            .find(|episode| episode.id == id)
            .cloned())
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> CapabilityResult<Vec<Episode>> {
        let mut episodes: Vec<Episode> = self
            .episodes
            .lock()
            .expect("锁")
            .iter()
            .filter(|episode| episode.session_id == session_id)
            .cloned()
            .collect();
        episodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        episodes.truncate(n);
        Ok(episodes)
    }

    fn append_stream(&self, _kind: StreamKind, _entry: HistoryEntry) -> CapabilityResult<()> {
        Ok(())
    }

    fn list_stream(
        &self,
        _kind: StreamKind,
        _session_id: &str,
        _n: usize,
    ) -> CapabilityResult<Vec<HistoryEntry>> {
        Ok(Vec::new())
    }
}

fn test_policy() -> Arc<Mutex<PermissionPolicy>> {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    Arc::new(Mutex::new(policy))
}

/// 装一条真链路: 真工作区注册表 (受管根) + 真联系人注册表 (落盘) + 真 CLI 审计 + 真网关路由。
async fn router_over(root: std::path::PathBuf, data_dir: std::path::PathBuf) -> axum::Router {
    let panel = Arc::new(CliPanelData::new(
        Arc::new(InMemorySessionStore::new()),
        FakeMemory::new(),
        test_policy(),
        true,
        data_dir.clone(),
    ));
    let workspaces = Arc::new(CliWorkspacePanel::new(root));
    bind_managed_workspace(&workspaces, "默认工作区").expect("绑定宿主受管根");
    let contacts = Arc::new(
        CliContactPanel::new_persistent(data_dir).expect("打开联系人注册表 (fail-closed)"),
    );
    let services = GatewayServices {
        workspaces: Some(workspaces as Arc<dyn WorkspacePanel>),
        contacts: Some(contacts as Arc<dyn ContactPanel>),
        audit: Some(panel.clone() as Arc<dyn AuditQuery>),
        audit_commands: Some(panel as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::UnprotectedDefault;
    canonical_router_with_state(state)
}

async fn call(
    router: axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(match body {
            Some(body) => Body::from(body.to_string()),
            None => Body::empty(),
        })
        .expect("request");
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    let response = router.oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

/// 建一个联系人 (真落盘), 返回它的 DTO。
async fn create_contact(router: axum::Router, display_name: &str) -> serde_json::Value {
    let (status, body) = call(
        router,
        "POST",
        "/v1/agents/contacts",
        Some(serde_json::json!({"display_name": display_name})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    body["contact"].clone()
}

/// 1. 真落盘 + 重启可见 + 读面无路径。
#[tokio::test]
async fn contact_record_is_persisted_and_survives_a_new_wiring() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir.clone()).await;

    let contact = create_contact(router.clone(), "校对发布清单").await;
    let contact_id = contact["contact_id"]
        .as_str()
        .expect("contact_id")
        .to_string();
    assert!(!contact_id.is_empty(), "身份必须由后端铸造: {contact}");
    assert_eq!(contact["status"], "draft", "新联系人先是草稿: {contact}");
    assert_eq!(
        contact["memory_scope"], "none",
        "未声明作用域不得乱写: {contact}"
    );
    assert_eq!(contact["workspace_binding"], serde_json::Value::Null);

    // 真躺在磁盘上。
    let record = data_dir
        .join("agents")
        .join("contacts")
        .join(&contact_id)
        .join("contact.json");
    assert!(
        record.exists(),
        "联系人记录必须真落盘: {}",
        record.display()
    );
    let text = std::fs::read_to_string(&record).expect("record");
    assert!(text.contains(&contact_id), "落盘记录必须含身份: {text}");

    // 读面不得泄露宿主存储位置。
    let text = contact.to_string();
    assert!(
        !text.contains(":\\") && !text.contains("://"),
        "读面不得含宿主路径: {text}"
    );

    // 换一条链 (新装配 = 重启) 读得到同一个人。
    let router = router_over(managed_root.clone(), data_dir.clone()).await;
    let (status, body) = call(router.clone(), "GET", "/v1/agents/contacts", None).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["count"], 1, "重启后联系人必须还在: {body}");
    assert_eq!(body["contacts"][0]["contact_id"], contact_id.as_str());
}

/// 2. 真乐观并发: 陈旧修订号 = 409, 联系人逐字段不变。
#[tokio::test]
async fn stale_revision_conflicts_and_leaves_the_contact_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir.clone()).await;

    let contact = create_contact(router.clone(), "校对发布清单").await;
    let contact_id = contact["contact_id"]
        .as_str()
        .expect("contact_id")
        .to_string();
    let revision = contact["revision"].as_u64().expect("revision");

    // 缺修订号 ⇒ 400。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({"persona": "盯住发布清单的每一行"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    assert_eq!(body["error"]["code"], "invalid_request");

    // 陈旧修订号 ⇒ 409 + details。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": revision + 7,
            "persona": "盯住发布清单的每一行",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");
    assert_eq!(body["error"]["code"], "contact_conflict");
    assert_eq!(body["error"]["details"]["expected_revision"], revision + 7);
    assert_eq!(body["error"]["details"]["actual_revision"], revision);

    // 冲突后逐字段不变。
    let (status, after) = call(
        router.clone(),
        "GET",
        &format!("/v1/agents/contacts/{contact_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        after["contact"], contact,
        "冲突后联系人必须逐字段不变: after={after} before={contact}"
    );

    // 正确修订号 ⇒ 放行, 且修订号推进。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": revision,
            "persona": "盯住发布清单的每一行",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(
        body["contact"]["persona"], "盯住发布清单的每一行",
        "人设必须落进去: {body}"
    );
    assert_eq!(
        body["contact"]["persona_revision"], 1,
        "人设版本由后端递增: {body}"
    );
    assert!(
        body["contact"]["revision"].as_u64().expect("revision") > revision,
        "写面改动必须推进修订号: {body}"
    );
}

/// 3. 归档是软删除: 默认列表不含、`include_archived=true` 才含、单体仍可读、终态不可复活。
#[tokio::test]
async fn contact_archive_is_a_tombstone_that_cannot_come_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir.clone()).await;

    let contact = create_contact(router.clone(), "校对发布清单").await;
    let contact_id = contact["contact_id"]
        .as_str()
        .expect("contact_id")
        .to_string();
    let revision = contact["revision"].as_u64().expect("revision");

    // 归档 (带正确修订号)。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/agents/contacts/{contact_id}/archive"),
        Some(serde_json::json!({"expected_revision": revision})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["contact"]["status"], "archived");

    // 默认列表不含墓碑。
    let (status, body) = call(router.clone(), "GET", "/v1/agents/contacts", None).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["count"], 0, "默认列表不得含已归档: {body}");

    // 显式带上才含。
    let (status, body) = call(
        router.clone(),
        "GET",
        "/v1/agents/contacts?include_archived=true",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["count"], 1);
    assert_eq!(body["contacts"][0]["status"], "archived");

    // 单体仍可读 (墓碑是事实, 不是消失)。
    let (status, body) = call(
        router.clone(),
        "GET",
        &format!("/v1/agents/contacts/{contact_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["contact"]["status"], "archived");

    // 再改 ⇒ 403 终态。
    let archived_revision = body["contact"]["revision"].as_u64().expect("revision");
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": archived_revision,
            "persona": "想复活",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    assert_eq!(body["error"]["code"], "contact_archived");

    // 终态: 磁盘上仍是墓碑, 字段没变。
    let (status, body) = call(
        router.clone(),
        "GET",
        &format!("/v1/agents/contacts/{contact_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["contact"]["status"], "archived");
    assert_eq!(body["contact"]["revision"], archived_revision);
    assert_eq!(body["contact"]["persona"], serde_json::Value::Null);
}

/// 4. 绑定: 必须过工作区事实校验; 绑定档只存 id + 修订号 (无快照、无根路径)。
#[tokio::test]
async fn contact_binding_is_validated_and_stored_without_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir.clone()).await;

    // 宿主受管根 = 一个真工作区。
    let (status, body) = call(router.clone(), "GET", "/v1/panel/workspaces", None).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let workspace = &body["workspaces"][0];
    let workspace_id = workspace["workspace_id"]
        .as_str()
        .expect("workspace_id")
        .to_string();
    let workspace_revision = workspace["revision"].as_u64().expect("revision");

    let contact = create_contact(router.clone(), "校对发布清单").await;
    let contact_id = contact["contact_id"]
        .as_str()
        .expect("contact_id")
        .to_string();
    let revision = contact["revision"].as_u64().expect("revision");

    // 修订号不匹配 ⇒ 409 绑定冲突, 字段零变化。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": revision,
            "workspace_binding": {"workspace_id": workspace_id, "revision": workspace_revision + 3},
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");
    assert_eq!(body["error"]["code"], "contact_binding_conflict");
    assert_eq!(
        body["error"]["details"]["reason"],
        "workspace_revision_mismatch"
    );

    let (status, after) = call(
        router.clone(),
        "GET",
        &format!("/v1/agents/contacts/{contact_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        after["contact"], contact,
        "绑定冲突后联系人必须逐字段不变: after={after}"
    );

    // 合法绑定 ⇒ 放行, DTO 回显 id + 修订号。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": revision,
            "workspace_binding": {"workspace_id": workspace_id, "revision": workspace_revision},
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(
        body["contact"]["workspace_binding"]["workspace_id"],
        workspace_id.as_str()
    );
    assert_eq!(
        body["contact"]["workspace_binding"]["revision"],
        workspace_revision
    );

    // 绑定档真落盘, 且**只有** id + 修订号 (没有工作区快照、没有根路径)。
    let binding_file = data_dir.join("agents").join("contact-bindings.json");
    assert!(
        binding_file.exists(),
        "绑定档必须真落盘: {}",
        binding_file.display()
    );
    let text = std::fs::read_to_string(&binding_file).expect("binding");
    assert!(
        text.contains(&workspace_id),
        "绑定档必须含工作区身份: {text}"
    );
    assert!(
        !text.contains("root_display") && !text.contains("root_kind"),
        "绑定档不得存工作区快照: {text}"
    );
    assert!(
        !text.contains(&managed_root.display().to_string()),
        "绑定档不得含绝对根路径: {text}"
    );

    // 重启后绑定还在 (绑定档是真源)。
    let router = router_over(managed_root.clone(), data_dir.clone()).await;
    let (status, body) = call(
        router.clone(),
        "GET",
        &format!("/v1/agents/contacts/{contact_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(
        body["contact"]["workspace_binding"]["workspace_id"],
        workspace_id.as_str()
    );

    // 显式 `null` = 解绑 (与「字段缺失 = 不动」是两态, 不许混)。
    let current_revision = body["contact"]["revision"].as_u64().expect("revision");
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": current_revision,
            "workspace_binding": serde_json::Value::Null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(
        body["contact"]["workspace_binding"],
        serde_json::Value::Null,
        "显式 null 必须解绑: {body}"
    );
    let text = std::fs::read_to_string(&binding_file).expect("binding");
    assert!(
        !text.contains(&workspace_id),
        "解绑后档里不该再有这条绑定: {text}"
    );
}

/// 5. 可检索审计: 建/改/归档/被拒都落行, 详情里没有绝对路径。
#[tokio::test]
async fn contact_actions_are_retrievable_in_the_audit_face() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir.clone()).await;

    let contact = create_contact(router.clone(), "校对发布清单").await;
    let contact_id = contact["contact_id"]
        .as_str()
        .expect("contact_id")
        .to_string();
    let revision = contact["revision"].as_u64().expect("revision");

    // 改 (放行)。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({
            "expected_revision": revision,
            "persona": "盯住发布清单的每一行",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let updated_revision = body["contact"]["revision"].as_u64().expect("revision");

    // 过期修订号 (被拒, 409) —— 必须在归档**之前**试: 归档是终态, 之后一律 403。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({"expected_revision": 999, "persona": "正文"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");

    // 归档 (放行)。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/agents/contacts/{contact_id}/archive"),
        Some(serde_json::json!({"expected_revision": updated_revision})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");

    // 归档后再改 (被拒, 403 终态)。
    let (status, body) = call(
        router.clone(),
        "PATCH",
        &format!("/v1/agents/contacts/{contact_id}"),
        Some(serde_json::json!({"expected_revision": 999, "persona": "正文"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    assert_eq!(body["error"]["code"], "contact_archived");

    let (status, audit) = call(router.clone(), "GET", "/v1/panel/audit", None).await;
    assert_eq!(status, StatusCode::OK, "body={audit}");
    let rows = audit["events"].as_array().expect("audit rows");
    let contact_rows: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|row| row["event"] == "gateway.contact_action")
        .collect();
    assert!(!contact_rows.is_empty(), "联系人动作必须落审计行: {audit}");
    let details: Vec<String> = contact_rows
        .iter()
        .map(|row| row["detail"].as_str().unwrap_or_default().to_string())
        .collect();
    for action in [
        "action=contact.create",
        "action=contact.update",
        "action=contact.archive",
        "action=contact.deny",
    ] {
        assert!(
            details.iter().any(|detail| detail.contains(action)),
            "动作必须可检索 ({action}): {details:?}"
        );
    }
    for detail in &details {
        assert!(
            !detail.contains(&managed_root.display().to_string())
                && !detail.contains(&data_dir.display().to_string()),
            "审计详情不得含绝对路径: {detail}"
        );
    }

    // 审计同时落了 JSONL (真落盘)。
    let audit_file = data_dir.join("daemon-audit.jsonl");
    let text = std::fs::read_to_string(&audit_file).expect("audit jsonl");
    assert!(
        text.contains("gateway.contact_action"),
        "审计必须落盘: {text}"
    );
}
