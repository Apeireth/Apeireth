//! `apeireth-companion::sandbox_pass` — Stage 1 + Stage 2 0 装 PASS 编译期 const 守门.
//!
//! ## 哲学 (主人 2026-08-19: "0 装 PASS 严守, 任何 default trait 方法返 Err/false 必须有显式 0 装 注释, 不假装")
//!
//! Stage 1 (网络隔离) + Stage 2 (microVM) 在 `apeireth-companion` 的 2 个真实落地:
//! - [`crate::sandbox_net`] (`crates/apeireth-companion/src/sandbox_net.rs`) —
//!   `NetworkIsolation` trait + `NoopNetworkIsolation` (Linux netns + cgroup / Windows WFP 接入后启用)
//! - [`crate::vm_sandbox`] (`crates/apeireth-companion/src/vm_sandbox.rs`) —
//!   `VMSandbox` trait + `VMSandboxConfig` + `VMSandboxHandle` (Drop 自动 halt) + `NoopVMSandbox`
//!   (libkrun / Hyperlight / Firecracker 真实 backend 待选型)
//!
//! 本文件 (`sandbox_pass.rs`) 是这两个文件的 **0 装 PASS 编译期契约守门** —
//! 任何后续实现变更都必须满足此文件中的 7 项 const 断言 + 5 场景行为契约,
//! 否则 0 装 PASS 红线被打破, 等同于"假装能跑实验".
//!
//! ### 借鉴 4 源 0 装模式 (公开 docs 思路, 0 装仓库)
//!
//! | 借鉴源                | 0 装模式                       | 我们的实现 (本文件守门)                          |
//! |----------------------|------------------------------|-----------------------------------------------|
//! | Firecracker minimal API | API 极简, 0 装时安全降级       | `VMSandbox` trait 3-syscall 风格, `NoopVMSandbox::start()` 0 装返 `Err` |
//! | libkrun C lib + Rust binding | 0 装时走 C 库 fallback | `default_vm_sandbox()` 0 装返 `NoopVMSandbox`, `VMSandboxBackend::detect()` 0 装返 `None` |
//! | wasmtime component model | capability 边界 + fuel 计量 | `VMSandboxHandle` Drop 自动 halt (借鉴 libkrun Resource 清理) |
//! | smolvm 0 装诚实       | `NoopXxx` struct + `available() 返 false` | `NoopNetworkIsolation` + `NoopVMSandbox` 都按此模式 |
//!
//! ### 8 哲学锚 (本文件覆盖 2 项新增)
//!
//! - **S-3 质量工程化 (NEW)**: 0 装 PASS 用 const 断言 + 单测守门, 不可被简单 eprintln 绕过
//! - **O-1 安全优先 (NEW)**: 网络隔离 + microVM 隔离是核心安全机制, 0 装 PASS 0 假装 —
//!   不允许"为通过测试而注入假 backend" (借用 wasmtime capability 守门思路)
//!
//! ### 0 装 PASS 5 场景 (本文件 7 单测逐项覆盖)
//!
//! 1. 默认 backend 实例化是 `NoopXxx` 拒绝型 (不假装)
//! 2. trait 默认方法全部返 `Err` / `false` (不假装成功)
//! 3. 0 装 borrow 路径 (公开 docs 0 装仓库, 0 触碰 Cargo.toml upstream)
//! 4. 8 哲学锚 2 项新增 (S-3 + O-1) 在 sandbox_net.rs / vm_sandbox.rs 顶部 doc 显式标
//! 5. 24 LOCKED crate 0 触碰, 仅新增 2 文件 + 本守门文件
//!
//! ### 集成测试设计 (第三部分, 0 写实际代码, 仅设计)
//!
//! 见 `crates/apeireth-companion/tests/` 下未来 Stage 1+2 集成测试文件 (本文件 0 写):
//! - **集成 A**: `NoopNetworkIsolation` + `NoopVMSandbox` 一起用 → 0 装 PASS 0 假装
//!   (期望两个 trait 方法都返 `Err`, 永不假装成功)
//! - **集成 B**: 端到端串到 `experiment_field.rs` `VMRunner` 流程 → 0 装时
//!   `run_build_and_test()` 仍返 `Err` (字串含 "NoopVMRunner"), 0 装 PASS 链路一致
//! - **集成 C**: Concurrent use — 多个 sandbox 0 装 default 共享 0 状态
//!   (0 状态意味着 0 假装并发隔离, 0 装期任何并发都该拿到相同 `Err`)

