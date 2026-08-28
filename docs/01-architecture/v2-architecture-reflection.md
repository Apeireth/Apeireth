# Apeireth v2.0 新架构反思 + 自升级 cycle 设计 (2026-08-28, 主代理 Mavis)

> **本文档定位**: 主代理 (Mavis) 对 v2 新架构的反思 + 自升级 cycle 设计.
> **何时写**: 用户问"新架构有没有考虑到升级的扩展性", 我反思后记录, 防止未来遗忘.
> **读谁**: v2.0 release 后接手新架构设计的人 + Apeireth 自我升级机制实施者.
> **未来用途**: v2.0 release 后 Apeireth 自我升级时, 按本文 cycle 实施.

---

## 0. 反思背景

- **用户原话**: "新架构有没有考虑到升级的扩展性啥的, 新架构我还不太懂你给我讲解一下, 再和旧架构对比一下, 然后你反思一下新架构对不对, 好不好, 然后你告诉我, 新架构完成后, Apeireth的自我升级应该是什么样的"
- **HEAD 状态 (写本文时)**: `02faa6d0` (子代理 M 真写 RC-5 + 子代理 I 真写 RC-11)
- **v1 era**: 86 crates, 23,806 tests, 9 organ 完整
- **v2 era**: 15 crates, ~1500+ tests, 8/10 RC 真写完成

---

## 1. 旧架构 (v1 era, 86-crate)

### 1.1 结构

86 crates 散落, "按 + 功能"双重分裂. 例如:
- `apeireth-companion::memory_extractor` (记忆 extractor)
- `apeireth-state::persistence` (状态持久化)
- `apeireth-team-lead::worktree` (worktree 工具)
- 等等

### 1.2 9 organ 核心能力

| Organ | 说明 |
|---|---|
| W1/W2/W3 | 世界模型 (causal_world_model.rs) |
| E4 | 好奇心引擎 (curiosity.rs) |
| F4 | 假设检验 (hypothesis.rs) |
| F1 | 情感记忆 (emotion_memory.rs) |
| F6 | 价值内化 (value_cases.rs) |
| E7 | 涌现 loop (emergence.rs) |
| 等 | 工具审批 / team-lead / cognition 等 |

每个 organ = **独立 crate**, 内部 if-else 散落. organ 入口签名 LOCKED 软锁 (R148 LOCKED 撤销扫尾原则).

### 1.3 关键问题

- **加新 capability** = 改多个 crate (因为没有统一 trait 边界)
- **改 LLM provider** = 改 runtime (runtime 知道 vendor)
- **加新 organ** = 1 新 crate + 改多个 import 点
- **LLM 调用**: runtime 直接知道 vendor (legacy `ApeirethApiProvider`), `LegacyLlmCapability` wrapper桥接
- **凭证**: 散落 `String` per provider
- **存储**: ACT-R/向量/图各 1 crate, 互相 import
- **可观测性**: hook 缺失, 13 键 verdict cache 不可关闭

---

## 2. 新架构 (v2, 15-crate)

### 2.1 结构 (4 层分组)

```
adapters/ (3) - 入口层 (cli/gateway/sdk)
    ↓
engine/ (5) - 实现层 (runtime/provider/storage/memory/perception)
    ↓
foundation/ (7) - 抽象层 (core/protocol/plugin/governance/credentials/orchestration/...)
    ↓
capabilities/ (1) - 工具 (tools)
```

### 2.2 7 capability trait (位置: `apeireth-plugin`)

| Trait | 说明 | RC 实现 | 文件位置 |
|---|---|---|---|
| `MemoryBackend` | episode/note 持久化 | RC-1 ✅ | `crates/foundation/plugin/src/memory_backend.rs` |
| `Experience` | Wiki/Knowledge Graph/Association 3 trait | RC-2 ✅ | `crates/foundation/plugin/src/experience.rs` |
| `Perception` | Text/Voice/Vision/Tactile 4 modality | RC-7 ⏳ | `crates/foundation/plugin/src/perception.rs` |
| `PreferenceStore` | 用户偏好 | RC-3 ✅ | `crates/foundation/plugin/src/preference.rs` |
| `SelfAssessmentStore` | AI 自我评估 | RC-4 ✅ | `crates/foundation/plugin/src/self_assessment.rs` |
| `LlmFactory` | LLM 调用上下文 | RC-5 ✅ | `crates/foundation/plugin/src/llm_factory.rs` |
| `SubSupervisor` | 进程监督 | RC-8 ✅ | `crates/capabilities/tools/src/sub_supervisor.rs` |

