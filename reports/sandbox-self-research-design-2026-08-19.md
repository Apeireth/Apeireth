# Apeireth 沙盒借鉴 4 源对比 + 自研设计文档

```
[Document-Meta]
Document:        reports/sandbox-self-research-design-2026-08-19.md
Version:         0.1-R-design (research + 自研设计, 0 改任何 src/)
Date:            2026-08-19
Baseline HEAD:   master @ 2026-08-19 (workspace.version = 1.2.0 per Cargo.toml:228;
                 24 LOCKED crate 入口签名已降级 R148 仅保 3 项不可变脊柱:
                 Self-Disable / L0 HA / 13 键 verdict cache)
Source-of-Truth: 代码 (per S-2 实事求是); 4 源参考公开文档 + 思路, 不接仓库
0 触碰 src/:     严守 (仅产出 reports/ 文档, 0 改 enum/const/Cargo.toml)
RFC 2119 关键词: 必须 (MUST) / 应当 (SHOULD) / 可以 (MAY) / 不得 (MUST NOT)
```

> **本报告性质**: 借鉴挖掘 + 自研架构设计, **0 改任何源码**。基于
> `crates/apeireth-companion/src/{sandbox,experiment_field,job_object,restricted_token,app_container}.rs`
> 实查 + B 站 UP 主 2026-08-19 沙盒分析 + 4 源 (smolvm / Firecracker / libkrun / wasmtime) 公开文档思路。
>
> **严守 4 条**: 借鉴只借思路不借代码; 0 装 PASS 严守; 9 重守门 v9 严守; 8 哲学锚穿透。

---

## §0 TL;DR

| 项 | 结论 |
|---|---|
| **核心论断** | B 站 UP 主 §5.3 "隔离单元是进程生命周期，蠕虫感染单元是数据和网络——两者根本不对位" 是真因; 5 层用户态防线 (洋葱门/审批链/MOVE-STAY/Job Object/最小权限) 补的是"进程级", 真正的蠕虫感染需要"数据/网络级"隔离 — **借鉴 4 源就是为了补这一段空缺**。 |
| **借鉴价值排序** | (1) Firecracker minimal API 哲学 ★★★★★ → 自研 `VMSandbox` trait 1 方法;<br>(2) libkrun C 库 + Rust 绑定分层 ★★★★☆ → 自研 `apeireth-sandbox-ffi` trait 中间层;<br>(3) wasmtime fuel metering ★★★☆☆ → `SandboxConfig.fuel_meter` 字段 (LLM 成本);<br>(4) smolvm capability boundary ★★☆☆☆ → 我们已有 4 哲学锚 + 8 哲学锚体系替代, **不直接借鉴**。 |
| **3 阶段自研** | Stage 1 网络隔离 (trait + stub); Stage 2 microVM trait (trait + stub); Stage 3 集成 (0 装 PASS 默认)。 |
| **0 装 PASS** | 所有 trait 0 装时返诚实 `Err`; 不与现有 `NoopVMRunner` (`crates/apeireth-companion/src/experiment_field.rs:50`) 冲突; 9 重守门 v9 + 13 键 verdict cache 严守。 |
| **不做的事** | 不接 smolvm 仓库; 不抄 Firecracker API 形状; 不引 `wasmtime`/`libkrun` 依赖 (任何外部依赖引入必须经 9 重 v9 守门 + 主人审); 不动 24 LOCKED crate 入口签名; 不改 workspace.version。 |

---

## §1 Part 1 — 4 源对比表 (借鉴深度评估)

### §1.1 4 源横向对比矩阵

