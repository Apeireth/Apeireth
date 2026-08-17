//! `apeireth-companion::sandbox` — 沙盒参数口 (B3 沙盒包参数化 + S1 最小权限执行).
//!
//! 三层结构 (集成而非分立):
//! - [`SandboxConfig`]: 内存/CPU/超时 + 完整性级别 + deny-only SID + 目录 ACL 根 +
//!   可选 AppContainer 档. 从套件清单 (`suites.rs` SuiteDef.sandbox) 或权限包
//!   (`packs.rs` PermissionPack.sandbox) 可配; 非法参数回退默认并 eprintln 记录.
//! - [`SandboxBackend`] trait: 物理隔离后端参数口 (B3 方向 ③, Sandboxie/landlock 留口).
//! - 限额生效机制: Windows 侧由 `job_object.rs` (Job Object EXTENDED_LIMIT +
//!   CPU rate control + 超限留痕) 消费本配置; 非 Windows 为 no-op (诚实标注).
//!
//! **S1 加固**: [`IntegrityLevel`] / [`WellKnownSid`] 与 [`crate::restricted_token`]
//! [`crate::directory_acl`] [`crate::app_container`] 共同补"Job Object 之外的权限洞" —
//! 子进程原本仍持父进程完整 token, S1 用 `CreateRestrictedToken` 去特权+deny-only SID,
//! 设 `TokenIntegrityLevel` (low/medium 可配), 以及对工具沙盒根目录应用 read-only DACL
//! (与 `apeireth-tool-filesystem` 的 `APEIRETH_TOOL_FS_ROOTS` 协作).
//!
//! 0 装 PASS 红线: Sandboxie (第三方软件, Windows) 与 landlock (Linux 内核 LSM) 与
//! AppContainer (高危档, 主管 AppContainer profile 复杂) 均**只留 trait 口, 未接实现** —
//! `available()` 如实返回 false, `status()` 说明原因.

/// 默认单次调用超时 (秒; 与 tool_bridge 历史硬编码一致).
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 进程完整性级别 (Windows Mandatory Integrity Control, MIC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityLevel {
    /// 不强制设完整性 (worker 继承父进程值). 默认 — 0 装 PASS.
    Untrusted,
    /// 低完整性 (SECURITY_MANDATORY_LOW_RID = 4096). 沙盒首选.
    Low,
    /// 中完整性 (SECURITY_MANDATORY_MEDIUM_RID = 8192). 与标准用户同档.
    Medium,
}

impl IntegrityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrityLevel::Untrusted => "untrusted",
            IntegrityLevel::Low => "low",
            IntegrityLevel::Medium => "medium",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "low" => IntegrityLevel::Low,
            "medium" | "med" => IntegrityLevel::Medium,
            _ => IntegrityLevel::Untrusted,
        }
    }
}

/// 常用 Well-Known SID 标识 (deny-only 清单里指向"绝不要的组").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WellKnownSid {
    BuiltinAdministrators,
    World,
    AuthenticatedUser,
    Interactive,
}

impl WellKnownSid {
    pub fn as_str(&self) -> &'static str {
        match self {
            WellKnownSid::BuiltinAdministrators => "BUILTIN\\Administrators",
            WellKnownSid::World => "WORLD",
            WellKnownSid::AuthenticatedUser => "NT AUTHORITY\\Authenticated User",
            WellKnownSid::Interactive => "NT AUTHORITY\\INTERACTIVE",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "builtin\\administrators" | "administrators" | "builtin_admin" => {
                Some(WellKnownSid::BuiltinAdministrators)
            }
            "world" => Some(WellKnownSid::World),
            "authenticated_user" | "authenticated" | "au" => Some(WellKnownSid::AuthenticatedUser),
            "interactive" => Some(WellKnownSid::Interactive),
            _ => None,
        }
    }
}

