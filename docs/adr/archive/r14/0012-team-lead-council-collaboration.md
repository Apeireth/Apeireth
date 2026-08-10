# ADR-0012: apeireth-team-lead 跟 apeireth-council 协同规则

```
[Document-Meta]
Document: docs/adr/0012-team-lead-council-collaboration.md
Version: Manual-Rev-A
R-Cycle: R19+
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + leader 复核)
```

> **决策**: A 方案（ADR-0011）拍板后，需要定义 `apeireth-team-lead` 跟 `apeireth-council` 怎么协同——两个 crate 角色不同但必须协作。
>
> **状态**: 🔍 草拟 (待 Mavis 拍板 + leader 复核)
> **日期**: 2026-08-05
> **决策者**: Mavis + 主人 + 架构师
> **作者**: technical_writer
> **性质**: 第十二个 ADR — 记录 R19+ `apeireth-team-lead`（团队 leader 角色，构造 supervisor prompt）跟 `apeireth-council`（7 强制 Advisor 平行审议）之间的**协作规则**——两者**不耦合**，通过 trait 注入连接，council 缺实现时走 NoopVotingTrigger 兜底。
>
> **依据**: `ADR-0011` §决策 5（7 advisor voting 通过 trait 注入）+ `ADR-0010` §决策（apeireth-mcp::team 集成路径）+ `APEIRETH-CONVENTIONS` §9 6 锚穿透 + 蓝图 §5.2 第 7 行。
>
> **约束**: ❌ 不修改任何 LOCKED 文档 / Cargo.toml / crates/ 源码；仅新增命名空间 `docs/adr/0012-*.md` 独立 ADR。

---

## 状态

🔍 **草拟** (2026-08-05): A 方案已拍板（ADR-0011），本 ADR 草拟中；待 Mavis 拍板后由 architect2 实施。

---

## 背景（Context）

### 关键问题：team-lead 跟 council 怎么协同？

| crate | 角色 | 层级 | 职责 |
|---|---|---|---|
| `apeireth-team-lead`（A 方案新 crate, ADR-0011） | **团队 leader 角色** | Agent-level | 构造 supervisor prompt（含"if critical decision → trigger council vote"） |
| `apeireth-council`（R17 战略 0-4, 2740 LOC） | **7 强制 Advisor 平行审议** | Application-level | safety/performance/philosophy/history/strategy/ethics/legal 7 席投票 + 加权 synthesis |

**必须协同**:
- team-lead 构造的 prompt 包含触发条件："if critical decision → trigger council vote"
- council 投票结果回传给 team-lead 决定是否执行

**不能耦合**:
- ❌ team-lead **不能直接** import `apeireth-council`（避免循环依赖 + 单测困难）
- ❌ council **不能反向** import `apeireth-team-lead`（职责单向）
- ❌ 不能走"事件总线 = 松耦合万能药"（当前 crate 数量没到需要 bus 的阶段）

**解法核心**: **trait 注入**——`apeireth-protocol` 定义抽象 trait，`apeireth-team-lead` 持有 `Arc<dyn Trait>`，实施时注入具体实现。

### SpectrAI v0.9.21 现状（事实证据）

| 维度 | 数值 | 引用 |
|---|---|---|
| **supervisorPrompt.ts LOC** | 808 | `.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\supervisorPrompt.ts` |
| **7 advisor voting 触发点** | 1 处内嵌 | `buildSupervisorPrompt(tools, advisors)` 内 |
| **现有 bug** | mid-task 3 处根因 | 蓝图 §B.3 |
| **council 真实实现** | R21+ P2 计划 | 蓝图 §6 路线图 |

### 主人 2026-08-05 拍板事项

| 决策 | 内容 | 影响 |
|---|---|---|
| **A 方案采纳** | `apeireth-team-lead` 命名 | 本 ADR 才有"team-lead"主体 |
| **1:1 翻译** | supervisorPrompt.ts 808 LOC 保守先 | 7 advisor voting 翻译后要 trait 抽象 |
| **深度集成** | 不 patch fork | 跟 council 也是深度集成 |
| **不假装 council 已实现** | R21+ P2 真实 LLM 接入 | R19 阶段走 NoopVotingTrigger 兜底 |

---

## 决策（Decision）

