//! # apeireth-credentials
//!
//! **Apeireth 多 Provider 凭证 skeleton** — 1:1 翻译 v0.9.21 商业版
//! `@anthropic-ai/credentials` 集成面. R20 阶段 6 估缺, 估补 1.0 release 必做的
//! 凭证管理子 crate.
//!
//! ⚠️ **STUB MODE (R20 阶段 6 skeleton)**: 当前 crate 是 **skeleton** —
//! API 表面按 v0.9.21 商业版 `out/main` 1:1 翻译, 但 **5 Provider 全部 stub**,
//! 返 `Err(CredentialsError::NotImplemented)`. **0 真接商业版 credentials SDK**.
//! R21+ 续真接.
//!
//! ## 8 大核心模块 (per task spec §1)
//!
//! | # | 模块 | 编译期常量 | 用途 |
//! |---|------|----------|------|
//! | 1 | `provider` | `PROVIDER_KINDS` (5) | 5 Provider 抽象 (Anthropic / OpenAI / Google / Azure / Local) |
//! | 2 | `auth` | `AUTH_METHOD_COUNT` (5) | 5 鉴权方式 (API key / OAuth2 / JWT / IAM / mTLS) |
//! | 3 | `token` | `TOKEN_MANAGER_METHODS` (5) | Token 生命周期 (get/refresh/revoke/is_valid/expires_at) |
//! | 4 | `scope` | `SCOPE_COUNT` (5) | 5 Scope 权限 (Read/Write/Admin/Owner/Root) |
//! | 5 | `rotation` | `ROTATION_STRATEGY_COUNT` (4) | 4 轮换策略 (manual/time/count/hybrid) |
//! | 6 | `audit` | `AUDIT_EVENT_KINDS` (4) | 4 审计事件 (get/put/rotate/revoke) |
//! | 7 | `error` | `K1_VARIANT_NAMES` (5) | 10+ 错误类型 + 5 K-1 强校验 |
//! | 8 | `lib.rs` | — | 主入口 + `CredentialsManager` 5 Provider 切换 |
//!
//! ## 6 哲学 anchor 穿透 (per `v09021-rust-translation-blueprint-RIVAL`)
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 `@anthropic-ai/credentials` 商业版, 0 业务重设计
//! - **S-2 实事求是**: 估 800-1000 LOC (skeleton ~700 LOC), 0 过度设计, R21+ 续真接
//! - **O-2 走在前人肩上**: 借鉴 RFC 6749 (OAuth 2.0) / RFC 7519 (JWT) / RFC 8705 (mTLS) 工业标准
//! - **O-3 干到底**: 5 Provider × 5 鉴权 × 4 轮换 × 5 Scope × 4 audit = 8 模块 + 25+ 测试
//! - **O-4 任何人都能接手**: 跟 `apeireth-keyring` / `apeireth-i18n` / `apeireth-voice` 同骨架
//! - **O-5 不假装**: 所有 stub 返 `NotImplemented` + `warn!` 日志, 0 假装已对接商业版
//!
//! ## 8 项不修改承诺 (per R19 + 主人 19:37 "全用 rust" + 守工程哲学铁律)
//!
//! 1. **0 触碰 24 LOCKED crate** (apeireth-core / api / tui 等)
//! 2. **0 改 workspace Cargo.toml 其他字段** (只 + 1 个 members 行, 整合 #2 改 workspace = true)
//! 3. **0 写真实凭证 (key / secret / token)** (所有 fixture 用 "sk-test-1234" 等占位)
//! 4. **0 改 K-1 强校验守门 (5 鉴权空值校验必保留)**
//! 5. **0 改 5 Provider 枚举顺序** (Anthropic / OpenAI / Google / Azure / Local)
//! 6. **0 改 5 Scope 顺序** (Read < Write < Admin < Owner < Root)
//! 7. **0 改 4 轮换策略名** (manual / time / count / hybrid)
//! 8. **0 改 4 审计事件名** (get / put / rotate / revoke)
//!
//! ## 引用文档 (4 份)
//!
//! 1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md` (RIVAL 蓝图)
//! 2. `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\chunks\credentials-*.js` (v0.9.21 1:1 翻译源)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\m3-hallucination-defense-2026-08-05.md` (m3 防御)
//! 4. `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-keyring\src\lib.rs` (凭证存储参考, P0 安全铁律)
//!
//! ## 状态: ⏳ skeleton (R20 阶段 6 实施)
//!
//! - ✅ 5 Provider trait 实现 (stub 返 `NotImplemented`)
//! - ✅ 5 鉴权方式 (API key / OAuth 2.0 / JWT / IAM / mTLS)
//! - ✅ 5 Scope 级别 (Read / Write / Admin / Owner / Root)
//! - ✅ 4 轮换策略 (Manual / Time / Count / Hybrid)
//! - ✅ Token 管理 (5 方法: get / refresh / revoke / is_valid / expires_at)
//! - ✅ Audit 4 事件 (Get / Put / Rotate / Revoke)
//! - ✅ CredentialsManager 5 Provider 切换
//! - ✅ 25+ 集成测试
//! - ⏳ R21+ 续真接商业版 credentials SDK

