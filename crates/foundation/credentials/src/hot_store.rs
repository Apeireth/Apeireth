//! 真热更凭据解析层: 运行时凭据库 + 分层 resolver (凭据按请求现解析)。
//!
//! **位置**: `apeireth-credentials` (foundation), 与 [`KeyringCredentialResolver`]
//! 同位 —— 都实现 `apeireth_plugin::CredentialResolver` trait, 供 runtime 注入。
//!
//! **动机**: provider capability 每次 `complete` 都经 `CredentialResolver::resolve`
//! 现取 key (见 `apeireth-provider` 的 `resolve_key`), 所以"下一请求生效"的关键
//! 在于 resolver 的读取源要能看到 `/v1/admin/config` 的运行时写入。本模块提供:
//!
//! - [`HotCredentialStore`] — admin 运行时凭据库 (逻辑凭据名 → 值)。只存
//!   脱敏 [`Secret`], `Debug` 不出明文; 读取按调用现查, 不缓存进任何请求路径对象。
//! - [`LayeredCredentialResolver`] — 分层解析: 主层 (运行时凭据库) 未命中时
//!   回落到次层 (keyring / env 启动值)。优先级 = **admin 运行时凭据库 >
//!   keyring/env 启动值**, 与 `/v1/admin/config` "admin 补丁覆盖启动值"的
//!   契约一致。
//!
//! **凭据值生命周期**: 本 store 是凭据的运行时存放点 (与 OS keyring 同角色),
//! 值不进入任何客户端/能力对象; 每次请求现解析出的 `Secret` 只活过该次请求
//! (请求构造 → 发出 → drop), 无"构造时烧进内存"的步骤。
//!
//! **0 触碰 LOCKED**: 不改 `apeireth-plugin` 的 trait 边界, 不改 provider 实现。

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use apeireth_plugin::{CredentialResolver, Secret};

/// admin 运行时凭据库: 逻辑凭据名 → 运行时值 (脱敏存储)。
///
/// 写侧 = `/v1/admin/config` 的凭据写入端口 (gateway `CredentialWriter` 接线);
/// 读侧 = [`LayeredCredentialResolver`] 主层。每次 `resolve` 现查 map,
/// 所以一次写入对**下一次**请求立即生效, 无失效窗口、无缓存清理问题。
#[derive(Default)]
pub struct HotCredentialStore {
    entries: RwLock<BTreeMap<String, Secret>>,
}

impl HotCredentialStore {
    /// 空的运行时凭据库。
    pub fn new() -> Self {
        Self::default()
    }

    /// 写入 (或覆盖) 一个逻辑凭据名的运行时值。
    ///
    /// 空值等同 [`HotCredentialStore::remove`]: 空字符串不能冒充已配置凭据
    /// (与 `EnvCredentialResolver` 对空 env 的处理一致)。
    pub fn set(&self, name: &str, value: impl Into<String>) {
        let value = value.into();
        let mut entries = self
            .entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if value.is_empty() {
            entries.remove(name);
        } else {
            entries.insert(name.to_string(), Secret::new(value));
        }
    }

    /// 现查运行时值 (未命中 = None)。返回脱敏 [`Secret`] 的克隆。
    pub fn get(&self, name: &str) -> Option<Secret> {
        self.entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(name)
            .cloned()
    }

    /// 移除一个运行时值 (回落到启动值解析)。返回是否确有条目被移除。
    pub fn remove(&self, name: &str) -> bool {
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(name)
            .is_some()
    }

    /// 当前条目数 (仅计数, 无凭据名/值)。
    pub fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl CredentialResolver for HotCredentialStore {
    fn resolve(&self, name: &str) -> Option<Secret> {
        self.get(name)
    }
}

impl fmt::Debug for HotCredentialStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entries = self
            .entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let names: Vec<&str> = entries.keys().map(String::as_str).collect();
        f.debug_struct("HotCredentialStore")
            .field("names", &names)
            .field("values", &"<redacted>")
            .finish()
    }
}

