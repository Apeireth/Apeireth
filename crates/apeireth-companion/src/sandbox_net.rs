//! `apeireth-companion::sandbox_net` — Stage 1 网络隔离 (per B 站 UP 主 5.4).
//!
//! ## 哲学 (主人 2026-08-19: Stage 1 网络隔离落地)
//!
//! 现有 5 层沙盒 (洋葱门 / 审批链 / MOVE-STAY / Job Object / 最小权限) **都是进程卫生**,
//! 不防蠕虫 — UP 主 5.3 论断. 蠕虫杀的不是单个进程, 是**整个网络可达面**.
//!
//! 5.4 正确做法分两条:
//! - 一次性 VM (如 Firecracker / libkrun microVM);
//! - AppContainer + WFP 出站默认拒绝 + 目录虚拟化;
//! - **网络层隔离** (Stage 1 走这条).
//!
//! ## 借鉴 4 源 (0 接库, 仅借鉴接口语义)
//!
//! | 源       | 借鉴                                                         |
//! |----------|--------------------------------------------------------------|
//! | Firecracker | minimal API surface (小 API = 小攻击面)                   |
//! | libkrun  | C lib + Rust binding 分层 (netns + cgroup 接口范式)          |
//! | wasmtime | 组件模型 (capability 边界)                                  |
//! | smolvm   | 0 装诚实 (NoopXxx + `available() = false`)                  |
//!
//! ## 0 装 PASS 红线 (smolvm 模式)
//!
//! Linux 侧 netns + cgroup / Windows 侧 WFP 命名空间 **只留 trait 口** —
//! [`NoopNetworkIsolation`] 如实返回:
//! - `available() = false`
//! - `status()` 明示未实装
//! - `apply_to_child()` 返 `Err` (不假装已隔离, 调用方必须接住)
//!
//! 实装 [`NetnsNetworkIsolation`] (Linux) / [`WfpNetworkIsolation`] (Windows) 时
//! 才真正生效 — 接口契约不变, 仅改实现.
//!
//! ## API 速览
//!
//! - [`NetworkIsolationLevel`]: 4 档 (None / LoopbackOnly / DefaultDenyWithWhitelist / ForceDeny)
//! - [`NetworkIsolationConfig`]: 策略参数 (whitelist / inbound / DNS)
//! - [`NetworkIsolation`] trait: 应用 + 清理
//! - [`NoopNetworkIsolation`]: 0 装 stub (smolvm 风格)
//! - [`default_network_isolation`]: 工厂 (平台检测, 0 装返 Noop)
//! - [`assert_isolated`]: 安全断言 (None 0 强制, 其它档必须 available)
//!
//! ## 与现有沙盒关系
//!
//! 配合 [`crate::sandbox::SandboxConfig`] 消费: 高危工具可同时启用 Job Object +
//! 受限 token + 网络隔离 (S1 + Stage 1 组合). 进程卫生 + 网络可达面 = 两层防线.

use std::process::Command;

/// 网络隔离级别 (4 档 — 借鉴 wasmtime 隔离级别思路).
///
/// **0 装 PASS**: 默认 [`None`] (继承父进程网络可达面). 任何非 None 档必须经过
/// [`assert_isolated`] 严守 — trait 未实装时 `apply_to_child` 必返 `Err`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkIsolationLevel {
    /// 无隔离 (默认, 0 装 PASS). 继承父进程网络可达面.
    None,
    /// 完全断网 — 仅允许本地 loopback (`127.0.0.0/8` / `::1`).
    /// 适合: 纯本地推理 / 文件 IO 工具.
    LoopbackOnly,
    /// 默认 deny 出站 + 白名单放行 (S4 出站策略同款).
    /// 适合: 需要访问特定 API 的工具 (e.g. `api.openai.com`).
    DefaultDenyWithWhitelist,
    /// 强制 deny — 任何出站包丢弃 (含 DNS). debug 用, 用于验证隔离生效.
    ForceDeny,
}

