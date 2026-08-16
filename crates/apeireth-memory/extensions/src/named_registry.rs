//! # MemoryProviderRegistry — 按名称注册/查询的接线注册表
//!
//! "telemetry cache 接线"派工的接线入口 (机制而非补丁, 集成而非分立).
//!
//! ## 与 [`crate::registry::ProviderRegistry`] 的区别
//!
//! - [`crate::registry::ProviderRegistry`]: 编译期 9 字段聚合容器 (kind 固定索引), 借鉴 Golutra
//!   `default_providers` 装配模式, 一次 `build()` 定形.
//! - **本模块 `MemoryProviderRegistry`**: 运行时**按名称**注册/查询 (name → `Arc<dyn MemoryProvider>`),
//!   支持 9 provider 的**列表 / 选择 / fallback**, 供 telemetry/cache 或高级用户按配置名选后端,
//!   未知名称报错, 可设默认 provider.
//!
//! ## 设计
//!
//! - `BTreeMap<String, Arc<dyn MemoryProvider>>` 持 provider (0 新依赖, 0 引 indexmap).
//! - 名称用 `ProviderKind::as_str()` (e.g. `"in_memory"` / `"redis"` / `"sqlite"`), 也可自定义别名.
//! - `get` 未知名称 → `MemoryProviderError::Other` (7 variant 契约 0 改, 复用兜底 variant).
//! - fallback 两档: `select(name, fallback)` 调用方给兜底; `select_or_default(name)`
//!   先查注册名, 再查 registry 默认, 都没有才报错.
//! - `MemoryProviderRegistry::from_env()` 便捷装配: 读 `APEIRETH_MEMORY_PROVIDER`
//!   (见 [`crate::factory`]) 把选中的 provider 注册为默认.
//!
//! **不假装**:
//! - 本 registry 是"接线入口", **不是**数据通路: 主链路 (apeireth-memory 主 store / telemetry cache)
//!   默认不依赖它 (有意决策, 见 crate 顶部 `//!` 接线现状). 谁注册谁使用, 0 预创建全局 state.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{MemoryProvider, ProviderKind};

/// **MemoryProviderRegistry**: 按名称注册/查询的接线注册表.
///
/// 9 provider 的运行时名称索引 (name → provider), 支持列表/选择/fallback.
#[derive(Clone, Default)]
pub struct MemoryProviderRegistry {
    /// name → provider 映射 (BTreeMap = 天然字典序, `list()` 稳定排序, 0 新依赖).
    providers: BTreeMap<String, Arc<dyn MemoryProvider>>,
    /// 默认 provider 名称 (`set_default` 设置; `select_or_default` 兜底用).
    default_name: Option<String>,
}

impl std::fmt::Debug for MemoryProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // dyn MemoryProvider 0 Debug → 只打名称表 + 默认名 (不泄漏 provider 内部).
        f.debug_struct("MemoryProviderRegistry")
            .field("providers", &self.list())
            .field("default_name", &self.default_name)
            .finish()
    }
}

impl MemoryProviderRegistry {
    /// 新建空 registry.
    pub fn new() -> Self {
        Self {
            providers: BTreeMap::new(),
            default_name: None,
        }
    }

    /// 注册 provider (按名称).
    ///
    /// 重复名称 → `Err(MemoryProviderError::Other)` (防静默覆盖, 覆盖用 [`Self::register_or_replace`]).
    pub fn register(
        &mut self,
        name: impl Into<String>,
        provider: Arc<dyn MemoryProvider>,
    ) -> MemoryProviderResult<()> {
        let name = name.into();
        if self.providers.contains_key(&name) {
            return Err(MemoryProviderError::Other {
                msg: format!("memory provider `{name}` already registered"),
            });
        }
        self.providers.insert(name, provider);
        Ok(())
    }

