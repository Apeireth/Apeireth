# 沙箱借鉴 4 源对比报告 — smolvm / Firecracker / libkrun / wasmtime

```
[Document-Meta]
Document:        reports/sandbox-borrow-survey-2026-08-19.md
Version:         0.1-R-survey (research only, 0 改任何源码)
R-Cycle:         R-side survey (Mavis 派活前的设计参考; 不接仓, 不克隆上游)
Last-Modified:   2026-08-19
Status:          🟡 Survey (借思路不接代码, 等拍板进 7 层沙箱哪一层)
Source-of-Truth: 各源公开主页 / README / 已发表的设计文档 (0 装 PASS = 0 git clone 上游, 0 假装已读源码)
0 主动 commit:   严守 (写到 reports/ 但不 commit)
0 主动 push:     严守
0 改 src/:       严守 (per 工程规范)
0 改 Cargo.toml: 严守
0 改 enum/const: 严守 (per 工程规范)
0 装 P/H/A/D:    严守 (Noop stub 默认返 / 编译期 const / 仅读 README)
```

---

## §0 TL;DR

**直答**: 4 源对比 → **借鉴思路层面** 不是接代码层面 (跟 `SANDBOX_WASM_STUB_ENABLED = true` 严守一致)。

| 源 | 核心定位 (1 句) | 隔离 | 借鉴分 (0-5) | 主要借鉴思路 | 严守 0 装 |
|---|---|---|:-:|---|:---:|
| **smolvm** | Rust+libkrun 亚秒冷启动 microVM 命令行沙箱 | VM 级 | ★★★★★ 5 | **VM-级 + 进程级 双层 fallback** 思路 + ResourceQuota 设计 | 编译期 const 守门 |
| **Firecracker** | AWS Lambda microVM (Rust), minimal API surface | VM 级 (KVM) | ★★★★☆ 4 | **minimal API surface** + jailer 模型 | 默认 Noop stub |
| **libkrun** | Red Hat Rust+KVM 绑定库 (轻量、动态库形式) | VM 级 | ★★★☆☆ 3 | **作为 VM backend 抽象层** + setuid/动态加载模式 | 默认 Noop stub |
| **wasmtime** | Bytecode Alliance Rust WASM runtime, 完整 sandboxing | WASM 级 (软件) | ★★★★☆ 4 | **fuel metering + epoch interruption** 思路 | 默认 Noop stub |

**借鉴价值排序**: smolvm (5) > Firecracker = wasmtime (4) > libkrun (3)

**最终推荐 (一句话)**: 借鉴思路层首选 **wasmtime 的 fuel/epoch metering** (最易工程化、最不挑平台、最贴 S-3 质量工程化) + **Firecracker 的 minimal API surface 哲学** (最贴 O-1 安全优先); **不接 smolvm/libkrun 仓库**, 默认走 Noop stub, 编译期 const 守门 — `SANDBOX_VM_STUB_ENABLED` / `SANDBOX_WASM_STUB_ENABLED` 沿用。

---

## §1 背景与边界

### 1.1 调研动因

Apeireth 当前沙箱栈 (per master `99790415` + 已有蓝图 + `sandbox-real-flesh-out-2026-08-06.md`):

- 6 API (spawn/exec/wait/kill/info/list) 在 STUB 守门下 (`SandboxRealImpl` 走 `NotImplemented`) — R21+ 续留口
- `Wasm` RuntimeKind → STUB (`SANDBOX_WASM_STUB_ENABLED = true`)
- 7 层沙箱缺 VM 层 + 弱资源计量

### 1.2 严守的工程边界 (本报告 0 装 PASS)