impl NetworkIsolationLevel {
    /// 序列化 (小写稳定字符串 — 写配置文件 / 环境变量均用此格式).
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkIsolationLevel::None => "none",
            NetworkIsolationLevel::LoopbackOnly => "loopback_only",
            NetworkIsolationLevel::DefaultDenyWithWhitelist => "default_deny_with_whitelist",
            NetworkIsolationLevel::ForceDeny => "force_deny",
        }
    }

    /// 反序列化 — 不识别字符串回退 [`None`] (0 装默认, 不假装已隔离).
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "none" | "off" | "disabled" | "" => NetworkIsolationLevel::None,
            "loopback_only" | "loopback" | "lo" => NetworkIsolationLevel::LoopbackOnly,
            "default_deny_with_whitelist" | "default_deny" | "whitelist" | "deny_with_whitelist" => {
                NetworkIsolationLevel::DefaultDenyWithWhitelist
            }
            "force_deny" | "force" | "deny_all" => NetworkIsolationLevel::ForceDeny,
            _ => NetworkIsolationLevel::None, // 0 装默认, 不假装
        }
    }
}

/// 网络隔离配置 (策略参数).
///
/// 与 [`NetworkIsolationLevel`] 配合: `level` 决定主策略, 其它字段在对应档下生效.
/// - `LoopbackOnly` 忽略 whitelist/inbound (除 loopback 外一律断)
/// - `DefaultDenyWithWhitelist` 生效 whitelist (CIDR 或域名)
/// - `ForceDeny` 忽略一切 (debug)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkIsolationConfig {
    /// 主隔离档.
    pub level: NetworkIsolationLevel,
    /// 出站白名单 (CIDR `10.0.0.0/8` 或域名 `api.openai.com`).
    /// 仅 `DefaultDenyWithWhitelist` 档生效.
    pub outbound_whitelist: Vec<String>,
    /// 入站是否允许 (默认 false, 借鉴 libkrun 默认 deny-inbound).
    pub allow_inbound: bool,
    /// DNS 是否允许 (默认 false; 即便 level = DefaultDenyWithWhitelist, DNS 也需显式开).
    /// 关闭时子进程无法解析域名 (但可用 IP 直连).
    pub allow_dns: bool,
}

impl Default for NetworkIsolationConfig {
    fn default() -> Self {
        Self {
            level: NetworkIsolationLevel::None,
            outbound_whitelist: Vec::new(),
            allow_inbound: false,
            allow_dns: false,
        }
    }
}

/// 网络隔离 trait (借鉴 Firecracker minimal API surface).
///
/// **契约 (smolvm 0 装模式)**:
/// - `available()` 必须如实: 未实装时返 `false`, 调用方据此决定是否走 0 装 PASS.
/// - `apply_to_child()` 在未实装时必须返 `Err`, **绝不**返回 `Ok(())` 假装已隔离.
/// - `cleanup()` 在未实装时返 `Ok(())` (无副作用, 清理必然成功).
pub trait NetworkIsolation: Send + Sync + std::fmt::Debug {
    /// 是否实装 (0 装: 返 `false`).
    fn available(&self) -> bool;

    /// 状态描述 (人类可读, 0 装时明示未实装原因).
    fn status(&self) -> String;

    /// 把隔离配置应用到子进程 — 通过 `pre_exec` 或平台 API 注入.
    ///
    /// Linux 计划: `unshare(CLONE_NEWNET)` + `nftables` / `tc` 限制;
    /// Windows 计划: `WFP` (Windows Filtering Platform) 命名空间 + 进程 token.
    ///
    /// **0 装契约**: trait 未实装时必返 `Err`, 不返回 `Ok(())`.
    fn apply_to_child(&self, cmd: &mut Command, cfg: &NetworkIsolationConfig) -> Result<(), String>;

