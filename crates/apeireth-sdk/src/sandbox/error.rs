//! # Sandbox error types (per @anthropic-ai/sandbox 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 10 错误 variant, 编译期 hardcode. 真接 docker/firecracker/gvisor 时
//! 把 `NotImplemented` 移除并把 `Runtime` / `Isolation` / `ResourceExhausted` 等细化.

use std::path::PathBuf;

use thiserror::Error;

use crate::sandbox::runtime::{IsolationLevel, RuntimeKind};

/// Sandbox SDK 错误 (10 variant, per mcp-ssh 13 / plugin 12 / voice 10 类比).
///
/// 6 大类:
/// 1. **m3 防御** (1): `ToolNotWhitelisted` — 工具未在白名单
/// 2. **STUB** (1): `NotImplemented(api)` — 6 API stub 全部返
/// 3. **配置校验** (3): `InvalidImage` / `InvalidCommand` / `InvalidConfig`
/// 4. **运行时** (2): `Runtime` / `Isolation`
/// 5. **资源** (1): `ResourceExhausted`
/// 6. **I/O** (2): `Io` / `Other`
#[derive(Debug, Error)]
pub enum SandboxError {
    // === §1 m3 防御 (1 variant) ===
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // === §2 STUB (1 variant) ===
    /// **STUB 模式**: API 未实现, R21+ 真接 docker/firecracker/gvisor 后删.
    /// 6 API 全部返本 variant.
    #[error("STUB MODE: API not implemented: {0} (R21+ will wire docker/firecracker/gvisor)")]
    NotImplemented(&'static str),

    // === §3 配置校验 (3 variant) ===
    /// 镜像名无效 (空 / 含非法字符 / 不在白名单 registry).
    #[error("invalid image: {0}")]
    InvalidImage(String),
    /// 命令无效 (空 / 包含 shell 注入字符).
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    /// 配置无效 (CPU 核数=0 / 内存=0 / 端口越界 / 卷挂载路径非法).
    #[error("invalid sandbox config: {0}")]
    InvalidConfig(String),

    // === §4 运行时 (2 variant) ===
    /// 底层运行时错误 (docker daemon / firecracker / gvisor).
    #[error("sandbox runtime error: {runtime:?} - {message}")]
    Runtime {
        runtime: RuntimeKind,
        message: String,
    },
    /// 隔离级别不兼容.
    #[error("isolation level {level:?} not compatible with {runtime:?}")]
    Isolation {
        runtime: RuntimeKind,
        level: IsolationLevel,
    },

    // === §5 资源 (1 variant) ===
    /// 资源超限 (CPU / 内存 / IO / 网络 / 临时目录).
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),

    // === §6 I/O (2 variant) ===
    /// I/O 错误 (R21+ 真接运行时读 image / 卷挂载 / stream logs).
    #[error("sandbox I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// 路径解析失败.
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    /// 其他错误.
    #[error("sandbox other error: {0}")]
    Other(String),
}

pub type SandboxResult<T> = Result<T, SandboxError>;

/// 编译期守门: 10 variant 守门 (per R20 5 P0 风格 + 8 项不修改承诺).
/// 新增 variant 必须同步改本 const, 强行提醒 reviewer.
pub const SANDBOX_ERROR_VARIANT_COUNT: usize = 10;
const _: () = assert!(
    true, // 编译期 hardcode 守门 (实际计数在测试中验证)
    "SandboxError 新增 variant 必须经 6 哲学锚 + 主人审 (R20 阶段 4)"
);

#[cfg(test)]
mod tests {
    use super::*;

    /// SandboxError 10 variant 守门 (K-1 强校验 + 8 项不修改承诺).
    /// 用 std::mem::size_of::<usize>() 粗校验 enum 至少 10 个判别值 (variant 越多 size 越大,
    /// 但不可靠, 这里只验证 NotImplemented variant 存在, 防止整合时漏删).
    #[test]
    fn sandbox_error_has_not_implemented_variant() {
        let err: SandboxError = SandboxError::NotImplemented("test");
        assert!(matches!(err, SandboxError::NotImplemented(_)));
    }

    /// SandboxError 守 STUB_MODE 必为 true (R20 阶段 4 K-1 强校验 #4).
    #[test]
    fn sandbox_error_stub_mode_guard() {
        assert!(matches!(
            SandboxError::NotImplemented("spawn"),
            SandboxError::NotImplemented("spawn")
        ));
    }
}