| 源 | 核心定位 | 借鉴价值 | 借鉴的具体设计元素 | 0 装 PASS 适配 | 风险 / 坑 |
|---|---|:---:|---|:---:|---|
| **smolvm** (klispweify) | Rust 写 WASM 沙盒, 0 star orphan, capability boundary 思路 | ★★☆☆☆ (低) | 思路 #1: capability boundary → 我们已有 4 哲学锚 + 8 哲学锚体系 (R126 NEW S-3 + O-1, 见 `crates/apeireth-core/src/eight_anchors.rs:55`); capability 在 Rust 类型系统已天然支持, 不需 WASM runtime | **不直接借鉴** — 0 star orphan 项目无维护保障, 我们用 Rust trait + 编译期 hardcode 已覆盖 | (1) 0 star orphan 项目明天可能消失, 接 = 维护孤岛; (2) WASM 沙盒 vs 我们的 OS 进程级沙盒抽象层错位, 引入会双轨 |
| **Firecracker** (AWS Lambda) | microVM 隔离 (KVM), minimal API surface (3 syscall: instance creation / API socket / VM start), 一次性 VM 哲学 | ★★★★★ (高) | 思路 #2: `VMRunner` trait 1 方法 → 加 `VMSandbox` trait 1 方法 (`spawn_and_destroy`); minimal API = 单 trait 1 方法而非多层 trait 链; 一次性 = VM 失败即丢弃, 不修 | trait 默认 Noop 实现 (`available() -> false`, `spawn_and_destroy() -> Err("...")`), 同 `SandboxBackend` 模式 (`sandbox.rs:213-218`); 集成路径 = `exec_worker.rs` 可选启用 | (1) microVM 启动 ~125ms 开销 vs 进程级 ~10ms, 不是所有调用都需要 VM, 应按"高危工具"分类启用; (2) KVM 依赖 Linux 内核 ≥4.4, Windows 路径靠 WHP/Hypervisor.framework, 跨平台需双实现 |
| **libkrun** (Red Hat) | C 库 (krun.c) + Rust 绑定 (libkrun-rs), KVM/Hypervisor.framework 后端; 抽象层 C 库 + 语言绑定分层 | ★★★★☆ (中高) | 思路 #3: C 库 + Rust binding 分层 → 自研 `apeireth-sandbox-ffi` trait 中间层 (`SandboxFFI`), 上层 VMSandbox trait 不直接绑 libkrun API | trait 中间层默认 Noop (`probe_available() -> false`), 集成路径 = 上层 VMSandbox 检测 trait 可用再调; 跨平台 Windows 路径同等 stub | (1) C 库 binding 维护成本 (FFI unsafe 全收敛 1 文件, 同 `job_object.rs:29 #![allow(unsafe_code)]` 模式); (2) libkrun 是 KVM/HVF 后端, 不等于"全平台 microVM", Windows 必须 Hyper-V 或 WSL2 后端 |
| **wasmtime** (Bytecode Alliance) | Rust WASM runtime, 组件模型 (WASI Preview 2), fuel metering (确定性执行), epoch interruption | ★★★☆☆ (中) | 思路 #4: fuel metering → `SandboxConfig.fuel_meter: Option<u64>` 字段 (1 fuel = 1 wasm instruction, 我们借用为 "1 fuel = 1 tool call operation unit"); 组件模型 → 我们 trait 接口已是组件化 (`fn run_build_and_test(&self, artifact: &str) -> Result<Verdict, String>`, `experiment_field.rs:45`) | `fuel_meter` 默认 None (不计量, 0 假装); 真接 fuel 算法需独立模块, 不与现有 `cpu_percent` (1-100, `sandbox.rs:91`) 字段冲突 (双轨: cpu = 物理 CPU rate, fuel = 逻辑操作单位) | (1) fuel metering 仅适用确定性执行 (WASM), 借用概念到 OS 进程需定义"1 fuel = ?"映射, 映射不准 = 度量失真; (2) wasmtime 引入为重量依赖 (~6MB 二进制, 编译时间 +40%), 与"轻量 LLM agent" 定位冲突 |

### §1.2 借鉴深度评估细则

#### §1.2.1 smolvm — capability boundary (不直接借鉴)

**为什么不直接借鉴**: smolvm 是 0 star orphan 项目 (主人分析: 维护性 0/5, 设计思路 2/5), Apeireth 已具备等价能力 —
我们的 **8 哲学锚 (S-1/S-2/S-3 质量工程化 NEW/O-1 安全优先 NEW/O-2/O-3/O-4/O-5, per baseline 2026-08-19)** +
**9 重守门 v9 (lineage v6→v7→v8→v9, per Cargo.toml:289 hard_walls)** + **13 键 verdict cache** 共同构成 capability boundary 的工程化对应物。

**具体替代方案 (per O-2 走在前人肩上)**: `crates/apeireth-core/src/eight_anchors.rs:197` panic hardcode 8 锚不可改;
`crates/apeireth-companion/src/sandbox.rs:213-218` `SandboxBackend` trait — capability boundary 由 Rust 类型系统天然实现, 无需 WASM runtime。

**关键判断**: smolvm 用 WASM 解决"安全执行任意代码", 我们用 OS 进程级 Job Object (`crates/apeireth-companion/src/job_object.rs:1` `KILL_ON_JOB_CLOSE`) + RestrictedToken (`crates/apeireth-companion/src/restricted_token.rs:1` `DISABLE_MAX_PRIVILEGE` + `TokenIntegrityLevel`) 解决同一问题, 但路径不同; **借的是思路** (capability 边界, 最小特权), **不是实现** (WASM vs OS)。

#### §1.2.2 Firecracker — minimal API surface (高借鉴)

**借鉴的核心**: Firecracker 整个 API 只有 3 个 syscall (instance 创建 / API socket / VM start), 我们同样应保持 **VMRunner 1 方法** (`experiment_field.rs:42-46`) + **新 VMSandbox 1 方法** (Stage 2 设计 §2.3), 而不是层层 trait 链。

**为什么是"spawn_and_destroy" 而不是 "create+start+stop+destroy"**: Firecracker 实例生命周期 = 单次任务, **一次性 VM** — 创建即销毁, 不复用, 不重启, 不 patch; 借鉴到我们 = 1 调用 1 VM 生命周期。

#### §1.2.3 libkrun — C 库 + Rust 绑定分层 (中高借鉴)

**借鉴的核心**: libkrun 把"hypervisor 交互"封到 C 库 (`krun.c`), Rust 层 (`libkrun-rs`) 只做 safe wrapper;
我们同样应把"OS 原生 FFI" 收敛到 `apeireth-sandbox-ffi` trait 中间层, 上层 `VMSandbox` 不直接调 FFI (对齐现有 `job_object.rs:29` unsafe 全收敛单文件模式)。

**Stage 2 借鉴**: 新增 `apeireth-sandbox-ffi` trait, 内部 unsafe FFI, 外部 safe trait; **不引 libkrun 依赖**, 但保留 trait 形状 (C 库 + Rust binding 分层)。

#### §1.2.4 wasmtime — fuel metering (中借鉴, 概念层)

