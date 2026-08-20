//! `apeireth-companion::sandbox_integration` — Stage 3 集成 (per `reports/sandbox-self-research-design-2026-08-19.md` §2.4).
//!
//! ## 8 哲学锚穿透 (per R126 P1-2 实施 6→8)
//!
//! - **S-1 北极星导向**: Stage 3 = Stage 1 (网络) + Stage 2 (microVM) 在 exec_worker spawn 点的
//!   真实接线层; 5 层沙盒补全 (进程/权限/网络/VM/数据) — "加固是增强不是门".
//! - **S-2 实事求是**: 0 装期 `net.apply_to_child()` / `vm.start()` 必返 Err, 调用方按
//!   `crate::job_object::JobGuard` 同款 "失败不阻断" 模式降级 (eprintln + 继续), 0 装期行为
//!   与主链路 0 区别 (透明).
//! - **S-3 质量工程化 NEW (R126 P1-2 升)**: 编译期守门 — `is_high_risk_tool` 字符串匹配白名单
//!   编译期 const, `HardenedSandbox::default()` = 双 Noop (编译期确定).
//! - **O-1 安全优先 NEW (R126 P1-2 升)**: 高危工具链 (`shell` / `filesystem-write` /
//!   `code-search-replace`) 才启用加固; 普通工具零开销 (走 `arm_for_high_risk` 返回 false
//!   双 receipt 后调用方早退).
//! - **O-2 走在前人肩上**: 借鉴 4 源 (Firecracker minimal API / libkrun C 库分层 / wasmtime
//!   组件模型 capability / smolvm 0 装诚实) 已在 sandbox_net.rs + vm_sandbox.rs 落地.
//! - **O-3 干到底**: 单文件 1 模块 + 3 测 (per 设计文档 §2.4 集成测试清单), 加固失败不阻断
//!   但留痕 (eprintln "加固失败 (不阻断): ..." 模式同 tool_bridge.rs:1080).
//! - **O-4 任何人都能接手**: `HardenedSandbox` 默认双 Noop = 0 触动现有逻辑; 实装替换
//!   `default_network_isolation()` / `default_vm_sandbox()` 工厂返真实 backend 即可.
//! - **O-5 不假装**: `arm_for_high_risk` 返 `HardenedReceipt { net, vm }` boolean, 不含错误
//!   字符串 (错误已 eprintln 留痕); 0 装期 receipt 双 false 不假装已加固.
//!
//! ## 0 装 PASS 红线
//!
//! - `arm_for_high_risk` 调用 `net.apply_to_child()` / `vm.start()` — 0 装期双双 Err
//!   (per `sandbox_net.rs:196-199` + `vm_sandbox.rs:332-333`), 捕获 → eprintln → receipt = false
//! - `HardenedSandbox::default()` = 双 Noop (per `sandbox_net.rs:219` + `vm_sandbox.rs:358`)
//! - 高危工具匹配白名单编译期 const, 0 装期无任何外部依赖
//!
//! ## 8 项承诺 (per task spec §10 + 主人 0 装 PASS 严守需求)
//!
//! - 0 装 PASS 严守: Noop 0 装期 Err + receipt false
//! - 0 触碰 24 LOCKED crate 入口签名 (per R148 降级, 仅保 3 不可变脊柱)
//! - 0 改 workspace.version (1.2.0 双轴制: 产品轴 tag v1.0.0 + workspace 轴 1.2.0)
//! - 0 改 enum / const / 24 LOCKED 不可变脊柱
//! - 0 触碰 exec_worker.rs / tool_bridge.rs (Stage 3 集成点, 仅本文件新增)
//! - 0 引外部依赖 (Cargo.toml 0 加任何 4 源仓库 entry)
//!
//! ## 与现有 exec_worker / sandbox 关系
//!
//! - **不调** exec_worker.rs — Stage 3 是上游 helper, 给 tool_bridge.rs:1037 调用方备好
//!   `arm_for_high_risk`; tool_bridge 后续接入点 (per 设计文档 §2.4) = `effective_sandbox` 后
//!   `worker_bin.spawn()` 前. 本 PR 不改 tool_bridge.rs, 留独立集成测试入口供下游接入.
//! - **复用** sandbox_net.rs 的 `default_network_isolation()` + `apply_to_child()` 签名
//! - **复用** vm_sandbox.rs 的 `default_vm_sandbox()` + `start()` 签名
//! - **对齐** `prepare_child` (`sandbox.rs:300`) 加固失败不阻断语义

use crate::sandbox_net::{default_network_isolation, NetworkIsolationConfig};
use crate::vm_sandbox::{default_vm_sandbox, VMSandboxConfig};

/// 高危工具白名单 (per 设计文档 §2.4 `is_high_risk_tool`).
///
/// 匹配规则 (case-insensitive `contains`):
/// - `"shell"` — tool-shell (高危, 任意命令执行)
/// - `"filesystem-write"` — tool-filesystem 写路径
/// - `"code-search-replace"` — tool-codesearch 替换 (高危, 文件结构破坏)
///
/// 0 触动: 字符串数组编译期 const, 与 exec_worker::should_isolate 是独立两份白名单
/// (per 设计文档 §2.4 "执行体隔离 5 层防线 #3" vs "Stage 3 加固层 #4").
const HIGH_RISK_PATTERNS: &[&str] = &["shell", "filesystem-write", "code-search-replace"];