/// 沙盒资源参数 (per-call 隔离 worker 的资源上限).
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxConfig {
    /// 内存上限 (MB, 整个 worker 进程树的 committed 内存).
    pub memory_limit_mb: Option<u64>,
    /// CPU 限速 (1-100, 占一个逻辑核的百分比硬上限).
    pub cpu_percent: Option<u32>,
    /// CPU 时间上限 (秒, 单个 worker 进程 user-mode 累计 CPU 时间).
    pub cpu_time_secs: Option<u64>,
    /// 单次调用超时 (秒). 默认 [`DEFAULT_TIMEOUT_SECS`].
    pub timeout_secs: u64,
    /// S1: 强制覆盖 worker 完整性级别 (None = 不强制, 走 Job Object 仅加固).
    pub integrity_level: Option<IntegrityLevel>,
    /// S1: deny-only SID 清单 (从父 token 剥离这些组, 防止权限膨胀).
    pub deny_only_sids: Vec<WellKnownSid>,
    /// S1: 工具沙盒根目录 ACL 收紧 (read-only DACL).
    pub directory_acl_roots: Vec<std::path::PathBuf>,
    /// S1: 可选 AppContainer 档 (高危, 0 装默认 false).
    pub use_app_container: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: None,
            cpu_percent: None,
            cpu_time_secs: None,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            integrity_level: None,
            deny_only_sids: Vec::new(),
            directory_acl_roots: Vec::new(),
            use_app_container: false,
        }
    }
}

impl SandboxConfig {
    /// 是否配置了任何资源限额 (内存/CPU). 无 → Job Object 只做生命周期加固.
    pub fn has_limits(&self) -> bool {
        self.memory_limit_mb.is_some() || self.cpu_percent.is_some() || self.cpu_time_secs.is_some()
    }

    /// S1: 是否启用了任何权限加固项 (完整性级别 / deny-only SID / 目录 ACL / AppContainer).
    pub fn has_privilege_hardening(&self) -> bool {
        self.integrity_level.is_some()
            || !self.deny_only_sids.is_empty()
            || !self.directory_acl_roots.is_empty()
            || self.use_app_container
    }

    /// 参数口: 从 JSON 解析 (套件清单/权限包的配置载体).
    ///
    /// 非法参数**回退默认**并 eprintln 记录 (不静默, 不阻断执行).
    pub fn from_json(v: &serde_json::Value) -> Self {
        let warn = |field: &str, why: &str| {
            eprintln!("[sandbox] 配置 {field} 非法 ({why}), 回退默认值");
        };
        let mut cfg = Self::default();
        if let Some(m) = v.get("memory_limit_mb") {
            match m.as_u64() {
                Some(mb) if mb > 0 => cfg.memory_limit_mb = Some(mb),
                Some(_) => warn("memory_limit_mb", "为 0"),
                None => warn("memory_limit_mb", "非正整数"),
            }
        }
        if let Some(p) = v.get("cpu_percent") {
            match p.as_u64() {
                Some(pct) if (1..=100).contains(&pct) => cfg.cpu_percent = Some(pct as u32),
                Some(_) => warn("cpu_percent", "不在 1..=100"),
                None => warn("cpu_percent", "非正整数"),
            }
        }
        if let Some(t) = v.get("cpu_time_secs") {
            match t.as_u64() {
                Some(s) if s > 0 => cfg.cpu_time_secs = Some(s),
                Some(_) => warn("cpu_time_secs", "为 0"),
                None => warn("cpu_time_secs", "非正整数"),
            }
        }
        if let Some(t) = v.get("timeout_secs") {
            match t.as_u64() {
                Some(s) if s > 0 => cfg.timeout_secs = s,
                Some(_) => warn("timeout_secs", "为 0, 用默认 30s"),
                None => warn("timeout_secs", "非正整数, 用默认 30s"),
            }
        }
        if let Some(s) = v.get("integrity_level").and_then(|x| x.as_str()) {
            cfg.integrity_level = Some(IntegrityLevel::parse(s));
        }
        if let Some(arr) = v.get("deny_only_sids").and_then(|x| x.as_array()) {
            for (i, item) in arr.iter().enumerate() {
                if let Some(s) = item.as_str() {
                    match WellKnownSid::parse(s) {
                        Some(sid) => cfg.deny_only_sids.push(sid),
                        None => warn("deny_only_sids", &format!("第 {i} 项 `{s}` 不识别, 跳过")),
                    }
                } else {
                    warn("deny_only_sids", &format!("第 {i} 项非字符串, 跳过"));
                }
            }
        }
        if let Some(arr) = v.get("directory_acl_roots").and_then(|x| x.as_array()) {
            for (i, item) in arr.iter().enumerate() {
                if let Some(s) = item.as_str() {
                    if s.is_empty() {
                        warn("directory_acl_roots", &format!("第 {i} 项为空, 跳过"));
                    } else {
                        cfg.directory_acl_roots.push(std::path::PathBuf::from(s));
                    }
                } else {
                    warn("directory_acl_roots", &format!("第 {i} 项非字符串, 跳过"));
                }
            }
        }
        if let Some(b) = v.get("use_app_container").and_then(|x| x.as_bool()) {
            if b {
                eprintln!(
                    "[sandbox] use_app_container=true 已记录, 但 AppContainer 档 trait 口 0 装 PASS \
                     (当前不会强制启用, 留作后续高危场景扩展)"
                );
            }
            cfg.use_app_container = b;
        }
        cfg
    }
}

