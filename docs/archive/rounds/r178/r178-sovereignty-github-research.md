# R178 GitHub 优秀项目调研 — sovereignty 模块 (sandbox/micro-VM/形式化/能力系统)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R178
> **日期**: 2026-08-13
> **范围**: apeireth-sovereignty 子系统: ToolIsolation / Self-Disable / 形式化验证 / 能力系统
> **状态**: 调研为升级预备. 3 不可变脊柱 (Self-Disable 判定 / L0 HA 物理多签 / 13 键 verdict cache 语义) 0 触碰.
> **接续**: R170 Hyperlight 调研是本档的前置.

---

## 0. 上下文

apeireth-sovereignty 32 个 src 文件, 8KB+ 真实代码 / 文件. 核心职责:
- Self-Disable 判定 (3 不可变脊柱之一)
- L0 HA 物理多签 (3 不可变脊柱之一)
- 13 键 verdict cache (3 不可变脊柱之一)
- Tool isolation (3 后端: Process/Container/WASM, 大部分 stub)
- 形式化 (apeireth-formal Kani proofs)
- 7 重守门 (seven_fold_guard.rs)

本档调研 sovereignty 之上的 **可升级面** (不碰 3 脊柱).

---

## 1. Micro-VM / Sandbox 层

### 1.1 microsandbox (zerocore-ai/microsandbox) — **强 RECOMMENDED**

- **GitHub**: https://github.com/zerocore-ai/microsandbox
- **License**: Apache 2.0
- **核心定位**: \"Secure, lightweight, fast booting sandbox for AI agents\" — **就是为我们这种场景设计的**
- **能力**:
  - KVM-based micro-VM (Linux)
  - < 200ms cold start
  - 完整的 agent sandbox API: lifecycle / fs / net / process control
  - OCI image 兼容
  - Rust SDK + Python SDK
  - 既能 sandbox AI agent 也能 sandbox untrusted code
- **关键设计点**:
  - 每个 tool call 独立 VM
  - VM ID 可追溯 + audit log
  - 支持 exec / file / network 细粒度策略
  - 内置 git clone / npm install / pip install 等开发环境 API

**为什么强推荐**:
- 名称 / 定位 / API 设计 100% 对应我们的 peireth-sovereignty::ToolIsolation 抽象 (R170 设计)
- 比 Hyperlight 更成熟 (有完整 SDK), 比 Firecracker 更轻量 (single binary)
- License 友好 (Apache 2.0)
- 文档/示例完整

**集成方案 (草案)**:
`
ust
// apeireth-sovereignty/src/tool_isolation/microsandbox.rs
use microsandbox_rs::Sandbox;

pub struct MicrosandboxIsolation {
    pool: Vec<Sandbox>,
    policy: IsolationPolicy,
}

#[async_trait]
impl ToolIsolation for MicrosandboxIsolation {
    async fn prepare(&self, tool_id: &ToolId, policy: &IsolationPolicy)
        -> Result<IsolationHandle, IsolationError> {
        let sb = Sandbox::builder()
            .image(policy.image())
            .memory(policy.memory_mb())
            .cpu(policy.cpu_shares())
            .build().await?;
        Ok(IsolationHandle::new(sb.id()))
    }
    // ...
}
`

**优先级**: 比 R170 调研的 Hyperlight 优先 — 因为:
1. Hyperlight 还在快速演进, 文档不全
2. microsandbox 已经有成熟 SDK
3. 我们的 ToolIsolation 抽象两层都适用, 先用 microsandbox 落地

### 1.2 Firecracker (firecracker-microvm/firecracker) — **不直接集成, 但学习标杆**

- **Stars**: 27K+
- **主语言**: Rust
- **License**: Apache 2.0
- **AWS Lambda 后端**: production 验证
- **优势**: 工业级 microVM, 极简设备模型
- **劣势**: 启动 125ms+ (比 Hyperlight 慢), 需要独立进程

**为什么不上手**:
- 我们不需要跑 1000+ 并发 VM
- 125ms cold start 对单 tool call 偏慢
- **但**: 它的 audit log 设计和 VM exit code 标准化值得我们学习

### 1.3 gVisor (google/gvisor) — **不选**

- Go 写的 user-space kernel
- 启动慢, 资源重
- 我们用 microVM 路线不需要 user-space kernel

### 1.4 bubblewrap / Landlock / nsjail — **学习材料**

- bubblewrap: flatpak 用的轻量沙箱
- Landlock: Linux 内核 LSM (5.13+)
- nsjail: Google 的 process sandbox
- 价值: 这些项目演示了 \"进程级 + 内核级\" 隔离的极限. 我们 ProcessBackend fallback 可以借鉴 Landlock 实现.