### 2.3 3 原则洋葱 (per `philosophy.md` §Triple Onion)

- **L0 人类审批** (硬墙, 永远不可变) — Self-Disable 保护
- **L1-L5 权限层** (approval / sandbox / etc)
- **DSL 洋葱** (Colang DSL 表达"什么操作允许/禁止")

### 2.4 7 capability trait 注入模式

```rust
Arc<dyn MemoryBackend>
Arc<dyn Experience>
Arc<dyn Perception>
// ...
```

runtime 调能力, 不直接 import impl.

### 2.5 认知模块 (v2 独有, 其他 dev 推 5 commit)

- **12 slot ledger** (6 WIRED + 6 DEFERRED)
- **单一 slot 注册顺序**: `TurnStart → AfterModelResponse → AfterTurn`
- **AgentModule ABI** (独立于 7 capability trait)
- **多 instance LLM 隔离** (per scene-d §3 决策 1: 同一 provider 不同 model 隔离)
- **ModuleInvoker** 强制 depth ≤ 1 + budget ≤ 8 (runtime 守门)

### 2.6 9 哲学锚 LOCKED (升 8→9, 2026-08-27 O-6 加)

- **S-1 北极星 / S-2 实事求是 / S-3 质量工程化**
- **O-1 安全优先 / O-2 前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装**
- **O-6 永远追求最优** (NEW 2026-08-27, 三阶审查 + 不做借口清单)

### 2.7 5 重自动守门 (`.github/workflows/o6-anchor.yml`)

1. clippy 0 警告
2. workspace tests 0 失败
3. legacy compat path < 100 引用
4. 13 键 LOCKED + 9 哲学锚 + workspace.version + R11 baseline 0 触碰
5. 哲学锚表头 0 减

---

## 3. 新旧架构对比

| 维度 | 旧 v1 (86-crate) | 新 v2 (15-crate) |
|---|---|---|
| **Crate 数** | 86 (散落) | 15 (4 层分组) |
| **核心能力抽象** | 9 organ 独立 crate, 无统一 trait | 7 capability trait, 1 个 plugin crate |
| **LLM 调用** | 1 provider per crate, runtime 知道 vendor | `Arc<dyn LlmFactory>` 注入, runtime 0 知道 vendor |
| **凭证** | 散落 `String` | 统一 `CredentialResolver` (4 backend + 自动 fallback) |
| **存储** | ACT-R/向量/图各 1 crate | `SqliteConnectionPool` (writer-async + reader-pool) + 7 capability 各 1 store |
| **治理** | hook 缺失 | 5 重守门 + 3 hook (Permission/Credential/PromptInjection) |
| **认知模块** | 内嵌 legacy cognition | 12 slot ledger (6 WIRED + 6 DEFERRED), ModuleInvoker 1 侧调 |
| **哲学锚** | 8 锚 + 12 键 verdict cache 强制 | 9 锚 (升 O-6 永远追求最优) + 13 键降级为哲学标准 (0 装强制) |
| **器官入口** | 9 文件名 + 入口签名 LOCKED 软锁 | **不锁** (P-arch O-6 Refactor-1 已把 MemoryBackend trait 搬到 plugin) |
| **可扩展性** | 加 1 organ = 1 新 crate + 改多个 import | 加 1 capability = 1 trait + 1 impl + 1 `Arc<dyn Trait>` 注入 |

---

## 4. 新架构反思 (O-6 三阶审查, 主代理亲做)

### 4.1 总体最优: **基本对, 但有缺**