**正式确立 "team-lead 通过 trait 注入 council voting 触发器" 路径**, 按 4 项硬决策:

### 决策 1: 新增 trait `AdvisorVotingTrigger`（位于 `apeireth-protocol` 抽象层）

**trait 签名**（估 80 LOC, 放 `apeireth-protocol/src/team.rs`）:

```rust
#[async_trait]
pub trait AdvisorVotingTrigger: Send + Sync {
    /// 判定当前 action 是否需要触发 council voting
    fn should_trigger_vote(&self, action: &TeamAction) -> bool;

    /// 异步请求 council 投票, 返回加权结果
    async fn request_vote(&self, action: TeamAction) -> VoteOutcome;
}

pub struct TeamAction {
    pub kind: ActionKind,         // Spawn / SendInput / Cancel / etc.
    pub risk_grade: RiskGrade,    // critical / high / medium / low / info
    pub context: ActionContext,   // 触发时的上下文
}

pub struct VoteOutcome {
    pub approved: bool,           // 是否通过
    pub votes: Vec<AdvisorVote>,  // 7 席详情（debug 用）
    pub synthesis: String,        // 加权 synthesis 文案
}
```

**位置理由**:
- `apeireth-protocol` 是抽象层（不依赖具体 crate），放这里不会产生循环依赖
- 跟 ADR-0003 trait-interlock 22 enum 风格统一
- 跟 ADR-0007 compat-components layer 兼容

### 决策 2: `apeireth-team-lead` 持有 `Arc<dyn AdvisorVotingTrigger>`，不直接依赖 council

**实施方式**:
- team-lead 构造函数接受 `voting: Arc<dyn AdvisorVotingTrigger>` 参数
- **不**在 `apeireth-team-lead` 内部 `use apeireth_council::*`
- 实施时（apeireth-team-lead R19 阶段 3）由 bootstrap 层注入具体实现
- 单测时注入 `NoopVotingTrigger`（默认通过）

**好处**:
- ✅ team-lead 可独立单测（不依赖 council 真实 LLM）
- ✅ council 真实 LLM 集成 R21+ P2 实施时不影响 team-lead
- ✅ 中间可注入 mock / fake / 真实实现，灵活

### 决策 3: 实施时注入 `CouncilVotingTrigger` 实现（位于 `apeireth-council::bridge`）

**实现位置**: `apeireth-council/src/bridge.rs`（估 100 LOC）

**实装内容**:
- `CouncilVotingTrigger` struct（持有 `Arc<apeireth_council::Council>`）
- 实现 `AdvisorVotingTrigger` trait
- `should_trigger_vote`: 按风险分级映射（critical→全 7 席, high→5 席, medium→3 席, low→1 席, info→0 席）
- `request_vote`: 调 council 真实 LLM 投票 + 加权 synthesis

**时间窗**:
- R19 阶段 3: 实施 `apeireth-team-lead` 时**同步**实施 `CouncilVotingTrigger`（含 mock LLM）
- R21+ P2: 接入真实 LLM（minimax m3 / OpenAI / etc.）

### 决策 4: 缺 council crate 实装时（mock），注入 `NoopVotingTrigger`（默认通过）

**实装位置**: `apeireth-team-lead/src/mocks.rs`（估 30 LOC）

**行为**:
- `should_trigger_vote` → 永远 `false`（不触发投票）
- `request_vote` → 永远 `approved: true` + 空 votes + 空 synthesis

**用途**:
- R19 阶段 3 早期：council crate 还没实装 bridge，先用 Noop
- 单测：跑 team-lead 单测时不依赖 council
- 集成测试：跑端到端时临时替换

**注意**:
- ❌ NoopVotingTrigger **不等于** 真实 voting
- ✅ Noop 是"占位符"，R21+ P2 必替换

---

## 后果（Consequences）

### 正面

- ✅ **team-lead 跟 council 解耦**: 两者可独立实装、独立单测、独立演进
- ✅ **trait 抽象可测**: 注入 mock 后可端到端测 team-lead
- ✅ **R21+ 替换 council 实装不影响 team-lead**: `AdvisorVotingTrigger` 接口稳定
- ✅ **跟 ADR-0003 trait-interlock 风格统一**: 抽象 trait 都放 `apeireth-protocol`
- ✅ **+150 LOC 总成本可控**: 80 LOC trait + 100 LOC bridge + 30 LOC mock