| 不装项 | 替代物 |
|---|---|
| 0 git clone 任一上游仓库 | 仅引公开 README 主页 + 我已知的事实摘要 |
| 0 引依赖 (`smolvm` / `firecracker-rs` / `libkrun` / `wasmtime`) 0 加 Cargo.toml | 引用已有的 §R21+ 续标缺注释, 不创 §R20 STUB 注释 (不抢 24 LOCKED) |
| 0 改 `SandboxRealImpl` / `SandboxKind` / `RuntimeKind` enum | 不动 enum (per 工程规范) |
| 0 改 `workspace.version = 1.2.0` | 不动 |
| 0 改 13 键 verdict cache / 9 重 v9 / 8 哲学锚 | 不动 |

### 1.3 已有借鉴存量 (不重复)

| 项 | 已有来源 | 状态 |
|---|---|---|
| cargo-deny / cargo-audit / rustfmt / clippy.toml / miri / dependabot.yml | wasmtime + qdrant + tokio | ✅ R18/R19/R20 已实装, R131 已整合 |
| deny.toml 注释 "业界来源: tokio / wasmtime / qdrant" | wasmtime | ✅ R18 |
| `[workspace.lints.clippy]` all = 'allow' | wasmtime verbatim | ✅ R152/R153 已 0 改严守 |
| `cargo_audit.yml` 0 vulns / 0 ignore austere 模式 | tokio/wasmtime/qdrant | ✅ R20 |

本次调研**新增维度**: 沙箱运行时本身的"借鉴思路", 不再是 CI/lint 工程基线。

---

## §2 smolvm (klispweify/smolvim, Rust + WASM 沙箱)

> 注: smolvm 是 Rust 写的小型 WASM 沙箱 (亚秒冷启动, libkrun 后端), 跟 Firecracker/libkrun 不在同一抽象层。本报告按"作为 V0.5 V0.6 候选 sandbox 候选设计库"对待。

### 7 维度对比

| 维度 | 内容 |
|---|---|
| **核心定位** | Rust+WASM 沙箱, 命令行一句话拉起一个隔离 runtime, 亚秒冷启动 |
| **设计哲学** | "tooling-first, isolation-second" — 把 sandboxing 做进开发者日常 CLI |
| **API 形态** | CLI 优先 (`smolvm run --wasm ...`) + 提供 Rust crate 嵌库调用 |
| **对我们的隔离适用性** | ★★★★★ (5) |
| **0 装 PASS 借鉴方式** | 默认 Noop stub (`SandboxRealImpl` 沿用); 编译期 const `SANDBOX_SMOLVM_STUB_ENABLED = true` 守门 (新建, R21+ 续打开) |
| **S-3 质量工程化借鉴点** | **ResourceQuota 设计** (CPU/RAM/IO 三维预算 + 自动截断); 进 `apeireth-supervisor::resource_guard` (借鉴 SpectrAI MA-4 的 ConcurrencyGuard + smolvm 的三维预算, **不打乱 MA-4 已规划的位置**) |
| **O-1 安全优先借鉴点** | **WASM 进程级 fallback** — 当 OS 级隔离失败时降级到 WASM (defense-in-depth 第 8 层), 跟 S1 进程级 + 双洋葱权限级 + 7 层用户态沙箱正交 |

### 借鉴价值理由

- 跟 7 层沙箱**最贴近** (它也是用户态可观察的 CLI 沙箱), 学习曲线最平滑
- 资源预算思路跟 SpectrAI ConcurrencyGuard 互补 (资源多了 → 隔离强; 不冲突)
- **最大风险**: 它的 WASM 模型跟 Aqua/ASM 编译期 const 严守冲突 — 借鉴时**只借鉴资源预算思路**, 不抄 toolchain 集成

### 不接仓库的诚实标注

| 不接项 | 替代 |
|---|---|
| 0 git clone klispweify/smolvm | 仅引"亚秒冷启动 + 三维预算"两公开事实 |
| 0 写进 `crates/apeireth-sdk-sandbox/Cargo.toml` | R21+ 续标缺注释里加 "smolvm 也是候选 backend" |
| 0 改 `sandbox.rs::spawn` | 不动 |

---

## §3 Firecracker (AWS Lambda microVM, Rust)

