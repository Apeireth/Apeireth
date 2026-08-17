# 阶段 2 决策：核心架构形态 (2026-07-30)

> **范围**: R14 Rust 重写核心架构形态决策 (阶段 2 第二项)
> **触发**: 用户最新指示 "你给我推荐吧" (关于架构形态)
> **依据**: 阶段 1 灵感 §1.3 (多进程 + supervisor + actor + 异构) + Erlang/OTP supervisor 模型 + 调研 (Hermes Agent Rust / OpenClaw Gateway)
> **配套文档**: `stage2-decisions-tech-stack.md` (技术栈) + `inspiration-stage1-2026-07-30.md` (灵感)

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-architecture.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 2/12) |
| **决策** | **B+E = 多进程 supervisor (外) + actor (内) + 异构子进程 (兼容组件)** |
| **依据** | 阶段 1 §1.3 + Erlang/OTP supervisor 模型 + Hermes/OpenClaw 调研 |

---

## 1. 决策总览

```
apeireth-supervisor (主进程, Rust)
  ├── apeireth-core-supervisor (Level 1, rest_for_one)
  │     ├── apeireth-asi (主 AI)
  │     ├── apeireth-memory
  │     └── apeireth-philosophy
  ├── apeireth-council-supervisor (Level 1, one_for_one)
  │     ├── 7 个 persistent 顾问
  │     └── N 个 dynamic/ephemeral 顾问
  ├── apeireth-plugin-supervisor (Level 1, transient)
  │     └── 异构子进程 (PyO3 / WASM / subprocess / HTTP)
  └── apeireth-upgrade-supervisor (Level 1, rest_for_one)
        └── sandbox-validator
```

> **[TODO-P0-05 阶段 3 启动前 待拆分]** — §1 决策总览中 `apeireth-core-supervisor (Level 1, rest_for_one)` 含主 AI+memory+philosophy 强耦合, **中央 AI 人格连续性不能依赖单一 PID**; 待拆分 = 主 AI supervisor 与 memory/philosophy supervisor 解耦, 中间用持久化快照 + D2 §4 主体连续性 ID + D2 §5 6 历史流桥接 (引自 `stage2-decisions-drift-revision-tracker.md` §2.5)。**[TODO-OWNER]** architect + backend_engineer + database_engineer。**[TODO-STAGE]** 阶段 3 (画图纸) 启动前 P0 拆分。**不删原文不动原措辞**, 修订 = 追加新树节点 + 跨引用跟踪表。

---

## 2. 候选对比表

| 范式 | 升级 | 隔离 | 异构 | 复杂度 | 推荐 |
|------|------|------|------|--------|------|
| A. 单进程多线程 | ❌ 难 | ❌ 弱 | ❌ 难 | 低 | ❌ |
| **B. 多进程 supervisor** ⭐ | ✅ 易 | ✅ 强 | ✅ 易 | 中 | ✅ |
| C. 微服务 / 边车 | ✅ 易 | ✅ 强 | ✅ 易 | 高 | ❌ 运维重 |
| D. actor 框架 | ⚠️ 中 | ⚠️ 中 | ⚠️ 难 | 中 | ⚠️ 单用不够 |
| **E. 异构子进程** ⭐ | ✅ 易 | ✅ 强 | ✅ 天生 | 中 | ✅ |
| **B+E** ⭐⭐ | ✅ | ✅ | ✅ | 中 | ✅✅ 推荐 |

---

## 3. 为什么选 B+E

### 不选 A 单进程多线程

- ❌ 升级难（要重启整个进程）— 违反"自我升级"灵感（§6 沙盒 + 洋葱测试矩阵）
- ❌ 隔离弱（一个 panic 影响全部）
- ❌ 异构难（PyO3 嵌入比子进程重）

### 不选 C 微服务

- ❌ 巨型基地哲学反对"为了灵活增加运维成本"
- ❌ 微服务治理（服务发现 / 熔断 / 限流）复杂
- ✅ 但是 inspiration §15 OpenClaw Gateway 是**单长生命周期进程**——已经是"非典型微服务"

### 不选 D actor 单用

- ⚠️ actor 框架（如 actix / ractor）是好抽象，但**没有进程级隔离**
- ⚠️ actor 适合在**进程内**做并发，不适合跨进程
- ✅ **actor 在 B 内用是对的**（同进程内组件用 actor 模式）

### B+E 的优势

- ✅ **进程级隔离** — 子进程挂了只影响自己
- ✅ **升级友好** — 可以热替换子进程（甚至热替换 supervisor 自己）
- ✅ **异构友好** — 子进程可以是 Python/Go/JS/WASM
- ✅ **崩溃恢复** — Erlang/OTP supervisor 树自动重启
- ✅ **巨型基地哲学** — 允许冗余（多个子进程）、过度设计（多层 supervisor）

---

## 4. supervisor 树详细设计 (Erlang/OTP 风格)