/// 物理隔离后端参数口 (B3 方向 ③): 平台级沙盒机制的统一接口.
pub trait SandboxBackend {
    fn name(&self) -> &'static str;
    fn available(&self) -> bool;
    fn status(&self) -> &'static str;
    fn render_params(&self, cfg: &SandboxConfig) -> Vec<String>;
}

/// Sandboxie-Plus 参数口 (Windows 第三方沙盒软件).
pub struct SandboxieBackend;

impl SandboxBackend for SandboxieBackend {
    fn name(&self) -> &'static str {
        "Sandboxie-Plus"
    }
    fn available(&self) -> bool {
        false
    }
    fn status(&self) -> &'static str {
        "trait 口已备, 未接: Windows 侧已由 Job Object 覆盖 (job_object.rs); \
         Sandboxie 为第三方软件栈 (Start.exe 包裹 worker), 接入属后续增强"
    }
    fn render_params(&self, cfg: &SandboxConfig) -> Vec<String> {
        vec![
            "Start.exe".to_string(),
            "/box:ApeirethWorker".to_string(),
            "/wait".to_string(),
            format!("timeout_secs={}", cfg.timeout_secs),
        ]
    }
}

/// landlock 参数口 (Linux 内核 LSM, 文件系统 syscall 沙盒).
pub struct LandlockBackend;

impl SandboxBackend for LandlockBackend {
    fn name(&self) -> &'static str {
        "landlock"
    }
    fn available(&self) -> bool {
        false
    }
    fn status(&self) -> &'static str {
        "trait 口已备, 未接: landlock 是 Linux 专属内核机制 (Windows 无对应物); \
         真接点为 worker 进程内 landlock(2) 自约束, Linux 实现属后续工作"
    }
    fn render_params(&self, _cfg: &SandboxConfig) -> Vec<String> {
        Vec::new()
    }
}

/// 平台后端清单 (诚实口径: 当前两个后端均未接, Windows 限额走 Job Object).
pub fn backends() -> Vec<Box<dyn SandboxBackend>> {
    vec![Box::new(SandboxieBackend), Box::new(LandlockBackend)]
}

/// S1 准备结果: 受限 token + 目录 ACL 守卫.
/// 持有至 worker 退出 (Drop 自动还原 ACL + CloseHandle token).
#[cfg(windows)]
pub struct PreparedChild {
    pub token: Option<crate::restricted_token::RestrictedToken>,
    pub dir_acl: Option<crate::directory_acl::DirAclGuard>,
}

#[cfg(windows)]
impl std::fmt::Debug for PreparedChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedChild")
            .field("token", &self.token.is_some())
            .field("dir_acl", &self.dir_acl.is_some())
            .finish()
    }
}

#[cfg(not(windows))]
#[derive(Debug)]
pub struct PreparedChild {
    pub token: Option<crate::restricted_token::RestrictedToken>,
    pub dir_acl: Option<crate::directory_acl::DirAclGuard>,
}

/// S1 阶段前置: 受限 token + 目录 ACL 一次性准备 (用于 spawn worker 之前).
///
/// **失败语义**: 加固失败**不阻断执行** (加固是增强不是门) — 上层
/// eprintln 记录后继续原 token / 无 ACL 收紧.
///
/// 跨平台: needs_hardening 但非 Windows 时直返 Err (0 装 PASS); 调用方按需降级.
#[cfg(windows)]
pub fn prepare_child(cfg: &SandboxConfig) -> Result<PreparedChild, String> {
    let mut token = None;
    let mut dir_acl = None;

    if cfg.integrity_level.is_some() || !cfg.deny_only_sids.is_empty() {
        let rt_cfg = crate::restricted_token::RestrictedTokenConfig::from_sandbox(cfg);
        match crate::restricted_token::create_restricted_token(&rt_cfg) {
            Ok(t) => token = Some(t),
            Err(e) => eprintln!("[sandbox] 受限 token 创建失败 (降级为原 token): {e}"),
        }
    }

    if !cfg.directory_acl_roots.is_empty() {
        let dir_cfg = crate::directory_acl::DirAclConfig::from_sandbox(cfg);
        match crate::directory_acl::apply_read_only_acl(&dir_cfg) {
            Ok(g) => dir_acl = Some(g),
            Err(e) => eprintln!("[sandbox] 目录 ACL 收紧失败 (降级为无 ACL): {e}"),
        }
    }

    Ok(PreparedChild { token, dir_acl })
}