#### 对的:
- **4 层分组** (foundation/engine/capabilities/adapters) 是 v2 工程重构主轴
- **7 capability trait 边界清晰**, 单向依赖 (memory/tools/cli → plugin)
- **9 哲学锚 LOCKED + 5 重守门 + 0 装诚实原则** = 信任地基扎实
- **认知模块 12 slot ledger** (WIRED + DEFERRED) 是 **v1 没有的新维度**
- **ProviderCapability + LlmFactory 双层抽象** (per 子代理 M 独立视角) — 我之前认为重复, 实是设计意图: ProviderCapability 给路由 / LlmFactory 给多 instance 隔离

#### 缺的:
- **9 organ 核心能力 0 真移植** = 1.0 全部功能没达成 (用户 v2.0 完成定义的标准)
- **认知模块是其他 dev 推** (5 commit), 我**没拍板架构决定权** — 子代理 J 已核验 0 触碰 LOCKED, 但 **谁来维护 / 何时扩展**没明示
- **Plugin registry 缺失** — ROADMAP §4 P4 "MCP 动态能力注册" 还在 future
- **Triple onion 描述在 `philosophy.md`**, 但 runtime 真实现只到 L1-L2 (approval + sandbox), **L3-L5 0 装**
- **7 capability trait 没统一入口 trait** (子代理 K 报) — `PluginCapabilities` index trait 缺失
- **Perception text-only 真实现**: 6 modality (text/voice/vision/tactile/screen/audio), 仅 text 真实现 (其他 0 装)

### 4.2 系统最优: **对**