**借鉴的核心**: wasmtime fuel metering 实现"确定性执行" — 同一段 WASM 消耗同一份 fuel, **不可绕过**, 不可无限循环;
我们借用概念到 `SandboxConfig.fuel_meter: Option<u64>`, 计量单位定义 "1 fuel = 1 tool call operation unit" (粗略)。

**风险声明 (per S-2 实事求是)**: fuel 借用概念到 OS 进程**不是确定性** (1 进程 syscall 调用次数 ≠ 1 进程 instruction),
仅作为粗粒度成本估算; **不能用于"防 DoS"**, 仅用于"LLM 自报成本"。

---

## §2 Part 2 — 自研 3 阶段架构

### §2.1 整体路线图

```
┌────────────────────────────────────────────────────────────────────┐
│ 现状 (master HEAD 2026-08-19)                                       │
│  5 层防线 (洋葱门/审批链/MOVE-STAY/Job Object/最小权限)              │
│  0 装 PASS: SandboxieBackend/LandlockBackend/AppContainer 全 false │
│  NoopVMRunner: 诚实 Err ("VM 实验场未接入")                          │
│  见: sandbox.rs:223-261 + experiment_field.rs:50-56                  │
├────────────────────────────────────────────────────────────────────┤
│ Stage 1 (1-2 周, 本文档设计完成即可开)                                │
│  网络隔离 trait (NetIsolation / Linux netns / Windows WFP)          │
│  默认 NoopNetIsolation (0 装 PASS)                                   │
│  见: §2.2 详细设计                                                   │
├────────────────────────────────────────────────────────────────────┤
│ Stage 2 (2-3 周, 待 Stage 1 验过后)                                  │
│  microVM trait (VMSandbox 1 方法 + SandboxFFI 中间层)                │
│  默认 NoopVMSandbox + NoopSandboxFFI                                 │
│  见: §2.3 详细设计                                                   │
├────────────────────────────────────────────────────────────────────┤
│ Stage 3 (1 周, 待 Stage 2 验过后)                                    │
│  exec_worker.rs 集成 VMSandbox 可选启用 + SandboxConfig.fuel_meter   │
│  默认全 0 装, 高危工具链 (tool-shell / tool-filesystem 写) 才启用      │
│  见: §2.4 详细设计                                                   │
└────────────────────────────────────────────────────────────────────┘
```

### §2.2 Stage 1 — 网络隔离 trait (借鉴 Firecracker minimal API + libkrun netns)

**借鉴来源**: Firecracker minimal API (1 方法原则) + libkrun netns (C 库 + Rust binding 分层)
**借鉴目的**: B 站 UP 主 §5.4 论断 "AppContainer + WFP 出站默认拒绝" 是 Windows 路径正解; 我们借鉴**思路**不抄代码, 用 Rust trait 表达。

#### §2.2.1 核心 trait 设计要点 (设计示意, 0 触碰 src/, 仅 spec)

| 组件 | 设计 | 对齐参考 |
|---|---|---|
| **trait `NetIsolation`** (1 方法, Firecracker minimal 哲学) | `fn isolate_outbound(&self, cfg: &NetIsolationConfig) -> Result<NetGuard, String>` | `sandbox.rs:213-218` 4 方法 → 借鉴收 1 方法 |
| **struct `NetIsolationConfig`** | `{ allow_list: Vec<(String, u16)>, isolation_ttl_secs: u64, reason: String }` (空白名单 = 全拒绝) | `sandbox.rs:87` SandboxConfig 极简风格 |
| **struct `NetGuard`** (RAII, Drop 自动撤销) | `{ isolation_id: u64, backend_name: &'static str }` | `job_object.rs:62` JobGuard Drop 模式 |
| **trait `NetIsolationBackend`** (平台 trait) | `name() / available() / status()` 三件套 | `sandbox.rs:213` SandboxBackend 同款 |
| **struct `NoopNetIsolation`** (0 装 PASS) | `available() -> false`; `isolate_outbound` 返 `Err("NoopNetIsolation: 网络隔离后端未接入 (Linux 走 netns; Windows 走 WFP; 0 装 PASS)")` | `sandbox.rs:223-241` SandboxieBackend 同款 |

**落地文件**: `crates/apeireth-companion/src/net_isolation.rs` (Stage 1 唯一新增文件, 0 改其他文件)

#### §2.2.2 0 装 stub 行为

