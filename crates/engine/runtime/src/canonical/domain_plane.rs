//! 用户域会话平面 (W3-① 多户数据隔离·星群数据层) + 逐域运行时实例面 (W3-③ 星群正身):
//! 域解析 + 按域路由会话写闸 + 每户伙伴运行时实例与可配对话上下文。
//!
//! 星群架构 (每户独立伙伴实例) 的会话面路由地基:
//!
//! ```text
//! 请求 → ClientIdentity (令牌绑定) → user_id → [`DomainSessionPlane::resolve`]
//!      → UserDomain → [`DomainSessionPlane::sessions`] → 该域的 SessionManager
//! ```
//!
//! W3-③ 每户伙伴实例 (星群正身, 「每户各得其伴」):
//!
//! ```text
//! UserDomain ──▶ [`DomainRuntimePlane::runtime_for`] ──▶ 该域的伙伴 [`Runtime`]
//!      │            (默认域 = 组合根注入的现 Runtime, 零变化)
//!      └───────▶ [`DomainRuntimePlane::context_for`] ──▶ [`DomainRuntimeContext`]
//!                   (域文档 persona_text/model 覆盖, 缺省回落全局)
//! ```
//!
//! - **每域恰一把会话写闸**: 域的 [`SessionManager`] 由组合根注入 (默认用户域
//!   注入的是运行时自己那把 —— 回合与设置写同一闸, W2-③ 承诺不拆);
//! - **无用户 = 默认域** (单户兼容): [`DomainSessionPlane::resolve`] 对未绑定
//!   user_id 的身份回默认用户域;
//! - **越权拒绝即帧**: 点名他域走 [`UserDomain::authorize`] —— 跨域一律
//!   [`UserDomainError::Forbidden`] (目标存在与否同帧, 不泄露他域存在);
//! - **坏域配置拒开**: 解析走注册表严格打开, 坏标记 →
//!   [`UserDomainError::Refused`] (本域真因, 不动他域)。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::user_domain::{
    DomainOp, DomainRegistry, DomainUserId, UserDomain, UserDomainError,
};

use super::runtime::Runtime;
use super::session::SessionManager;

/// 用户域会话平面: 域注册表 + 每域一把会话写闸。
pub struct DomainSessionPlane {
    registry: DomainRegistry,
    sessions: BTreeMap<DomainUserId, Arc<SessionManager>>,
}

impl std::fmt::Debug for DomainSessionPlane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainSessionPlane")
            .field("registry", &self.registry)
            .field("domains", &self.sessions.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl DomainSessionPlane {
    /// 组装平面: 注册表 + 已开的每域会话写闸 (含默认域)。
    ///
    /// 缺席的域在 [`DomainSessionPlane::sessions`] 处被拒 (域数据面未开),
    /// 不静默回落到别的域。
    pub fn new(
        registry: DomainRegistry,
        sessions: BTreeMap<DomainUserId, Arc<SessionManager>>,
    ) -> Self {
        Self { registry, sessions }
    }

    /// 域注册表 (打开/校验/采用语义见 `apeireth_core::user_domain`)。
    pub fn registry(&self) -> &DomainRegistry {
        &self.registry
    }

    /// 本平面已开的域 id 全表。
    pub fn domain_ids(&self) -> Vec<DomainUserId> {
        self.sessions.keys().cloned().collect()
    }

    /// 域解析 (请求 → 身份绑定 → 用户域):
    ///
    /// - `None` (无用户) → 默认用户域 (单户兼容);
    /// - `Some(id)` 且 id 未开域 (域标记不存在) → 默认用户域 (兼容延伸:
    ///   尚未开通专属数据域的绑定用户共享默认域, 与单户形态一致);
    /// - `Some(id)` 且域配置坏 → [`UserDomainError::Refused`] (**坏域配置拒开**,
    ///   不静默回落默认域、不带病服务, 真因给审计);
    /// - `Some("default")` / 已开域的 `Some(id)` → 该域。
    pub fn resolve(&self, bound_user: Option<&str>) -> Result<UserDomain, UserDomainError> {
        let Some(raw) = bound_user else {
            return self.registry.load_default();
        };
        let id = DomainUserId::new(raw).map_err(|error| {
            UserDomainError::Refused(format!("bound user domain refused: {error}"))
        })?;
        if id.as_str() == apeireth_core::user_domain::DEFAULT_DOMAIN_ID {
            return self.registry.load_default();
        }
        match self.registry.load(&id) {
            Ok(domain) => Ok(domain),
            Err(UserDomainError::NotFound(_)) => self.registry.load_default(),
            Err(error) => Err(UserDomainError::Refused(format!(
                "user domain `{id}` refused to open: {error}"
            ))),
        }
    }

    /// 取一个域的会话写闸 (本域句柄; 域数据面未开 → [`UserDomainError::Refused`])。
    pub fn sessions(&self, domain: &UserDomain) -> Result<&Arc<SessionManager>, UserDomainError> {
        self.sessions.get(&domain.user_id).ok_or_else(|| {
            UserDomainError::Refused(format!(
                "user domain `{}` has no open session plane",
                domain.user_id
            ))
        })
    }

    /// 域级授权 (点名他域句柄/资源的判定口): 语义见 [`UserDomain::authorize`]。
    pub fn authorize(
        &self,
        caller: &UserDomain,
        target: &DomainUserId,
        op: DomainOp,
    ) -> Result<(), UserDomainError> {
        caller.authorize(target, op)
    }
}

/// 一个域的对话运行时上下文 (W3-③ 星群「每户各得其伴」的数据面):
/// 模型配置 + 性格/人设文本, **缺省回落全局**。
///
/// 来源是该域的域文档 (`users/<id>/domain.json` 的 `persona_text` / `model`
/// 覆盖字段, StoredDoc 拒开兼容加字段); 两个字段都缺省时, 回合行为与既有
/// 单户形态逐字一致 (默认域零变化)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomainRuntimeContext {
    /// 域级模型覆盖 (缺省 `None` = 回落全局默认模型)。
    pub model: Option<String>,
    /// 域级性格/人设文本 (缺省 `None` = 不注入人设种子, 回落全局行为)。
    pub persona_text: Option<String>,
}