```
apeireth-supervisor (Level 0)
  │
  ├── apeireth-core-supervisor (Level 1, strategy=rest_for_one)
  │     │  任何一个挂了, 重启它和之后启动的
  │     ├── apeireth-asi (主 AI = Sovereignty trait 实现)
  │     ├── apeireth-memory (A/M 层经验沉淀)
  │     └── apeireth-philosophy (V3 9 键 + 5 项不假装)
  │
  ├── apeireth-council-supervisor (Level 1, strategy=one_for_one)
  │     │  任何一个挂了, 只重启它
  │     ├── advisor-safety (V1121 风险识别)
  │     ├── advisor-performance (V1130 wallclock)
  │     ├── advisor-philosophy (V3 9 键 + 主哲学 anchor)
  │     ├── advisor-history (前人经验 + 失败案例库)
  │     ├── advisor-strategy (ASI 北极星导向)
  │     ├── advisor-ethics (V1132 SSRF + 隐私)
  │     ├── advisor-legal (默认 off, 可启用)
  │     └── advisor-dynamic × N (动态生成)
  │
  ├── apeireth-plugin-supervisor (Level 1, strategy=transient)
  │     │  异构子进程, 异常退出才重启
  │     ├── python-llm-plugin (PyO3 桥)
  │     ├── wasm-sandbox × N (WASM 沙箱)
  │     ├── subprocess-plugin × N (其他语言)
  │     └── http-mcp × N (外部 MCP 服务)
  │
  └── apeireth-upgrade-supervisor (Level 1, strategy=rest_for_one)
        │  OTA 升级验证
        ├── sandbox-validator (沙盒验证升级意图)
        ├── mirror-builder (构建镜像)
        └── traffic-shifter (切换流量)
```

> **[TODO-P0-05 阶段 3 启动前 待拆分]** — §4 supervisor 树详细设计中 `apeireth-core-supervisor (Level 1, strategy=rest_for_one)` 下含主 AI+memory+philosophy 强耦合 (Erlang/OTP 重启策略), **中央 AI 人格连续性不能依赖单一 PID**; 待拆分 = 主 AI supervisor 与 memory/philosophy supervisor 解耦, 中间用持久化快照 + D2 §4 主体连续性 ID 桥接 (引自 `stage2-decisions-drift-revision-tracker.md` §2.5)。**[TODO-OWNER]** architect + backend_engineer + database_engineer。**[TODO-STAGE]** 阶段 3 启动前 P0 拆分。**不删原文不动原措辞**, 修订 = 追加新树节点 + 跨引用跟踪表。

---

## 5. 进程间通信 (IPC)

| 通信类型 | 工具 | 性能 | 适用 |
|---------|------|------|------|
| 同进程 actor | `tokio::sync::mpsc` | ⚡ ns | 主 AI ↔ 智囊团 |
| 同进程广播 | `tokio::sync::broadcast` | ⚡ ns | 系统事件 |
| 同进程共享状态 | `tokio::sync::RwLock` | ⚡ ns | 配置 / 缓存 |
| 父子进程 | Unix domain socket + bincode | 🚀 μs | supervisor ↔ 子进程 |
| 异构子进程 | pipe + JSON/MessagePack | 🚀 μs | Rust ↔ Python |
| 外部服务 | gRPC + protobuf / HTTP + JSON | 🐢 ms | 远程 API |

**通信层抽象** (`apeireth-bus` crate 候选):
```rust
pub trait Bus: Send + Sync {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError>;
    async fn subscribe(&self, topic: &str) -> Result<Subscription, BusError>;
    async fn request(&self, target: &str, req: Request) -> Result<Response, BusError>;
}

// 多种实现
pub struct InprocBus;          // 同进程 (mpsc/broadcast)
pub struct UnixSocketBus;     // 父子进程 (Unix domain socket + bincode)
pub struct PipeBus;           // 异构 (pipe + JSON)
pub struct GrpcBus;           // 外部 (gRPC + protobuf)
```

---

## 6. Erlang/OTP 重启策略

| 策略 | 含义 | Apeireth 适用 |
|------|------|--------------|
| `one_for_one` | 一个挂了, 只重启它 | 智囊团顾问 (互相独立) |
| `rest_for_one` | 一个挂了, 重启它和之后启动的 | 主 AI + memory + philosophy (强耦合) |
| `one_for_all` | 一个挂了, 重启所有 | plugin supervisor (避免状态不一致) |
| `transient` | 只在异常退出时重启 | plugin 子进程 |
| `permanent` | 任何退出都重启 | 主进程 / supervisor |
| `temporary` | 任何退出都不重启 | 临时任务 |

> **[TODO-P0-05 阶段 3 启动前 待拆分]** — §6 Erlang/OTP 重启策略表中 `rest_for_one` 行 "主 AI + memory + philosophy (强耦合)" 是单一 PID 风险; 待拆分 = 主 AI 与 memory/philosophy 解耦为独立 supervisor, 用 D2 §4 主体连续性 ID + 持久化快照桥接 (引自 `stage2-decisions-drift-revision-tracker.md` §2.5)。**[TODO-OWNER]** architect + backend_engineer + database_engineer。**[TODO-STAGE]** 阶段 3 启动前 P0 拆分。**不删原文不动原措辞**, 修订 = 追加新策略行 + 跨引用跟踪表。

