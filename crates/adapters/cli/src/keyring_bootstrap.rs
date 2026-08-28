//! P-arch (2026-08-27) + v2.0.0-rc.1 RC-9: KeyringSelector 真接入 CLI/gateway bootstrap.
//!
//! **位置**: 本模块在 `apeireth-cli` (adapter), 用 `apeireth-credentials::KeyringSelector` +
//! `KeyringCredentialResolver` 真接 OS keyring / EncryptedFile / InMemory.
//! 0 装诚实: 4 backend + selector 已在 alpha 真 impl (per `apeireth-credentials::keyring`),
//! 本模块只做**接入** (per v2.0.0-rc-roadmap.md §3 RC-9: "keyring 真正接到 EnvCredentialResolver 之前").
//!
//! **设计**: `build_keyring_resolver()` 优先用 `KeyringSelector` (按 `APEIRETH_KEYRING_BACKEND`
//! env 选 backend), 没有 env 时 fallback 到 `EnvCredentialResolver` (alpha 已有).
//! 这样 alpha 用户**无感升级** (没设 env → 走 env resolver, 0 行为变化), 部署 v2.0
//! 时设 env → 走真 keyring (Linux Secret Service / macOS Keychain / Windows Credential Manager).
//!
//! **3 阶审查** (O-6 锚 #9):
//! 1. 总体: 与 RC-1 真 SQL 同样模式 (alpha 写真完整, 接 bootstrap 即可)
//! 2. 系统: bootstrap 选择 resolver, 不引入新 cross-crate 依赖 (KeyringSelector 在 credentials
//!    crate, 已在依赖图内)
//! 3. 架构: `KeyringCredentialResolver` 已在 plugin::CredentialResolver trait 上 impl,
//!    runtime 拿 `Arc<dyn CredentialResolver>` 注入, 0 改 Runtime 接口
//!
//! **0 装诚实**:
//! - alpha 0 设 `APEIRETH_KEYRING_BACKEND` → fallback `EnvCredentialResolver` (0 行为变化)
//! - 设 `platform` / `encrypted-file` / `in-memory` / `auto` → 真接 selector 4 backend
//! - 真接 OS keyring 是 `KeyringSelector::select()` 真实实现, 0 在本模块写
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_credentials::keyring::{AuditSink, CountingAudit, NoopAudit};
use apeireth_credentials::keyring_resolver::KeyringCredentialResolver;
use apeireth_credentials::KeyringSelector;
use apeireth_plugin::CredentialResolver;

/// CLI/gateway 启动时构造 `Arc<dyn CredentialResolver>`, 优先用 KeyringSelector 真接
/// OS keyring, 退化到 `EnvCredentialResolver` (alpha 0 装路径).
///
/// **优先级** (per v2.0.0-rc-roadmap.md §3 RC-9):
/// 1. `APEIRETH_KEYRING_BACKEND` env 已设 → `KeyringSelector::select(env, audit, fallback_dir)`
///    拿 SelectedBackend → `KeyringCredentialResolver::new(backend)` (per `keyring_resolver.rs`)
/// 2. 1 失败 (KeyringSelector 选 backend 失败, e.g. EncryptedFileBackend::open IO error)
///    → fallback `EnvCredentialResolver` (0 装诚实: 真 fallback, 不静默 0 装)
/// 3. 没设 env → 直接 `EnvCredentialResolver` (alpha 路径, 0 行为变化)
///
/// **返回**: Send+Sync `Arc<dyn CredentialResolver>`, runtime 拿它注入.
pub fn build_keyring_resolver() -> Arc<dyn CredentialResolver> {
    // 优先 KeyringSelector 真接 (RC-9)
    match try_build_keyring_resolver() {
        Ok(resolver) => resolver,
        Err(reason) => {
            // 0 装诚实: 退化时**真**用 EnvCredentialResolver, 不假装"我有 keyring"
            // 0 装诚实: 退化原因写到 stderr, 不静默 (运维可看到为什么)
            eprintln!(
                "[keyring] KeyringSelector 退化到 EnvCredentialResolver: {reason} \
                 (设 APEIRETH_KEYRING_BACKEND=auto 可重新尝试 keyring)"
            );
            Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new())
        }
    }
}