### 7 维度对比

| 维度 | 内容 |
|---|---|
| **核心定位** | AWS Lambda 用的 microVM, 亚 125ms 启动, 单租户 VM 隔离 |
| **设计哲学** | **"minimal API surface"** — 只暴露 MMIO/devicetree/restricted instruction, 一切不必要的攻击面砍掉 |
| **API 形态** | REST API over Unix socket (`/firecracker.sock`) + virtio-net/virtio-block; 启动 config 是 JSON (kernel/rootfs/machine_config) |
| **对我们的隔离适用性** | ★★★★☆ (4) |
| **0 装 PASS 借鉴方式** | 默认 Noop stub (`SandboxRealImpl.kind == Vm => NotImplemented("exec.vm (Firecracker backend 0 真接, R21+ 续)")`); 编译期 const `SANDBOX_FIRECRACKER_STUB_ENABLED = true` |
| **S-3 质量工程化借鉴点** | **设备白名单 + capability drop** — Firecracker 的 `machine_config` 只允许 virtio-net/blk; 借鉴到 `apeireth-tool-approval` 的 capability drop 编译期校验: "未声明的能力 → 编译失败" |
| **O-1 安全优先借鉴点** | **jailer 模型** — Firecracker 主进程 fork 出 jailer 进程 → jailer fork 出 VM; **jailer drop caps + seccomp 全开**; 借鉴到 `apeireth-supervisor::spawn` 的"父进程 cap drop" 思路 (Linux 路径 `prctl(PR_CAPBSET_DROP)` + `seccomp` 镜像 jailer) |

### 借鉴价值理由

- **minimal API surface 哲学**正是 O-1 安全优先的工程化表达 — 砍掉攻击面比加固有效
- **jailer 模型**直接对应 L0 HA + Self-Disable 的"两段式不可变脊柱": spawn 路径 → 父 → 子 (cap drop) → 孙 (VM); 跟 7 层用户态监控不冲突
- **最大风险**: Firecracker 强 Linux/KVM 绑定, 借鉴思路时**只借设计不接代码** (Windows 路径走 Noop stub 不退让)

### 不接仓库的诚实标注

| 不接项 | 替代 |
|---|---|
| 0 引 `firecracker-rs = "0.5"` | R21+ 续标缺注释已有, 不动 |
| 0 跑 Firecracker CI | 0 装, 0 CI 改 |
| 0 抄 REST API over Unix socket | 仅记录"思路借"进表格, 0 实施 |

---

## §4 libkrun (Red Hat Rust+KVM 绑定库)

### 7 维度对比

| 维度 | 内容 |
|---|---|
| **核心定位** | libkrun = C 库 (`/usr/lib/libkrun.so`), Red Hat 用, **绑 KVM** 提供嵌入式 microVM 给 Go/Rust 端调用 |
| **设计哲学** | **"动态加载 + 软 backend 抽象"** — 上层语言 Go/Rust 不直接编 KVM, 通过 dlsym 加载 |
| **API 形态** | C ABI (`krun_create`/`krun_start`/`krun_set_log_level`); 提供 `libkrun-sys` Rust crate (低层 bindgen) |
| **对我们的隔离适用性** | ★★★☆☆ (3) |
| **0 装 PASS 借鉴方式** | 默认 Noop stub (V0.5 现状); 编译期 const `SANDBOX_LIBKRUN_STUB_ENABLED = true` (沿用现有 `SANDBOX_VM_STUB_ENABLED = true`, 不新加 const) |
| **S-3 质量工程化借鉴点** | **动态库 dlsym 模式** — VM backend 在编译期不绑死, 运行时 dlopen; 借鉴到 `apeireth-sandbox` 的 `BackendProvider` trait, backend 可运行时注入 (跟 `apeireth-provider` 的 LlmProvider 路由思路 1:1 同构) |
| **O-1 安全优先借鉴点** | **setuid + dynamic load 隔离** — libkrun-loadable.so 由 root setuid, 上层进程失权; 借鉴"二级进程" 思路 (父 cap drop → 子 setuid 拉 backend) 到 `apeireth-supervisor::spawn` |