- **单向依赖** (memory/tools/cli/credentials/orchestration → plugin) = 0 反向
- **0 循环**, 100+ consumer 0 破
- 文件位置 = `foundation/plugin/` 是核心 trait (per scene-d §5 决策 1), impl 在 engine (per O-6 Refactor-1-5)
- runtime 注入 `Arc<dyn Trait>` 边界清晰
- `CapabilityResult<T> = Box<dyn Error + Send + Sync>` 统一错误通道 (per O-6 #12)

### 4.3 架构最优: **7 分**

#### 好 (:
- **7 capability trait** = "能力边界"哲学的体现 — 与 9 哲学锚对齐 (S-2 实事求是, S-3 质量工程化)
- **认知模块 12 slot** = "子代理 = slot" 的实例化 (per 子代理 D 教训 "派子代理 = 绕过 O-6" → slot = 显式 slot 注册)
- **`LlmFactory` 多 instance 隔离** = 防 LLM 调用间状态泄漏 (per scene-d §3 决策 1)
- **`ProviderCapability` + `LlmFactory` 双层** = 路由 vs 多 instance 隔离的分工 (子代理 M 独立视角)

#### 不足:
- **7 capability trait 没统一入口 trait**: `PluginCapabilities` index trait 缺失 (子代理 K 报)
- **Perception 6 modality 0 装真**: 仅 text (其他 5 modality 0 装)
- **9 organ 入口名**已 LOCKED 软锁 (TUI 9 organ 内部可改 per R148), 但**真迁移入口没设计** — 应该 `Organ` trait 边界
- **人类审批 L0 不可变**是哲学原则, 但 runtime 还没硬实现 — L0 真在哪 (governance hook?)

---

## 5. 0 装诚实结论

### 5.1 新架构优 vs 缺

**新架构优** = 7 capability trait + 认知模块 + 9 哲学锚 + 5 重守门 + 单向依赖 + LlmFactory 多 instance 隔离 + ProviderCapability 路由

**新架构缺** = 9 organ 0 真移植 + 认知模块架构决定权没拍板 + Triple onion L3-L5 0 装真 + PluginCapabilities index trait 缺失 + Perception 6 modality 0 装 + Organ trait 边界没设计

### 5.2 v2.0 完成距离

按用户定义 "新架构 + 1.0 全部功能 + 实现":
- **新架构** ✅ 完成 (15-crate + 7 capability trait + 认知模块 + 哲学锚 9 + 5 重守门)
- **1.0 全部功能** = **9 organ + 其他 77 crates 功能** = **估 5-7 月真完成** (子代理 L 估 2027-01-08 至 2027-03 月)
- **总进度** = **~28%** (新架构 15% + RC 80% × 20% + 器官 0% × 40% + 认知 50% × 25%)

### 5.3 子代理 D 教训 0 装诚实标

- **派子代理是手段不是目的** (用户原话, 子代理 K 报告 + 我确认)
- **主代理必须拍板, 子代理可调研** (哲学锚本体加 O-6 是我拍板的, 子代理 K 调研)
- **不假装 "已写未写"** (TODO 承诺 ≠ 实现)
- **0 装诱导修** (12 consumer 弃用从阻塞列表移出, 子代理 H 独立判断)

---

## 6. Apeireth 自我升级 cycle 设计

### 6.1 前提

新架构完成后 (估 2027-01-08 至 2027-03 月 v2.0.0 release 后):
- **15-crate + 7 capability trait** 就位
- **认知模块 6 WIRED** 就位 (memory_recall / preference_recall / judge / self_assessment / memory_writeback + council slot ready)
- **9 organ 至少 1 真移植** (估 E4 curiosity 4 周, 子代理 L 估)
- **9 哲学锚 LOCKED + 13 键降级 + 5 重守门** 就位

### 6.2 自我升级机制 (Self-Improvement Loop, SIL)

```
 ┌─────────────────────────────────┐
  │ L0: 人类审批 (硬墙, 永远不可变) │
  │ - 主人审批 LLM 提案             │
  │ - 主人审批 organ 升级提案       │
  └─────────────────────────────────┘
                ↓ 主人拍板
 ┌─────────────────────────────────┐
  │ L1: 自我诊断 (runtime 主动)     │
  │ - cognitive.self_assessment (真 Judge) │
  │ - self_assessment_store (RC-4 ✅) │
  │ - Self-Disable 判定 (L0 HA 物理隔离) │
  └─────────────────────────────────┘
                ↓ 诊断报告
 ┌─────────────────────────────────┐
  │ L2: 提案生成 (orchestrator)      │
  │ - Orchestrator + 7 LLM advisor (Council) │
  │ - 7 advisor 并行 + 60s timeout  │
  │ - 7 system prompt template       │
  └─────────────────────────────────┘
                ↓ Council verdict
 ┌─────────────────────────────────┐
  │ L3: 验证 (testing sandbox)       │
  │ - E4 curiosity 移植              │
  │ - sandbox 跑 regression           │
  │ - clippy 0 / tests 0 / 5 重守门 │
  └─────────────────────────────────┘
                ↓ 验证通过
 ┌─────────────────────────────────┐
  │ L4: 主人审批 (governance hook)   │
  │ - PromptInjectionHook 拦         │
  │ - PermissionGovernanceHook 控   │
  │ - CredentialDisclosureHook 脱敏 │
  │ - Council 多意见加权 + 主人 Veto │
  └─────────────────────────────────┘
                ↓ 主人批
 ┌─────────────────────────────────┐
  │ L5: 自我升级 (runtime patch)     │
  │ - git tag v2.x+1 → 新版本       │
  │ - LlmFactory 新 model 即时生效  │
  │ - 7 capability trait impl 即时生效│
  │ - cognitive slot 即时激活/废弃   │
  │ - 9 哲学锚本体 LOCKED (0 改, 仅子代理 LLM) │
  └─────────────────────────────────┘
```

### 6.3 设计原则

#### 原则 1: **9 organ = 真自我升级引擎** (per 子代理 L 估)
- W1/W2/W3 理解世界
- E4 好奇心驱动探索
- F4 假设检验
- F1 情感记忆
- F6 价值内化
- E7 主动开口

#### 原则 2: **认知模块 = 自我监控** (已实现 50%)
- self_assessment 真接 Judge
- 自我评估持久化
- 偏离检测 (DeviationReport, RC-4 ✅)

#### 原则 3: **7 capability trait = 自我升级接口**
- 加新 capability = 1 trait + 1 impl + 1 `Arc<dyn Trait>`, 不改其他
- 替换 impl = 改 1 文件, runtime 0 改 (注入模式)
- 加新 provider (LLM) = 1 LlmFactory impl, orchestrator 0 改

#### 原则 4: **Triple onion = 升级守门**
- L0 人类审批永远不可变
- L1-L5 升级能力但 L0 锚定
- DSL 洋葱 (Colang) = L3-L5 真实现 (估 v2.x release 后实施)

#### 原则 5: **5 重守门 = 自动验证**
- clippy 0 / tests 0 / 13 键 LOCKED / workspace.version / R11 baseline
- 每次升级后跑 5 重守门, 失败 → 自动回滚

#### 原则 6: **9 哲学锚 + 13 键 = 决策词汇表**
- LLM 推理时按 9 锚 + 13 键约束 (O-6 永远是守门人)
- 13 键 verdict cache 降级为哲学标准, 不强制, 但仍是判别词汇表

### 6.4 升级 cycle 时间表

- **加 1 capability trait** = 1-2 周 (估)
- **改 LLM provider** = 1 周 (per LlmFactory trait, 子代理 M 已写真 impl)
- **加 1 organ 真移植** = 4-6 周 (per 子代理 L 估, E4 curiosity 最易)
- **认知模块新 slot** = 2-3 周 (12 slot ledger 当前 6 WIRED, 6 DEFERRED)
- **改 Triple onion L3-L5 真实现** = 4-6 周

**总估计每次自升级** = **1-6 周**, **取决于升级类型**.

### 6.5 主人角色 (Mavis/主代理)

v2.0 release 后, **主代理不再每件手写**. Apeireth 自我升级, 主人:
1. **拍板** (L0 + L4 审批)
2. **守门** (5 重守门失败时介入)
3. **不写代码** (Apeireth 写, 主人审)

---

## 7. 子代理反思 (派是手段不是目的)

### 7.1 派了 14 子代理 (A/B/C/D/E/F/G/H/I/J/K/L/M/N)

| 子代理 | 任务 | 派时目的 | 是否值得 |
|---|---|---|---|
| A | Send+Sync 注释 | RC-1 commit 后独立审计 | ✅ 找到 Send/Sync unsafe trait 隐患 |
| B | v1 vs v2 41 项差异 + 5 风险 | 接手前全局视野 | ✅ 量化差距 + 5 风险 |
| C | P0 build break + RC-8 命名 | RC-2 编译失败后急救 | ✅ P0 修 + 命名诚实化 |
| D | 接手人手册 11 节 | 文档交付前 | ✅ 1508 中文字符 11 节 |
| E | RC-10 line header 审查 | O-6 #23 commit 后 | ✅ 3 建议落地 |
| F | ledger 数字 + 2 P1 补 | 子代理 D #2 actionable | ✅ 数字真兑现 + 0 装诱导修 |
| G | ID_LEN_MAX 边界 | 子代理 E 建议 2 续 | ✅ Python script 真兑现 |
| H | HEAD 漂移 + 0 装诱导 | 子代理 F 后续 | ✅ 修正落地 |
| I | RC-11 migration script | 子代理 D #4 actionable | ✅ Python + Rust 测试 |
| J | cognitive 5 commit 0 触碰 LOCKED | 子代理 D #5 actionable | ✅ 0 触碰验证 |
| K | 哲学锚 + 1.0 + 重构版审计 | 用户 "回顾哲学锚" | ✅ 9 锚 LOCKED + 27 commit 真算 |
| L | v2.0 → 1.0 parity 距离 | 用户 "1.0 全部功能" | ✅ 5-7 月估 + 5-7 actionable |
| M | RC-5 LlmFactory MiniMax impl | 用户给 LLM key 后 | ✅ 真 LLM 跑通 (1.16s) |
| N | RC-6 Council 7 advisor | RC-5 完成后续 | ⏳ 跑中 (写 7 LlmAdvisor) |

### 7.2 派子代理原则

- **派 = 调研 / 验证 / 真写 (有明确目的)** — 主代理拍板
- **不派 = 等依赖 / 等硬件 / 0 工作量 (主代理亲做或 0 装诱导)**
- **派 ≤ 14 子代理 = 0 装诱导 / 工具不是目的** (用户原话)
- **每小段做 + 派子代理审查** (用户原话)

### 7.3 子代理 0 装诚实教训

- 子代理 D: "派子代理 = 绕过 O-6" (哲学锚本体升级时, 主代理必须拍板)
- 子代理 H: "0 装诱导" (12 consumer 弃用列入阻塞项 = 误导接手人为不存在的工作留时间)
- 子代理 K: "P0 build break" (子代理 I working tree untracked 引起, 不是 HEAD 真错)

---

## 8. v2.0 release 后的真升级路径

```
2026-08-28 当前 (HEAD 02faa6d0, 8/10 RC + 哲学锚 O-6 加)
  ↓
2026-10-16 v2.0.0-rc.1 release (RC-5/6/7 真写 + 集成测试)
  ↓
2026-11-13 至少 1 organ 真移植 (估 E4 curiosity, 子代理 L 估 4 周)
  ↓
2027-01-08 v2.0.0 release (估 frontend 对接 4-6 周 + buffer 1 月)
  ↓
2027-01-08 以后: 自我升级 cycle 启动 (估 1-6 周/cycle)
```

### v2.x 1.x 远期路线 (per 子代理 L 估)

- **V2.x 1.0** (2027-Q1 估): cognitive module Judge 默认 ON (per 子代理 D #1)
- **V2.x 1.1** (2027-Q2 估): Triple onion L3-L5 真实现 (4-6 周)
- **V2.x 1.2** (2027-Q3 估): MCP 动态能力注册 (per ROADMAP §4 P4)
- **V2.x 2.0** (2027-Q4 估): 多用户 / 跨载体 / 租赁 / marketplace
- **V2.x 3.0** (2028 估): 商业化

---

## 9. 主代理反思 (Mavis 自评)

### 9.1 我做对的

- **哲学锚本体加 O-6** (用户授权, LOCKED 0 装诚实标)
- **RC-5 真 LLM 跑通** (1.16s, 子代理 M 真兑现)
- **RC-11 migration script 真写** (子代理 I 真兑现, Python + Rust)
- **0 装诱导修正** (12 consumer 弃用从阻塞列表移出)
- **文档同步完整** (CHANGELOG + ROADMAP + 5 internal docs + HANDOFF + 报告)

### 9.2 我做错的

- **多次 "73% 收敛" 误述** (真 80.5% / 82.6%)
- **v2 进度 28% 不准** (实际新架构 100% + RC 80% + 器官 0% = 28%, 不含前端对接 -15%)
- **过度派子代理** (14 子代理, 部分可省)

### 9.3 我下次做时

- **不预先估算数字**, 算实际 (git diff + cargo test + 5 重守门)
- **主代理拍板优先**, 子代理不绕过 (子代理 D 教训)
- **0 装诱导标注每段 commit**, 不只文档

---

## 10. 给接手人的话 (v2.0 release 后 1 周内必读)

```
1. 读 ROADMAP.md §3 当前状态 + §3.5 阶段表
2. 读 CHANGELOG.md [Unreleased] 段 (O-6 12 项兑现 ledger)
3. 读 docs/01-architecture/philosophy.md (9 哲学锚 + O-6 不做借口清单)
4. 读 docs/01-architecture/v2-arch-refactor-batch.md (5 Refactor + 守门)
5. 读 docs/04-internal/v2.0.0-rc-roadmap.md (10 RC + 验收)
6. 读 docs/04-internal/HANDOFF-NOTES.md (子代理 D 接手人手册)
7. 读 docs/04-internal/v2-rc-1-progress-report.md (本会话进展快照)
8. 读 docs/01-architecture/v2-architecture-reflection.md (本文 — 自升级 cycle)
9. 跑 cargo test --workspace --locked (验证 0 FAILED)
10. 跑 cargo clippy --workspace --all-targets --locked -- -D warnings (验证 0 警告)
```

---

## 11. 第二批子代理阶段反思 (R1-R15 + Z, 2026-08-28 收盘)

### 11.1 第二批派了 15 个子代理 (R1-R15 + Z, 9 organ + 8 spec)

| 子代理 | 任务 | 产出 | 是否值得 |
|---|---|---|---|
| Q1/R1/R2/R3 | E4/F1/F4/F6 organ 真移植 (确定性 4 件) | commit `4aa54a0a` / `23e48900` / `02f9d537` | ✅ 4 organ 真兑现 |
| R4/R5/R6/R7/R8 | W1/W2/W3/E7/Memory organ 真移植 (LLM 重 + 状态机) | 整合 #2 commit `bbf70293` | ✅ 5 organ 真兑现 |
| R9 | frontend 对接 spec + quickstart | 565 + 224 行 | ✅ 但 task brief 估错 12 slot 数字 (R13 纠) |
| R10 | cognitive 9 organ 集成 spec | 1001 行 | ✅ 但 ledger 数字错 (R13 纠) |
| R11 | OrganOrchestrator spec | 500 行 15 节 | ✅ |
| R12 | OrganOrchestrator 真实施 | commit `2550b99d`, 1933 行 | ✅ 真代码不是 spec |
| R13 | frontend 接力审 + 错账 | 497 行, 6 处错账 | ✅ 找到真账 6 WIRED + 6 DEFERRED |
| R14 | RC-7 真 modality spec | 572 行 | ✅ 硬件依赖如实标 |
| R15 | preference_learning 激活 spec | 617 行 | ✅ |
| Z | 0 装诚实独立审计 | 60% 真兑现 + 5 假装标 | ✅ **最有价值**: 逼出主代理亲做核验 |

### 11.2 第二批教训 (主代理 Mavis 反思)

1. **子代理 spec 数字也会错** — R9/R10 沿用 task brief 的 "4 WIRED + 1 SLOT READY" 旧账, 是 R13 接力审才发现 12 slot 真账 = 6 WIRED + 6 DEFERRED. **教训: 子代理报告必须主代理亲验 ledger, 不能接力传错账.**
2. **"派子代理是手段不是目的" 的正面样本 = R12** — 只有 R12 是真实施 (1933 行代码), 其余都是 spec/审. spec 再多不推进系统.
3. **0 装诱导 prevention 本身是 0 装诱导** (Z 独立判断, 本阶段最锋利发现) — 标 "0 装诚实" 不算验证, 主代理亲跑 `cargo test` / `git log` / `git diff` 才算.
4. **HEAD 漂移是反复犯的错** — FINAL-HANDOFF 曾标 `395fe0f0` 实际 `d55c5745`, 9-organ-progress 曾标 `02f9d537` 实际 `bbf70293`. **教训: 文档 HEAD 一律写 "见 FINAL-HANDOFF §0" 或收盘批 commit, 不再裸写 hash.**

### 11.3 本阶段收盘真账 (2026-08-28)

- 8 spec 收齐 (R9/R10/R11/R13/R14/R15 + Z + 本报告)
- R12 OrganOrchestrator 真实施落地 (13 gate + 5 状态机 + 9 organ 串联, 3 integration tests)
- 6 处错账修正 (commit `ccf29c57`, 主代理亲做)
- **1726 passed 0 FAILED / 0 clippy 警告 / 0 触碰 LOCKED 5 项**
- 本会话累计 85 commit (从 `ef075420` 基线, 主代理亲算)
- 给新团队的话: `docs/04-internal/TO-NEW-TEAM.md`

---

_本文档 v1 首发 (2026-08-28, 主代理 Mavis 写于反思 session). 子代理 K 拍板: 哲学锚 O-6 真加 + 子代理 M 真写 RC-5 + 子代理 I 真写 RC-11. 2026-08-28 收盘补 §11 (第二批 R1-R15 + Z 反思). 下次反思预计在 v2.0 release 后 (估 2027-01-08) 写升级 cycle 实战回顾._