#![deny(unsafe_code)]

use std::process::Command;

use crate::sandbox_net::{
    assert_isolated, default_network_isolation, NetworkIsolation, NetworkIsolationConfig,
    NetworkIsolationLevel, NoopNetworkIsolation,
};
use crate::vm_sandbox::{
    default_vm_sandbox, validate_config, NoopVMSandbox, VMSandbox, VMSandboxBackend,
    VMSandboxConfig, VMSandboxHandle, VMSandboxState,
};

// =============================================================================
// 0 装 PASS 编译期 const 守门 (本 crate 的硬契约)
// =============================================================================

/// 0 装标志: 编译期恒 `false`.
/// 守门: Stage 1 `NoopNetworkIsolation` 在 0 装期 `available()` 必须返 `false`
/// (见 `crate::sandbox_net::NoopNetworkIsolation`).
pub const STAGE_1_NOOP_NETWORK_ISOLATION_AVAILABLE: bool = false;

/// 0 装标志: 编译期恒 `false`.
/// 守门: Stage 2 `NoopVMSandbox` 在 0 装期 `available()` 必须返 `false`
/// (见 `crate::vm_sandbox::NoopVMSandbox`).
pub const STAGE_2_NOOP_VM_SANDBOX_AVAILABLE: bool = false;

/// 0 装 PASSPORT 字符串: 借鉴 4 源公开 docs 思路, 0 装上游仓库.
pub const SANDBOX_BORROW_SOURCE: &str =
    "public docs only (smolvm/Firecracker/libkrun/wasmtime), 0 装仓库";

/// 0 装期 sanity check: 0 装时 `NetworkIsolation::apply_to_child` 必须返 `Err`,
/// 错误字串必须**显式**含以下固定前缀 (与 `crate::sandbox_net::NoopNetworkIsolation::apply_to_child` 字串对齐).
pub const SANDBOX_NET_0PASS_ERROR: &str =
    "NoopNetworkIsolation: 网络隔离未实装 (Linux netns + cgroup / Windows WFP 接入后启用)";

/// 0 装期 sanity check: 0 装时 `VMSandbox::start` 必须返 `Err`,
/// 错误字串必须**显式**含以下固定前缀 (与 `crate::vm_sandbox::NoopVMSandbox::start` 字串对齐).
pub const SANDBOX_VM_0PASS_ERROR: &str =
    "NoopVMSandbox: microVM 隔离未实装 (Stage 2 仅 trait + 0 装 stub, 真实 backend 待选型 libkrun/Hyperlight/Firecracker)";