| 调用方 | 调用方式 | 0 装时行为 |
|---|---|---|
| `exec_worker.rs` (MOVE/STAY 分界, 见 5 层防线 #3) | `let _ = NoopNetIsolation.isolate_outbound(...)` | 返 `Err`, eprintln 记录 "网络隔离未启用 (0 装 PASS)", **不阻断执行** (加固是增强不是门, 同 `sandbox.rs:295`) |
| 高危 tool 入口 (`tool-shell` / `tool-filesystem` 写) | 同上 | 同样降级, 不阻断 |
| 测试 (Stage 1 集成测试) | `assert!(r.is_err())` | 通过 (0 装时必须 Err) |

#### §2.2.3 集成测试设计 (3 测试, 落地文件 `crates/apeireth-companion/tests/net_isolation_stage1.rs`)

| 测试 | 断言 |
|---|---|
| `noop_is_honest_about_unavailability` | `available() == false`; `status()` 含"未接" |
| `noop_isolate_outbound_returns_err` | `isolate_outbound` 返 `Err`; 错误信息含"0 装 PASS"或"未接入" |
| `trait_is_object_safe` | `Box<dyn NetIsolationBackend>` 可构造 (trait object 兼容) |

### §2.3 Stage 2 — microVM trait (借鉴 Firecracker minimal API + libkrun C 库分层)

**借鉴来源**: Firecracker minimal API (1 方法 VMSandbox) + libkrun C 库 + Rust binding 分层
**借鉴目的**: B 站 UP 主 §5.4 "一次性 VM (Firecracker/microVM/QEMU)" 是高危工具正解; 我们用 trait stub 表达, 不接 libkrun 仓库, 0 装 PASS 默认。

#### §2.3.1 核心 trait 设计要点 (设计示意, 0 触碰 src/, 仅 spec)

| 组件 | 设计 | 对齐参考 |
|---|---|---|
| **trait `VMSandbox`** (1 方法, Firecracker 一次性 VM 哲学) | `fn spawn_and_destroy(&self, cfg: &VMConfig) -> Result<VMReceipt, String>` | `experiment_field.rs:42` VMRunner 1 方法 |
| **struct `VMConfig`** | `{ kernel_path/rootfs_path: Option<PathBuf>, memory_limit_mb: u64, vcpu_count: u32, timeout_secs: u64, command: Vec<String> }` (kernel/rootfs Stage 2 占位 None) | `sandbox.rs:87` SandboxConfig 极简风格 |
| **struct `VMReceipt`** | `{ vm_id: u64, exit_code: Option<i32>, stdout/stderr: String, elapsed_ms: u64, killed_by_timeout: bool }` | `experiment_field.rs:33-39` Verdict enum (Pass/Fail) 升级为结构化 receipt |
| **trait `SandboxFFI`** (libkrun 风格 FFI 中间层) | `backend_name() -> &'static str` + `probe_available() -> bool` | `job_object.rs:32` unsafe FFI 收敛模式 (此处 trait 化) |
| **struct `NoopVMSandbox`** (0 装 PASS) | `spawn_and_destroy` 返 `Err("NoopVMSandbox: microVM 后端未接入 (Linux 走 KVM; Windows 走 Hyper-V/WSL2; 真接点为 libkrun binding 或自研 hypervisor shim; 0 装 PASS, 不假装能跑 VM)")` | `experiment_field.rs:50` NoopVMRunner 风格 |
| **struct `NoopSandboxFFI`** (0 装 PASS) | `backend_name = "noop"`, `probe_available = false` | `sandbox.rs:223` SandboxieBackend `available=false` 风格 |

**落地文件**: `crates/apeireth-companion/src/vm_sandbox.rs` + `crates/apeireth-companion/src/sandbox_ffi.rs` (Stage 2 唯一 2 个新增文件)

#### §2.3.2 0 装 stub 行为与冲突检查

| 调用方 | 调用方式 | 0 装时行为 |
|---|---|---|
| `ExperimentField.run` (`experiment_field.rs:113`) 现有路径 | 已用 `NoopVMRunner`, **不变** (本 trait 与 VMRunner 并存) | 仍走 `VMRunner`, 返 Err "VM 实验场未接入" |
| 未来 Stage 3 exec_worker 集成 | `exec_worker.rs` 加可选 VMSandbox 调用 | 0 装时同 Err, 不与现有 MOVE/STAY 分界冲突 |
| Stage 2 集成测试 | 直接构造 `NoopVMSandbox` | 验证 trait 形状 + 错误信息 |

**关键冲突检查 (per §0 严守)**: `NoopVMSandbox` (Stage 2 新增) 与 `NoopVMRunner` (`experiment_field.rs:50`) **不冲突** —
前者是 OS-level microVM (一次性 spawn+destroy), 后者是 experiment VM (构建+测试回执); 抽象层不同, 调用方不同, 共存。

#### §2.3.3 集成测试设计 (3 测试, 落地文件 `crates/apeireth-companion/tests/vm_sandbox_stage2.rs`)

| 测试 | 断言 |
|---|---|
| `noop_vm_sandbox_is_honest` | 错误信息同时含 "0 装 PASS" + "不假装" 双关键词 |
| `ffi_trait_is_separable_from_vm_sandbox` | `Box<dyn SandboxFFI>` 与 `Box<dyn VMSandbox>` 互不依赖, 各自独立 (libkrun 风格分层验证) |
| `noop_vm_sandbox_does_not_break_existing_noop_vm_runner` | 与现有 `ExperimentField::run(NoopVMRunner)` 共存, VMRunner 行为未变 (错误信息仍含"未接入") |

### §2.4 Stage 3 — 集成 (0 装 PASS 默认)

**集成目标**: 在 `exec_worker.rs` (5 层防线 #3 MOVE/STAY 分界) 加**可选** VMSandbox + NetIsolation 调用点;
默认全 0 装, 高危工具链 (`tool-shell` / `tool-filesystem` 写) 才启用。

| 组件 | 设计 | 对齐参考 |
|---|---|---|
| **fn `is_high_risk_tool(name)`** | 匹配 `"shell" \| "filesystem-write" \| "code-search-replace"` (per O-1 安全优先, 见 `eight_anchors.rs:101`) | 新增, 后续可扩展 |
| **struct `HardenedSandbox`** | `{ net: Box<dyn NetIsolation>, vm: Box<dyn VMSandbox> }` | `ExperimentField::runner` (`experiment_field.rs:77`) 双 trait 持有模式 |
| **`impl Default`** | 双 Noop (0 装 PASS 默认) | `SandboxConfig::default()` (`sandbox.rs:106-119`) 风格 |
| **fn `arm_for_high_risk(tool, cfg)`** | 同时调 `net.isolate_outbound` (allow_list=空=全拒绝) + `vm.spawn_and_destroy`; 0 装双双 Err, 返 receipt (加固是增强不是门, 同 `sandbox.rs:295`) | `prepare_child` (`sandbox.rs:300`) 加固失败不阻断语义 |
| **struct `HardenedReceipt`** | `{ net: bool, vm: bool, tool: String }` (boolean, 不含错误字符串) | `PreparedChild` (`sandbox.rs:271`) 简化版 |

**集成测试** (3 测试, 落地文件 `crates/apeireth-companion/tests/sandbox_integration_stage3.rs`):

| 测试 | 断言 |
|---|---|
| `high_risk_tool_triggers_arm_both_layers` | `is_high_risk_tool("shell")=true`; `arm_for_high_risk` 返 receipt 两条 `false`, 不 panic |
| `low_risk_tool_does_not_arm` | `is_high_risk_tool("fetch")` / `"search"` 都 `false` |
| `default_sandbox_uses_noop_double` | `HardenedSandbox::default()` 双 Noop, 双 Err |

**落地文件**: `crates/apeireth-companion/src/sandbox_integration.rs` (Stage 3 唯一新增文件)

---

## §3 Part 3 — 借鉴 4 源的设计元素清单 (思路不代码)

### §3.1 思路 1 (smolvm-like capability boundary): 用我们 4 哲学锚 + 8 哲学锚体系

**借鉴的思路 (per O-2 走在前人肩上, 不抄代码)**: smolvm 用 WASM 强制 capability 边界 (线性类型 + 资源句柄);
我们的等价实现是 **8 哲学锚编译期 hardcode** (`crates/apeireth-core/src/eight_anchors.rs:197` panic 8 锚不可改) +
**9 重守门 v9** (`Cargo.toml:289` hard_walls B4) + **13 键 verdict cache** (`Cargo.toml:289` A3 = 12 原 + PHL-07) 共同构成 capability boundary 的工程化对应物。

**为什么不直接借鉴 smolvm 代码**: smolvm 是 0 star orphan, WASM 沙盒 vs OS 进程级沙盒抽象层错位;
**借鉴的是思路** (强制 capability 边界), **不接仓库** (0 star 项目无维护保障, 引 = 维护孤岛)。

**等价的我们现有能力**:

| smolvm capability | 我们对应物 | 来源 |
|---|---|---|
| WASM 线性类型 (一次性) | Rust `Drop` RAII guard (e.g. `JobGuard` `job_object.rs:62`) | 编译期 + 类型系统 |
| WASM 资源句柄 (capability token) | Rust `Option<RestrictedToken>` (`restricted_token.rs:270`) — `None`=无权, `Some(token)`=持有能力 | 类型系统天然能力 |
| WASM host function 隔离 | `exec_worker.rs` MOVE/STAY 分界 (5 层防线 #3) | 现有代码 |
| 强制最小特权 | `DISABLE_MAX_PRIVILEGE` + `TokenIntegrityLevel` LOW (`restricted_token.rs:11-13`) | 现有代码 |

**结论**: 不直接借鉴 smolvm, 思路已被我们 8 哲学锚 + 9 重 v9 覆盖, 0 接仓库 0 装 PASS 严守。

### §3.2 思路 2 (Firecracker minimal API): VMRunner trait 1 方法 + VMSandbox trait 1 方法

**借鉴的思路**: Firecracker 整个 API 只有 3 个 syscall, **minimal API surface** = 1 trait 1 方法 (不是 trait 链)。

**我们已有 (1 方法)**: `crates/apeireth-companion/src/experiment_field.rs:42-46` VMRunner 1 方法 (借鉴 Firecracker 已落地)。
**Stage 2 新增 (1 方法)**: 见 §2.3.1 `VMSandbox::spawn_and_destroy` 1 方法。

**为什么不写 4 方法 (create/start/stop/destroy)**: Firecracker 一次性 VM 哲学 = 创建即销毁, 1 生命周期 = 1 调用;
写成 4 方法会引入"VM 复用"语义, 与一次性原则冲突; **1 方法** 是 minimal API 的真髓, 不是装饰。

### §3.3 思路 3 (libkrun C 库 + Rust binding): apeireth-sandbox-ffi trait 中间层

**借鉴的思路**: libkrun 把 hypervisor 交互封到 C 库 (`krun.c`), Rust binding (`libkrun-rs`) 只做 safe wrapper;
我们把"OS 原生 FFI" 收敛到 `SandboxFFI` trait 中间层, 上层 `VMSandbox` trait 不直接调 FFI。

**对应现有模式 (`job_object.rs:29`)**: unsafe 全收敛单文件 `#![allow(unsafe_code)]` + 上层 trait 不暴露 FFI (`sandbox.rs:213` SandboxBackend)。

**Stage 2 新增 `SandboxFFI` trait (libkrun 风格分层)**: 见 §2.3.1.2。

**为什么不直接引 libkrun-rs 依赖**: 引外部依赖必须经 9 重 v9 守门 + 主人审, 当前 0 装 PASS 严守,
保留 trait 形状即可拿到分层价值, 0 引入代价。

### §3.4 思路 4 (wasmtime fuel metering): SandboxConfig 加 fuel_meter 字段 (LLM 成本)

**借鉴的思路**: wasmtime fuel metering 实现"确定性执行成本计量", 我们借用概念到 `SandboxConfig.fuel_meter`,
作为**粗粒度 LLM 自报成本** 字段 (非 DoS 防护, 仅成本估算)。

**Stage 3 设计 (待 9 重 v9 守门通过后开, 当前 0 装 PASS 不写)**:

```rust
// 拟修改 crates/apeireth-companion/src/sandbox.rs:87 SandboxConfig 字段
// 注: 本文档 0 触碰 src/, 仅为设计示意, 实际写代码须经主人审 + 9 重 v9 守门
pub struct SandboxConfig {
    // ... 现有字段 (sandbox.rs:87-104) ...
    /// 借鉴 wasmtime fuel metering: 1 fuel = 1 tool call operation unit.
    /// 默认 None (不计量, 0 假装); 真接时由 LLM 自报 + Job Object 双锚定.
    pub fuel_meter: Option<u64>,
}
```

**风险声明 (per S-2 实事求是)**: OS 进程 fuel 借用**不是确定性** (1 进程 syscall 调用次数 ≠ 1 instruction);
仅作为粗粒度成本估算, **不能用于"防 DoS"**, **不能替代 Job Object CPU rate control** (`sandbox.rs:91` `cpu_percent`);
两者并存: `cpu_percent` = 物理 CPU rate, `fuel_meter` = 逻辑操作单位, **双轨不冲突**。

---

## §4 Part 4 — 0 装 PASS 严守承诺

### §4.1 5 阶段验收清单 (借鉴 4 源, 不接库)

| 阶段 | 验收项 | 通过标准 | 0 装 PASS 严守点 |
|---|---|:---:|---|
| **阶段 0 (当前 baseline)** | master HEAD 5 层防线完整, 现有 0 装 PASS trait 已落地 | ✅ baseline 通过 | `sandbox.rs:223` SandboxieBackend `available=false`; `experiment_field.rs:50` NoopVMRunner 诚实 Err; `job_object.rs:1` 严守不假装 |
| **阶段 1 (网络隔离)** | NetIsolation trait + Noop + 集成测试 | ✅ 待落地 (本文档设计完成) | `NoopNetIsolation.isolate_outbound` 必须返 Err; 错误信息含"未接入"; 0 改现有 `SandboxBackend` |
| **阶段 2 (microVM trait)** | VMSandbox trait + SandboxFFI trait + Noop + 集成测试 | ✅ 待落地 (本文档设计完成) | `NoopVMSandbox.spawn_and_destroy` 必须返 Err; 0 触碰 `experiment_field.rs:42` 现有 VMRunner trait |
| **阶段 3 (集成)** | exec_worker 可选 VMSandbox 调用 + HighRiskTool 判定 + Default HardenedSandbox | ✅ 待 Stage 2 验过后 | 0 装时 `arm_for_high_risk` 双 Noop 返 false, 不阻断执行 (加固是增强不是门, 同 `sandbox.rs:295`) |
| **阶段 4 (长期)** | 真接 libkrun 或自研 hypervisor shim + 真接 WFP/netns + wasmtime fuel 算法 | 🔵 路线图, 0 装期不动; 真接须经 9 重 v9 守门 + 主人审 | 0 装时所有 trait 仍返 Err, 不假装已接; 9 重 v9 lineage v6→v7→v8→v9 严守 |

### §4.2 失败语义透明 (所有 trait 方法返 Result, 0 装时 Err 透明)

| trait 方法 | 成功语义 (真接时) | 失败语义 (0 装时) | 错误信息必含关键词 |
|---|---|---|---|
| `NetIsolation::isolate_outbound` | 创建 NetGuard, 白名单生效 | `Err("NoopNetIsolation: 网络隔离后端未接入 (Linux 走 netns; Windows 走 WFP; 0 装 PASS)")` | "未接入" + "0 装 PASS" |
| `VMSandbox::spawn_and_destroy` | spawn VM → 执行 → destroy → VMReceipt | `Err("NoopVMSandbox: microVM 后端未接入 (Linux 走 KVM; Windows 走 Hyper-V/WSL2; 真接点为 libkrun binding 或自研 hypervisor shim; 0 装 PASS, 不假装能跑 VM)")` | "未接入" + "0 装 PASS" + "不假装" |
| `VMRunner::run_build_and_test` (现有) | VM 内构建+测试 → Verdict | `Err("NoopVMRunner: VM 实验场未接入 (smol-vm/libkrun 实现 VMRunner 时启用)")` (现有, `experiment_field.rs:54`) | 现有规范, 不改 |
| `SandboxBackend::available` (现有) | true (真接时) | `false` (现有, `sandbox.rs:227` SandboxieBackend) | 现有规范, 不改 |
| `SandboxFFI::probe_available` (Stage 2 新增) | true (探测到 hypervisor) | `false` (Noop) | `backend_name="noop"` |
| `HardenedSandbox::arm_for_high_risk` (Stage 3 新增) | net + vm 都 true | net=false, vm=false, 但**不阻断执行** (加固是增强不是门) | receipt 含 boolean, 不含错误字符串 |

**关键 (per O-5 不假装, 见 `eight_anchors.rs:101`)**: 所有错误信息**必须**可 grep 到"未接入"或"0 装 PASS",
**禁止** 用模糊错误 (如 "internal error" / "fail" / "未实现") 替代; 0 假装已接 = 0 假装已实现。

### §4.3 测试矩阵 (Stage 1 + Stage 2 + 集成)

```
┌──────────────────────────────────────────────────────────────────────┐
│ Stage 1 集成测试 (net_isolation_stage1.rs)                            │
│  ✓ noop_is_honest_about_unavailability                              │
│  ✓ noop_isolate_outbound_returns_err                                 │
│  ✓ trait_is_object_safe                                              │
├──────────────────────────────────────────────────────────────────────┤
│ Stage 2 集成测试 (vm_sandbox_stage2.rs)                               │
│  ✓ noop_vm_sandbox_is_honest                                         │
│  ✓ ffi_trait_is_separable_from_vm_sandbox                            │
│  ✓ noop_vm_sandbox_does_not_break_existing_noop_vm_runner            │
├──────────────────────────────────────────────────────────────────────┤
│ Stage 3 集成测试 (sandbox_integration_stage3.rs)                     │
│  ✓ high_risk_tool_triggers_arm_both_layers                           │
│  ✓ low_risk_tool_does_not_arm                                        │
│  ✓ default_sandbox_uses_noop_double                                  │
├──────────────────────────────────────────────────────────────────────┤
│ 回归测试 (不破现有)                                                    │
│  ✓ sandbox.rs 16 测试全过 (existing, sandbox.rs:337-541)              │
│  ✓ experiment_field.rs 5 测试全过 (existing, experiment_field.rs:189-281)│
│  ✓ job_object.rs 现有测试全过 (不依赖本设计)                          │
└──────────────────────────────────────────────────────────────────────┘
```

### §4.4 不冲突承诺 (与现有 trait 共存)

| 新 trait | 不冲突的现有 trait | 共存路径 |
|---|---|---|
| `NetIsolation` (Stage 1) | `SandboxBackend` (`sandbox.rs:213`) | 两者并存: SandboxBackend 管 OS 进程级 (Job Object 消费, `sandbox.rs:8`); NetIsolation 管网络级 (出站隔离, Stage 1 新); 调用方独立 |
| `VMSandbox` (Stage 2) | `VMRunner` (`experiment_field.rs:42`) | 两者并存: VMRunner = experiment VM (构建+测试); VMSandbox = OS microVM (一次性 spawn+destroy); 抽象层不同, 调用方不同 |
| `SandboxFFI` (Stage 2) | `imp` 模块 (`job_object.rs:32`) | 两者并存: 现有 `imp` 收敛 Job Object FFI; 新 `SandboxFFI` 收敛 hypervisor FFI; unsafe 全收敛 1 文件 (同 `#![allow(unsafe_code)]` 模式) |
| `HardenedSandbox` (Stage 3) | `prepare_child` (`sandbox.rs:300`) | 两者并存: prepare_child 管子进程受限 token + 目录 ACL; HardenedSandbox 管高危工具 VM + 网络隔离; 调用点不同 (前者 spawn 前, 后者 tool 调用前) |

### §4.5 9 重守门 v9 + 13 键 verdict cache + 3 项不可变脊柱 (综合声明, per `Cargo.toml:289` hard_walls)

| 守门 # | 内容 | 本设计严守动作 |
|---|---|---|
| 1 | 24 LOCKED crate 入口签名 (R148 撤销后仅保 3 项不可变脊柱) | 0 触碰 24 LOCKED crate 入口; 所有 trait 新增在 `apeireth-companion` (非 LOCKED 范围) |
| 2 | workspace.version 1.2.0 (B2 双轴制, per `Cargo.toml:228`) | 0 触碰 `Cargo.toml:228`; 0 触碰任何 crate version |
| 3 | V0.5 24 维 (B3) | 0 触碰 `V05_DIM_COUNT = 24`; 不动 naming-v05 |
| 4 | 9 重守门 v9 lineage v6→v7→v8→v9 (B4) | 不动守门; 真接 libkrun/wasmtime 须经守门 (per §4.1 阶段 4) |
| 5 | 8 哲学锚 S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 (B5) | 0 触碰 `eight_anchors.rs`; 引用哲学锚但不修改枚举 |
| 6 | 三洋葱 R125-5 升级 (B6) | 0 触碰 onion crate; 本设计是 onion 外层加固, 不入洋葱 |
| 7 | 9 organ 内部 fn (B7) | 0 触碰 organ 入口; 不动 state crate |
| 8 | R11 baseline 3 值 0.8682/0.8532/0.9063 (A1) | 0 触碰 baseline 数值; 不引入新基线 |
| 9 | 13 键 verdict cache = 12 原 + PHL-07 (A3) | 0 增 verdict key, 0 改 cache 结构 |
| **3 项不可变脊柱** (R148 撤销扫尾) | **Self-Disable** + **L0 HA** + **13 键 verdict cache** | 三件全部 0 触碰 (Self-Disable 与 L0 HA 与守门 #9 verdict cache 三件, 是 baseline 撤销后仅保不可变脊柱) |

---

## §5 RFC 2119 风格语义总表 (必须 / 应当 / 可以 / 不得)

| 行为 | 适用范围 |
|---|---|
| **必须 (MUST)** 所有 0 装 trait 方法返 `Err` 含"未接入"或"0 装 PASS"; `available()` / `probe_available()` 必须返 `false`; 0 触碰 `Cargo.toml:228` workspace.version; 0 触碰 24 LOCKED crate 入口签名 | 全文 |
| **必须 (MUST)** Stage 1/2/3 Noop 实现均不与现有 `NoopVMRunner` (`experiment_field.rs:50`) 冲突; 0 改 `VMRunner` trait 形状 | Stage 1/2/3 |
| **必须 (MUST)** 新 trait 文件**仅**写入 `crates/apeireth-companion/src/{net_isolation,vm_sandbox,sandbox_ffi,sandbox_integration}.rs` | Stage 1/2/3 |
| **必须 (MUST)** 真接 libkrun / wasmtime / smolvm 须经 9 重 v9 守门 + 主人审; 0 装期 0 引外部依赖 | 阶段 4 |
| **应当 (SHOULD)** 错误信息含自描述字段 (后端名 + 原因 + 阶段号); 新增 `#[derive(Debug)]` + `Send + Sync`; 集成测试覆盖"0 装时" + "非法 cfg" + "不破坏现有" 3 类场景 | Stage 1/2/3 |
| **可以 (MAY)** 借鉴 wasmtime fuel 概念加 `fuel_meter: Option<u64>` 字段到 `SandboxConfig` (Stage 3+ 路线图); Stage 3 `is_high_risk_tool` 白名单可扩展 | Stage 3+ |
| **不得 (MUST NOT)** 使用模糊错误信息 ("internal error" / "fail" / "未实现") 替代"未接入"/"0 装 PASS" | 全文 |
| **不得 (MUST NOT)** 在 0 装期触碰 enum (`PhilosophicalAnchor8` 8 锚, `IntegrityLevel` 3 档, `ExperimentStatus` 5 状态) 或 `const` (`DEFAULT_TIMEOUT_SECS = 30`, `V05_DIM_COUNT = 24`) | 全文 |
| **不得 (MUST NOT)** 直接借鉴 smolvm 仓库代码 (0 star orphan); 抄 Firecracker API 形状 (minimal 思路可借鉴, 形状不抄); 0 装期引外部依赖 (libkrun / libkrun-rs / wasmtime / smolvm) | 全文 |

---

## §6 文档元信息 (per O-4 任何人都能接手)

| 项 | 内容 |
|---|---|
| 报告路径 | `reports/sandbox-self-research-design-2026-08-19.md` |
| 引用文件 | `crates/apeireth-companion/src/sandbox.rs:1-542`, `experiment_field.rs:1-282`, `job_object.rs:1-462`, `restricted_token.rs:1-548`, `app_container.rs:1-...`, `crates/apeireth-core/src/eight_anchors.rs:1-...`, `Cargo.toml:228,289` |
| 借鉴 4 源 | smolvm (klispweify) / Firecracker (AWS Lambda) / libkrun (Red Hat) / wasmtime (Bytecode Alliance) — 仅参考公开文档, 0 接仓库 |
| 8 哲学锚穿透 | S-1 (沙盒服务 ASI 北极星) / S-2 (4 源对比 + 借鉴深度实查, 0 编造) / S-3 (设计 + 测试覆盖) / O-1 (0 装 PASS + 9 重 v9 严守) / O-2 (借鉴思路不抄代码) / O-3 (3 阶段 + 4 部分写到底) / O-4 (本元信息表 + 行号引用) / O-5 (失败语义透明 + 错误信息必含"未接入"/"0 装 PASS") |
| 0 触碰承诺 | 0 改 `src/` / 0 改 `Cargo.toml` / 0 改 enum/const / 0 引外部依赖 |
| 借鉴 ID | R-side sandbox-2026-08-19-borrow-4sources (等待整合 #6 commit 时机拍板) |
| 后续路径 | (a) 主人审 → (b) Stage 1 落地 (1-2 周) → (c) Stage 1 测试全过 → (d) Stage 2 落地 → (e) Stage 3 集成; 全程 9 重 v9 + 13 键 verdict cache 不破 |

> **0 主动 commit 严守**: 本文档写到 `reports/` 但 0 主动 commit, 等整合 #6 commit 时机拍板 (per `Cargo.toml:289` C1 0 主动 commit 已放宽 R126 → 实际指"调研报告"类文档仍待主人拍板)。
>
> **0 主动 push 严守**: 0 主动 push, 等 1.0 release 配 GitHub remote (per `Cargo.toml:289` 0 主动 push 严守)。

---

## §7 中文 commit msg 模板 (待整合时用)

```
docs(sandbox): 借鉴 4 源 (smolvm/Firecracker/libkrun/wasmtime) + 自研 3 阶段架构设计

- 严守 0 装 PASS (Noop 返诚实 Err, 不与 NoopVMRunner 冲突) + 9 重 v9 + 13 键 verdict cache
  + 3 项不可变脊柱 (Self-Disable/L0 HA/verdict cache) + workspace.version 1.2.0
  + 0 触碰 24 LOCKED crate 入口 + 8 哲学锚穿透 (S-1/S-2/S-3 质量工程化/O-1 安全优先/O-2/O-3/O-4/O-5)
- 借鉴深度: Firecracker minimal API ★★★★★ / libkrun C 库分层 ★★★★☆
  / wasmtime fuel 概念 ★★★☆☆ / smolvm capability ★★☆☆☆ (不直接借)
- 3 阶段架构: Stage 1 NetIsolation / Stage 2 VMSandbox+SandboxFFI / Stage 3 HardenedSandbox 集成
- 报告: reports/sandbox-self-research-design-2026-08-19.md
- 借用 ID: R-side sandbox-2026-08-19-borrow-4sources
```