---

## 2. 形式化验证 (Formal Verification)

### 2.1 Kani (model-checking-ai/kani) — **当前在用, 持续跟进**

- **GitHub**: https://github.com/model-checking-ai/kani
- **License**: Apache 2.0
- **现状**: apeireth-formal 已经用 Kani 写 proofs
- **缺失**: 3 关键 proof 待补 (per R175 盘点)
  - Self-Disable 判定路径不可能越权
  - L0 HA 物理多签最少签名数约束
  - 13 键 verdict cache 不变量
- **下一步**: 跟 AWS 团队学, 看他们如何用 Kani 验证 s2n-quic

### 2.2 Creusot (creusot-rs/creusot) — **学习**

- Rust 符号验证器, 基于 Why3
- 适合验证 trait invariant
- 比 Kani 更适合 \"trait 必须实现什么\" 的证明

### 2.3 Prusti (viperproject/prusti) — **学习**

- Viper-based Rust verifier
- 适合验证函数前置/后置条件
- 与 Kani 互补

### 2.4 coq-of-rust — **观望**

- Rust -> Coq 翻译, 学术级
- 学习价值高, 实际部署难

---

## 3. 能力系统 (Capability-based Security)

### 3.1 Cloudflare Workers / Wasmtime capability — **学习**

- 资源/能力必须显式传递
- 默认无能力 (default deny)
- 我们的 Permission Onion L0-L5 类似, 但更细化

### 3.2 Capsicum (FreeBSD) — **学习材料**

- 操作系统级 capability
- 太底层, 但设计原则值得借鉴

### 3.3 wasmtime (bytecodealliance/wasmtime) — **已在用**

- 16K+ stars
- capability-based resource limiting
- host functions 显式定义

---

## 4. Self-Disable / 不可逆熔断 (3 不可变脊柱)

这一层没有开源对标, 因为 Self-Disable 是 Apeireth 独创设计. 调研目的是找 **类似的 agent 失控 kill switch 实践**.

### 4.1 OpenAI Evals / Anthropic Constitutional AI

- 不开源, 不可比
- 但他们公开承认: \"self-restraint is unreliable at AGI scale\"
- 这正是我们 Self-Disable 形式化验证的意义所在

### 4.2 NIST AI RMF (风险管理框架)

- 不是项目, 是政府标准
- 我们 R175+ 路线可以参考 NIST AI RMF 的 \"Map/Measure/Manage/Govern\" 框架
- 但不引入新依赖, 仅在文档中引用

### 4.3 现有的 kill switch 实践

- Kubernetes liveness probes: 简单但不可信
- AWS IAM permission boundary: 静态, 不能动态回收
- Linux cgroup freezer: 进程级, 但 agent 可 fork
- **Apeireth Self-Disable 的独特性**: 形式化 + 物理多签 + 24h 反思期 + 三级响应

---

## 5. 升级优先级

| 优先级 | 项 | 项目 | 预计工作量 | ROI |
|---|---|---|---|---|
| 🥇 | ToolIsolation 升级到 microsandbox | zerocore-ai/microsandbox | 3 days | 极高 |
| 🥈 | Kani proofs 补完 3 缺失 | (已在用) | 2 days | 高 |
| 🥉 | ProcessBackend fallback 加 Landlock | Linux LSM | 1 day | 中 |
| 4 | Creusot / Prusti 评估 | (观望) | 1 day | 低 |
| 5 | Self-Disable 横向对标文档 | 无 | 0.5 day | 低 (宣传价值) |

---

## 6. 0 触碰声明

- 3 不可变脊柱: 0 触碰 (本调研仅升级 ToolIsolation / Kani proofs / ProcessBackend, 不动 Self-Disable / L0 HA / verdict cache)
- workspace.version 1.2.0: 0 改
- V0.5 / V1136 / 9键: 0 改

---

## 7. 参考链接

- microsandbox: https://github.com/zerocore-ai/microsandbox
- Firecracker: https://github.com/firecracker-microvm/firecracker
- gVisor: https://github.com/google/gvisor
- bubblewrap: https://github.com/containers/bubblewrap
- Landlock: https://docs.kernel.org/userspace-api/landlock.html
- nsjail: https://github.com/google/nsjail
- Kani: https://github.com/model-checking-ai/kani
- Creusot: https://github.com/creusot-rs/creusot
- Prusti: https://github.com/viperproject/prusti
- wasmtime: https://github.com/bytecodealliance/wasmtime
- Hyperlight (R170): https://github.com/hyperlight-dev/hyperlight
- NIST AI RMF: https://www.nist.gov/itl/ai-risk-management-framework
- AWS s2n-quic (Kani 实战案例): https://github.com/aws/s2n-quic