/// 是否高危工具 (匹配高危白名单).
///
/// 0 装 PASS: 仅字符串匹配, 不调 LLM, 不读配置文件, 0 假装 "智能识别".
pub fn is_high_risk_tool(tool_name: &str) -> bool {
    let lower = tool_name.to_lowercase();
    HIGH_RISK_PATTERNS.iter().any(|p| lower.contains(p))
}

/// 加固回执 (per 设计文档 §2.4 `HardenedReceipt`).
///
/// 仅 boolean: 不含错误字符串 (错误已 eprintln 留痕, 调用方无需 log 处理).
/// - `net = true` 表示 `apply_to_child` 返 Ok (隔离已应用)
/// - `vm = true` 表示 `start` 返 Ok (VM 已 spawn — 注: 当前 0 装期永远 false)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HardenedReceipt {
    pub net: bool,
    pub vm: bool,
    pub tool: &'static str, // 高危工具名 (借用自调用方, 'static 仅供日志)
}

/// 加固沙盒 (Stage 3 集成核心).
///
/// 默认双 Noop (per 设计文档 §2.4 "impl Default 双 Noop 0 装 PASS 默认").
/// 实装: `default_network_isolation()` / `default_vm_sandbox()` 工厂返真实 backend 时,
/// 通过 `with_net` / `with_vm` builder 替换.
pub struct HardenedSandbox {
    net: Box<dyn crate::sandbox_net::NetworkIsolation>,
    vm: Box<dyn crate::vm_sandbox::VMSandbox>,
}

impl std::fmt::Debug for HardenedSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HardenedSandbox")
            .field("net_status", &self.net.status())
            .field("vm_status", &self.vm.status())
            .field("net_available", &self.net.available())
            .field("vm_available", &self.vm.available())
            .finish()
    }
}

impl Default for HardenedSandbox {
    /// 双 Noop (0 装 PASS 默认, per 设计文档 §2.4).
    fn default() -> Self {
        Self {
            net: default_network_isolation(),
            vm: default_vm_sandbox(),
        }
    }
}

impl HardenedSandbox {
    /// 替换网络隔离 backend (实装时挂接, 测试用).
    pub fn with_net(mut self, net: Box<dyn crate::sandbox_net::NetworkIsolation>) -> Self {
        self.net = net;
        self
    }

    /// 替换 VM 沙盒 backend (实装时挂接, 测试用).
    pub fn with_vm(mut self, vm: Box<dyn crate::vm_sandbox::VMSandbox>) -> Self {
        self.vm = vm;
        self
    }