### 负面

- ⚠️ **trait 抽象可能太宽或太窄**: 第一次设计不完美，迭代 1-2 次
- ⚠️ **+150 LOC**: 依赖树增 1 个 trait 文件 + 1 个 bridge 文件
- ⚠️ **NoopVotingTrigger 兜底可能掩盖 bug**: 如果配错会以为"永远通过"是真的投票通过
- ⚠️ **5 关键不假装**: 后续实施不能假装 trait 抽象 1 次就完美

### 中和

- 🛡️ **NoopVotingTrigger 加 warning log**: 启动时打 "WARN: using NoopVotingTrigger" 防误配
- 🛡️ **trait 抽象先小后大**: 第 1 版只 2 个方法（`should_trigger_vote` + `request_vote`），后续按需加
- 🛡️ **bridge 实装留 TODO**: `CouncilVotingTrigger` 标 `#[todok = "R21+ P2 真实 LLM 接入"]`
- 🛡️ **集成测试覆盖 trait + 2 个实现**: team-lead 测 + council 测 + 端到端测

---

## 备选方案（Alternatives Considered）

### 选项 A: **trait 注入** ✅（采纳）

**优点**:
- ✅ team-lead 跟 council 解耦，可独立单测
- ✅ mock / fake / 真实实现可替换
- ✅ 跟 ADR-0003 trait-interlock 风格统一
- ✅ +150 LOC 总成本可控

**缺点**:
- ⚠️ 抽象可能太宽或太窄，迭代 1-2 次稳定
- ⚠️ NoopVotingTrigger 兜底可能掩盖 bug（要加 warning log）

**决策**: **采纳**（A 方案 + 本 ADR 决策 1-4）

### 选项 B: **直接依赖** ❌（否决）

**方式**: `apeireth-team-lead` 直接 `use apeireth_council::*`，调 `council.vote(action)`。

**优点**:
- ⚠️ 简单直接，不用设计 trait

**缺点**:
- ❌ **紧耦合**: team-lead 编译依赖 council
- ❌ **循环依赖风险**: council 也可能引用 team-lead 的 prompt 类型
- ❌ **违反单一职责**: team-lead 关心 leader 角色，council 关心平行审议，混合
- ❌ **单测困难**: 测 team-lead 必须 mock 整个 council
- ❌ **替换 council 难**: R21+ 换 LLM 实现要改 team-lead

**决策**: ❌ 否决（违反解耦原则 + 单测困难）

### 选项 C: **事件总线** ❌（备选）

**方式**: 走 `apeireth-bus`（未来 crate）事件总线，team-lead 发 `VoteRequestEvent`，council 监听 + 投票 + 回 `VoteResultEvent`。

**优点**:
- ✅ 极松耦合（通过事件解耦）
- ✅ 多订阅者（其他 crate 可同时听）

**缺点**:
- ❌ **过早设计**: 当前 crate 数量 41，没到需要 bus 的阶段
- ❌ **增加复杂度**: +1 个 bus crate + 事件定义 + 订阅/发布 + 序列化
- ❌ **async 链路变长**: vote 结果回传要等事件循环
- ❌ **调试困难**: 事件流不如直接调用直观
- ❌ **当前没 apeireth-bus crate**: 实施要先建 bus

**决策**: ❌ 备选（保留，等 R22+ crate 数量爆炸时再考虑）

---

## 实施路径

| 阶段 | 时间窗 | 内容 | 估 LOC |
|---|---|---|---|
| **R19 阶段 3** | 2026-08 ~ 2026-09 | 实施 `apeireth-team-lead` + `AdvisorVotingTrigger` trait | 850 + 80 |
| **R19 阶段 3** | 2026-08 ~ 2026-09 | 实施 `apeireth-council::bridge::CouncilVotingTrigger`（mock LLM） | 100 |
| **R19 阶段 3** | 2026-08 ~ 2026-09 | 实施 `apeireth-team-lead::mocks::NoopVotingTrigger` | 30 |
| **R19 阶段 3** | 2026-08 ~ 2026-09 | 集成测试: trait + 2 实现 + 端到端 | 100 |
| **R21+ P2** | 待定 | 接入 council 真实 LLM（minimax m3 / OpenAI / etc.） | 待估 |