/// 跨平台: needs_hardening 时返 Err (0 装 PASS), 否则返空 stub.
#[cfg(not(windows))]
pub fn prepare_child(cfg: &SandboxConfig) -> Result<PreparedChild, String> {
    if cfg.has_privilege_hardening() {
        return Err("S1 prepare_child: 非 Windows 平台未实现 (0 装 PASS, 走 no-op)".to_string());
    }
    Ok(PreparedChild {
        token: crate::restricted_token::create_restricted_token(&crate::restricted_token::RestrictedTokenConfig {
            integrity_level: None,
            deny_only_sids: Vec::new(),
            default_dacl_open: false,
            app_container_roots: Vec::new(),
        })
        .ok(),
        dir_acl: crate::directory_acl::apply_read_only_acl(&crate::directory_acl::DirAclConfig::default()).ok(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_is_unlimited_with_30s_timeout() {
        let c = SandboxConfig::default();
        assert!(!c.has_limits());
        assert!(!c.has_privilege_hardening());
        assert_eq!(c.timeout_secs, DEFAULT_TIMEOUT_SECS);
        assert_eq!(c.integrity_level, None);
        assert!(c.deny_only_sids.is_empty());
        assert!(c.directory_acl_roots.is_empty());
        assert!(!c.use_app_container);
    }

    #[test]
    fn from_json_parses_valid_config() {
        let c = SandboxConfig::from_json(&json!({
            "memory_limit_mb": 512, "cpu_percent": 50, "cpu_time_secs": 10, "timeout_secs": 60
        }));
        assert_eq!(c.memory_limit_mb, Some(512));
        assert_eq!(c.cpu_percent, Some(50));
        assert_eq!(c.cpu_time_secs, Some(10));
        assert_eq!(c.timeout_secs, 60);
        assert!(c.has_limits());
    }

    #[test]
    fn from_json_invalid_falls_back_to_default() {
        let c = SandboxConfig::from_json(&json!({
            "memory_limit_mb": 0, "cpu_percent": 200, "cpu_time_secs": -3, "timeout_secs": "abc"
        }));
        assert_eq!(c, SandboxConfig::default());
    }

    #[test]
    fn from_json_empty_is_default() {
        assert_eq!(SandboxConfig::from_json(&json!({})), SandboxConfig::default());
        assert_eq!(SandboxConfig::from_json(&json!(null)), SandboxConfig::default());
    }

    #[test]
    fn backends_are_honest_not_available() {
        for b in backends() {
            assert!(!b.available(), "{} 未接必须如实返回 false", b.name());
            assert!(b.status().contains("未接"), "{} 状态须诚实标注未接", b.name());
        }
    }

    #[test]
    fn sandboxie_renders_param_template() {
        let p = SandboxieBackend.render_params(&SandboxConfig { timeout_secs: 60, ..Default::default() });
        assert!(p.iter().any(|s| s.contains("timeout_secs=60")));
        assert!(LandlockBackend.render_params(&SandboxConfig::default()).is_empty());
    }

    #[test]
    fn integrity_level_parses_known_values() {
        assert_eq!(IntegrityLevel::parse("low"), IntegrityLevel::Low);
        assert_eq!(IntegrityLevel::parse("LOW"), IntegrityLevel::Low);
        assert_eq!(IntegrityLevel::parse("Medium"), IntegrityLevel::Medium);
        assert_eq!(IntegrityLevel::parse("med"), IntegrityLevel::Medium);
        assert_eq!(IntegrityLevel::parse("untrusted"), IntegrityLevel::Untrusted);
        assert_eq!(IntegrityLevel::parse("high"), IntegrityLevel::Untrusted);
        assert_eq!(IntegrityLevel::parse("system"), IntegrityLevel::Untrusted);
        assert_eq!(IntegrityLevel::parse("garbage"), IntegrityLevel::Untrusted);
    }

    #[test]
    fn integrity_level_as_str_is_stable() {
        assert_eq!(IntegrityLevel::Low.as_str(), "low");
        assert_eq!(IntegrityLevel::Medium.as_str(), "medium");
        assert_eq!(IntegrityLevel::Untrusted.as_str(), "untrusted");
    }

    #[test]
    fn wellknown_sid_roundtrips_through_parse() {
        assert_eq!(WellKnownSid::parse("BUILTIN\\Administrators"), Some(WellKnownSid::BuiltinAdministrators));
        assert_eq!(WellKnownSid::parse("administrators"), Some(WellKnownSid::BuiltinAdministrators));
        assert_eq!(WellKnownSid::parse("world"), Some(WellKnownSid::World));
        assert_eq!(WellKnownSid::parse("authenticated_user"), Some(WellKnownSid::AuthenticatedUser));
        assert_eq!(WellKnownSid::parse("interactive"), Some(WellKnownSid::Interactive));
        assert_eq!(WellKnownSid::parse("garbage"), None);
    }

    #[test]
    fn from_json_parses_s1_privilege_hardening() {
        let c = SandboxConfig::from_json(&json!({
            "integrity_level": "low",
            "deny_only_sids": ["BUILTIN\\Administrators", "world"],
            "directory_acl_roots": ["C:\\sandbox\\root"],
            "use_app_container": true,
        }));
        assert_eq!(c.integrity_level, Some(IntegrityLevel::Low));
        assert_eq!(c.deny_only_sids, vec![WellKnownSid::BuiltinAdministrators, WellKnownSid::World]);
        assert_eq!(c.directory_acl_roots, vec![std::path::PathBuf::from("C:\\sandbox\\root")]);
        assert!(c.use_app_container);
        assert!(c.has_privilege_hardening());
    }

    #[test]
    fn from_json_s1_partial_keeps_unset_fields_off() {
        let c = SandboxConfig::from_json(&json!({"integrity_level": "Medium"}));
        assert_eq!(c.integrity_level, Some(IntegrityLevel::Medium));
        assert!(c.deny_only_sids.is_empty());
        assert!(c.directory_acl_roots.is_empty());
        assert!(!c.use_app_container);
    }

    #[test]
    fn from_json_s1_invalid_drops_only_bad_items() {
        let c = SandboxConfig::from_json(&json!({
            "deny_only_sids": ["world", "garbage", "BUILTIN\\Administrators"],
            "directory_acl_roots": ["C:\\valid", "", "C:\\also-valid"],
        }));
        assert_eq!(c.deny_only_sids, vec![WellKnownSid::World, WellKnownSid::BuiltinAdministrators]);
        assert_eq!(
            c.directory_acl_roots,
            vec![std::path::PathBuf::from("C:\\valid"), std::path::PathBuf::from("C:\\also-valid")]
        );
    }

    #[test]
    fn from_json_s1_integrity_garbage_string_falls_back_untrusted() {
        let c = SandboxConfig::from_json(&json!({"integrity_level": "extreme"}));
        assert_eq!(c.integrity_level, Some(IntegrityLevel::Untrusted));
    }

    #[test]
    fn backward_compat_old_json_still_works() {
        let c = SandboxConfig::from_json(&json!({"memory_limit_mb": 256}));
        assert_eq!(c.memory_limit_mb, Some(256));
        assert_eq!(c.integrity_level, None);
        assert!(c.deny_only_sids.is_empty());
        assert!(c.directory_acl_roots.is_empty());
        assert!(!c.use_app_container);
    }

    #[test]
    fn prepare_child_default_is_no_op() {
        // 默认没有任何 harden 项 → 跨平台 prepare_child 路径不应 panic.
        let cfg = SandboxConfig::default();
        let p = prepare_child(&cfg).expect("default cfg 无 harden 应 Ok");
        assert!(p.token.is_none());
        assert!(p.dir_acl.is_none());
    }

    #[test]
    fn prepare_child_with_hardening_fails_off_windows() {
        let cfg = SandboxConfig {
            integrity_level: Some(IntegrityLevel::Low),
            ..Default::default()
        };
        #[cfg(not(windows))]
        {
            let r = prepare_child(&cfg);
            assert!(r.is_err(), "非 Windows 平台请求 hardening 应诚实返 Err");
            let err = r.unwrap_err();
            assert!(err.contains("非 Windows"), "错误信息应诚实标注: {err}");
        }
    }
}