#![warn(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

use crate::audit::{AuditEvent, AuditEventKind, AuditLog};
use crate::error::{CredentialsError, CredentialsResult};
use crate::provider::{
    AzureProvider, GoogleProvider, LocalProvider, OpenAIProvider, Provider, SecretString,
};
use crate::rotation::RotationStrategy;
use crate::token::{ProviderTokenManager, TokenManager};

// ============================================================================
// §1 公共模块导出 (re-export 全部子模块)
// ============================================================================

pub mod audit;
pub mod auth;
pub mod error;
pub mod provider;
pub mod rotation;
pub mod scope;
pub mod token;

// ============================================================================
// §1.5 Re-exports 常用类型 (doctest 友好)
// ============================================================================

pub use crate::auth::AuthMethod;
pub use crate::provider::{AnthropicProvider, ProviderKind};
pub use crate::scope::Scope;

// ============================================================================
// §2 编译期 hardcode 常量 (K-1 强校验 + 设计表 §2.4.1 + v0.9.21 1:1)
// ============================================================================

/// Credentials schema 版本 (向前兼容字段, R21+ 改格式时 bump).
///
/// K-1 强校验 #1: 编译期 hardcode, 不写 `"1"` 字符串 elsewhere.
pub const CREDENTIALS_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #2: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译,
/// 不写 "SpectrAI" / "HeySpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB 模式守门**: 编译期 hardcode `true`. R21+ 整合时改 `false` + 真接
/// 商业版 credentials SDK. 5 Provider 全部返 `Err(CredentialsError::NotImplemented)`.
///
/// K-1 强校验 #3: 守 stub 模式不漏防.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必为 true (R21+ 改 false 时同步改本断言).
const _: () = assert!(STUB_MODE == true, "STUB_MODE must be true until R21");

/// 查 STUB_MODE 状态 (m3 防御 + 调试用).
///
/// R21+ 真接商业版后, 整个 `STUB_MODE` 块 + `is_stub_mode` 函数保留 (向后兼容).
#[must_use]
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// **5 Provider 编译期守门** (K-1 强校验 #4).
pub const PROVIDER_COUNT: usize = 5;

/// **5 鉴权方式编译期守门** (K-1 强校验 #5).
pub const AUTH_METHOD_COUNT: usize = 5;

/// **5 Scope 级别编译期守门** (K-1 强校验 #6).
pub const SCOPE_COUNT: usize = 5;

/// **4 轮换策略编译期守门** (K-1 强校验 #7).
pub const ROTATION_STRATEGY_COUNT: usize = 4;

/// **4 审计事件编译期守门** (K-1 强校验 #8).
pub const AUDIT_EVENT_COUNT: usize = 4;

/// 默认 scope 级别 (Read, 最低权限).
pub const DEFAULT_SCOPE: Scope = Scope::Read;

/// 默认轮换策略 (Manual, 0 自动轮换, 留 admin 手动).
pub const DEFAULT_ROTATION_STRATEGY: RotationStrategy = RotationStrategy::Manual;

/// 默认审计日志容量 (1000 事件, 满后覆盖最早).
pub const DEFAULT_AUDIT_CAPACITY: usize = 1000;