**实施步骤（按 R19 阶段 3 排）**:

1. **第 1 步**: architect2 实施 `AdvisorVotingTrigger` trait（`apeireth-protocol/src/team.rs`）
2. **第 2 步**: integration 实施 `apeireth-team-lead` 构造函数（接受 `Arc<dyn AdvisorVotingTrigger>`）
3. **第 3 步**: code_reviewer 实施 `NoopVotingTrigger` mock
4. **第 4 步**: integration 实施 `CouncilVotingTrigger` bridge（mock LLM 兜底）
5. **第 5 步**: agent-orchestrator 写集成测试
6. **第 6 步**: leader 跑端到端 + R-Measure 验证不掉 baseline

**单测策略**:
- team-lead 单测: 注入 `NoopVotingTrigger`
- council bridge 单测: 注入 `MockLlmClient`
- 端到端测: 真实 2 个实现 + 真实 m3 测 prompt 敏感度

---

## 关键不假装（5 项）

1. **不假装 trait 抽象 1 次就完美** — 第一次设计可能太宽或太窄，迭代 1-2 次稳定
2. **不假装 council bridge 实装立刻能跑** — 要等 council 真实 LLM 集成 R21+ P2 才有意义
3. **不假装 team-lead 跟 council 协同 0 bug** — m3 测后迭代，prompt 敏感度未知
4. **不假装 trait 跟现有 12 子规范 100% 兼容** — 实施时验证，冲突走 ADR 修订
5. **不假装 NoopVotingTrigger 等同真实 voting** — Noop 是 mock，"永远通过"不等于"投票通过"

---

## 不修改承诺

- 阶段 1+2+3 LOCKED 文档
- v2/v4/v4.1 LOCKED
- 阶段 4 核心文档 LOCKED (6ca80776)
- 阶段 5 施工文档 LOCKED (631 行)
- v6 基础架构
- R11 baseline (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
- APEIRETH-CONVENTIONS.md
- VERSIONING.md
- GLOSSARY.md
- START-CONSTRUCTION.md
- apeireth-legacy/
- workspace version 1.0.0 (semver 严格)

---

## 哲学 anchor（6 项穿透）

| 锚 | 来源 | 穿透点 |
|---|---|---|
| **S-1** | 22:33 | 6 anchor ASI 完整性 — team + council 双层架构（leader 调度 + council 审议） |
| **S-2** | 17:43 | 6 anchor 实验室 — trait 注入可测（mock + bridge + 真实实现可替换） |
| **O-5** | 17:58 | 6 anchor 12 急救 — 协同点 1:1 翻译保留（"if critical decision → trigger council vote"） |
| **O-2** | 19:33 | 6 anchor 4 分类 — 角色严格区分（team-lead ≠ supervisor ≠ council） |
| **O-3** | 23:44 | 6 anchor 决策清单 — ADR 模板（背景/决策/后果/备选/实施/不假装/不修改承诺） |
| **O-4** | 00:56 | 6 anchor 12 统一 — 跟现有 12 子规范统一（trait-interlock + compat-components 兼容） |

---

## 关联文档

- `ADR-0010` apeireth-mcp 来自 SpectrAI 翻译
- `ADR-0011` apeireth-team-lead 新 crate（A 方案）
- `ARCHITECTURE.md` §5.2 集成映射表
- `docs/stage4/glossary-spectrAI-additions-2026-08-05.md`（8 词条草拟）
- `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md`（蓝图主文档）
- `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`（R11 LOCKED 引用）
- `APEIRETH-CONVENTIONS.md` §9 6 锚穿透

---

## 拍板记录

| 时间 | 决策 | 影响 |
|---|---|---|
| 2026-08-05 13:34 | 主人拍板 `apeireth-team-lead` A 方案（ADR-0011） | 触发本 ADR 起草 |
| 2026-08-05 | Mavis 默认 A 方案 trait 注入（背景/决策 1-4） | 待 leader 复核 |
| 待 Mavis 拍板 | trait 抽象签名（决策 1） | 由 Mavis 跟 architect2 协商 |
| 待 leader 复核 | 5 关键不假装 | 由 leader 跑 m3 验证 |