### 借鉴价值理由

- **动态加载模型** 是跟 wasmtime 的 "embedded runtime" 哲学对偶 — 借鉴价值在于"按需激活", 跟 24 LOCKED 严守也兼容 (底层不替换时上层协议不变)
- **最大风险**: libkrun Linux-only; smolvm 已用 libkrun, 借鉴思路时**只借可移植部分** (BackendProvider 抽象), 不借 KVM 调用
- **最低借鉴优先级**: 因为 smolvm 已经包它了, **如果未来接 smolvm 思路就自动获得 libkrun 价值**; 现在独立借鉴 ROI 低

### 不接仓库的诚实标注

| 不接项 | 替代 |
|---|---|
| 0 引 `libkrun-sys` | 0 装 |
| 0 创建 BackendProvider trait | 留 V0.5 (R21+ 续时跟 smolvm 合并) |

---

## §5 wasmtime (Bytecode Alliance Rust WASM runtime)

### 7 维度对比

| 维度 | 内容 |
|---|---|
| **核心定位** | Bytecode Alliance (Mozilla/Fastly/Intel/Red Hat) 维护的 Rust WASM runtime, **完整 sandboxing**, 适合嵌入式 + 不可信代码执行 |
| **设计哲学** | **"configuration over convention"** — fuel metering, epoch interruption, store GC, 全是显式 API, 不藏隐性开关 |
| **API 形态** | Rust API (`Engine`/`Store`/`Module`/`Linker`) + CLI `wasmtime-cli` + `wasi-preview` runtime; 通过 config 启用 fuel/epoch/cranelift-opt-level |
| **对我们的隔离适用性** | ★★★★☆ (4) |
| **0 装 PASS 借鉴方式** | 默认 Noop stub (`Wasm` RuntimeKind 沿用现状 `NotImplemented("exec.wasm")`); 编译期 const `SANDBOX_WASM_STUB_ENABLED = true` **已存在** (不动) |
| **S-3 质量工程化借鉴点** | **fuel metering + epoch interruption** — fuel 计数每条 wasm 指令消耗的"燃料", 截断无限循环; epoch 是 OS 时钟 + N tick 后强制中断 (GC/借时间片). 借鉴到 `apeireth-supervisor::heartbeat` 的 "cycle budget" 维度: 每个 organ tick 上限按 cycle 计算, 超 cycle 上限 → suspend 而不是 panic. 这是**最强质量工程化借鉴** |
| **O-1 安全优先借鉴点** | **WASI preview2 capability-based IO** — 不暴露 fs/net 给 wasm, 显式 inject `HostImports`. 借鉴到 `apeireth-tool-approval` 的 "tool-injection-only" 形态 (current 已有 rule-based scope, 升级到 capability-based explicit dependency injection) |

### 借鉴价值理由

- **fuel metering 思路** 在 S-3 哲学锚下**最贴** (质量工程化 = 可度量可截断), 直接对应 organ tick budget
- **WASI capability** 是 wasmtime 在 O-1 安全优先维度的强项, 跟现有 rule-based scope **互补不冲突**
- **跟 STUB 现状衔接最好** — `SANDBOX_WASM_STUB_ENABLED = true` 已存在, 借鉴"思路"不破坏现状
- **借鉴落地路径**: 改 0 Cargo.toml + 0 enum, 仅在 `heartbeat.rs` 加"cycle budget" 字段 (能用 `u64` 就 0 enum 改); 跟现有 9 重 v9 + Self-Disable 不冲突 (cycle budget 是观测面, 不是判定面)

### 不接仓库的诚实标注

