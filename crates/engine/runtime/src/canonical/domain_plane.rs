//! 用户域会话平面 (W3-① 多户数据隔离·星群数据层): 域解析 + 按域路由会话写闸。
//!
//! 星群架构 (每户独立伙伴实例) 的会话面路由地基:
//!
//! ```text
//! 请求 → ClientIdentity (令牌绑定) → user_id → [`DomainSessionPlane::resolve`]
//!      → UserDomain → [`DomainSessionPlane::sessions`] → 该域的 SessionManager
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
}