    /// 注册 provider (按 `ProviderKind` 规范名, e.g. `ProviderKind::Sqlite` → `"sqlite"`).
    pub fn register_kind(
        &mut self,
        kind: ProviderKind,
        provider: Arc<dyn MemoryProvider>,
    ) -> MemoryProviderResult<()> {
        self.register(kind.as_str(), provider)
    }

    /// 注册或覆盖 (重复名称静默覆盖旧 provider).
    pub fn register_or_replace(
        &mut self,
        name: impl Into<String>,
        provider: Arc<dyn MemoryProvider>,
    ) {
        self.providers.insert(name.into(), provider);
    }

    /// 查询 provider (按名称).
    ///
    /// 未知名称 → `Err(MemoryProviderError::Other)` (报错信息带当前已注册列表).
    pub fn get(&self, name: &str) -> MemoryProviderResult<Arc<dyn MemoryProvider>> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| MemoryProviderError::Other {
                msg: format!(
                    "unknown memory provider `{name}` (registered: {})",
                    self.list().join(", ")
                ),
            })
    }

    /// 查询 provider (按名称, `Option` 版, 0 报错).
    pub fn get_optional(&self, name: &str) -> Option<Arc<dyn MemoryProvider>> {
        self.providers.get(name).cloned()
    }

    /// 已注册名称列表 (字典序).
    pub fn list(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// 是否已注册该名称.
    pub fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// 已注册 provider 数.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// 是否空 registry.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// 移除 provider (按名称), 返是否移除成功.
    ///
    /// 移除默认 provider 时默认随之清空.
    pub fn remove(&mut self, name: &str) -> bool {
        let removed = self.providers.remove(name).is_some();
        if removed && self.default_name.as_deref() == Some(name) {
            self.default_name = None;
        }
        removed
    }

    /// 设默认 provider (按名称).
    ///
    /// 未知名称 → Err (0 假装默认存在).
    pub fn set_default(&mut self, name: &str) -> MemoryProviderResult<()> {
        if !self.providers.contains_key(name) {
            return Err(MemoryProviderError::Other {
                msg: format!("cannot set default: unknown memory provider `{name}`"),
            });
        }
        self.default_name = Some(name.to_string());
        Ok(())
    }

    /// 当前默认 provider 名称.
    pub fn default_name(&self) -> Option<&str> {
        self.default_name.as_deref()
    }

    /// 默认 provider (未设默认 → `None`).
    ///
    /// 命名 `default_provider` 而非 `default`: 避免遮蔽 `Default::default` (E0061 陷阱).
    pub fn default_provider(&self) -> Option<Arc<dyn MemoryProvider>> {
        self.default_name
            .as_ref()
            .and_then(|n| self.providers.get(n).cloned())
    }

    /// 选择 provider: 注册过 → 返它; 未知 → 返调用方 fallback (0 报错).
    pub fn select(&self, name: &str, fallback: Arc<dyn MemoryProvider>) -> Arc<dyn MemoryProvider> {
        self.providers.get(name).cloned().unwrap_or(fallback)
    }

    /// 选择 provider: 注册过 → 返它; 未知 → 返 registry 默认; 都无 → Err.
    pub fn select_or_default(&self, name: &str) -> MemoryProviderResult<Arc<dyn MemoryProvider>> {
        if let Some(p) = self.providers.get(name) {
            return Ok(p.clone());
        }
        match self.default_provider() {
            Some(p) => Ok(p),
            None => Err(MemoryProviderError::Other {
                msg: format!(
                    "unknown memory provider `{name}` and no registry default set (registered: {})",
                    self.list().join(", ")
                ),
            }),
        }
    }

    /// 便捷装配: 读 env (见 [`crate::factory::ProviderFactory::from_env`]),
    /// 把选中的 provider 按规范名注册并设为默认.
    ///
    /// 0 env 配置时默认 `in_memory` (进程内, 0 外部依赖).
    pub fn from_env() -> MemoryProviderResult<Self> {
        let (kind, provider) = crate::factory::ProviderFactory::from_env()?;
        let mut reg = Self::new();
        reg.register_kind(kind, provider)?;
        reg.set_default(kind.as_str())?;
        Ok(reg)
    }
}

