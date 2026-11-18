//! B3b-1 回合入口闸：先 admit 后 append，拒绝即零写入。
//!
//! 闸插在回合入口最早处（`execute_outcome_locked` 内、任何消息 append 之前）。
//! 绑定不再成立 ⇒ 回合直接失败 ⇒ 本回合零写入：用户消息没落盘、会话 revision
//! 没推进、provider 一次都没被叫。
//!
//! 断言全部对着可观测事实（消息条数 / revision / provider 调用次数 / 错误变体），
//! 不用被测代码自己算出来的值当答案；每条断言都做过反向验伪（见交付说明）。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::agent_contact::{
    Contact, ContactCreateRequest, ContactRegistry, ContractMemoryScope, InMemoryContactRegistry,
    PersonaText,
};
use apeireth_core::kernel::system_clock;
use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::agent_binding::{
    resolve_agent_binding, AgentBinding, AgentBindingRequest,
};
use apeireth_runtime::canonical::workspace::{
    InMemoryWorkspaceRegistry, Workspace, WorkspaceCreateRequest, WorkspacePermissionScope,
    WorkspaceRegistry,
};
use apeireth_runtime::{Runtime, RuntimeError, Session, TurnRequest};
use async_trait::async_trait;

const NOW_MS: u64 = 1_700_000_000_000;
const MODEL: &str = "gate-chat-model";
const REPLY: &str = "gate says hi";

/// 一个只回固定文本的 provider：闸测试只需要「回合真的跑起来了」这一事实。
struct GateChatProvider {
    id: CapabilityId,
    calls: AtomicUsize,
}