/// **K-1 强校验编译期守门**: 5 鉴权空值校验 (per 5 K-1 错误变体).
pub const K1_STRONG_VALIDATION_VARIANTS: [&str; 5] = [
    "EmptyApiKey",
    "EmptyOAuthClientId",
    "EmptyJwtAudience",
    "EmptyIamRole",
    "InvalidMtlsCertPath",
];

/// 编译期守门: 5 K-1 强校验变体.
const _: () = assert!(K1_STRONG_VALIDATION_VARIANTS.len() == 5);

/// **m3 防御**: 8 工具白名单 (K-1 强校验: 编译期 hardcode, 不可运行时增删).
///
/// 5 Provider trait 方法 (get_token / refresh / revoke / is_valid / expires_at)
/// + 3 轮换操作 (rotate / should_rotate / describe) = 8.
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_credentials_get_token",
    "apeireth_credentials_refresh",
    "apeireth_credentials_revoke",
    "apeireth_credentials_is_valid",
    "apeireth_credentials_expires_at",
    "apeireth_credentials_rotate",
    "apeireth_credentials_should_rotate",
    "apeireth_credentials_describe_strategy",
];

/// 编译期守门: TOOL_WHITELIST 长度 == 8.
pub const TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// **m3 防御**: 校验工具调用是否在白名单内. 不在则拒绝
/// (返 `CredentialsError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> CredentialsResult<()> {
    if !TOOL_WHITELIST.contains(&tool) {
        Err(CredentialsError::ToolNotWhitelisted(tool.to_string()))
    } else {
        Ok(())
    }
}

// ============================================================================
// §3 CredentialsManager (5 Provider 切换 + Token 管理 + Audit 集成)
// ============================================================================

/// 凭证管理器 (5 Provider 切换 + Token 生命周期 + Audit 集成).
///
/// 1:1 翻译 v0.9.21 商业版 `@anthropic-ai/credentials` 主入口. R20 阶段 6 skeleton
/// 阶段用 5 Provider 各 1 个 stub, R21+ 续真接.
///
/// ## 用法 (per task spec §9 demo)
///
/// ```no_run
/// use apeireth_credentials::{
///     AnthropicProvider, AuthMethod, CredentialsManager, ProviderKind, Scope,
/// };
///
/// # async fn demo() -> anyhow::Result<()> {
/// let mut mgr = CredentialsManager::new(Scope::Write)?;
/// mgr.add_provider(
///     ProviderKind::Anthropic,
///     Box::new(AnthropicProvider::new(AuthMethod::ApiKey {
///         api_key: "sk-test-1234".to_string(),
///     })?),
/// )?;
/// let _ = mgr.get_token(ProviderKind::Anthropic).await; // 返 NotImplemented
/// # Ok(())
/// # }
/// ```
pub struct CredentialsManager {
    /// 当前默认 scope (K-1 强校验).
    scope: Scope,
    /// 5 Provider 映射 (ProviderKind → Box<dyn Provider>).
    providers: HashMap<ProviderKind, Box<dyn Provider>>,
    /// 默认轮换策略.
    rotation_strategy: RotationStrategy,
    /// 审计日志 (4 事件不可绕过).
    audit_log: Arc<AuditLog>,
}

impl std::fmt::Debug for CredentialsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialsManager")
            .field("scope", &self.scope)
            .field("provider_count", &self.providers.len())
            .field("rotation_strategy", &self.rotation_strategy.describe())
            .field("audit_capacity", &DEFAULT_AUDIT_CAPACITY)
            .finish()
    }
}

impl CredentialsManager {
    /// 构造 (默认 scope = Read, 默认轮换 = Manual, 默认审计容量 1000).
    pub fn new(scope: Scope) -> CredentialsResult<Self> {
        Ok(Self {
            scope,
            providers: HashMap::new(),
            rotation_strategy: DEFAULT_ROTATION_STRATEGY,
            audit_log: Arc::new(AuditLog::with_capacity(DEFAULT_AUDIT_CAPACITY)),
        })
    }

    /// 构造 (自定义 scope + rotation + audit capacity).
    pub fn with_options(
        scope: Scope,
        rotation_strategy: RotationStrategy,
        audit_capacity: usize,
    ) -> CredentialsResult<Self> {
        Ok(Self {
            scope,
            providers: HashMap::new(),
            rotation_strategy,
            audit_log: Arc::new(AuditLog::with_capacity(audit_capacity)),
        })
    }