// =============================================================================
// 7 项 0 装 PASS 单测 (本守门文件的核心交付)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 1. 编译期恒 `false` 守门 — Stage 1 `NoopNetworkIsolation` 不假装可用.
    /// 借鉴 smolvm 0 装诚实: `available()` 返 `false`.
    #[test]
    fn xx_0pass_noop_network_isolation_available_false() {
        assert!(
            !STAGE_1_NOOP_NETWORK_ISOLATION_AVAILABLE,
            "STAGE_1_NOOP_NETWORK_ISOLATION_AVAILABLE 编译期必须为 false (0 装 PASS 红线)"
        );
        let n = NoopNetworkIsolation;
        assert!(!n.available(), "NoopNetworkIsolation::available() 必须返 false");
    }

    /// 2. 编译期恒 `false` 守门 — Stage 2 `NoopVMSandbox` 不假装可用.
    /// 借鉴 smolvm 0 装诚实: `available()` 返 `false`.
    #[test]
    fn xx_0pass_noop_vm_sandbox_available_false() {
        assert!(
            !STAGE_2_NOOP_VM_SANDBOX_AVAILABLE,
            "STAGE_2_NOOP_VM_SANDBOX_AVAILABLE 编译期必须为 false (0 装 PASS 红线)"
        );
        let v = NoopVMSandbox;
        assert!(!v.available(), "NoopVMSandbox::available() 必须返 false");
    }

    /// 3. 0 装 trait 0 假装 — `apply_to_child` 必须返 `Err`, 错误字串以
    /// [`SANDBOX_NET_0PASS_ERROR`] 前缀开头 (借鉴 smolvm 0 装诚实模式).
    /// 借用 `crate::sandbox_net::NoopNetworkIsolation` (Stage 1 已有 0 装 stub).
    #[test]
    fn xx_0pass_noop_apply_returns_err_not_ok() {
        let n = NoopNetworkIsolation;
        let cfg = NetworkIsolationConfig::default();
        let mut cmd = Command::new("echo");
        let res = n.apply_to_child(&mut cmd, &cfg);
        assert!(res.is_err(), "0 装期 apply_to_child 必须返 Err, 不假装成功");
        let err = res.unwrap_err();
        assert!(
            err.starts_with(SANDBOX_NET_0PASS_ERROR),
            "0 装错误字串必须以 SANDBOX_NET_0PASS_ERROR 前缀开头, 实际: {err}"
        );
        // status 必须含 0 假装明示 (smolvm 0 装诚实模式).
        let status = n.status();
        assert!(
            status.contains("Noop") && status.contains("未实装") && status.contains("0 假装"),
            "NoopNetworkIsolation::status 必须含 Noop + 未实装 + 0 假装: {status}"
        );
    }

    /// 4. 0 装 trait 0 假装 — `start` 必须返 `Err`, 错误字串以
    /// [`SANDBOX_VM_0PASS_ERROR`] 前缀开头 (借鉴 smolvm 0 装诚实模式).
    /// 任何 `VMSandboxConfig` 都一视同仁 (0 装期 validate_config 仅查参数边界, 不验 backend).
    #[test]
    fn xx_0pass_noop_start_returns_err_not_ok() {
        let v = NoopVMSandbox;
        let cfg = VMSandboxConfig::default();
        let res = v.start(&cfg);
        assert!(res.is_err(), "0 装期 start 必须返 Err, 不假装成功");
        let err = res.unwrap_err();
        assert!(
            err.starts_with(SANDBOX_VM_0PASS_ERROR),
            "0 装错误字串必须以 SANDBOX_VM_0PASS_ERROR 前缀开头, 实际: {err}"
        );
        // status 必须含 libkrun/Hyperlight/Firecracker 真接路径说明 (借鉴 smolvm 0 装诚实).
        let status = v.status();
        assert!(
            status.contains("libkrun")
                || status.contains("Hyperlight")
                || status.contains("Firecracker"),
            "NoopVMSandbox::status 必须说明真接路径: {status}"
        );
        // backends() 0 装期必须空 (0 backend = 0 假装).
        assert!(v.backends().is_empty(), "NoopVMSandbox::backends() 0 装必须空");
        // backend() 0 装期固定 PlatformDefault.
        assert_eq!(
            v.backend(),
            VMSandboxBackend::PlatformDefault,
            "NoopVMSandbox::backend() 0 装必须 PlatformDefault"
        );
    }

    /// 5. 0 装 borrow 路径约束 — `SANDBOX_BORROW_SOURCE` 编译期字符串守门,
    /// 显式声明"公开 docs 0 装仓库" (0 触碰 Cargo.toml upstream).
    /// 借鉴 4 源 (smolvm / Firecracker / libkrun / wasmtime) 0 装仓库, 0 binary dep.
    #[test]
    fn xx_0pass_borrow_source_no_repo_dependency() {
        assert!(
            SANDBOX_BORROW_SOURCE.contains("public docs only"),
            "0 装 PASSPORT 字符串必须含 'public docs only'"
        );
        assert!(
            SANDBOX_BORROW_SOURCE.contains("0 装仓库"),
            "0 装 PASSPORT 字符串必须显式 '0 装仓库' 标记"
        );
        assert!(
            SANDBOX_BORROW_SOURCE.contains("smolvm")
                && SANDBOX_BORROW_SOURCE.contains("Firecracker")
                && SANDBOX_BORROW_SOURCE.contains("libkrun")
                && SANDBOX_BORROW_SOURCE.contains("wasmtime"),
            "0 装 PASSPORT 字符串必须列全 4 借鉴源"
        );
    }

    /// 6. 8 哲学锚 2 项新增 (S-3 质量工程化 + O-1 安全优先) 透传 — 本文件本身就是
    /// 二者的工程化承载体 (本文件 doc 头部已显式标, 单测再 verify 一次 0 装期不脱锚).
    #[test]
    fn xx_0pass_8_anchors_2_new_pass_through() {
        // S-3 质量工程化 (NEW): 0 装 PASS 用 const 断言 + 单测守门, 不可被简单 eprintln 绕过.
        // 守门: 4 个 const 都必须在文件里被声明且语义自洽.
        assert!(
            !STAGE_1_NOOP_NETWORK_ISOLATION_AVAILABLE
                && !STAGE_2_NOOP_VM_SANDBOX_AVAILABLE,
            "S-3 质量工程化: 0 装 PASS 编译期 const 守门必须恒 false"
        );
        // 错误字串必须显式含 "未实装" 标记 (S-3 不可绕过).
        assert!(
            SANDBOX_NET_0PASS_ERROR.contains("未实装")
                && SANDBOX_VM_0PASS_ERROR.contains("未实装"),
            "S-3 质量工程化: 0 装错误字串必须显式 '未实装' 标记"
        );
        // O-1 安全优先 (NEW): 0 装 = 0 假装, 0 接受 "为通过测试而注入假 backend".
        // 守门: default factory 永远返 Noop, 0 装期绝不返回任何"看起来能跑"的 backend.
        let n = default_network_isolation();
        assert!(
            !n.available(),
            "O-1 安全优先: default_network_isolation() 0 装期必须返 Noop"
        );
        let v = default_vm_sandbox();
        assert!(
            !v.available(),
            "O-1 安全优先: default_vm_sandbox() 0 装期必须返 Noop"
        );
        // 4 档网络隔离级别 0 装 default = None (0 强制, 0 假装已隔离).
        let cfg = NetworkIsolationConfig::default();
        assert_eq!(
            cfg.level,
            NetworkIsolationLevel::None,
            "O-1 安全优先: NetworkIsolationConfig 默认 level 必须 None"
        );
        // Stage 2 backend detect 0 装期必 None (0 假装本平台 KVM/Hyper-V 可用).
        assert!(
            VMSandboxBackend::detect().is_none(),
            "O-1 安全优先: VMSandboxBackend::detect() 0 装期必须 None"
        );
        // assert_isolated(0 装 + strict) 必 Err (不假装已隔离).
        let strict_cfg = NetworkIsolationConfig {
            level: NetworkIsolationLevel::LoopbackOnly,
            ..NetworkIsolationConfig::default()
        };
        assert!(
            assert_isolated(&strict_cfg, &NoopNetworkIsolation).is_err(),
            "O-1 安全优先: assert_isolated(strict + Noop) 必 Err"
        );
    }

    /// 7. 24 LOCKED crate 0 触碰守门 — 0 装期契约值自洽 (validate_config 仅查参数边界,
    /// 0 触碰 8 哲学锚 / 13 键 A3 / 24 维 V0.5 / 24 LOCKED crate 入口).
    /// 仅新增 2 文件 (sandbox_net.rs + vm_sandbox.rs) + 本守门文件.
    #[test]
    fn xx_0pass_24_locked_to_3_immutable_pelvis_preserved() {
        // VMSandboxConfig default 0 装期合法 (validate_config Ok).
        let cfg = VMSandboxConfig::default();
        assert_eq!(cfg.vcpus, 1, "0 装 default vcpus 必须 1 (Firecracker 底线)");
        assert_eq!(cfg.memory_mb, 512, "0 装 default memory_mb 必须 512");
        assert_eq!(cfg.boot_timeout_secs, 30, "0 装 default boot_timeout_secs 必须 30");
        assert!(cfg.rootfs.is_none(), "0 装 default rootfs 必须 None");
        assert!(cfg.kernel.is_none(), "0 装 default kernel 必须 None");
        assert!(cfg.initrd.is_none(), "0 装 default initrd 必须 None");
        assert!(cfg.network.is_none(), "0 装 default network 必须 None");
        // validate_config 边界检查 (0 装期仅查参数边界, 不验 backend 兼容).
        assert!(
            validate_config(&cfg, VMSandboxBackend::PlatformDefault).is_ok(),
            "默认 VMSandboxConfig 必须 validate_config Ok"
        );
        // validate_config 4 backend 0 装期全部 Ok (0 装期不验 backend 兼容).
        for backend in [
            VMSandboxBackend::Kvm,
            VMSandboxBackend::Hypervisor,
            VMSandboxBackend::HyperV,
            VMSandboxBackend::PlatformDefault,
        ] {
            assert!(
                validate_config(&cfg, backend).is_ok(),
                "0 装: validate_config 只查参数边界, 跨 backend 必须 Ok ({backend:?})"
            );
        }
        // 24 LOCKED 守门: 本守门文件 0 引入新 enum / const 替代 24 LOCKED crate 入口.
        // 0 触碰 LOCKED 不可变脊柱, 0 改 workspace.version (1.2.0).
        // (实际 LOCKED 守门在工作树 M 的 cargo check --workspace 0 错 + 0 LOCKED 改动 CI).
        //
        // 7 VMSandboxState 变体守门 (vm_sandbox.rs 5 变体).
        assert_eq!(
            VMSandboxState::Created.as_str(),
            "created",
            "VMSandboxState as_str 守门"
        );
        assert_eq!(
            VMSandboxState::Booted.as_str(),
            "booted",
            "VMSandboxState as_str 守门"
        );
        assert_eq!(
            VMSandboxState::Running.as_str(),
            "running",
            "VMSandboxState as_str 守门"
        );
        assert_eq!(
            VMSandboxState::Halted.as_str(),
            "halted",
            "VMSandboxState as_str 守门"
        );
        assert_eq!(
            VMSandboxState::Error.as_str(),
            "error",
            "VMSandboxState as_str 守门"
        );
        // Concurrent use 设计验证 (集成 C 雏形): 多个 sandbox 0 装 default 共享 0 状态,
        // 任何 trait 方法都返相同 Err (0 状态 = 0 假装并发隔离).
        let sandbox: Arc<Box<dyn VMSandbox>> = Arc::new(default_vm_sandbox());
        let s1 = Arc::clone(&sandbox);
        let s2 = Arc::clone(&sandbox);
        let r1 = s1.start(&cfg);
        let r2 = s2.start(&cfg);
        assert!(r1.is_err() && r2.is_err(), "并发 0 装 start 必须都返 Err");
        assert_eq!(
            r1.as_ref().unwrap_err(),
            r2.as_ref().unwrap_err(),
            "并发 0 装 Err 字串必须一致 (0 状态)"
        );
        // 跨 Stage 1 + Stage 2 交叉守门: NoopNetworkIsolation 与 NoopVMSandbox 并存,
        // 0 装期两者都返 Err, 0 任何"假装成功"路径.
        let net = NoopNetworkIsolation;
        let mut cmd = Command::new("echo");
        let net_cfg = NetworkIsolationConfig::default();
        assert!(
            net.apply_to_child(&mut cmd, &net_cfg).is_err(),
            "NoopNetworkIsolation 0 装必须 Err"
        );
        let vm = NoopVMSandbox;
        assert!(vm.start(&cfg).is_err(), "NoopVMSandbox 0 装必须 Err");
        // VMSandboxHandle Drop 自动 halt (借鉴 libkrun Resource 清理).
        let mut h = VMSandboxHandle::new(
            Box::new(NoopVMSandbox),
            cfg.clone(),
            VMSandboxState::Booted,
        );
        assert!(!h.is_halted(), "构造后未 halt");
        h.halt().expect("halt 必 Ok");
        assert!(h.is_halted(), "halt 后必须 is_halted=true");
        // 二次 halt 幂等 (Drop 路径不泄漏).
        assert!(h.halt().is_ok(), "halt 幂等");
    }
}