---

## 7. actor 模式 (进程内并发)

虽然选 B+E，但**进程内**用 actor 模式管组件：

```rust
// apeireth-core 用 actor 模式
pub struct CoreActor {
    state: Arc<RwLock<CoreState>>,
    inbox: mpsc::Receiver<CoreMessage>,
}

impl CoreActor {
    pub async fn run(mut self) {
        while let Some(msg) = self.inbox.recv().await {
            self.handle(msg).await;
        }
    }
    
    async fn handle(&mut self, msg: CoreMessage) {
        match msg {
            CoreMessage::Decision(situation) => self.decide(situation).await,
            CoreMessage::CouncilCheck(opinion) => self.check_council(opinion).await,
            // ...
        }
    }
}
```

**actor 在 B 内的优势**:
- ✅ 组件状态隔离
- ✅ 消息驱动, 无锁
- ✅ 容易扩展（加 actor 不影响其他）
- ✅ 容易测试（mock message）

---

## 8. 自我升级 + supervisor 协同

```
OTA 升级流程:
  1. 主 AI 生成 UpgradeIntent (manifest)
  2. 智囊团强制审核 (E-3 守门)
  3. 物理多签 (Layer 5)
  4. apeireth-upgrade-supervisor 启动 sandbox-validator
  5. sandbox 跑完整集成测试 (洋葱测试矩阵)
  6. PASS → traffic-shifter 切换
     FAIL → 自动回滚 + 存档
  
关键设计:
  - supervisor 进程**永不被升级**（E 层不可改）
  - 被升级的是 supervisor 下的子进程
  - 升级采用"双实例 + 流量切换"模式（Erlang/OTP 标准做法）
  - 升级是 LLM 智能决策, 但执行是机械的
```

> **[TODO-P0-01 阶段 3 启动前 待修订]** — supervisor 进程可被提案修改, 修改本身按 §18.6 + D2 §15.2 优先解释权触发**最高权重 MEWG + 多人 + 多 AI + 物理多签 + 反思期**五重治理 (引自 `stage2-decisions-drift-revision-tracker.md` §2.1)。**[TODO-OWNER]** architect + devops_engineer + philosophy_guardian (E 层相关双签)。**[TODO-STAGE]** 阶段 3 (画图纸) 启动前 P0 修订。**不删原文不动原措辞**, 修订 = 追加新行 + 跨引用跟踪表。

---

## 9. 阶段 2 第二项收尾判定

核心架构形态已沉淀：**B+E = 多进程 supervisor + actor + 异构子进程**。

**R14 增量**:
- 新增候选 crate: `apeireth-supervisor` (Erlang/OTP 风格 supervisor 树)
- 新增候选 crate: `apeireth-bus` (统一通信总线抽象)
- 在已存在 crate 内加 actor 模式 (apeireth-core / apeireth-memory 等)
- plugin 接入走 Unix domain socket (PyO3) / pipe (异构) / HTTP (MCP)

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (架构服务 ASI 方向)
- 主 17:43 S-2 (基于真实需求, 不模仿他人)
- 主 17:58 O-5 (不假装, supervisor 严格按规则)
- 主 19:33 O-2 (Erlang/OTP 是经过验证的范式)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者都能跑)

**下一步**: 阶段 2 第三项 — **crate 划分** (基于 §14 候选 30 个 + B+E 架构)

---

## 附录: Rust 升级友好性 (用户问题补充)

**Rust 升级的机制**:
- `rustup` 管理多版本并存
- `rust-toolchain.toml` 锁定当前版本
- `cargo update` 自动适配依赖
- `MSRV` (`Cargo.toml` 的 `rust-version`) 声明最低支持

**升级流程** (走自我升级):
1. `rustup toolchain install <new-version>`
2. 编辑 `rust-toolchain.toml`
3. `cargo build + cargo test` 全量验证
4. `cargo clippy + rustfmt check`
5. PASS → commit；FAIL → 回退

**Apeireth 升级 Rust 版本 = 走 OTA 流程**:
- 沙盒里编译 + 跑集成测试
- 通过后切换流量
- 主 AI 自己做，不需要外部介入

**MSRV 策略**:
- 仓库 `rust-version = "1.80"` 是最低支持
- 开发可用 1.85+ / nightly
- 发布产物用 1.80（兼容性最广）
- 巨型基地哲学：**"用最新，写最低"**

---

_主哲学 anchor 6 个全贯穿: 主 22:33 (S-1) + 主 17:43 (S-2) + 主 17:58 (O-5) + 主 19:33 (O-2) + 主 23:44 (O-3) + 主 00:56 (O-4)._
_核心架构形态已沉淀. 下一步等用户确认进入阶段 2 第三项 (crate 划分)._