    /// 添加 Provider.
    ///
    /// R20 阶段 6 skeleton 阶段不立即 verify (verify 留给 R21+ 真接).
    ///
    /// 注: skeleton 阶段不加锁, 假定调用方在初始化阶段串行 add. R21+ 真接时
    /// 改成 Arc<tokio::sync::RwLock<HashMap<...>>> + .write().await.
    pub fn add_provider(
        &mut self,
        kind: ProviderKind,
        provider: Box<dyn Provider>,
    ) -> CredentialsResult<()> {
        self.providers.insert(kind, provider);
        info!(provider = %kind, "credentials_manager: provider added");
        Ok(())
    }

    /// 移除 Provider.
    pub fn remove_provider(&mut self, kind: ProviderKind) -> CredentialsResult<bool> {
        let removed = self.providers.remove(&kind).is_some();
        info!(provider = %kind, removed, "credentials_manager: provider removed");
        Ok(removed)
    }

    /// 列出已添加的 Provider.
    #[must_use]
    pub fn list_providers(&self) -> Vec<ProviderKind> {
        self.providers.keys().copied().collect()
    }

    /// 当前 scope.
    #[must_use]
    pub fn scope(&self) -> Scope {
        self.scope
    }

    /// 当前轮换策略.
    #[must_use]
    pub fn rotation_strategy(&self) -> RotationStrategy {
        self.rotation_strategy
    }

    /// 设置轮换策略.
    pub fn set_rotation_strategy(&mut self, strategy: RotationStrategy) {
        self.rotation_strategy = strategy;
    }

    /// 审计日志句柄.
    #[must_use]
    pub fn audit_log(&self) -> Arc<AuditLog> {
        Arc::clone(&self.audit_log)
    }

    /// **核心**: 获取指定 Provider 的 token.
    ///
    /// 自动记录 `AuditEventKind::Get` 事件.
    /// R20 阶段 6 skeleton 阶段: Provider stub 返 `NotImplemented`.
    pub async fn get_token(&self, kind: ProviderKind) -> CredentialsResult<SecretString> {
        // 越权检查
        Scope::can_perform(self.scope, Scope::Read)?;
        let provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| CredentialsError::ProviderNotFound(kind.to_string()))?;
        let requester = format!("scope={}", self.scope);
        let token = provider.get_token().await;
        // 不管成功失败都记录 audit (P0 安全铁律)
        let event = match &token {
            Ok(_) => AuditEvent::get(kind, requester),
            Err(e) => AuditEvent::new(
                AuditEventKind::Get,
                kind,
                requester,
                format!("get_token failed: {}", e),
            ),
        };
        if let Err(audit_err) = self.audit_log.record(event).await {
            warn!(error = %audit_err, "credentials_manager: audit record failed");
        }
        token
    }

    /// **核心**: 刷新指定 Provider 的 token.
    ///
    /// 自动记录 `AuditEventKind::Rotate` 事件 (per SOC 2 审计要求).
    pub async fn refresh_token(&self, kind: ProviderKind) -> CredentialsResult<SecretString> {
        Scope::can_perform(self.scope, Scope::Write)?;
        let provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| CredentialsError::ProviderNotFound(kind.to_string()))?;
        let requester = format!("scope={}", self.scope);
        let new_token = provider.refresh().await;
        let event = AuditEvent::rotate(
            kind,
            requester,
            self.rotation_strategy,
            None, // skeleton 阶段无 old_expires
            None, // skeleton 阶段无 new_expires
        );
        if let Err(audit_err) = self.audit_log.record(event).await {
            warn!(error = %audit_err, "credentials_manager: audit record failed");
        }
        new_token
    }

    /// **核心**: 撤销指定 Provider 的 token.
    ///
    /// 自动记录 `AuditEventKind::Revoke` 事件.
    pub async fn revoke_token(&self, kind: ProviderKind, reason: &str) -> CredentialsResult<()> {
        Scope::can_perform(self.scope, Scope::Admin)?;
        let provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| CredentialsError::ProviderNotFound(kind.to_string()))?;
        let requester = format!("scope={}", self.scope);
        let result = provider.revoke().await;
        let event = AuditEvent::revoke(kind, requester, reason);
        if let Err(audit_err) = self.audit_log.record(event).await {
            warn!(error = %audit_err, "credentials_manager: audit record failed");
        }
        result
    }

    /// **核心**: 检查 Provider token 是否有效.
    pub async fn is_token_valid(&self, kind: ProviderKind) -> CredentialsResult<bool> {
        let provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| CredentialsError::ProviderNotFound(kind.to_string()))?;
        Ok(provider.is_valid().await)
    }

    /// **核心**: 获取 Provider token 过期时间.
    pub async fn expires_at(&self, kind: ProviderKind) -> CredentialsResult<Option<chrono::DateTime<chrono::Utc>>> {
        let provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| CredentialsError::ProviderNotFound(kind.to_string()))?;
        Ok(provider.expires_at().await)
    }

    /// **核心**: 触发轮换 (基于 should_rotate 逻辑).
    ///
    /// R20 阶段 6 skeleton 阶段: 返 `NotImplemented` (留 R21+ 真接).
    pub async fn maybe_rotate(&self, kind: ProviderKind) -> CredentialsResult<chrono::DateTime<chrono::Utc>> {
        Scope::can_perform(self.scope, Scope::Write)?;
        // 简单演示 should_rotate 逻辑 (skeleton 阶段)
        let now = chrono::Utc::now();
        let should = self.rotation_strategy.should_rotate(None, 0);
        if should && self.rotation_strategy != RotationStrategy::Manual {
            // skeleton: 永远 NotImplemented
            return self.rotation_strategy.rotate().await;
        }
        let _ = kind; // skeleton 阶段未用
        let _ = now;
        warn!("credentials_manager: maybe_rotate stub, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "credentials_manager_maybe_rotate",
        ))
    }

    /// 包装 Provider 为 TokenManager (R20 阶段 6 stub).
    ///
    /// **R21+ 真接时实现**: 当前返 `NotImplemented` (Box<dyn Provider> 不可 Clone,
    /// 需要重构 providers 字段为 Arc<dyn Provider>).
    pub fn as_token_manager(&self, kind: ProviderKind) -> CredentialsResult<Box<dyn TokenManager>> {
        let _provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| CredentialsError::ProviderNotFound(kind.to_string()))?;
        // R21+ 真接: 改成 Arc<dyn Provider> 包装, 返 Box<dyn TokenManager>
        Err(CredentialsError::NotImplemented(
            "credentials_manager_as_token_manager: R21+ 续 (Box<dyn Provider> 不可 Clone)",
        ))
    }
}