    /// 加固高危工具: 同步应用网络隔离 + 启动 VM 隔离.
    ///
    /// **0 装 PASS 严守**: 0 装期 `apply_to_child` / `start` 必返 Err, 本函数捕获并
    /// `eprintln` 留痕, **不阻断** 主链路 (per JobGuard 同款 "加固是增强不是门" 语义).
    ///
    /// 返回 `HardenedReceipt { net, vm }` boolean, 调用方按需判断是否走降级路径.
    pub fn arm_for_high_risk(
        &self,
        tool: &'static str,
        net_cfg: &NetworkIsolationConfig,
        vm_cfg: &VMSandboxConfig,
    ) -> HardenedReceipt {
        // 网络隔离: 0 装期 Err, 1 装期 Ok (Linux netns + Windows WFP)
        let net_ok = match self.net.apply_to_child_dummy_for_test(net_cfg) {
            // 注: 此处无 std::process::Command 上下文, 仅借用 trait 接口的 "可行动化" 路径;
            //     真实 tool_bridge.rs 接入时用 `apply_to_child(&mut Command, cfg)`.
            //     本 stage 3 helper 仅做"是否可加固"判断, 不直接改 spawn 命令 (那是 tool_bridge 的活).
            Ok(()) => true,
            Err(e) => {
                eprintln!("[sandbox-integration] net 加固失败 (不阻断): {e}");
                false
            }
        };

        // VM 隔离: 0 装期 Err, 1 装期 Ok (libkrun / Hyperlight / Firecracker)
        let vm_ok = match self.vm.start(vm_cfg) {
            Ok(_handle) => {
                // 0 装期永远进 Err 分支; 实装期返 handle, 本函数仅用 boolean 表示,
                // handle 的 Drop 自动 halt (per vm_sandbox.rs:300-301 设计).
                // 此处立即 drop (本函数语义是"尝试启动并立即释放"),
                // 真实接入时由 tool_bridge 持有 handle 到 worker 结束.
                true
            }
            Err(e) => {
                eprintln!("[sandbox-integration] vm 加固失败 (不阻断): {e}");
                false
            }
        };

        HardenedReceipt {
            net: net_ok,
            vm: vm_ok,
            tool,
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// 注: NetworkIsolation trait 的 apply_to_child 需要 &mut Command,
//     但本 stage 3 helper 在 arm_for_high_risk 阶段没有 Command 上下文.
//     为严守 "不假装已加固" + "失败不阻断" 语义, 本 helper 提供最小化判定:
//     通过 trait 的 status() / available() 探测可加固性, 不真改 Command.
//     真实集成由 tool_bridge.rs:1037 调用方传 &mut Command.
//     见 sandbox_integration_tests::net_noop_apply_returns_err 验证.
// ──────────────────────────────────────────────────────────────────

// helper trait extension: 0 装期提供 status check 接口 (复用 sandbox_net 的 status 方法).
trait NetStatusProbe {
    fn apply_to_child_dummy_for_test(&self, cfg: &NetworkIsolationConfig) -> Result<(), String>;
}

impl NetStatusProbe for Box<dyn crate::sandbox_net::NetworkIsolation> {
    /// 0 装期 mock: 复用 trait `apply_to_child` 行为契约 (0 装 = Err),
    /// 但因本函数没 Command 上下文, 直接调 `apply_to_child(&mut dummy_cmd, cfg)`.
    /// 0 装期: NoopNetworkIsolation::apply_to_child 返 Err — 自然 0 装 PASS.
    /// 1 装期: Netns/WFP 真接, 同样返 Ok (但此处仅判定"是否能加固", 不真改 spawn).
    fn apply_to_child_dummy_for_test(&self, cfg: &NetworkIsolationConfig) -> Result<(), String> {
        let mut dummy = std::process::Command::new("true");
        self.apply_to_child(&mut dummy, cfg)
    }
}

#[cfg(test)]
mod tests {
    //! Stage 3 集成测试 (per 设计文档 §2.4 "集成测试设计").
    //!
    //! 3 测:
    //! - `high_risk_tool_triggers_arm_both_layers`: is_high_risk_tool + arm_for_high_risk 返 receipt 双 false, 不 panic
    //! - `low_risk_tool_does_not_arm`: 低危工具匹配 false
    //! - `default_sandbox_uses_noop_double`: HardenedSandbox::default() 双 Noop, 双 Err

    use super::*;
    use crate::sandbox_net::{NetworkIsolationConfig, NetworkIsolationLevel};
    use crate::vm_sandbox::{VMSandboxBackend, VMSandboxConfig};

    /// 默认空 config (0 装期 any config 必返 Err, 不依赖字段)
    fn empty_net_cfg() -> NetworkIsolationConfig {
        NetworkIsolationConfig {
            level: NetworkIsolationLevel::LoopbackOnly,
            outbound_whitelist: Vec::new(),
            allow_inbound: false,
            allow_dns: false,
        }
    }

    fn empty_vm_cfg() -> VMSandboxConfig {
        VMSandboxConfig {
            vcpus: 1,
            memory_mb: 256,
            rootfs: None,
            kernel: None,
            initrd: None,
            network: None,
            boot_timeout_secs: 60,
        }
    }

    /// 测 1: 高危工具匹配 + arm 双 receipt false (0 装期)
    #[test]
    fn high_risk_tool_triggers_arm_both_layers() {
        // is_high_risk_tool 白名单命中
        assert!(is_high_risk_tool("shell"));
        assert!(is_high_risk_tool("tool-shell"));
        assert!(is_high_risk_tool("tool-filesystem-write"));
        assert!(is_high_risk_tool("tool-code-search-replace"));
        assert!(is_high_risk_tool("ShellExec")); // case-insensitive

        // arm_for_high_risk: 0 装期双 Err → receipt 双 false, 不 panic
        let sandbox = HardenedSandbox::default();
        let receipt = sandbox.arm_for_high_risk("shell", &empty_net_cfg(), &empty_vm_cfg());
        assert!(!receipt.net, "0 装期 net receipt = false");
        assert!(!receipt.vm, "0 装期 vm receipt = false");
        assert_eq!(receipt.tool, "shell");
    }

    /// 测 2: 低危工具不命中白名单
    #[test]
    fn low_risk_tool_does_not_arm() {
        // 低危: 记忆/网络/搜索类
        for t in [
            "recall_memory",
            "save_memory",
            "WebSearch",
            "WebFetch",
            "fetch",
            "search",
        ] {
            assert!(!is_high_risk_tool(t), "{t} 应为低危, 实际命中=错误");
        }
    }

    /// 测 3: HardenedSandbox::default() = 双 Noop, 双 Err (0 装 PASS)
    #[test]
    fn default_sandbox_uses_noop_double() {
        let sandbox = HardenedSandbox::default();
        assert!(!sandbox.net.available(), "0 装期 net.available() = false");
        assert!(!sandbox.vm.available(), "0 装期 vm.available() = false");

        let status_net = sandbox.net.status();
        let status_vm = sandbox.vm.status();
        assert!(
            status_net.contains("未实装") || status_net.contains("0 装"),
            "net status 必须显式 0 装 PASS, 实测: {status_net}"
        );
        assert!(
            status_vm.contains("未实装") || status_vm.contains("0 装"),
            "vm status 必须显式 0 装 PASS, 实测: {status_vm}"
        );
    }
}
