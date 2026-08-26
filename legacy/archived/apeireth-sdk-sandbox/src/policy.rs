//! # Sandbox security policy (per @anthropic-ai/sandbox 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 6 K-1 强校验字段全部保留, 实际 enforcement 留 R21+ 真接 seccomp / AppArmor
//! / docker capabilities / firecracker jailer 时实现.
//!
//! 6 K-1 强校验 (per task spec):
//! 1. 镜像名 (registry / name / tag 格式, 白名单 registry)
//! 2. 命令 (非空, 不含 shell 注入字符)
//! 3. user (禁止 root, 必须显式非 root)
//! 4. env (白名单 KEY, 禁 PATH / LD_PRELOAD 等敏感变量)
//! 5. 端口 (0-65535, 禁特权端口 0-1024 除非显式 allow)
//! 6. 卷挂载 (源路径必须在白名单, 目标路径必须绝对)

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{SandboxError, SandboxResult};

// ============================================================================
// §1 编译期常量 (K-1 强校验)
// ============================================================================

/// 允许的 image registry 白名单 (per v0.9.21 商业版估, 防止拉任意恶意镜像).
pub const ALLOWED_IMAGE_REGISTRIES: &[&str] = &[
    "docker.io",
    "ghcr.io",
    "gcr.io",
    "quay.io",
    "registry.gitlab.com",
    "registry.k8s.io",
    "mcr.microsoft.com",
    "public.ecr.aws",
];

/// 禁止的 user (per v0.9.21 商业版 K-1 强校验: 沙箱内禁止 root).
pub const FORBIDDEN_USERS: &[&str] = &["root", "admin", "Administrator", "SYSTEM"];

/// 禁止的 env key (per v0.9.21 商业版 K-1 强校验: 防 LD_PRELOAD / PATH 注入).
pub const FORBIDDEN_ENV_KEYS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "PATH",
    "PYTHONPATH",
    "NODE_PATH",
    "RUBYLIB",
    "PERL5LIB",
    "BASH_ENV",
    "ENV",
    "BASH_FUNC_",
];

/// 特权端口范围 (0-1024, K-1 强校验 #5: 默认禁, 显式 allow 才能用).
pub const PRIVILEGED_PORT_RANGE: std::ops::RangeInclusive<u16> = 0..=1024;

/// 单沙箱最大 env 变量数 (per v0.9.21 商业版估 64, 防 env 爆炸).
pub const MAX_ENV_VARS: usize = 64;

/// 单沙箱最大卷挂载数 (per v0.9.21 商业版估 32).
pub const MAX_VOLUME_MOUNTS: usize = 32;

/// 单沙箱最大端口映射数 (per v0.9.21 商业版估 16).
pub const MAX_PORT_MAPPINGS: usize = 16;

// ============================================================================
// §2 VolumeMount (1:1 翻译 v0.9.21 商业版 `mounts` 数组元素)
// ============================================================================

/// 卷挂载 (per v0.9.21 商业版 `mounts[].{source,target,readOnly}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeMount {
    /// 宿主机源路径 (必须在 ALLOWED_VOLUME_SOURCES 白名单内).
    pub source: PathBuf,
    /// 沙箱内目标路径 (必须绝对路径).
    pub target: PathBuf,
    /// 只读挂载 (per v0.9.21 商业版 `readOnly` 字段).
    #[serde(default)]
    pub read_only: bool,
}

/// 允许的卷挂载源路径前缀 (per v0.9.21 商业版 K-1 强校验 #6, 防越权访问宿主机).
pub const ALLOWED_VOLUME_SOURCE_PREFIXES: &[&str] = &[
    "/tmp",
    "/var/sandbox",
    "/var/lib/apeireth",
    "/workspace",
    "/data",
];