/// 分层 [`CredentialResolver`]: 主层未命中回落次层, 按调用现解析。
///
/// 典型接线 (CLI/gateway 组装根):
/// `LayeredCredentialResolver::new(hot_store, keyring_or_env_resolver)` ——
/// admin 运行时写入的凭据 (主层) 覆盖 keyring/env 启动值 (次层), 且每次
/// `resolve` 都现查两层, 任一层的变更对下一次请求即刻可见。
pub struct LayeredCredentialResolver {
    primary: Arc<dyn CredentialResolver>,
    fallback: Arc<dyn CredentialResolver>,
}

impl LayeredCredentialResolver {
    /// 主层优先、次层兜底。两层都是句柄, 不含明文。
    pub fn new(
        primary: Arc<dyn CredentialResolver>,
        fallback: Arc<dyn CredentialResolver>,
    ) -> Self {
        Self { primary, fallback }
    }
}

impl CredentialResolver for LayeredCredentialResolver {
    fn resolve(&self, name: &str) -> Option<Secret> {
        self.primary
            .resolve(name)
            .or_else(|| self.fallback.resolve(name))
    }
}

impl fmt::Debug for LayeredCredentialResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 两层都是 trait 句柄 (无 Debug 约束), 只印结构, 不印凭据。
        f.debug_struct("LayeredCredentialResolver")
            .field("primary", &"<resolver handle>")
            .field("fallback", &"<resolver handle>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::StaticCredentials;

    #[test]
    fn runtime_override_wins_over_the_startup_layer() {
        let hot = Arc::new(HotCredentialStore::new());
        hot.set("provider.x.api_key", "sk-hot");
        let layered = LayeredCredentialResolver::new(
            hot,
            Arc::new(StaticCredentials::new().with("provider.x.api_key", "sk-startup")),
        );
        let got = layered.resolve("provider.x.api_key").expect("resolved");
        assert_eq!(got.expose(), "sk-hot");
    }

    #[test]
    fn missing_override_falls_back_to_the_startup_layer() {
        let hot = Arc::new(HotCredentialStore::new());
        let layered = LayeredCredentialResolver::new(
            hot,
            Arc::new(StaticCredentials::new().with("provider.x.api_key", "sk-startup")),
        );
        let got = layered.resolve("provider.x.api_key").expect("resolved");
        assert_eq!(got.expose(), "sk-startup");
    }

    #[test]
    fn a_write_is_visible_to_the_next_resolve_without_rebuild() {
        // 真热更核心断言: 同一个 resolver 句柄, 写入后下一次 resolve 现取新值
        // (凭据不被任何长生命周期对象缓存)。
        let hot = Arc::new(HotCredentialStore::new());
        let layered = LayeredCredentialResolver::new(
            Arc::clone(&hot) as Arc<dyn CredentialResolver>,
            Arc::new(StaticCredentials::new().with("provider.x.api_key", "sk-startup")),
        );
        assert_eq!(
            layered.resolve("provider.x.api_key").unwrap().expose(),
            "sk-startup"
        );
        hot.set("provider.x.api_key", "sk-decoy");
        assert_eq!(
            layered.resolve("provider.x.api_key").unwrap().expose(),
            "sk-decoy"
        );
        hot.remove("provider.x.api_key");
        assert_eq!(
            layered.resolve("provider.x.api_key").unwrap().expose(),
            "sk-startup"
        );
    }

    #[test]
    fn both_layers_missing_resolves_to_none() {
        let layered = LayeredCredentialResolver::new(
            Arc::new(HotCredentialStore::new()),
            Arc::new(StaticCredentials::new()),
        );
        assert!(layered.resolve("provider.none.api_key").is_none());
    }

    #[test]
    fn an_empty_value_is_removal_not_a_configured_key() {
        let hot = HotCredentialStore::new();
        hot.set("provider.x.api_key", "sk-hot");
        hot.set("provider.x.api_key", "");
        assert!(hot.get("provider.x.api_key").is_none());
        assert!(hot.is_empty());
    }

    #[test]
    fn debug_never_prints_values() {
        let hot = HotCredentialStore::new();
        hot.set("provider.x.api_key", "sk-super-secret-value");
        let printed = format!("{hot:?}");
        assert!(!printed.contains("sk-super-secret-value"), "{printed}");
        assert!(printed.contains("<redacted>"), "{printed}");
        assert!(printed.contains("provider.x.api_key"), "{printed}");
    }
}