// ============================================================================
// §4 Provider factory (5 Provider 简化构造)
// ============================================================================

/// 5 Provider factory (简化 CredentialsManager.add_provider 调用).
///
/// R20 阶段 6 skeleton 阶段: 直接构造, 0 额外初始化.
pub fn make_provider(kind: ProviderKind, auth: AuthMethod) -> CredentialsResult<Box<dyn Provider>> {
    match kind {
        ProviderKind::Anthropic => Ok(Box::new(AnthropicProvider::new(auth)?)),
        ProviderKind::OpenAI => Ok(Box::new(OpenAIProvider::new(auth)?)),
        ProviderKind::Google => Ok(Box::new(GoogleProvider::new(auth)?)),
        ProviderKind::Azure => {
            // Azure 需要 resource, 从 auth 中提取 (暂用占位)
            Ok(Box::new(AzureProvider::new(auth, "myresource")?))
        }
        ProviderKind::Local => Ok(Box::new(LocalProvider::new(
            auth,
            "http://localhost:8080",
        )?)),
    }
}

// ============================================================================
// §5 CredentialsConfig (serde 兼容, 1:1 翻译 v0.9.21 商业版配置)
// ============================================================================

/// 凭证配置 (serde 兼容, 用于 keyring 存盘).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialsConfig {
    /// Schema 版本.
    pub schema_version: String,
    /// 当前默认 Provider.
    pub default_provider: ProviderKind,
    /// 当前默认 Scope.
    pub default_scope: Scope,
    /// 当前默认轮换策略.
    pub rotation_strategy: RotationStrategy,
    /// 5 Provider 鉴权 (可选, 未配的不存).
    #[serde(default)]
    pub auths: HashMap<ProviderKind, AuthMethod>,
}