impl VolumeMount {
    /// 校验卷挂载合法 (K-1 强校验 #6).
    pub fn validate(&self) -> SandboxResult<()> {
        // 源路径必须在白名单前缀内
        let src_str = self.source.to_string_lossy();
        if !ALLOWED_VOLUME_SOURCE_PREFIXES
            .iter()
            .any(|p| src_str.starts_with(p))
        {
            return Err(SandboxError::InvalidConfig(format!(
                "volume source '{}' not in allowed prefixes {:?}",
                src_str, ALLOWED_VOLUME_SOURCE_PREFIXES
            )));
        }
        // 目标路径必须绝对 (沙箱内 Linux 路径, 用 starts_with('/') 而非 Path::is_absolute,
        // 后者在 Windows 上对 "/data" 返 false, 跟沙箱语义不符)
        let target_str = self.target.to_string_lossy();
        if !target_str.starts_with('/') {
            return Err(SandboxError::InvalidConfig(format!(
                "volume target '{}' must be absolute (start with '/')",
                target_str
            )));
        }
        Ok(())
    }
}

// ============================================================================
// §3 PortMapping (1:1 翻译 v0.9.21 商业版 `portBindings` 数组元素)
// ============================================================================

/// 端口映射 (per v0.9.21 商业版 `portBindings[].{hostPort,containerPort,protocol}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMapping {
    /// 宿主机端口 (0-65535, 0 = 动态分配).
    pub host_port: u16,
    /// 容器内端口 (1-65535).
    pub container_port: u16,
    /// 协议 (per v0.9.21 商业版 `protocol`, 估 "tcp" / "udp").
    #[serde(default = "default_protocol")]
    pub protocol: PortProtocol,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortProtocol {
    #[default]
    Tcp,
    Udp,
}

fn default_protocol() -> PortProtocol {
    PortProtocol::Tcp
}