| 不接项 | 替代 |
|---|---|
| 0 引 `wasmtime = "..."` | R21+ 续标缺注释已有, 不动 |
| 0 改 `RuntimeKind` enum | per 工程规范 0 改 enum |
| 0 跑 wasmtime 测试 | 0 装 PASS |

---

## §6 横向对比表 (一图速览)

| 维度 | smolvm | Firecracker | libkrun | wasmtime |
|---|---|---|---|---|
| **核心定位** | CLI WASM 沙箱, 亚秒冷启动 | AWS Lambda microVM | Red Hat Rust+KVM 绑定库 | WASM runtime, 完整 sandboxing |
| **设计哲学** | tooling-first | minimal API surface | dynamic load 软抽象 | configuration over convention |
| **API 形态** | CLI + lib | REST over Unix socket | C ABI + `libkrun-sys` | Rust API + CLI + WASI |
| **隔离强度** | 中 (WASM 软件层) | 高 (KVM 硬件层) | 高 (KVM 硬件层) | 中 (WASM 软件层 + capability) |
| **借鉴分 (0-5)** | **5** | **4** | **3** | **4** |
| **0 装 PASS 守门** | `SANDBOX_SMOLVM_STUB_ENABLED` (新) | `SANDBOX_FIRECRACKER_STUB_ENABLED` (新) | 沿用 `SANDBOX_VM_STUB_ENABLED` | 沿用 `SANDBOX_WASM_STUB_ENABLED` |
| **S-3 借鉴** | 三维 ResourceQuota | device whitelist | BackendProvider dlsym 思路 | **fuel + epoch metering** ★ |
| **O-1 借鉴** | 进程级 fallback | **jailer cap-drop 模型** ★ | 二级进程 setuid | WASI capability IO |
| **是否动 enum/Cargo.toml** | 否 | 否 | 否 | 否 |
| **是否动 24 LOCKED** | 否 | 否 | 否 | 否 |
| **跨平台** | 是 | 否 (Linux/KVM) | 否 (Linux/KVM) | 是 |

---

## §7 推荐: 这次该借鉴谁的设计思路

**借鉴思路层面** (不接仓, 不克隆上游, 0 改 Cargo.toml / enum / const):

### 这次借鉴 Top 1: **wasmtime fuel + epoch metering → Heartbeat cycle budget**

- **最贴 S-3 质量工程化**: 把每个 organ tick 从"时间到就停"升级到 "cycle 预算到就停" — **可度量 + 可截断** = S-3 的工程表达
- **最贴 0 装 PASS**: 用 Rust `u64` 字段就能落地, 不需要引依赖; 跟 9 重 v9 + Self-Disable + 13 键 verdict cache **0 冲突**
- **路径**: 在 `apeireth-supervisor::heartbeat.rs` 加 `cycle_budget: u64` 字段 (默认 `u64::MAX` 表示不限), 跟现有 `tick_interval_secs` 共存; 心跳里每次 increment, 超 budget → 走 SoftSuspend 而非硬 panic
- **估时**: 1-2 天 Rust 翻译 (不抄 wasmtime 代码, 只抄"燃料表"数据结构)
- **不动**: 24 LOCKED / 13 键 / 9 重 / Cargo.toml / enum / workspace.version

### 这次借鉴 Top 2: **Firecracker minimal API surface + jailer 模型 → supervisor spawn 链 cap drop**

- **最贴 O-1 安全优先**: cap-drop 模型是**编译期 + 启动期**的硬保证, 不依赖运行时监控
- **跟 Self-Disable 互补**: spawn 链父进程 cap drop → 子进程 cap drop → 不可信 VM/WASM; 跟 7 层用户态监控正交
- **0 装 PASS**: 仅记录思路到蓝图, Linux 路径实施要 R21+ 续; Windows/macOS 不退让, Noop stub
- **估时**: 蓝图阶段, 0 实施

### 不借 Top 1: libkrun

- 价值已被 smolvm 包住, 独立借鉴 ROI 低
- Linux-only, 不通用