impl CredentialsConfig {
    /// 构造默认配置.
    #[must_use]
    pub fn default_config() -> Self {
        Self {
            schema_version: CREDENTIALS_SCHEMA_VERSION.to_string(),
            default_provider: ProviderKind::Anthropic,
            default_scope: DEFAULT_SCOPE,
            rotation_strategy: DEFAULT_ROTATION_STRATEGY,
            auths: HashMap::new(),
        }
    }

    /// 编译期守门: schema_version 必为 "1".
    pub fn validate(&self) -> CredentialsResult<()> {
        if self.schema_version != CREDENTIALS_SCHEMA_VERSION {
            return Err(CredentialsError::Internal(format!(
                "credentials schema version mismatch: expected {}, got {}",
                CREDENTIALS_SCHEMA_VERSION, self.schema_version
            )));
        }
        Ok(())
    }
}

// ============================================================================
// §6 编译期守门 (K-1 强校验 + 5+5+5+4+4 = 23 项常量对齐)
// ============================================================================

/// **K-1 强校验守门总表** (5+5+5+4+4 = 23 项常量).
pub const K1_GUARD_TOTAL: usize =
    PROVIDER_COUNT + AUTH_METHOD_COUNT + SCOPE_COUNT + ROTATION_STRATEGY_COUNT + AUDIT_EVENT_COUNT;

const _: () = assert!(K1_GUARD_TOTAL == 5 + 5 + 5 + 4 + 4);

// ============================================================================
// §7 集成测试入口 (25+ 测试在 tests/test_credentials_in_process.rs)
// ============================================================================