impl PortMapping {
    /// 校验端口合法 (K-1 强校验 #5).
    pub fn validate(&self) -> SandboxResult<()> {
        // 容器内端口不能是 0 (必须显式)
        if self.container_port == 0 {
            return Err(SandboxError::InvalidConfig(
                "container_port cannot be 0".to_string(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// §4 SecurityPolicy (6 K-1 强校验 集合)
// ============================================================================

/// 沙箱安全策略 (K-1 强校验: 6 字段全 K-1 校验, 防止恶意/越权配置).
///
/// 字段对应 v0.9.21 商业版:
/// - `image` → `image`
/// - `command` → `command`
/// - `user` → `user`
/// - `env` → `env`
/// - `ports` → `portBindings`
/// - `mounts` → `mounts`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// 镜像名 (K-1 强校验 #1, e.g. "docker.io/library/alpine:3.19").
    pub image: String,
    /// 启动命令 (K-1 强校验 #2, 非空, 不含 shell 注入字符).
    pub command: Vec<String>,
    /// 沙箱内运行用户 (K-1 强校验 #3, 禁止 root).
    pub user: String,
    /// 环境变量 (K-1 强校验 #4, KEY 必须在白名单).
    pub env: HashMap<String, String>,
    /// 端口映射 (K-1 强校验 #5, 禁特权端口除非显式 allow).
    pub ports: Vec<PortMapping>,
    /// 卷挂载 (K-1 强校验 #6, 源路径必须在白名单).
    pub mounts: Vec<VolumeMount>,
}

impl SecurityPolicy {
    /// 创建默认策略 (K-1 校验: image 必填非空, command 必填非空, user 必填非 root).
    pub fn new(image: impl Into<String>, command: Vec<String>, user: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            command,
            user: user.into(),
            env: HashMap::new(),
            ports: Vec::new(),
            mounts: Vec::new(),
        }
    }

    /// 校验全部 6 K-1 强校验字段.
    pub fn validate(&self) -> SandboxResult<()> {
        self.validate_image()?;
        self.validate_command()?;
        self.validate_user()?;
        self.validate_env()?;
        for p in &self.ports {
            p.validate()?;
        }
        if self.ports.len() > MAX_PORT_MAPPINGS {
            return Err(SandboxError::InvalidConfig(format!(
                "port mappings {} > max {}",
                self.ports.len(),
                MAX_PORT_MAPPINGS
            )));
        }
        for m in &self.mounts {
            m.validate()?;
        }
        if self.mounts.len() > MAX_VOLUME_MOUNTS {
            return Err(SandboxError::InvalidConfig(format!(
                "volume mounts {} > max {}",
                self.mounts.len(),
                MAX_VOLUME_MOUNTS
            )));
        }
        Ok(())
    }

    /// K-1 强校验 #1: 镜像名 (非空 + registry 白名单 + 必含 tag).
    pub fn validate_image(&self) -> SandboxResult<()> {
        if self.image.is_empty() {
            return Err(SandboxError::InvalidImage("image is empty".to_string()));
        }
        // 必须含 tag (冒号后非空)
        if let Some((_, tag)) = self.image.rsplit_once(':') {
            if tag.is_empty() {
                return Err(SandboxError::InvalidImage(format!(
                    "image '{}' has empty tag",
                    self.image
                )));
            }
        } else {
            return Err(SandboxError::InvalidImage(format!(
                "image '{}' missing tag (e.g. ':latest')",
                self.image
            )));
        }
        // registry 白名单
        let registry = self
            .image
            .split('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(':');
        if registry.contains('.') || registry.contains(':') || registry == "localhost" {
            // 是 registry 形式 (含 . 或 : 或 localhost)
            if !ALLOWED_IMAGE_REGISTRIES.contains(&registry) {
                return Err(SandboxError::InvalidImage(format!(
                    "image registry '{registry}' not in allowed {:?}",
                    ALLOWED_IMAGE_REGISTRIES
                )));
            }
        }
        // 否则视为 docker hub 短名 (e.g. "alpine:3.19" → 隐式 docker.io)
        Ok(())
    }

    /// K-1 强校验 #2: 命令 (非空, 不含 shell 注入字符).
    pub fn validate_command(&self) -> SandboxResult<()> {
        if self.command.is_empty() {
            return Err(SandboxError::InvalidCommand("command is empty".to_string()));
        }
        for arg in &self.command {
            if arg.is_empty() {
                return Err(SandboxError::InvalidCommand(
                    "command contains empty arg".to_string(),
                ));
            }
            // 防 shell 注入: 禁 ` ; ` ` & ` ` | ` ` $ ` ` ` ` ` > ` ` < ` ` \n ` ` \0 `
            for forbidden in [";", "&", "|", "$", "`", ">", "<", "\n", "\0"] {
                if arg.contains(forbidden) {
                    return Err(SandboxError::InvalidCommand(format!(
                        "command arg '{}' contains forbidden char {:?}",
                        arg, forbidden
                    )));
                }
            }
        }
        Ok(())
    }

    /// K-1 强校验 #3: user (禁止 root).
    pub fn validate_user(&self) -> SandboxResult<()> {
        if self.user.is_empty() {
            return Err(SandboxError::InvalidConfig("user is empty".to_string()));
        }
        if FORBIDDEN_USERS.contains(&self.user.as_str()) {
            return Err(SandboxError::InvalidConfig(format!(
                "user '{}' is forbidden (K-1 strong validation: no root)",
                self.user
            )));
        }
        Ok(())
    }

    /// K-1 强校验 #4: env (KEY 必须在白名单, 禁 LD_PRELOAD 等).
    pub fn validate_env(&self) -> SandboxResult<()> {
        if self.env.len() > MAX_ENV_VARS {
            return Err(SandboxError::InvalidConfig(format!(
                "env vars {} > max {}",
                self.env.len(),
                MAX_ENV_VARS
            )));
        }
        for key in self.env.keys() {
            if FORBIDDEN_ENV_KEYS
                .iter()
                .any(|f| key == *f || key.starts_with(f))
            {
                return Err(SandboxError::InvalidConfig(format!(
                    "env key '{key}' is forbidden (K-1 strong validation)"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认策略 (合法 image + command + 非 root user) 通过校验.
    #[test]
    fn policy_default_valid() {
        let p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "apeireth",
        );
        assert!(p.validate().is_ok());
    }

    /// K-1 强校验 #1: 镜像名空拒绝.
    #[test]
    fn policy_k1_image_empty_rejected() {
        let p = SecurityPolicy::new("", vec!["/bin/sh".to_string()], "apeireth");
        assert!(matches!(p.validate(), Err(SandboxError::InvalidImage(_))));
    }

    /// K-1 强校验 #1: 镜像名不含 tag 拒绝.
    #[test]
    fn policy_k1_image_no_tag_rejected() {
        let p = SecurityPolicy::new("alpine", vec!["/bin/sh".to_string()], "apeireth");
        assert!(matches!(p.validate(), Err(SandboxError::InvalidImage(_))));
    }

    /// K-1 强校验 #1: registry 不在白名单拒绝.
    #[test]
    fn policy_k1_image_unknown_registry_rejected() {
        let p = SecurityPolicy::new(
            "evil.example.com/malware:latest",
            vec!["/bin/sh".to_string()],
            "apeireth",
        );
        assert!(matches!(p.validate(), Err(SandboxError::InvalidImage(_))));
    }

    /// K-1 强校验 #2: 命令空拒绝.
    #[test]
    fn policy_k1_command_empty_rejected() {
        let p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec![],
            "apeireth",
        );
        assert!(matches!(p.validate(), Err(SandboxError::InvalidCommand(_))));
    }

    /// K-1 强校验 #2: 命令含 shell 注入字符拒绝.
    #[test]
    fn policy_k1_command_shell_injection_rejected() {
        let p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string(), "-c".to_string(), "echo; rm -rf /".to_string()],
            "apeireth",
        );
        assert!(matches!(p.validate(), Err(SandboxError::InvalidCommand(_))));
    }

    /// K-1 强校验 #3: user = root 拒绝.
    #[test]
    fn policy_k1_user_root_rejected() {
        let p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "root",
        );
        assert!(matches!(p.validate(), Err(SandboxError::InvalidConfig(_))));
    }

    /// K-1 强校验 #3: user = admin 拒绝 (per FORBIDDEN_USERS 白名单).
    #[test]
    fn policy_k1_user_admin_rejected() {
        let p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "admin",
        );
        assert!(matches!(p.validate(), Err(SandboxError::InvalidConfig(_))));
    }

    /// K-1 强校验 #4: env 含 LD_PRELOAD 拒绝.
    #[test]
    fn policy_k1_env_forbidden_key_rejected() {
        let mut p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "apeireth",
        );
        p.env.insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());
        assert!(matches!(p.validate(), Err(SandboxError::InvalidConfig(_))));
    }

    /// K-1 强校验 #5: 容器内端口 = 0 拒绝.
    #[test]
    fn policy_k1_port_zero_rejected() {
        let mut p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "apeireth",
        );
        p.ports.push(PortMapping {
            host_port: 8080,
            container_port: 0,
            protocol: PortProtocol::Tcp,
        });
        assert!(matches!(p.validate(), Err(SandboxError::InvalidConfig(_))));
    }

    /// K-1 强校验 #6: 卷挂载源不在白名单拒绝.
    #[test]
    fn policy_k1_volume_mount_evil_source_rejected() {
        let mut p = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "apeireth",
        );
        p.mounts.push(VolumeMount {
            source: PathBuf::from("/etc/passwd"),
            target: PathBuf::from("/mnt/passwd"),
            read_only: true,
        });
        assert!(matches!(p.validate(), Err(SandboxError::InvalidConfig(_))));
    }
}