    /// 清理隔离 (子进程退出时调用, 借鉴 libkrun 清理思路).
    ///
    /// **0 装契约**: trait 未实装时返 `Ok(())` (无副作用, 清理无意义).
    fn cleanup(&self) -> Result<(), String>;
}

/// 0 装 stub — 网络隔离未实装时的诚实占位 (smolvm 风格).
///
/// 所有调用方拿到它都必须:
/// - `available()` 返 `false` → 走 0 装 PASS, 不假装已隔离.
/// - `apply_to_child()` 返 `Err` → 调用方必须接住并降级 (拒绝 / 改用更低权限档).
/// - `cleanup()` 返 `Ok(())` → 无副作用, 不污染进程.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopNetworkIsolation;

impl NetworkIsolation for NoopNetworkIsolation {
    fn available(&self) -> bool {
        false
    }

    fn status(&self) -> String {
        "NoopNetworkIsolation: 未实装 (0 装 PASS, 0 假装已隔离; Linux netns+cgroup / Windows WFP 接入后启用)".into()
    }

    fn apply_to_child(&self, _cmd: &mut Command, _cfg: &NetworkIsolationConfig) -> Result<(), String> {
        Err("NoopNetworkIsolation: 网络隔离未实装 (Linux netns + cgroup / Windows WFP 接入后启用)".into())
    }

    fn cleanup(&self) -> Result<(), String> {
        // 0 装无副作用, 清理必然成功 — 不假装 "清理成功" 也无意义.
        Ok(())
    }
}

/// 工厂: 返回当前平台的默认实装 (0 装时回退 [`NoopNetworkIsolation`]).
///
/// **平台分支计划**:
/// - Linux: 检测 `unshare(CLONE_NEWNET)` 可用性 → 返 [`NetnsNetworkIsolation`] (实装时)
/// - Windows: 检测 WFP 可用性 → 返 [`WfpNetworkIsolation`] (实装时)
/// - 任何平台 trait 未实装时: 返 [`NoopNetworkIsolation`] (0 装 PASS 红线).
///
/// **当前状态**: 仅返 Noop (Stage 1 trait 口预留, 实装待下一轮).
pub fn default_network_isolation() -> Box<dyn NetworkIsolation> {
    // 平台检测占位 — 实装时按 `#[cfg(target_os = "linux")]` / `"windows"` 分支.
    // 当前一律返 Noop, 严守 0 装 PASS 红线.
    Box::new(NoopNetworkIsolation)
}