// 单元测试 (内联)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthMethod;
    use crate::provider::{
        AnthropicProvider, AzureProvider, GoogleProvider, LocalProvider, OpenAIProvider,
        PROVIDER_KINDS,
    };
    use crate::rotation::{count_default, hybrid_default, time_default};

    fn fixture_api_key() -> AuthMethod {
        AuthMethod::ApiKey {
            api_key: "sk-test-1234".to_string(),
        }
    }

    // --- 编译期常量守门 (5+5+5+4+4 = 23 项) ---

    #[test]
    fn test_constants_provider_count_5() {
        assert_eq!(PROVIDER_COUNT, 5);
        assert_eq!(ProviderKind::all().len(), 5);
        assert_eq!(PROVIDER_KINDS.len(), 5);
    }

    #[test]
    fn test_constants_auth_method_count_5() {
        assert_eq!(AUTH_METHOD_COUNT, 5);
        assert_eq!(AuthMethod::ALL_TYPE_NAMES.len(), 5);
    }

    #[test]
    fn test_constants_scope_count_5() {
        assert_eq!(SCOPE_COUNT, 5);
        assert_eq!(crate::scope::ALL_SCOPES.len(), 5);
    }

    #[test]
    fn test_constants_rotation_count_4() {
        assert_eq!(ROTATION_STRATEGY_COUNT, 4);
        assert_eq!(RotationStrategy::ALL_NAMES.len(), 4);
    }

    #[test]
    fn test_constants_audit_count_4() {
        assert_eq!(AUDIT_EVENT_COUNT, 4);
        assert_eq!(AuditEventKind::all().len(), 4);
    }

    #[test]
    fn test_k1_strong_validation_5_variants() {
        // 5 K-1 强校验变体
        assert_eq!(K1_STRONG_VALIDATION_VARIANTS.len(), 5);
        for v in K1_STRONG_VALIDATION_VARIANTS.iter() {
            // 5 错误码必须存在
            match *v {
                "EmptyApiKey" => {
                    let _ = CredentialsError::EmptyApiKey;
                }
                "EmptyOAuthClientId" => {
                    let _ = CredentialsError::EmptyOAuthClientId;
                }
                "EmptyJwtAudience" => {
                    let _ = CredentialsError::EmptyJwtAudience;
                }
                "EmptyIamRole" => {
                    let _ = CredentialsError::EmptyIamRole;
                }
                "InvalidMtlsCertPath" => {
                    let _ = CredentialsError::InvalidMtlsCertPath(String::new());
                }
                _ => panic!("unknown K-1 variant: {v}"),
            }
        }
    }

    #[test]
    fn test_tool_whitelist_8() {
        assert_eq!(TOOL_WHITELIST_COUNT, 8);
        assert_eq!(TOOL_WHITELIST.len(), 8);
    }

    #[test]
    fn test_m3_validate_tool_call() {
        // m3 防御: 白名单校验
        let valid = validate_tool_call("apeireth_credentials_get_token", &serde_json::json!({}));
        assert!(valid.is_ok());
        let invalid = validate_tool_call("apeireth_credentials_bogus_tool", &serde_json::json!({}));
        assert!(matches!(
            invalid,
            Err(CredentialsError::ToolNotWhitelisted(_))
        ));
    }

    #[test]
    fn test_stub_mode_true() {
        assert!(STUB_MODE);
        assert!(is_stub_mode());
    }

    // --- CredentialsManager 集成测试 ---

    #[test]
    fn test_manager_new_default() {
        let mgr = CredentialsManager::new(Scope::Read).expect("new");
        assert_eq!(mgr.scope(), Scope::Read);
        assert_eq!(mgr.rotation_strategy(), DEFAULT_ROTATION_STRATEGY);
        assert!(mgr.list_providers().is_empty());
    }

    #[test]
    fn test_manager_with_options() {
        let mgr = CredentialsManager::with_options(Scope::Admin, time_default(), 500)
            .expect("with_options");
        assert_eq!(mgr.scope(), Scope::Admin);
        assert_eq!(mgr.rotation_strategy(), time_default());
    }

    #[test]
    fn test_manager_add_remove_5_providers() {
        let mut mgr = CredentialsManager::new(Scope::Write).expect("new");
        for kind in ProviderKind::all() {
            mgr.add_provider(kind, make_provider(kind, fixture_api_key()).expect("make"))
                .expect("add");
        }
        assert_eq!(mgr.list_providers().len(), 5);
        mgr.remove_provider(ProviderKind::Anthropic).expect("remove");
        assert_eq!(mgr.list_providers().len(), 4);
        assert!(!mgr.list_providers().contains(&ProviderKind::Anthropic));
    }

    #[test]
    fn test_manager_get_token_provider_not_found() {
        let mgr = CredentialsManager::new(Scope::Read).expect("new");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let err = mgr.get_token(ProviderKind::Anthropic).await.unwrap_err();
            assert!(matches!(err, CredentialsError::ProviderNotFound(_)));
        });
    }

    #[test]
    fn test_manager_get_token_insufficient_scope() {
        // Read 不能 get_token (Read 级别需要 Write 越权检查, 但 get_token 实际只要求 Read)
        // 这里测的是 revoke_token 需要 Admin, Read 不能 revoke
        let mut mgr = CredentialsManager::new(Scope::Read).expect("new");
        mgr.add_provider(
            ProviderKind::Anthropic,
            make_provider(ProviderKind::Anthropic, fixture_api_key()).expect("make"),
        )
        .expect("add");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // revoke 需要 Admin, Read 越权
            let err = mgr.revoke_token(ProviderKind::Anthropic, "test").await.unwrap_err();
            assert!(matches!(err, CredentialsError::InsufficientScope { .. }));
        });
    }

    #[test]
    fn test_manager_get_token_anthropic_returns_not_implemented() {
        let mut mgr = CredentialsManager::new(Scope::Read).expect("new");
        mgr.add_provider(
            ProviderKind::Anthropic,
            make_provider(ProviderKind::Anthropic, fixture_api_key()).expect("make"),
        )
        .expect("add");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let err = mgr.get_token(ProviderKind::Anthropic).await.unwrap_err();
            assert!(matches!(err, CredentialsError::NotImplemented(_)));
        });
    }

    #[test]
    fn test_manager_audit_log_integration() {
        // get_token 失败时记录 Get 事件
        let mut mgr = CredentialsManager::new(Scope::Read).expect("new");
        mgr.add_provider(
            ProviderKind::OpenAI,
            make_provider(ProviderKind::OpenAI, fixture_api_key()).expect("make"),
        )
        .expect("add");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let _ = mgr.get_token(ProviderKind::OpenAI).await; // 返 NotImplemented
            let events = mgr.audit_log().list_by_kind(AuditEventKind::Get).await;
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].provider, ProviderKind::OpenAI);
        });
    }

    // --- 5 Provider factory 测试 ---

    #[test]
    fn test_factory_5_providers() {
        for kind in ProviderKind::all() {
            let p = make_provider(kind, fixture_api_key()).expect("make");
            assert_eq!(p.kind(), kind);
        }
    }

    #[test]
    fn test_factory_k1_validates() {
        // 空 api_key 必拒
        let empty = AuthMethod::ApiKey {
            api_key: String::new(),
        };
        let err = make_provider(ProviderKind::Anthropic, empty).unwrap_err();
        assert!(matches!(err, CredentialsError::EmptyApiKey));
    }

    // --- CredentialsConfig 测试 ---

    #[test]
    fn test_credentials_config_default() {
        let cfg = CredentialsConfig::default_config();
        assert_eq!(cfg.schema_version, CREDENTIALS_SCHEMA_VERSION);
        assert_eq!(cfg.default_provider, ProviderKind::Anthropic);
        assert_eq!(cfg.default_scope, DEFAULT_SCOPE);
        assert_eq!(cfg.rotation_strategy, DEFAULT_ROTATION_STRATEGY);
        assert!(cfg.auths.is_empty());
    }

    #[test]
    fn test_credentials_config_validate() {
        let cfg = CredentialsConfig::default_config();
        cfg.validate().expect("valid config");
        let mut bad = cfg.clone();
        bad.schema_version = "999".to_string();
        let err = bad.validate().unwrap_err();
        assert!(matches!(err, CredentialsError::Internal(_)));
    }

    #[test]
    fn test_credentials_config_serde_roundtrip() {
        let mut cfg = CredentialsConfig::default_config();
        cfg.auths
            .insert(ProviderKind::Anthropic, fixture_api_key());
        let json = serde_json::to_string(&cfg).expect("serialize");
        let parsed: CredentialsConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, cfg);
    }

    // --- 4 轮换策略集成测试 ---

    #[test]
    fn test_rotation_4_strategies_via_manager() {
        for strategy in [
            RotationStrategy::Manual,
            time_default(),
            count_default(),
            hybrid_default(),
        ] {
            let mgr = CredentialsManager::with_options(Scope::Owner, strategy, 100)
                .expect("with_options");
            assert_eq!(mgr.rotation_strategy(), strategy);
        }
    }

    // --- 5 Provider 真实类型测试 (确保 make_provider 返正确类型) ---

    #[test]
    fn test_5_provider_constructors() {
        let _a: Box<dyn Provider> = Box::new(
            AnthropicProvider::new(fixture_api_key()).expect("anthropic"),
        );
        let _o: Box<dyn Provider> = Box::new(
            OpenAIProvider::new(fixture_api_key()).expect("openai"),
        );
        let _g: Box<dyn Provider> = Box::new(
            GoogleProvider::new(fixture_api_key()).expect("google"),
        );
        let _z: Box<dyn Provider> = Box::new(
            AzureProvider::new(fixture_api_key(), "r").expect("azure"),
        );
        let _l: Box<dyn Provider> = Box::new(
            LocalProvider::new(fixture_api_key(), "http://localhost:8080").expect("local"),
        );
    }
}

// ============================================================================
// §8 Error 新增变体 (ToolNotWhitelisted + ProviderNotFound) - m3 防御集成
// ============================================================================

// 扩展 CredentialsError, 加上 m3 防御的 ToolNotWhitelisted + ProviderNotFound.
// 注: 这些变体在 error.rs 之外加 (这里) 是为了避免 m3 防御细节污染主 error.rs.
// 也可以在 error.rs 加, 但当前布局让 lib.rs 自己负责 m3 防御的 2 变体.

impl CredentialsError {
    /// m3 防御: 工具不在白名单内.
    ///
    /// 注: 这个方法返新错误, 借用原错误 (per thiserror 模式).
    pub fn tool_not_whitelisted(tool: impl Into<String>) -> Self {
        Self::ToolNotWhitelisted(tool.into())
    }

    /// Provider 未找到 (CredentialsManager.get_token 等).
    pub fn provider_not_found(kind: impl Into<String>) -> Self {
        Self::ProviderNotFound(kind.into())
    }
}

// 在 §7 单元测试里用了 ToolNotWhitelisted 和 ProviderNotFound, 必须在 error.rs 里有.
// 在 error.rs 已经定义过, 这里只补一个 lib.rs 内部的 helper 即可.