impl DomainRuntimeContext {
    /// 生效模型 (域覆盖优先, 缺省回落全局)。
    pub fn effective_model<'a>(&'a self, global: Option<&'a str>) -> Option<&'a str> {
        self.model.as_deref().or(global)
    }

    /// 生效会话种子 (回合的 system 指令): 域人设在前, 请求自带 system 在后;
    /// 人设缺省 = 请求 system **原样**回落 (零变化), 两者都缺省 = 不注入。
    pub fn seeded_system(&self, request_system: Option<&str>) -> Option<String> {
        match (self.persona_text.as_deref(), request_system) {
            (None, system) => system.map(str::to_string),
            (Some(persona), None) => Some(persona.to_string()),
            (Some(persona), Some(system)) => Some(format!("{persona}\n\n{system}")),
        }
    }
}

impl From<&UserDomain> for DomainRuntimeContext {
    fn from(domain: &UserDomain) -> Self {
        Self {
            model: domain.model.clone(),
            persona_text: domain.persona_text.clone(),
        }
    }
}

/// 逐域运行时实例面 (W3-③ 星群正身): 每域一个**伙伴运行时实例**
/// (该域的对话跑在自己的实例上 —— 会话写闸/记忆/配额/模型解析全按域),
/// 默认域 = 组合根注入的现 [`Runtime`] (**零变化**)。
///
/// 组合根在装配期为每个已开域组装伙伴实例并注册进来; 域的运行时上下文
/// (模型/人设覆盖) 现读域文档 ([`DomainRuntimePlane::context_for`]), 坏域配置
/// 拒开 ([`UserDomainError::Refused`])。域未注册伙伴实例 = 该域拒开
/// (不回落到别的域的实例, 不拿默认域代跑)。
#[derive(Debug)]
pub struct DomainRuntimePlane {
    sessions: Arc<DomainSessionPlane>,
    default: Arc<Runtime>,
    instances: BTreeMap<DomainUserId, Arc<Runtime>>,
}

impl DomainRuntimePlane {
    /// 组装运行时实例面: 会话平面 (域解析/写闸路由) + 默认域现运行时 +
    /// 每域伙伴实例。
    pub fn new(
        sessions: Arc<DomainSessionPlane>,
        default: Arc<Runtime>,
        instances: BTreeMap<DomainUserId, Arc<Runtime>>,
    ) -> Self {
        Self {
            sessions,
            default,
            instances,
        }
    }

    /// 会话平面 (域解析 + 每域写闸)。
    pub fn sessions_plane(&self) -> &Arc<DomainSessionPlane> {
        &self.sessions
    }

