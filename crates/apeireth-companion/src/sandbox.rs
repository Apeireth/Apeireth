//! `apeireth-companion::sandbox` — 沙盒参数口 (B3 沙盒包参数化).
//!
//! 三层结构 (集成而非分立):
//! - [`SandboxConfig`]: 内存/CPU/超时参数. 从套件清单 (`suites.rs` SuiteDef.sandbox)
//!   或权限包 (`packs.rs` PermissionPack.sandbox) 可配; 非法参数回退默认并 eprintln 记录.
//! - [`SandboxBackend`] trait: 物理隔离后端参数口 (B3 方向 ③, Sandboxie/landlock 留口).
//! - 限额生效机制: Windows 侧由 `job_object.rs` (Job Object EXTENDED_LIMIT +
//!   CPU rate control + 超限留痕) 消费本配置; 非 Windows 为 no-op (诚实标注).
//!
//! 0 装 PASS 红线: Sandboxie (第三方软件, Windows) 与 landlock (Linux 内核 LSM)
//! 均**只留 trait 口, 未接实现** — `available()` 如实返回 false, `status()` 说明原因.

/// 默认单次调用超时 (秒; 与 tool_bridge 历史硬编码一致).
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 沙盒资源参数 (per-call 隔离 worker 的资源上限).
///
/// 全部 Option/可配; None = 不限 (只保留 Job Object 生命周期加固).
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxConfig {
    /// 内存上限 (MB, 整个 worker 进程树的 committed 内存).
    /// 映射: Windows Job Object `JOB_OBJECT_LIMIT_PROCESS_MEMORY`.
    pub memory_limit_mb: Option<u64>,
    /// CPU 限速 (1-100, 占一个逻辑核的百分比硬上限).
    /// 映射: Job Object CPU rate control (HARD_CAP, Win8+); 设置失败降级不阻断.
    pub cpu_percent: Option<u32>,
    /// CPU 时间上限 (秒, 单个 worker 进程 user-mode 累计 CPU 时间).
    /// 映射: `JOB_OBJECT_LIMIT_PROCESS_TIME` (100ns tick). 确定性强, 限速的兜底口径.
    pub cpu_time_secs: Option<u64>,
    /// 单次调用超时 (秒). 默认 [`DEFAULT_TIMEOUT_SECS`] (超时 kill, 与历史行为一致).
    pub timeout_secs: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: None,
            cpu_percent: None,
            cpu_time_secs: None,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl SandboxConfig {
    /// 是否配置了任何资源限额 (内存/CPU). 无 → Job Object 只做生命周期加固.
    pub fn has_limits(&self) -> bool {
        self.memory_limit_mb.is_some() || self.cpu_percent.is_some() || self.cpu_time_secs.is_some()
    }

    /// 参数口: 从 JSON 解析 (套件清单/权限包的配置载体).
    ///
    /// 非法参数**回退默认**并 eprintln 记录 (不静默, 不阻断执行):
    /// - `memory_limit_mb` / `cpu_time_secs`: 非正整数 → None (不限)
    /// - `cpu_percent`: 非 1..=100 整数 → None (不限)
    /// - `timeout_secs`: 非正整数 → [`DEFAULT_TIMEOUT_SECS`]
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
        cfg
    }
}

/// 物理隔离后端参数口 (B3 方向 ③): 平台级沙盒机制的统一接口.
///
/// 语义: `available()` 诚实回答"本平台是否已接"; `render_params` 产出参数模板
/// (真接时用于包裹/约束 worker), 未接时模板仅供文档与后续实现参考.
pub trait SandboxBackend {
    /// 后端名.
    fn name(&self) -> &'static str;
    /// 当前平台是否已接 (0 装 PASS: 未接如实返回 false).
    fn available(&self) -> bool;
    /// 接入状态说明 (诚实标注: 为什么没接 / 覆盖了什么).
    fn status(&self) -> &'static str;
    /// 参数模板 (按配置渲染; 真接后的启动/约束参数).
    fn render_params(&self, cfg: &SandboxConfig) -> Vec<String>;
}

/// Sandboxie-Plus 参数口 (Windows 第三方沙盒软件).
///
/// 诚实: **未接**. Windows 侧执行体隔离已由 Job Object (job_object.rs) 覆盖;
/// Sandboxie 需第三方安装 (Start.exe 包裹启动 worker), 属后续可选增强.
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
        // 参数模板 (真接时: Start.exe /box:<盒子> /wait <worker>; 超时由桥侧 kill 兜底)
        vec![
            "Start.exe".to_string(),
            "/box:ApeirethWorker".to_string(),
            "/wait".to_string(),
            format!("timeout_secs={}", cfg.timeout_secs),
        ]
    }
}

/// landlock 参数口 (Linux 内核 LSM, 文件系统 syscall 沙盒).
///
/// 诚实: **未接**. landlock 是 Linux 专属内核机制 (Windows 无对应物);
/// 且 landlock 不是启动参数, 而是进程对自身调用 landlock(2) 收紧权限 —
/// 真接点在 worker 进程内部 (exec_worker), 本口只留接口与说明.
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
        // landlock 无启动参数可渲染 (见 status); 诚实返回空.
        Vec::new()
    }
}

/// 平台后端清单 (诚实口径: 当前两个后端均未接, Windows 限额走 Job Object).
pub fn backends() -> Vec<Box<dyn SandboxBackend>> {
    vec![Box::new(SandboxieBackend), Box::new(LandlockBackend)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_is_unlimited_with_30s_timeout() {
        let c = SandboxConfig::default();
        assert!(!c.has_limits());
        assert_eq!(c.timeout_secs, DEFAULT_TIMEOUT_SECS);
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
        // 每个非法字段独立回退 (0 阻断, eprintln 记录)
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
        // landlock 无启动参数 (诚实返回空)
        assert!(LandlockBackend.render_params(&SandboxConfig::default()).is_empty());
    }
}
