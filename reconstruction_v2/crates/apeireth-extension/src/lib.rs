//! apeireth-extension — 6 类插件 + extension.toml 严格 schema + 审核后注册 + 沙盒 + 调用审计
//!
//! v2 style rewrite with v1 pub API surface 100% covered.
//!
//! ## 6 类插件 (按 *执行语义* 分类)
//! 1. **Sync**        — 同步执行, 调用方阻塞等待结果
//! 2. **Async**       — 异步执行, 返回 `Future`
//! 3. **Static**      — 启动期一次性加载, 不可热替换
//! 4. **Service**     — 长驻 service, 启动后持续提供能力
//! 5. **MessagePreprocessor** — 消息中间件, 在消息路由前 transform
//! 6. **Hybrid**      — 同步入口 + 异步后端, 内部状态机切换
//!
//! ## 关键流程
//! 1. `extension.toml` 解析 → [`Manifest`] (严格 schema, 任何字段缺失/类型错即失败)
//! 2. [`audit::audit_manifest`] 审核 (schema + permissions + size limits + name uniqueness)
//! 3. 审核通过 → [`registry::AuditRegistry::register`] 才生效
//! 4. 调用走 [`registry::AuditRegistry::call`] → [`sandbox::Sandbox::check`] → 执行 → [`audit::AuditLog`]
//!
//! ## 不修改 LOCKED
//! - docs/stage1/inspiration-stage1-2026-07-30.md (LOCKED)
//! - docs/stage2/stage2-decisions-*.md (LOCKED)
//! - docs/stage3-blueprints/*.md (LOCKED)
//! - docs/stage4/architecture-*.md (LOCKED)
//! - docs/stage5/stage5-construction-document.md (LOCKED)

pub mod audit;
pub mod error;
pub mod manifest;
pub mod plugins;
pub mod registry;
pub mod sandbox;
pub mod traits;
pub mod types;

pub use error::{ExtensionError, Result};
pub use manifest::Manifest;
pub use plugins::{
    AsyncPlugin, HybridPlugin, MessagePreprocessorPlugin, ServicePlugin, StaticPlugin, SyncPlugin,
};
pub use registry::{AuditRegistry, RegistryStats};
pub use sandbox::{Permission, Sandbox, SandboxConfig};
pub use traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
pub use types::{AuditEntry, PluginKind};

/// crate 版本 (与 workspace.version 同步)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