impl GateChatProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.gate_chat").unwrap(),
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl ProviderCapability for GateChatProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(NormalizedResponse {
            id: "gate_resp_1".into(),
            model: request.model.clone(),
            content: REPLY.into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct GateChatPlugin {
    manifest: PluginManifest,
    provider: Arc<GateChatProvider>,
}

impl GateChatPlugin {
    fn new(provider: Arc<GateChatProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.gate").unwrap(),
                "1.0.0",
                "Gate chat plugin",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Gate chat provider",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for GateChatPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

/// 会话账本的可观测快照（零写入断言的度量尺）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct LedgerSnapshot {
    messages: usize,
    revision: u64,
}

async fn snapshot(runtime: &Runtime, session_id: &SessionId) -> LedgerSnapshot {
    let session = runtime
        .sessions()
        .load(session_id)
        .await
        .expect("session load succeeds")
        .expect("session exists");
    LedgerSnapshot {
        messages: session.messages.len(),
        revision: session.revision,
    }
}

/// 账本里所有消息的文本（用来钉「用户输入真的落盘了」）。
fn message_texts(session: &Session) -> Vec<String> {
    session
        .messages
        .iter()
        .map(|message| {
            message
                .content
                .iter()
                .map(|part| match part {
                    ContentPart::Text { text } => text.clone(),
                    _ => "[非文本]".to_string(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .collect()
}

fn user_message_count(session: &Session) -> usize {
    session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .count()
}

/// 造一个带绑定的会话（绑定已写进账本）。
fn session_with_binding(binding: AgentBinding) -> Session {
    let clock = system_clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.set_agent_binding(binding, clock.as_ref());
    session
}

/// 造一个空会话（未绑定）。
fn session_without_binding() -> Session {
    let clock = system_clock();
    Session::new(SessionId::new(), clock.as_ref())
}

/// 起一个带 provider 的运行时；注册表可缺席（缺席＝闸 fail-closed 的那档）。
async fn runtime_with(
    contacts: Option<Arc<dyn ContactRegistry>>,
    workspaces: Option<Arc<dyn WorkspaceRegistry>>,
) -> (Runtime, Arc<GateChatProvider>) {
    let provider = GateChatProvider::new();
    let plugin = GateChatPlugin::new(Arc::clone(&provider));
    let mut builder = Runtime::builder()
        .with_plugin(plugin)
        .with_default_model(MODEL);
    if let Some(registry) = contacts {
        builder = builder.with_contact_registry(Some(registry));
    }
    if let Some(registry) = workspaces {
        builder = builder.with_workspace_registry(Some(registry));
    }
    let runtime = builder.build().await.expect("runtime builds");
    (runtime, provider)
}

fn create_contact(
    registry: &InMemoryContactRegistry,
    display_name: &str,
    persona: Option<&str>,
) -> Contact {
    registry
        .create(ContactCreateRequest {
            display_name: display_name.to_string(),
            subtitle: None,
            avatar_ref: None,
            persona: persona.map(|raw| PersonaText::new(raw).expect("persona text is valid")),
            memory_scope: ContractMemoryScope::Session,
            now_ms: NOW_MS,
        })
        .expect("create contact")
}

fn create_workspace(registry: &InMemoryWorkspaceRegistry, root: &str) -> Workspace {
    registry
        .create(WorkspaceCreateRequest::managed(
            root,
            root,
            WorkspacePermissionScope::Read,
            NOW_MS,
        ))
        .expect("create workspace")
}

/// 造一份「此刻成立」的绑定（联系人 + 工作区都在册）。
fn resolve_binding(
    contact: &Contact,
    workspace: &Workspace,
    contacts: &InMemoryContactRegistry,
    workspaces: &InMemoryWorkspaceRegistry,
) -> AgentBinding {
    let request = AgentBindingRequest::for_current(contact, workspace);
    resolve_agent_binding(&request, contacts, workspaces, NOW_MS).expect("binding resolves")
}

// ---------------------------------------------------------------------------
// 组 1：绑定合法 ⇒ 回合照常 append（零回归的那一面）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_valid_binding_lets_the_turn_through_and_appends_the_user_message() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    let (runtime, provider) = runtime_with(
        Some(contacts.clone() as Arc<dyn ContactRegistry>),
        Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let response = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect("valid binding must let the turn through");
    assert_eq!(response.text, REPLY, "回合文本应来自 provider");

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(
        after.messages,
        before.messages + 3,
        "合法绑定 ⇒ 空会话先种一条 persona system，再落用户输入与助手回复"
    );
    assert!(
        after.revision > before.revision,
        "合法绑定 ⇒ 会话 revision 推进 ({} -> {})",
        before.revision,
        after.revision
    );
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        1,
        "provider 被叫了一次"
    );

    let stored = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("session load succeeds")
        .expect("session exists");
    assert_eq!(user_message_count(&stored), 1, "账本里恰好一条用户输入");
    assert!(
        message_texts(&stored)
            .iter()
            .any(|text| text == "hello gate"),
        "用户输入原文落在账本里: {:?}",
        message_texts(&stored)
    );
}

// ---------------------------------------------------------------------------
// 组 2：绑定引用已归档联系人 ⇒ 拒绝 ⇒ 零写入
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_archived_contact_binding_is_rejected_without_writing_the_turn() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    contacts
        .archive(&contact.contact_id(), contact.revision(), NOW_MS + 1)
        .expect("archive contact");

    let (runtime, provider) = runtime_with(
        Some(contacts.clone() as Arc<dyn ContactRegistry>),
        Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("archived contact binding must be rejected");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}

// ---------------------------------------------------------------------------
// 组 3：绑定引用不存在的联系人 ⇒ 拒绝 ⇒ 零写入
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_missing_contact_binding_is_rejected_without_writing_the_turn() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    // 回合闸读的是另一份空的联系人注册表 ⇒ 绑定里的联系人查不到。
    let (runtime, provider) = runtime_with(
        Some(Arc::new(InMemoryContactRegistry::new()) as Arc<dyn ContactRegistry>),
        Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("missing contact binding must be rejected");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}

// ---------------------------------------------------------------------------
// 组 4：绑定里的人格修订号过期 ⇒ 拒绝 ⇒ 零写入
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_stale_persona_revision_binding_is_rejected_without_writing_the_turn() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    // 人格换了一版 ⇒ persona_revision 前进 ⇒ 绑定里的旧修订号过期。
    contacts
        .update_persona(
            &contact.contact_id(),
            contact.revision(),
            PersonaText::new("你是新守门人。").expect("persona text is valid"),
            NOW_MS + 1,
        )
        .expect("update persona");

    let (runtime, provider) = runtime_with(
        Some(contacts.clone() as Arc<dyn ContactRegistry>),
        Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("stale persona revision binding must be rejected");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}

// ---------------------------------------------------------------------------
// 组 5：工作区三种失效 ⇒ 拒绝 ⇒ 零写入
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_revoked_workspace_binding_is_rejected_without_writing_the_turn() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    workspaces
        .close(&workspace.workspace_id, workspace.revision, NOW_MS + 1)
        .expect("close workspace");

    let (runtime, provider) = runtime_with(
        Some(contacts.clone() as Arc<dyn ContactRegistry>),
        Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("revoked workspace binding must be rejected");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}

#[tokio::test]
async fn a_stale_workspace_revision_binding_is_rejected_without_writing_the_turn() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    workspaces
        .change_permission_scope(
            &workspace.workspace_id,
            workspace.revision,
            WorkspacePermissionScope::Write,
            NOW_MS + 1,
        )
        .expect("change permission scope");

    let (runtime, provider) = runtime_with(
        Some(contacts.clone() as Arc<dyn ContactRegistry>),
        Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("stale workspace revision binding must be rejected");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}

#[tokio::test]
async fn a_missing_workspace_binding_is_rejected_without_writing_the_turn() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    // 回合闸读的是另一份空的工作区注册表 ⇒ 绑定里的工作区查不到。
    let (runtime, provider) = runtime_with(
        Some(contacts.clone() as Arc<dyn ContactRegistry>),
        Some(Arc::new(InMemoryWorkspaceRegistry::new()) as Arc<dyn WorkspaceRegistry>),
    )
    .await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("missing workspace binding must be rejected");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}

// ---------------------------------------------------------------------------
// 组 6：未绑定会话 ⇒ 放行（零回归）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unbound_session_passes_the_gate_even_without_registries() {
    // 注册表完全缺席：未绑定会话不许被牵连（fail-closed 只对已绑定会话生效）。
    let (runtime, provider) = runtime_with(None, None).await;

    let session = session_without_binding();
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let response = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect("unbound session must pass the gate");
    assert_eq!(response.text, REPLY, "回合文本应来自 provider");

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(
        after.messages,
        before.messages + 2,
        "未绑定 ⇒ 回合照常 append (零回归)"
    );
    assert!(
        after.revision > before.revision,
        "未绑定 ⇒ 会话 revision 推进"
    );
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        1,
        "provider 被叫了一次"
    );
}

// ---------------------------------------------------------------------------
// 组 7：已绑定但注册表未装配 ⇒ fail-closed 拒绝 ⇒ 零写入
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_bound_session_without_registries_fails_closed_without_writing() {
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let contact = create_contact(&contacts, "gate-contact", Some("你是守门人。"));
    let workspace = create_workspace(&workspaces, "gate-workspace");
    let binding = resolve_binding(&contact, &workspace, &contacts, &workspaces);

    // 注册表没装配：已绑定会话必须 fail-closed（不回落默认 Agent）。
    let (runtime, provider) = runtime_with(None, None).await;

    let session = session_with_binding(binding);
    let session_id = session.id;
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("session saved");
    let before = snapshot(&runtime, &session_id).await;

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello gate").with_model(MODEL))
        .await
        .expect_err("bound session without registries must fail closed");
    assert!(
        matches!(error, RuntimeError::AgentBindingRejected { .. }),
        "错误应是 AgentBindingRejected, 实际: {error:?}"
    );

    let after = snapshot(&runtime, &session_id).await;
    assert_eq!(after, before, "拒绝 ⇒ 零写入 (消息数与 revision 都不动)");
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "拒绝 ⇒ provider 一次都没被叫"
    );
}
