# R170 sovereignty Hyperlight micro-VM 隔离调研

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R170 (research-only, no code)
> **日期**: 2026-08-13
> **borrow-id**: R170-SOV-BORROW-hyperlight-4.4k-stars-2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. 调研目的

apeireth-sovereignty 当前 Self-Disable 设计在 3 个不可变脊柱上 — L0 HA 物理隔离、Self-Disable 判定逻辑、13 键 verdict cache 语义。R170 不动 3 脊柱, 仅调研 apeireth-sovereignty 之上的 **tool isolation layer** 是否能升级到 micro-VM 级别, 替换当前的 Process/Container/WASM 三栈 stub。

调研对象: **Hyperlight** (https://github.com/hyperlight-dev/hyperlight) — 4.4K stars, Rust 写, micro-VM, <1ms cold start, KVM/Hyper-V/mshv 多 hypervisor 后端, Wasmtime 嵌入式集成。

---

## 1. Hyperlight 核心能力 (GitHub 公开数据, 2026-08-13)

| 维度 | 值 | 来源 |
|---|---|---|
| Stars | 4.4K | github.com/hyperlight-dev/hyperlight |
| 主语言 | Rust (97.4%) | 同上 |
| License | Apache-2.0 | 同上 |
| Cold start | < 1 ms | 项目 README |
| Hypervisor 后端 | KVM (Linux), Hyper-V (Windows), mshv (MS Hyper-V) | 项目文档 |
| Wasmtime 集成 | embedded (wasmtime fork) | 项目依赖 |
| 安全模型 | 每个 guest = 独立 micro-VM, VMX/SVM hardware-virt isolated | 设计文档 |
| Sandbox 模型 | 进程级 -> VM 级 | 同上 |

关键卖点 (与 sandbox.rs 当前对照):
- 超低冷启动 < 1ms — 远快于 Docker container (100ms+) 和 Firecracker (125ms+)
- hypervisor-native — 直接用 VMX/SVM CPU 指令, 不走 QEMU 用户态仿真
- Wasmtime embedded — Rust 应用直接链接, 不需要独立 wasm runtime 进程
- 3 hypervisor 后端统一抽象 — Hyperlight trait 抽象, 同一份代码 Linux/Windows 都能跑

---

## 2. 当前 apeireth-sandbox 状态评估

sandbox.rs 当前是 3 后端 trait stub:
- ContainerBackend (bollard / Docker) — 真接率 < 20%
- ProcessBackend (std::process) — 真接率 ~ 80%
- WasmBackend (wasmtime) — 真接率 ~ 30%

问题:
1. Container 启动慢, 不适合高频 tool call
2. Process 隔离弱, 仅靠 seccomp/AppArmor/Job Object
3. Wasmtime 真接率低, 仅用于 1-2 个演示工具

升级诉求: 想要 Process 速度 + Container 隔离 + Wasmtime 生态 — Hyperlight 正好三合一。

---

## 3. 设计草案: apeireth-sovereignty 之上的 Hyperlight 隔离层

> 强约束: 本节纯设计, 0 触碰 3 不可变脊柱。

### 3.1 架构图

```
+-----------------------------------------------+
|  apeireth-tool-runtime (tool call dispatcher) |
+-----------------------------------------------+
                      |
                      v
+-----------------------------------------------+
|  apeireth-sovereignty::ToolIsolation (新)    |  <-- R170+ 设计目标
|  - 选择 isolation backend (per tool policy)   |
|  - 监控 hypervisor health                    |
+-----------------------------------------------+
        |              |              |
        v              v              v
+-------------+  +-------------+  +-------------+
| Hyperlight  |  | Process     |  | Wasmtime    |
| (micro-VM)  |  | (fallback)  |  | (fallback)  |
+-------------+  +-------------+  +-------------+
```

### 3.2 关键 trait 设计 (草案, 待 R170+ 实施)

apeireth-sovereignty/src/tool_isolation.rs (新文件, R170+ 实施)

```rust
pub trait ToolIsolation: Send + Sync {
    /// 工具执行前的隔离环境准备
    async fn prepare(&self, tool_id: &ToolId, policy: &IsolationPolicy) 
        -> Result<IsolationHandle, IsolationError>;
    
    /// 工具执行 (在隔离环境中)
    async fn execute(&self, handle: IsolationHandle, 
        payload: ExecutionPayload) -> Result<ExecutionResult, IsolationError>;
    
    /// 隔离环境回收
    async fn teardown(&self, handle: IsolationHandle) -> Result<(), IsolationError>;
    
    /// hypervisor 健康检查 (per 24h reflection)
    async fn health_check(&self) -> Result<HealthReport, IsolationError>;
}

pub enum IsolationBackend {
    Hyperlight(HyperlightConfig),
    Process(ProcessConfig),
    Wasmtime(WasmtimeConfig),
}

pub struct IsolationPolicy {
    pub risk_level: RiskLevel,
    pub cpu_quota: Option<CpuQuota>,
    pub memory_limit: Option<MemoryLimit>,
    pub network_egress: Option<EgressAllowlist>,
    pub fs_readonly_paths: Vec<PathBuf>,
}
```

### 3.3 风险等级 -> 后端映射 (默认策略)

| Tool risk level | 默认 backend | 理由 |
|---|---|---|
| Critical (shell-exec, file-write-root) | Hyperlight | 强隔离, 速度够用 |
| High (陌生域名 http-fetch, code-exec) | Hyperlight | VM 隔离 |
| Medium (allowlist http-fetch, file-modify) | Process + seccomp/AppArmor | 速度优先 |
| Low (read-file, parse-json) | Process | 无需隔离开销 |
| Pure-compute (math, transform) | Wasmtime | 已有, 无需 VM |

策略可由工具注册时 override (apeireth-tool-registry::ToolMetadata.isolation_policy)。

### 3.4 与 3 不可变脊柱的关系 (R170 设计原则)

| 不可变脊柱 | R170 关系 |
|---|---|
| Self-Disable 判定逻辑 | 0 触碰 — ToolIsolation 是其上层消费者, 不替代 |
| L0 HA 物理隔离 | 0 触碰 — L0 HA 仍由 physical_multisig.rs 管 |
| 13 键 verdict cache 语义 | 0 触碰 — verdict 仍由 verdict_cache.rs 算 |

ToolIsolation 设计原则:
- 不参与 agent 是否应该被 disable 的判定 — 仅执行 tool 隔离
- 不参与 verdict key 是什么的计算 — 仅消费 verdict 决定 risk level
- 不参与物理多签如何启动的流程 — 仅在 prepare() 时根据 policy 决定 backend

这样 3 脊柱永远不被新的隔离层穿透。

---

## 4. Hyperlight 集成候选路径 (R170+ 评估)

| 路径 | 优 | 劣 |
|---|---|---|
| A. hyperlight 直接依赖 | 立即可用, 4.4K stars 社区 | 仍 0.x (pre-1.0), API 可能 break |
| B. fork 内置到 sovereignty | 完全可控, 0 外部 dep | 维护成本高, 失去上游更新 |
| C. cargo update 仅消费 | 折中 | 需要跟踪上游 breaking change |
| D. 自研 micro-VM 抽象层 | 极致可控, 0 引外部 dep | 工作量大, 短期不可行 |

R170 推荐路径: A + 抽象层包装 — 直接依赖 hyperlight, 但通过 ToolIsolation trait 抽象, 未来可平滑切换到 B/D。Cargo.toml 加一行: hyperlight = { version = "0.x", optional = true }, 配 feature micro-vm。

---

## 5. 工作量估算 (R170+ 实施阶段)

| 阶段 | 工时 | 交付 |
|---|---|---|
| R170 (本档) | 完成 | research doc |
| R170+1  | 0.5 day | ToolIsolation trait 骨架 + 3 backend stub |
| R170+2  | 1 day | Hyperlight 真接 (Linux KVM 优先), health_check |
| R170+3  | 1 day | policy 引擎 (risk_level 自动判定 + override) |
| R170+4  | 0.5 day | apeireth-tool-runtime 接入, 演示工具 |
| R170+5  | 1 day | Kani proofs: 3 backend 行为一致性 + isolation 不变量 |
| 总计 | 4 days | apeireth-sovereignty::ToolIsolation v1 |

前提: R170+ 不动 3 不可变脊柱。

---

## 6. R170 结论

- Hyperlight 4.4K stars, Rust-native, <1ms cold start, 3 hypervisor 后端 — 完全契合 sovereignty 隔离升级诉求
- 设计上 ToolIsolation trait 作为新隔离层, 与 3 不可变脊柱解耦 — 永不穿透
- 推荐实施路径 A (直接依赖 + 抽象层包装), 4 days 工作量
- R170 仅 research, 0 代码改动, R170+ 阶段开始实施

下一步: R171 (relation SurrealDB backend 调研) — 与 R170 同类 research-only 模式。