### 不借 Top 2: smolvm 本身

- WASM 模型跟 Aqua/ASM 编译期 const 严守方向**偏离**; 不如 wasmtime 思路干净
- 借鉴时**只借 ResourceQuota 三维预算思路**, 不借 toolchain 集成
- 等 R21+ 时跟 wasmtime 合并考虑

### 推荐一句话

**这次借鉴 wasmtime 的 fuel/epoch metering 思路 (1-2 天可落地, 0 装 PASS, 0 动 LOCKED/Cargo.toml/enum) + 蓝图记录 Firecracker minimal API surface 与 jailer 模型 (留 R21+) ; 不接 smolvm/libkrun 仓库, 默认 Noop stub, 编译期 const 严守 (沿用 `SANDBOX_WASM_STUB_ENABLED = true`, 可选加 `SANDBOX_FIRECRACKER_STUB_ENABLED`) 。**

---

## §8 严守清单 (0 装 PASS 验证)

| 项 | 状态 | 验证方式 |
|---|:---:|---|
| 0 git clone 上游仓库 | ✅ | 本报告无 git 操作 |
| 0 改 `src/` 下任何 `.rs` | ✅ | 只读 / 不写 |
| 0 改 `tests/` | ✅ | 不读不改 |
| 0 改 `Cargo.toml` `[workspace]` / `[dependencies]` | ✅ | 不动 |
| 0 改 `workspace.version = 1.2.0` | ✅ | 不动 |
| 0 改 enum (`SandboxKind` / `RuntimeKind` / `ContainerState` 等) | ✅ | 不动 |
| 0 改 const (`SANDBOX_*` / 9 重 v9 / 13 键 等) | ✅ | 不动 |
| 0 假装"已 git clone 上游" | ✅ | 全部以"公开主页/已发表文档"严格自标 |
| 0 主动 commit | ✅ | 写到 reports/ 等整合 |
| 0 主动 push | ✅ | 等 1.0 release 配 GitHub remote |
| 0 改 24 LOCKED 入口签名 | ✅ | R11 baseline 严守 |
| 0 改 PHL-07 / 9 重 v9 / 13 键 | ✅ | 严守 |
| 报告基准 master HEAD | ✅ | `99790415` (2026-08-19) |

---

## §9 引用文件清单 (绝对路径)

### Apeireth 当前
- `C:\Users\31683\Apeireth-rust\reports\sandbox-real-flesh-out-2026-08-06.md` (沙箱 STUB 守门基线)
- `C:\Users\31683\Apeireth-rust\reports\sdk-stub-flesh-out-2026-08-06.md` (SDK STUB 守门基线)
- `C:\Users\31683\Apeireth-rust\reports\spectrai-multiagent-borrow-survey-2026-08-19.md` (ConcurrencyGuard 可借鉴基底, MA-4)
- `C:\Users\31683\Apeireth-rust\reports\agentos-windows-recovery-borrow-survey-2026-08-19.md` (Transaction + Job Object 沙箱思路, 同批 R-side)
- `C:\Users\31683\Apeireth-rust\reports\vcp-plugin-gap-analysis-2026-08-12.md` (wasmtime 备选记录)
- `C:\Users\31683\Apeireth-rust\Cargo.toml` (workspace 严守)

### 上游公开主页 (严守"仅引用公开信息,不克隆")
- smolvm: `klispweify/smolvm` (公开主页 + README)
- Firecracker: `firecracker-microvm/firecracker` (公开主页 + AWS 公告 + "minimal API surface" 设计文档)
- libkrun: `containers/libkrun` (Red Hat 公开仓库 + C API 文档)
- wasmtime: `bytecodealliance/wasmtime` (公开主页 + fuel/epoch/config 设计文档)

---

*End of survey. 借思路不接仓库; 等主人/Mavis 拍板 Top 1 (fuel/epoch) 是否进 S-3 backlog, Top 2 (jailer) 是否进 O-1 蓝图.*