    /// 已注册伙伴实例的域 id 全表。
    pub fn instance_ids(&self) -> Vec<DomainUserId> {
        self.instances.keys().cloned().collect()
    }

    /// 已注册伙伴实例全表 (组合根给各实例接事件汇等装配操作用)。
    pub fn instances(&self) -> &BTreeMap<DomainUserId, Arc<Runtime>> {
        &self.instances
    }

    /// 该域的伙伴运行时实例:
    ///
    /// - 默认域 → 组合根注入的现 [`Runtime`] (零变化, 永远同一实例);
    /// - 非默认域且已注册 → 该域自己的伙伴实例;
    /// - 非默认域未注册 → [`UserDomainError::Refused`] (该域拒开, 不回落、
    ///   不拿默认域代跑 —— 代跑会把默认域的会话/记忆/配额面泄给该域)。
    pub fn runtime_for(&self, domain: &UserDomain) -> Result<&Arc<Runtime>, UserDomainError> {
        if domain.is_default() {
            return Ok(&self.default);
        }
        self.instances.get(&domain.user_id).ok_or_else(|| {
            UserDomainError::Refused(format!(
                "user domain `{}` has no open partner runtime instance",
                domain.user_id
            ))
        })
    }

    /// 该域的对话运行时上下文 (域文档现读: persona/model 覆盖, 缺省回落全局)。
    ///
    /// 坏域配置拒开: 域文档此刻打不开/越界 → [`UserDomainError::Refused`]
    /// (本域真因入审计, 不静默回落全局、不动他域)。
    pub fn context_for(
        &self,
        domain: &UserDomain,
    ) -> Result<DomainRuntimeContext, UserDomainError> {
        match self.sessions.registry().load(&domain.user_id) {
            Ok(loaded) => Ok(DomainRuntimeContext::from(&loaded)),
            Err(error) => Err(UserDomainError::Refused(format!(
                "user domain `{}` runtime context refused to open: {error}",
                domain.user_id
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::session::InMemorySessionStore;
    use apeireth_core::kernel::system_clock;
    use apeireth_core::user_domain::{DomainRole, UserDomainError};

    fn manager() -> Arc<SessionManager> {
        Arc::new(SessionManager::new(
            Arc::new(InMemorySessionStore::new()),
            system_clock(),
        ))
    }

    fn plane(root: &std::path::Path) -> (DomainSessionPlane, UserDomain, UserDomain) {
        let registry = DomainRegistry::open(root).unwrap();
        let alice = registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();
        let bob = registry.create("bob", "Bob", DomainRole::Member).unwrap();
        let default = registry.load_default().unwrap();
        let mut sessions = BTreeMap::new();
        sessions.insert(default.user_id.clone(), manager());
        sessions.insert(alice.user_id.clone(), manager());
        sessions.insert(bob.user_id.clone(), manager());
        (DomainSessionPlane::new(registry, sessions), alice, bob)
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-domain-plane-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolution_without_a_bound_user_lands_in_the_default_domain() {
        let root = temp_root("resolve");
        let (plane, alice, _) = plane(&root);
        assert!(plane.resolve(None).unwrap().is_default());
        assert_eq!(plane.resolve(Some("alice")).unwrap().user_id, alice.user_id);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_bound_user_resolves_by_provisioning_state_and_bad_configs_refuse() {
        let root = temp_root("unopen");
        let (plane, _, _) = plane(&root);
        // 未开域的绑定用户 → 默认域 (兼容延伸: 共享默认数据面, 与单户形态一致)。
        assert!(plane.resolve(Some("nobody")).unwrap().is_default());
        // 非法 id → 拒开 (不猜、不回落)。
        let err = plane.resolve(Some("../evil")).unwrap_err();
        assert!(matches!(err, UserDomainError::Refused(_)), "{err}");
        // 域在场但配置坏 → 拒开 (坏域配置拒开, 绝不静默回落默认域)。
        let marker = root.join("users/alice/domain.json");
        std::fs::write(&marker, b"{ not json").unwrap();
        let err = plane.resolve(Some("alice")).unwrap_err();
        assert!(matches!(err, UserDomainError::Refused(_)), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cross_domain_session_handle_requests_are_forbidden_while_own_domain_serves() {
        let root = temp_root("gate");
        let (plane, alice, bob) = plane(&root);
        assert!(plane.sessions(&alice).is_ok());
        assert!(plane.sessions(&bob).is_ok());
        // 点名他域 (即使存在) → Forbidden, 恒定形态。
        assert!(matches!(
            plane.authorize(&alice, &bob.user_id, DomainOp::Data),
            Err(UserDomainError::Forbidden(_))
        ));
        assert!(matches!(
            plane.authorize(&alice, &bob.user_id, DomainOp::Manage),
            Err(UserDomainError::Forbidden(_))
        ));
        assert!(plane
            .authorize(&alice, &alice.user_id, DomainOp::Data)
            .is_ok());
        // 角色门: Member 域管理被拒。
        assert!(matches!(
            plane.authorize(&bob, &bob.user_id, DomainOp::Manage),
            Err(UserDomainError::Forbidden(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_domain_without_an_open_session_plane_is_refused_rather_than_redirected() {
        let root = temp_root("no-plane");
        let registry = DomainRegistry::open(&root).unwrap();
        let carol = registry
            .create("carol", "Carol", DomainRole::Owner)
            .unwrap();
        let default = registry.load_default().unwrap();
        let mut sessions = BTreeMap::new();
        sessions.insert(default.user_id.clone(), manager());
        let plane = DomainSessionPlane::new(registry, sessions);
        match plane.sessions(&carol) {
            Ok(_) => panic!("未开数据面的域不得被放行"),
            Err(err) => assert!(matches!(err, UserDomainError::Refused(_)), "{err}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    // ------------------------------------------------------------------
    // 逐域运行时实例面 (W3-③ 星群正身)
    // ------------------------------------------------------------------

    type RuntimePlaneFixture = (
        DomainRuntimePlane,
        UserDomain,
        UserDomain,
        Arc<crate::canonical::runtime::Runtime>,
        Arc<crate::canonical::runtime::Runtime>,
        Arc<crate::canonical::runtime::Runtime>,
    );

    async fn runtime_plane(root: &std::path::Path) -> RuntimePlaneFixture {
        let registry = DomainRegistry::open(root).unwrap();
        let alice = registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();
        let bob = registry.create("bob", "Bob", DomainRole::Member).unwrap();
        let default = registry.load_default().unwrap();
        let mut sessions = BTreeMap::new();
        sessions.insert(default.user_id.clone(), manager());
        sessions.insert(alice.user_id.clone(), manager());
        sessions.insert(bob.user_id.clone(), manager());
        let sessions = Arc::new(DomainSessionPlane::new(registry, sessions));

        let default_runtime = Arc::new(
            crate::canonical::runtime::Runtime::builder()
                .build()
                .await
                .unwrap(),
        );
        let alice_runtime = Arc::new(
            crate::canonical::runtime::Runtime::builder()
                .build()
                .await
                .unwrap(),
        );
        let bob_runtime = Arc::new(
            crate::canonical::runtime::Runtime::builder()
                .build()
                .await
                .unwrap(),
        );
        let mut instances = BTreeMap::new();
        instances.insert(alice.user_id.clone(), alice_runtime.clone());
        instances.insert(bob.user_id.clone(), bob_runtime.clone());
        let plane = DomainRuntimePlane::new(sessions, default_runtime.clone(), instances);
        (
            plane,
            alice,
            bob,
            default_runtime,
            alice_runtime,
            bob_runtime,
        )
    }

    #[tokio::test]
    async fn the_default_domain_serves_the_injected_runtime_with_global_fallbacks() {
        let root = temp_root("rt-default");
        let (plane, _, _, default_runtime, _, _) = runtime_plane(&root).await;
        let default = plane.sessions_plane().registry().load_default().unwrap();

        // 默认域 = 组合根注入的现运行时 (同一实例, 零变化)。
        let served = plane.runtime_for(&default).unwrap();
        assert!(std::sync::Arc::ptr_eq(served, &default_runtime));

        // 无覆盖的域上下文 = 全局回落 (模型回落全局, 会话种子 = 请求 system 原样)。
        let context = plane.context_for(&default).unwrap();
        assert_eq!(context, DomainRuntimeContext::default());
        assert_eq!(
            context.effective_model(Some("global-model")),
            Some("global-model")
        );
        assert_eq!(context.effective_model(None), None);
        assert_eq!(
            context.seeded_system(Some("caller system")),
            Some("caller system".to_string()),
            "人设缺省时请求 system 必须逐字回落 (默认域零变化)"
        );
        assert_eq!(context.seeded_system(None), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn each_domain_gets_its_own_partner_instance_and_runtime_context() {
        let root = temp_root("rt-partners");
        let (plane, alice, bob, _, alice_runtime, bob_runtime) = runtime_plane(&root).await;
        let registry = plane.sessions_plane().registry().clone();
        registry
            .set_runtime_overrides(
                &alice.user_id,
                Some("Alice 的专属伙伴, 语气温和。".to_string()),
                Some("model-alice".to_string()),
            )
            .unwrap();
        registry
            .set_runtime_overrides(&bob.user_id, None, Some("model-bob".to_string()))
            .unwrap();

        // 实例面: 每域自己的伙伴实例 (互不共用, 也不共用默认域)。
        let alice_served = plane.runtime_for(&alice).unwrap();
        let bob_served = plane.runtime_for(&bob).unwrap();
        assert!(std::sync::Arc::ptr_eq(alice_served, &alice_runtime));
        assert!(std::sync::Arc::ptr_eq(bob_served, &bob_runtime));
        assert!(!std::sync::Arc::ptr_eq(alice_served, bob_served));
        assert_eq!(
            plane.instance_ids(),
            vec![alice.user_id.clone(), bob.user_id.clone()]
        );

        // 上下文面: 每域自己的模型/人设覆盖 (缺省回落全局)。
        let alice_context = plane.context_for(&alice).unwrap();
        assert_eq!(alice_context.model.as_deref(), Some("model-alice"));
        assert_eq!(
            alice_context.persona_text.as_deref(),
            Some("Alice 的专属伙伴, 语气温和。")
        );
        let bob_context = plane.context_for(&bob).unwrap();
        assert_eq!(bob_context.model.as_deref(), Some("model-bob"));
        assert_eq!(bob_context.persona_text, None);
        assert_eq!(
            bob_context.seeded_system(Some("sys")),
            Some("sys".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn one_domains_overrides_never_change_another_domains_context() {
        let root = temp_root("rt-adversarial");
        let (plane, alice, bob, _, _, _) = runtime_plane(&root).await;
        let registry = plane.sessions_plane().registry().clone();
        let default = registry.load_default().unwrap();

        // 对抗: 反复改 alice 的覆盖, bob 与默认域的上下文逐字不变。
        let bob_before = plane.context_for(&bob).unwrap();
        let default_before = plane.context_for(&default).unwrap();
        for (persona, model) in [
            (Some("p1".to_string()), Some("m1".to_string())),
            (Some("p2".to_string()), Some("m2".to_string())),
            (None, Some("m3".to_string())),
        ] {
            registry
                .set_runtime_overrides(&alice.user_id, persona, model)
                .unwrap();
            assert_eq!(
                plane.context_for(&bob).unwrap(),
                bob_before,
                "他域上下文被串改"
            );
            assert_eq!(
                plane.context_for(&default).unwrap(),
                default_before,
                "默认域上下文被串改"
            );
        }
        // 改坏 alice 的域配置 → alice 拒开, bob 照常 (一户坏配置不砖化整面)。
        std::fs::write(registry.layout().domain_file(&alice.user_id), b"{ not json").unwrap();
        let err = plane.context_for(&alice).unwrap_err();
        assert!(matches!(err, UserDomainError::Refused(_)), "{err}");
        assert_eq!(plane.context_for(&bob).unwrap(), bob_before);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn a_domain_without_a_partner_instance_is_refused_not_redirected() {
        let root = temp_root("rt-no-instance");
        let (plane, _, _, default_runtime, _, _) = runtime_plane(&root).await;
        let registry = plane.sessions_plane().registry().clone();
        let carol = registry
            .create("carol", "Carol", DomainRole::Owner)
            .unwrap();

        // 已开域但未注册伙伴实例 → 拒开 (绝不由默认域代跑)。
        match plane.runtime_for(&carol) {
            Ok(_) => panic!("未注册伙伴实例的域不得被放行"),
            Err(err) => assert!(matches!(err, UserDomainError::Refused(_)), "{err}"),
        }
        // 默认域永远是注入的那一个实例。
        let default = registry.load_default().unwrap();
        assert!(std::sync::Arc::ptr_eq(
            plane.runtime_for(&default).unwrap(),
            &default_runtime
        ));
        let _ = std::fs::remove_dir_all(&root);
    }
}