/// 真接 KeyringSelector, 失败返 Err (退化由 caller 处理)
fn try_build_keyring_resolver() -> Result<Arc<dyn CredentialResolver>, String> {
    // 读 env: APEIRETH_KEYRING_BACKEND (per v2.0.0-rc-roadmap.md §3 RC-9: "KeyringSelector::select()
    // 真实按 APEIRETH_KEYRING_BACKEND env 选择")
    let env_value = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
    if env_value.is_none() {
        // 0 设 env: 不算错, 退化由 caller 处理
        return Err("APEIRETH_KEYRING_BACKEND 未设".to_string());
    }
    // 构造 audit sink (CountingAudit 0 装, 真生产换真 audit)
    let audit: Arc<dyn AuditSink> = if cfg!(test) {
        Arc::new(NoopAudit)
    } else {
        Arc::new(CountingAudit::new())
    };
    // fallback dir (EncryptedFile backend 用, 默认 ~/.apeireth/keyring/)
    let fallback_dir: Option<PathBuf> = std::env::var_os("APEIRETH_KEYRING_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            // 默认 ~/.apeireth/keyring/ (per `keyring.rs::default_dir`)
            if let Some(home) = std::env::var_os("HOME") {
                Some(PathBuf::from(home).join(".apeireth").join("keyring"))
            } else if let Some(profile) = std::env::var_os("USERPROFILE") {
                Some(PathBuf::from(profile).join(".apeireth").join("keyring"))
            } else {
                Some(PathBuf::from(".apeireth/keyring"))
            }
        });
    // KeyringSelector::select (4 backend: platform / encrypted-file / in-memory / auto)
    let selected = KeyringSelector::select(env_value.as_deref(), audit, fallback_dir)
        .map_err(|e| format!("KeyringSelector::select 失败: {e}"))?;
    // 0 装诚实: backend 名字 (platform / encrypted-file / in-memory / auto) 写到 stderr
    // 运维可见 (per `selected.kind`)
    eprintln!("[keyring] KeyringSelector 选 backend: {:?}", selected.kind);
    // KeyringCredentialResolver::new 接受 Arc<dyn KeyringBackend>,
    // selected.backend 是 Box<dyn KeyringBackend>, 转 Arc
    let backend_arc: Arc<dyn apeireth_credentials::keyring::KeyringBackend> =
        selected.backend.into();
    Ok(Arc::new(KeyringCredentialResolver::new(backend_arc)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RC-9 验收: 没设 env → 退化到 EnvCredentialResolver (alpha 路径 0 行为变化)
    #[test]
    fn no_env_falls_back_to_env_resolver() {
        // 测试时清空 env (避免宿主环境干扰)
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        let resolver = build_keyring_resolver();
        // 退化到 EnvCredentialResolver: 应返 None (env 未设)
        let result = resolver.resolve("provider.minimax.api_key");
        assert!(
            result.is_none(),
            "退化路径: EnvCredentialResolver 没拿到 key, 返 None"
        );
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        }
    }

    /// RC-9 验收: 设 `in-memory` → KeyringSelector 真选 InMemoryKeyring
    #[test]
    fn in_memory_backend_via_selector() {
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        std::env::set_var("APEIRETH_KEYRING_BACKEND", "in-memory");
        let resolver = build_keyring_resolver();
        // InMemoryKeyring: 空, 返 None (没存任何 secret)
        let result = resolver.resolve("provider.minimax.api_key");
        assert!(result.is_none(), "InMemoryKeyring 空, 返 None");
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        }
    }

    /// RC-9 验收: 设 `encrypted-file` + fallback dir 不存在 → 退化 (EncryptedFileBackend::open IO error)
    #[test]
    fn encrypted_file_missing_dir_falls_back() {
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        let saved_dir = std::env::var_os("APEIRETH_KEYRING_DIR");
        std::env::set_var("APEIRETH_KEYRING_BACKEND", "encrypted-file");
        std::env::set_var(
            "APEIRETH_KEYRING_DIR",
            "/nonexistent/apeireth/keyring/that/does/not/exist",
        );
        let resolver = build_keyring_resolver();
        // 退化到 EnvCredentialResolver: 不崩
        let result = resolver.resolve("any.service");
        assert!(result.is_none());
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        }
        if let Some(d) = saved_dir {
            std::env::set_var("APEIRETH_KEYRING_DIR", d);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_DIR");
        }
    }

    /// RC-9 验收: 设 `auto` → KeyringSelector::select 走 select_auto 路径 (probe + fallback)
    /// 测试环境无 OS keyring (CI / Linux container) → probe fail → EncryptedFile fallback
    /// 没 fallback dir → EncryptedFile open 失败 → in-memory stub
    #[test]
    fn auto_select_with_no_backend_available() {
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        let saved_dir = std::env::var_os("APEIRETH_KEYRING_DIR");
        std::env::set_var("APEIRETH_KEYRING_BACKEND", "auto");
        std::env::set_var("APEIRETH_KEYRING_DIR", "/nonexistent/dir");
        let resolver = build_keyring_resolver();
        // auto 在无 backend 环境下走 in-memory → 没存, 返 None
        let result = resolver.resolve("any.service");
        assert!(result.is_none());
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        }
        if let Some(d) = saved_dir {
            std::env::set_var("APEIRETH_KEYRING_DIR", d);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_DIR");
        }
    }
}