/// 安全断言 — 严守 "0 装 PASS 但绝不假装已隔离" 契约.
///
/// 语义:
/// - `cfg.level = None` → 永返 `Ok` (0 装 PASS, 不强制隔离).
/// - `cfg.level != None` 且 `isol.available() = false` → 返 `Err` (不假装已隔离).
/// - `cfg.level != None` 且 `isol.available() = true` → 返 `Ok` (实装在岗, 调用方继续).
///
/// **借鉴 Firecracker 失败语义透明**: trait 未实装时返 Err 含可行动信息 (不是 silent).
pub fn assert_isolated(cfg: &NetworkIsolationConfig, isol: &dyn NetworkIsolation) -> Result<(), String> {
    if cfg.level == NetworkIsolationLevel::None {
        return Ok(()); // 0 装 PASS: 不强制隔离
    }
    if !isol.available() {
        return Err(format!(
            "NetworkIsolationLevel {:?} 配置但 trait 未实装 (0 装 PASS, 0 假装已隔离); \
             接 NetnsNetworkIsolation (Linux) / WfpNetworkIsolation (Windows) 后再启用",
            cfg.level
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────────────────────────────────────────────────────────
    // Happy path: 4 档 as_str ↔ parse 往返
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn level_as_str_roundtrip_4_variants() {
        let variants = [
            NetworkIsolationLevel::None,
            NetworkIsolationLevel::LoopbackOnly,
            NetworkIsolationLevel::DefaultDenyWithWhitelist,
            NetworkIsolationLevel::ForceDeny,
        ];
        for v in &variants {
            let s = v.as_str();
            let parsed = NetworkIsolationLevel::parse(s);
            assert_eq!(*v, parsed, "roundtrip fail: {v:?} ↔ {s:?}");
        }
    }

    #[test]
    fn level_parse_4_variants() {
        // 直接验证 parse 的别名 (用户写配置时可能用别名)
        assert_eq!(
            NetworkIsolationLevel::parse("none"),
            NetworkIsolationLevel::None
        );
        assert_eq!(
            NetworkIsolationLevel::parse("loopback_only"),
            NetworkIsolationLevel::LoopbackOnly
        );
        assert_eq!(
            NetworkIsolationLevel::parse("loopback"), // 别名
            NetworkIsolationLevel::LoopbackOnly
        );
        assert_eq!(
            NetworkIsolationLevel::parse("default_deny_with_whitelist"),
            NetworkIsolationLevel::DefaultDenyWithWhitelist
        );
        assert_eq!(
            NetworkIsolationLevel::parse("whitelist"), // 别名
            NetworkIsolationLevel::DefaultDenyWithWhitelist
        );
        assert_eq!(
            NetworkIsolationLevel::parse("force_deny"),
            NetworkIsolationLevel::ForceDeny
        );
        assert_eq!(
            NetworkIsolationLevel::parse("deny_all"), // 别名
            NetworkIsolationLevel::ForceDeny
        );
        // 大小写不敏感
        assert_eq!(
            NetworkIsolationLevel::parse("LOOPBACK_ONLY"),
            NetworkIsolationLevel::LoopbackOnly
        );
        // 未知值 → None (0 装默认, 不假装)
        assert_eq!(
            NetworkIsolationLevel::parse("not_a_real_level"),
            NetworkIsolationLevel::None
        );
        // 空字符串 → None
        assert_eq!(
            NetworkIsolationLevel::parse(""),
            NetworkIsolationLevel::None
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // 边界: 默认 0 装
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn default_config_is_none_level() {
        let cfg = NetworkIsolationConfig::default();
        assert_eq!(cfg.level, NetworkIsolationLevel::None);
        assert!(cfg.outbound_whitelist.is_empty());
        assert!(!cfg.allow_inbound);
        assert!(!cfg.allow_dns);
    }

    // ──────────────────────────────────────────────────────────────────
    // 0 装 PASS 红线 — NoopNetworkIsolation 必须诚实
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn noop_network_isolation_available_false() {
        let noop = NoopNetworkIsolation;
        assert!(!noop.available(), "0 装 PASS: available() 必须返 false");
    }

    #[test]
    fn noop_network_isolation_status_explains_no_op() {
        let noop = NoopNetworkIsolation;
        let s = noop.status();
        assert!(s.contains("Noop"), "status 必须含 Noop: {s}");
        assert!(s.contains("未实装"), "status 必须明示未实装: {s}");
        assert!(s.contains("0 假装"), "status 必须含 0 假装: {s}");
    }

    #[test]
    fn noop_apply_to_child_returns_err() {
        let noop = NoopNetworkIsolation;
        let cfg = NetworkIsolationConfig::default();
        let mut cmd = Command::new("echo");
        let res = noop.apply_to_child(&mut cmd, &cfg);
        let err = res.expect_err("0 装 PASS: apply_to_child 必须返 Err, 0 假装已隔离");
        assert!(err.contains("未实装"), "错误必须明示未实装: {err}");
        assert!(err.contains("Noop"), "错误必须含 Noop: {err}");
    }

    #[test]
    fn noop_apply_to_child_strict_level_still_errs() {
        // 即便 cfg.level 是严格档, 0 装 trait 必须返 Err (不假装).
        let noop = NoopNetworkIsolation;
        let cfg = NetworkIsolationConfig {
            level: NetworkIsolationLevel::LoopbackOnly,
            outbound_whitelist: vec!["api.openai.com".into()],
            allow_inbound: false,
            allow_dns: false,
        };
        let mut cmd = Command::new("echo");
        assert!(noop.apply_to_child(&mut cmd, &cfg).is_err());
    }

    #[test]
    fn noop_cleanup_returns_ok() {
        // 0 装无副作用 → 清理必然成功 (空操作无错).
        let noop = NoopNetworkIsolation;
        assert!(noop.cleanup().is_ok());
        // 多次调用仍 Ok
        assert!(noop.cleanup().is_ok());
        assert!(noop.cleanup().is_ok());
    }

    // ──────────────────────────────────────────────────────────────────
    // 安全断言 — assert_isolated 严守契约
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn assert_isolated_noop_none_level_ok() {
        // 0 装 PASS: None 档 0 强制隔离 → assert 永 Ok.
        let cfg = NetworkIsolationConfig::default();
        let noop = NoopNetworkIsolation;
        assert!(assert_isolated(&cfg, &noop).is_ok());
    }

    #[test]
    fn assert_isolated_noop_strict_level_errors() {
        // 0 装严守: 配 strict + 0 装 trait → 必返 Err (不假装已隔离).
        let noop = NoopNetworkIsolation;
        for strict in [
            NetworkIsolationLevel::LoopbackOnly,
            NetworkIsolationLevel::DefaultDenyWithWhitelist,
            NetworkIsolationLevel::ForceDeny,
        ] {
            let cfg = NetworkIsolationConfig {
                level: strict,
                ..NetworkIsolationConfig::default()
            };
            let err = assert_isolated(&cfg, &noop)
                .expect_err("strict + 0 装 trait 必须 Err (0 假装红线)");
            assert!(err.contains("未实装"), "错误必须明示未实装: {err}");
            assert!(err.contains("0 假装"), "错误必须含 0 假装: {err}");
        }
    }

    #[test]
    fn assert_isolated_available_trait_passes() {
        // Mock 一个 available() = true 的 trait impl → assert 应 Ok.
        #[derive(Debug)]
        struct MockAvailable;
        impl NetworkIsolation for MockAvailable {
            fn available(&self) -> bool {
                true
            }
            fn status(&self) -> String {
                "MockAvailable: 实装可用 (test only)".into()
            }
            fn apply_to_child(&self, _cmd: &mut Command, _cfg: &NetworkIsolationConfig) -> Result<(), String> {
                Ok(())
            }
            fn cleanup(&self) -> Result<(), String> {
                Ok(())
            }
        }
        let mock = MockAvailable;
        for strict in [
            NetworkIsolationLevel::LoopbackOnly,
            NetworkIsolationLevel::DefaultDenyWithWhitelist,
            NetworkIsolationLevel::ForceDeny,
        ] {
            let cfg = NetworkIsolationConfig {
                level: strict,
                ..NetworkIsolationConfig::default()
            };
            assert!(
                assert_isolated(&cfg, &mock).is_ok(),
                "available=true + strict 档必须 Ok ({strict:?})"
            );
        }
    }

    // ──────────────────────────────────────────────────────────────────
    // 工厂默认 — 0 装时必须返 Noop
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn default_network_isolation_returns_noop() {
        let isol = default_network_isolation();
        // 0 装 PASS: 默认工厂必须返 Noop (available = false).
        assert!(
            !isol.available(),
            "默认工厂 0 装时必须返 available=false (smolvm 0 装红线)"
        );
        let status = isol.status();
        assert!(
            status.contains("Noop"),
            "默认工厂 status 必须含 Noop: {status}"
        );
        // apply 必 Err (0 装严守)
        let mut cmd = Command::new("echo");
        let cfg = NetworkIsolationConfig::default();
        assert!(isol.apply_to_child(&mut cmd, &cfg).is_err());
    }
}