// =====================================================================
// 单元测试 (注册/查询/未知报错/列表/移除/默认/fallback = 15+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_provider::{ProviderConfig, ProviderScope};
    use async_trait::async_trait;
    use std::time::Duration;

    /// 测试用 dummy provider (7 通用方法返 Ok 占位, 只有 kind 有真实值).
    #[derive(Debug)]
    struct DummyProvider {
        kind: ProviderKind,
    }

    #[async_trait]
    impl MemoryProvider for DummyProvider {
        fn kind(&self) -> ProviderKind {
            self.kind
        }
        async fn set(&self, _key: &str, _value: &[u8]) -> MemoryProviderResult<()> {
            Ok(())
        }
        async fn get(&self, _key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
            Ok(None)
        }
        async fn delete(&self, _key: &str) -> MemoryProviderResult<()> {
            Ok(())
        }
        async fn exists(&self, _key: &str) -> MemoryProviderResult<bool> {
            Ok(false)
        }
        async fn clear(&self) -> MemoryProviderResult<()> {
            Ok(())
        }
        async fn size(&self) -> MemoryProviderResult<u64> {
            Ok(0)
        }
    }

    fn dummy(kind: ProviderKind) -> Arc<dyn MemoryProvider> {
        Arc::new(DummyProvider { kind })
    }

    fn in_memory_dummy() -> Arc<dyn MemoryProvider> {
        dummy(ProviderKind::InMemory)
    }

    #[test]
    fn new_registry_is_empty() {
        let r = MemoryProviderRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(r.list().is_empty());
        assert!(r.default_provider().is_none());
    }

    #[test]
    fn register_and_get_by_name() {
        let mut r = MemoryProviderRegistry::new();
        let p = dummy(ProviderKind::Sqlite);
        r.register("sqlite", p.clone()).unwrap();
        assert!(r.contains("sqlite"));
        assert_eq!(r.len(), 1);
        let got = r.get("sqlite").unwrap();
        assert_eq!(got.kind(), ProviderKind::Sqlite);
        assert!(Arc::ptr_eq(&got, &p));
    }

    #[test]
    fn register_kind_uses_canonical_name() {
        let mut r = MemoryProviderRegistry::new();
        r.register_kind(ProviderKind::Redis, dummy(ProviderKind::Redis))
            .unwrap();
        assert!(r.contains("redis"));
        assert!(r.get("redis").is_ok());
    }

    #[test]
    fn unknown_provider_returns_error() {
        let r = MemoryProviderRegistry::new();
        let err = r.get("redis").err().unwrap();
        assert!(matches!(err, MemoryProviderError::Other { .. }));
        assert!(
            err.to_string().contains("redis"),
            "错误信息应含未知名: {err}"
        );
    }

    #[test]
    fn get_optional_none_for_unknown() {
        let r = MemoryProviderRegistry::new();
        assert!(r.get_optional("s3").is_none());
    }

    #[test]
    fn duplicate_register_returns_error() {
        let mut r = MemoryProviderRegistry::new();
        r.register("sqlite", dummy(ProviderKind::Sqlite)).unwrap();
        let err = r
            .register("sqlite", dummy(ProviderKind::Sqlite))
            .unwrap_err();
        assert!(matches!(err, MemoryProviderError::Other { .. }));
        assert!(err.to_string().contains("already registered"));
        assert_eq!(r.len(), 1, "重复注册失败后不应覆盖");
    }

    #[test]
    fn register_or_replace_overwrites() {
        let mut r = MemoryProviderRegistry::new();
        r.register("sqlite", dummy(ProviderKind::Sqlite)).unwrap();
        r.register_or_replace("sqlite", dummy(ProviderKind::InMemory));
        assert_eq!(r.len(), 1);
        assert_eq!(r.get("sqlite").unwrap().kind(), ProviderKind::InMemory);
    }

    #[test]
    fn list_is_sorted_and_complete() {
        let mut r = MemoryProviderRegistry::new();
        r.register("redis", dummy(ProviderKind::Redis)).unwrap();
        r.register("in_memory", dummy(ProviderKind::InMemory))
            .unwrap();
        r.register("sqlite", dummy(ProviderKind::Sqlite)).unwrap();
        assert_eq!(r.list(), vec!["in_memory", "redis", "sqlite"]);
    }

    #[test]
    fn remove_drops_provider() {
        let mut r = MemoryProviderRegistry::new();
        r.register("s3", dummy(ProviderKind::S3)).unwrap();
        assert!(r.remove("s3"));
        assert!(!r.contains("s3"));
        assert!(!r.remove("s3"), "重复移除返 false");
    }

    #[test]
    fn set_default_and_default_round_trip() {
        let mut r = MemoryProviderRegistry::new();
        r.register("sqlite", dummy(ProviderKind::Sqlite)).unwrap();
        r.set_default("sqlite").unwrap();
        assert_eq!(r.default_name(), Some("sqlite"));
        assert_eq!(r.default_provider().unwrap().kind(), ProviderKind::Sqlite);
    }

    #[test]
    fn set_default_unknown_errors() {
        let mut r = MemoryProviderRegistry::new();
        let err = r.set_default("nonexistent").unwrap_err();
        assert!(matches!(err, MemoryProviderError::Other { .. }));
    }

    #[test]
    fn remove_clears_default() {
        let mut r = MemoryProviderRegistry::new();
        r.register("sqlite", dummy(ProviderKind::Sqlite)).unwrap();
        r.set_default("sqlite").unwrap();
        r.remove("sqlite");
        assert!(r.default_provider().is_none());
        assert!(r.default_name().is_none());
    }

    #[test]
    fn select_returns_registered_or_fallback() {
        let mut r = MemoryProviderRegistry::new();
        let sqlite = dummy(ProviderKind::Sqlite);
        r.register("sqlite", sqlite.clone()).unwrap();
        let fallback = in_memory_dummy();

        // 注册过 → 用它
        let picked = r.select("sqlite", fallback.clone());
        assert!(Arc::ptr_eq(&picked, &sqlite));

        // 未知 → fallback
        let picked = r.select("nonexistent", fallback.clone());
        assert!(Arc::ptr_eq(&picked, &fallback));
    }

    #[test]
    fn select_or_default_prefers_registered_then_default() {
        let mut r = MemoryProviderRegistry::new();
        let sqlite = dummy(ProviderKind::Sqlite);
        let redis = dummy(ProviderKind::Redis);
        r.register("sqlite", sqlite.clone()).unwrap();
        r.register("redis", redis.clone()).unwrap();
        r.set_default("redis").unwrap();

        // 注册过 → 它优先
        assert!(Arc::ptr_eq(
            &r.select_or_default("sqlite").unwrap(),
            &sqlite
        ));
        // 未知 → registry 默认
        assert!(Arc::ptr_eq(
            &r.select_or_default("nonexistent").unwrap(),
            &redis
        ));
    }

    #[test]
    fn select_or_default_errors_without_default() {
        let r = MemoryProviderRegistry::new();
        let err = r.select_or_default("sqlite").err().unwrap();
        assert!(matches!(err, MemoryProviderError::Other { .. }));
    }

    #[test]
    fn clone_is_independent_name_table_shares_providers() {
        let mut r = MemoryProviderRegistry::new();
        let p = dummy(ProviderKind::Hybrid);
        r.register("hybrid", p.clone()).unwrap();
        let mut c = r.clone();
        // clone 后注册互不影响
        c.register("file", dummy(ProviderKind::File)).unwrap();
        assert!(!r.contains("file"));
        assert_eq!(c.len(), 2);
        assert!(Arc::ptr_eq(&c.get("hybrid").unwrap(), &p));
    }
}
