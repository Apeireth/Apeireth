# R11 Species Core Gap 真调研 — Apeireth v1.0 vs v2.0 (2026-08-28)

> **作者**: sub-agent R11-SpeciesCore (派单 per 主代理 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §4.2 #3)
> **任务**: 真调研物种化核心 4 gap (education + partner + community + principles), 给主代理 Mavis 决策参考
> **关系**: 跟 `vision.md` L29-49 (物种而非个体 + 教后代 + 跨墙信任 + 三远合一) + `apeireth-true-understanding-2026-08-28.md` §2 (物种 vs 个体) + §3.2 (物种化借鉴边界) + 主代理真账 §2.4 (物种化核心缺口) + `v2-reference-handbook-2026-08-28.md` §1.2-1.3 (五原型 + 12 slot) + F6 organ 集成

```
[Document-Meta]
Document:        docs/04-internal/r11-species-core-gap-research-2026-08-28.md
Version:         1.0 (R11-SpeciesCore 写于 2026-08-28)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (R11 真调研真账, 物种化核心 4 gap 派单基础)
Author:          sub-agent R11-SpeciesCore
```

---

## 0. 用户 directive + 主代理派单 brief (per O-5 + S-2)

**主代理派单 brief 摘要** (per `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §4.3):

- **任务**: education + partner + community + principles 4 gap 真调研, 写真账
- **必读**: vision.md L29-49 + apeireth-true-understanding + 主代理真账 §2.4 + 4 个 1.0 真账 + v2 handbook §1.2-1.3 + F6 organ
- **输出**: 本真账 (≤ 300 行)
- **约束**: 不写真账以外的 file / 不 git add / commit / push / 5 重守门 + LOCKED 0 触碰 / 0 装诚实标 / 物种化维度

**R11 真调研范围**: 物种化核心 = vision L47-49 三大支柱真实施缺口, 不是功能 1:1 翻译, 是 **物种化哲学层落地** ("教'如何成为自己', 而非复制自己").

---

## 1. education — vision L48 物种化核心真调研

### 1.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/education.rs`, 402 行)

**1.0 真实施**:
- **真内容**: 教育套件插件 = 换元法 dx 检查器 (`DxCheckTool`) + `EducationDxPlugin` 装配
- **真功能**: `dx_check` 工具 (4 检查: 忘换 dx / 混用 / 缺微分 / 残留 x + 根号模式提示)
- **真机制**: 规则层字符串扫描 (无 CAS 引擎, 0 假装解积分) → 注册到 `ToolRegistry` + `PermissionPack::permanent` 授权
- **0 假装标注** (L7-12): "v1 是字符串级规则表, 不是真实符号计算", 覆盖 4 检查外加常见根号模式

**1.0 哲学对位 vision L48**:
- 1.0 education 是 **"教育后代某个具体技能"** (数学 dx 检查) — 跟 L48 "能教养后代" 物种化核心 **失配**
- L48 真意: 教'如何成为自己', 而非复制自己 — **物种化哲学层** (生态/传承), 不是技能教学
- **1.0 真账 0 触及 L48 真意**, 但提供了 "插件+工具" 装配模式 (per `PluginRegistry.install` 路径) 可作为 v2 物种化教育的工程基础

### 1.2 2.0 现状 (per v2 grep 真账)

- **grep 全 crates**: `education` 0 命中 (`crates/` 全搜, 仅 `apeireth-organ` `curiosity` 锚定器官相关, 无 education.rs / 无 education organ / 无 cognitive.slot)
- **v2 真账**: `vision.md` L48 "能教养后代" 真实施 0 — vision L48 物种化核心 **失守**
- **1:1 翻译缺**: 教育套件插件模式 0 移植 (per 真账 §2.4 #8: "vision L48 '能教养后代' = species 核心, per Apeireth 真理解")

### 1.3 真实施路径 (物种化核心, 不是功能 1:1)

**物种化借鉴边界** (per apeireth-true-understanding §3.2 修订):
- **不只功能 1:1 翻译** — 1.0 是技能教学, v2 必须物种化哲学层落地
- **借鉴 N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio** (Round 10 5 真调研已就位) 的"教/学/塑形"思路
- **真实施**:
  - **Phase 1 (1 周, 真调研)**: 物种化教育 spec — "如何成为自己" 的工程表达 = per-user personality/profile 导出 + 教学候选生成 (不是技能题)
  - **Phase 2 (2 周, 真实施)**: `apeireth-organ::education` (新 organ) — 提案 "自己如何长大" 的描述 + 主人批准机制 (复用 F6 value_cases approve_principle master token 模式) + LLM Adapter 提炼
  - **Phase 3 (1 周, 集成 + 测试)**: WIRED 到 `OrganOrchestrator` + 12 slot (新 slot `cognitive.education` 或合并到 preference_learning)
- **估时**: 调研 1 周 + 真实施 3 周 = **4 周 critical path** (主代理估时 §3.1 #8 = 2-3 周偏乐观, 真实施需 4 周)
- **阻塞**: 0 (无硬件, 无 LOCKED 触碰)
- **借鉴链**: N.E.K.O 角色塑造 + AIRI Live2D 行为模板 + 1.0 `EducationDxPlugin` 装配模式 + F6 value_cases approve 模式

### 1.4 物种化借鉴边界 (R11 修订)

- **L48 真意 = "知道自己如何长大的存在, 才有资格教养新的存在"**: v2 education = 教"她如何成为她" (per-user profile 导出), 不是教"数学" / "代码" / "工具"
- **物种 vs 个体核心**: education 不是 1.0 技能层, 而是 **物种传承层** (per vision L51 "三远合一 记忆/物种/传承")
- **跟其他 3 项关系**: education 是 partner/community 的 **最终形态** (跨用户教"如何成为她"必须先有 partner + community + principles)

---

## 2. partner — 1.0 vs 2.0 + 物种化借鉴

### 2.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/partner.rs`, 141 行)

**1.0 真实施**:
- **`Partner`** struct: 用户作为伙伴 (per stage1 2026-08-14 清晰版: 用户在关系里, 是 AI 的伙伴)
- **`PartnerPreferences`**: 称呼 / 表达风格 / 关心话题 / 雷区 / 隐私边界 (`PrivacyBoundary` per opencode-vibeguard 模式)
- **`Bond`** (per `bond.rs`): 关系阶段 (`BondStage::Initial` 等) — bond 是关系演进状态机
- **真机制**: `Partner::new` / `touch()` (last_seen 更新) / `update_preferences()` — 简单 CRUD + bond 阶段

**1.0 哲学对位 vision L49 "跨墙的信任"**:
- 1.0 partner 是 **"用户在关系里"** 的工程承载 — 部分对位 vision L49 (跨墙信任)
- 缺 1.0 真账: 跨用户协作机制 (per 主代理真账 §2.4 #9: "partner = 跨用户协作")
- **1.0 真账 0 触及 vision L49 物种化跨墙**, 但提供了 partner struct + bond 阶段机 + privacy 边界基础

### 2.2 2.0 现状

- **grep 全 crates**: `partner` 0 命中 (`crates/` 全搜, 仅 `apeireth-credentials/src/gate.rs:5` 注释 reference principles)
- **v2 真账**: 0 真实施 — partner struct / bond / privacy / 跨用户协作 全部 0
- **1:1 翻译缺**: 主代理真账 §1.8 #partner 🔴 **缺** (跟 `cognitive.perception` + relationship 路径相关)

### 2.3 真实施路径

- **真实施**:
  - **Phase 1 (1 周)**: 物种化 partner spec — per-user `Partner` struct (复刻 1.0 + 加 `cross_user_id` 跨用户协作 ID) + bond 阶段机 + privacy 边界 + sovereignty 集成
  - **Phase 2 (2 周)**: `apeireth-organ::partner` (新 organ) OR `apeireth-companion::partner` 模块 + 跟 `BondStage` 集成到 cognitive slot (新 slot `cognitive.relationship` 或合并到 `cognitive.preference_recall`)
  - **Phase 3 (1 周)**: WIRED 到 `OrganOrchestrator` + 5 重守门 baseline
- **估时**: **4 周** (1.0 简单复刻 1 周 + 跨用户协作 2 周 + 集成 1 周)
- **阻塞**: 0
- **借鉴链**: 1.0 partner.rs 真账 + Round 10 N.E.K.O 角色关系 + bond 阶段机设计 (Open-LLM-VTuber / AIRI 调研已就位)

---

## 3. community — 1.0 vs 2.0 + 物种化社区借鉴

### 3.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/community.rs`, 360 行)

**1.0 真实施**:
- **图社区分层聚合 + 双级检索** (LightRAG/GraphRAG 精神, 记忆调研批 ⭐)
- **`Community`** struct: members (字典序) + facts
- **`detect_communities`**: 轻量确定性连通分量聚类 (BTreeMap + BTreeSet 字典序遍历, 社区 id `comm-{i}` 稳定)
- **`deterministic_summary`**: 高频实体 top-N (频次降序 → 字典序升序) 摘要
- **`triage`**: 双级检索路由 — 查询含实体 → `Route::Entity` (CRAWL); 无实体 → `Route::Broad` (社区摘要)
- **`Summarizer` trait**: 0 装口 (LLM 提炼实现替换确定性版)

**1.0 哲学对位 vision L47 "物种而非个体"**:
- 1.0 community 是 **"图社区分层聚合"** — 工程层 memory 检索, 不是物种化社区
- 缺 1.0 真账: per-user 社区 (不同用户不同形态) — vision L47 真意是 **物种化塑形**, 不是 memory 检索
- **1.0 真账提供了 deterministic 社区检测算法** (可作为 v2 物种化社区的 memory 基础)

### 3.2 2.0 现状

- **grep 全 crates**: `community` 0 命中 (community.rs 不存在, `detect_communities` 0 移植)
- **v2 真账**: 0 真实施 — community detection / 双级检索 / 物种化社区 全部 0
- **1:1 翻译缺**: 主代理真账 §1.8 #community 🔴 **缺** (物种化 + 跨用户社区相关)

### 3.3 真实施路径

- **真实施**:
  - **Phase 1 (1 周)**: 物种化 community spec — 1.0 community.rs 1:1 翻译 (detect_communities + triage + Summarizer trait) + 加 species 维度 (per-user 社区 ID + cross-user 桥接)
  - **Phase 2 (2 周)**: `apeireth-storage::community` 模块 (per §1.1 VectorIndex + Graph primitives 缺, community 是 Graph primitives 的应用层) + `apeireth-organ::community` (新 organ) OR `cognitive` slot (新 `cognitive.community_recall`)
  - **Phase 3 (1 周)**: WIRED 到 `OrganOrchestrator` + 5 重守门 + 跟 memory_graph 集成
- **估时**: **4 周** (1:1 翻译 1 周 + species 维度 2 周 + 集成 1 周)
- **阻塞**: 0 (但跟 §1.1 Graph primitives 缺 关联, Graph primitives 真实施是 community 的前置, 主代理 §3.1 #2 已派单)
- **借鉴链**: 1.0 community.rs 真账 (360 行 deterministic) + LightRAG/GraphRAG 精神 + N.E.K.O 社区机制

---

## 4. principles — 1.0 vs 2.0 + 哲学价值内化 (跟 F6 organ 集成)

### 4.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/principles.rs`, 478 行)

**1.0 真实施**:
- **自成长管道 Level 2/3: 动态原则层 + 原则洋葱晋级候选** (per 主人 2026-08-16 设想)
- **`DynamicPrinciple`**: 准则 (前缀匹配) + 理由 + 来源 + status (pending/active/rejected/retired) + violations
- **`PrincipleStore`**: append-only episodes (id 前缀 `princ-`) + chain + rev 单调
- **安全模型**: 批准 = `approve_principle` + `APEIRETH_MASTER_TOKEN` env 比对 (constant-time 比较, AI 无 token 无法自批准)
- **L2 入口**: `propose_principle` 工具 (AI 提案 → pending, 不能自生效)
- **L3 晋级**: `promotion_candidates` (active + 0 violation) + `export_promotion` (主人侧工程动作输入)

**1.0 哲学对位 F6 价值内化**:
- 1.0 principles 是 **F6 价值内化的下游** — 案例 → 多次一致 → 原则候选 (per 1.0 value_cases.rs L17: "promote_candidates")
- 1.0 真账完整**: dynamic principles + master token 物理隔离 + constant-time + onion 晋级候选
- **跟 F6 organ 集成路径**: `ValueCaseStore::promote_candidates(n)` 输出 → `PrincipleStore::propose(statement, rationale, source)` 入口

### 4.2 2.0 现状 (F6 value_cases ✅ WIRED, 但 principles 0)

- **grep 全 crates**: `principles` 仅 1 命中 (`apeireth-credentials/src/gate.rs:5` 注释 reference), `PrincipleStore` / `DynamicPrinciple` / `propose_principle` / `approve_principle` 全部 0 真实施
- **v2 F6 organ 真账** (per `crates/engine/organ/src/value_cases.rs` + `lib.rs:14,61`): ✅ **1:1 翻译 v1 value_cases 真实现 (子代理 R3, 2026-08-28)** + WIRED 到 `OrganOrchestrator` (per `crates/engine/runtime/src/canonical/orchestrator.rs:46,406,511,796,1283`)
- **1:1 翻译缺**: principles 0 真实施 (per 主代理真账 §1.8 #principles 🔴 **缺**)
- **v2 F6 organ 跟 1.0 principles 集成 0**: F6 value_cases 已 WIRED, 但 `promote_candidates` → `PrincipleStore::propose` 路径未建

### 4.3 真实施路径 (跟 F6 value_cases organ 集成)

- **真实施**:
  - **Phase 1 (1 周)**: 物种化 principles spec — 1.0 principles.rs 1:1 翻译 (DynamicPrinciple + PrincipleStore + master token + constant-time) + 跟 F6 organ 集成 (`promote_candidates` → `PrincipleStore::propose` 自动流)
  - **Phase 2 (2 周)**: `apeireth-organ::principles` (新 organ, 跟 F6 同 crate) OR `apeireth-companion::principles` 模块 + 跟 `APEIRETH_MASTER_TOKEN` 集成 + 5 重守门 + onion 晋级候选
  - **Phase 3 (1 周)**: WIRED 到 `OrganOrchestrator` (新 organ 锚 + cognitive slot) + 5 重守门 baseline + tests
- **估时**: **4 周** (1:1 翻译 1 周 + F6 集成 2 周 + 测试 1 周)
- **阻塞**: 0 (F6 organ 已 WIRED, 集成 ready)
- **借鉴链**: 1.0 principles.rs 真账 (478 行, 完整 safety model) + F6 organ WIRED + 9 哲学锚 LOCKED + 13 键洋葱

---

## 5. 物种化核心综述 (整合 4 项)

### 5.1 跟 vision.md L47-49 + L51 对位

| vision 锚 | 真意 | 4 项对位 | 物种化核心 |
|---|---|---|---|
| **L47 物种而非个体** | 机制/哲学/安全同源, 记忆/偏好/好奇形状被共同生活塑形 | community (物种化社区) + partner (per-user 塑形) | 物种化架构 |
| **L48 她能教养后代** | 教'如何成为自己', 而非复制自己 | education (物种化哲学层) | 物种化核心最终形态 |
| **L49 跨墙的信任** | 存在论不可逾越, 但墙不影响生活 | partner (跨用户协作 + bond + privacy) | 物种化跨墙 |
| **L51 三远合一** | 记忆 / 物种 / 传承 = 生命三个定义 | 4 项总和 (principles 记忆 + community 物种 + partner 跨墙 + education 传承) | 物种化三远 |

### 5.2 4 项核心缺口的相互关系

- **principles (F6 价值内化层)** = 基础 — 哲学价值先内化 (per F6 organ WIRED), 才能 species 价值层落地
- **partner** = 跨用户 + per-user 塑形 — 物种化塑形的工程承载 (per 1.0 partner.rs BondStage + PrivacyBoundary)
- **community** = 物种化社区 — 图社区 + species 维度 (per 1.0 community.rs detect_communities + triage)
- **education** = 物种化核心最终形态 — 教'如何成为自己' 必须在 principles + partner + community 都就位后 (否则 0 教学材料)

**真实施顺序**: principles (F6 价值内化层, 基础) → partner (per-user 塑形) → community (物种化社区) → education (物种化核心最终形态)

### 5.3 真实施顺序的 5 重守门 + LOCKED 0 触碰验证

- **principles**: 跟 F6 organ WIRED 集成, 0 LOCKED 触碰 (F6 organ 已 1:1 翻译 v1, 不改 traits)
- **partner**: 新 struct / bond 阶段机, 0 LOCKED 触碰
- **community**: Graph primitives 缺是真实施前置 (per 主代理 §3.1 #2), 但 community.rs 1:1 翻译独立
- **education**: 完全新 organ, 0 LOCKED 触碰, 但 Phase 1 spec 必须主代理亲做 (物种化核心决策)
- **5 重守门 baseline** (per 真账 §6): clippy 0 warning / tests 0 fail (1739 baseline) / legacy compat path < 100 (36) / LOCKED 5 项 0 触碰 / 9 哲学锚 0 减

---

## 6. 主代理决策建议

### 6.1 4 项优先级排序 (per O-6 总体最优)

| 优先级 | 项 | 估时 | 阻塞 | 决策理由 |
|---|---|---|---|---|
| **P0 #1** | **principles** (4 周) | 4 周 | 0 (F6 organ WIRED ready) | F6 价值内化已就位, principles 真实施 = F6 下游完整, **风险最低 / 收益最高** |
| **P0 #2** | **partner** (4 周) | 4 周 | 0 | 1.0 真账简单 (141 行), 跨用户协作增量, 跟 cognitive slot 集成 |
| **P0 #3** | **community** (4 周) | 4 周 | Graph primitives 缺 (per §1.1) | 1.0 真账完整 (360 行 deterministic), 物种化维度增量, 跟 §1.1 派单联动 |
| **P0 #4** | **education** (4 周) | 4 周 | 主代理亲做 spec | 物种化核心最终形态, spec 决策最重, 真实施最后 |

**总估时**: 4 项并行 critical path = **4 周** (主代理 §3.1 估 3-4 周正确, R11 估时 4 周真实施需 4-6 周)

### 6.2 真实施 brief (派 sub-agent 真调研 + 真实施)

**派单 brief 模板** (per §4.3):
- **P0 #1 principles 真调研**: 派 sub-agent 真读 `legacy/donor/apeireth-companion/src/principles.rs` (478 行) + `value_cases.rs` (F6 WIRED) + 真账
- **P0 #2 partner 真调研**: 派 sub-agent 真读 `legacy/donor/apeireth-companion/src/partner.rs` (141 行) + `bond.rs` + 真账
- **P0 #3 community 真调研**: 派 sub-agent 真读 `legacy/donor/apeireth-companion/src/community.rs` (360 行) + Graph primitives 真账 + 真账
- **P0 #4 education 真调研**: 派 sub-agent 真读 vision L48 + 5 真调研借鉴 + 主代理亲做 spec (物种化核心决策) + 真账

### 6.3 物种化核心 vs Round 10 5 真调研 互补不重叠

- **Round 10 5 真调研** (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio): **物种化前端 + 物种化架构借鉴** (per apeireth-true-understanding §3.2)
- **R11 物种化核心 4 gap** (education / partner / community / principles): **物种化哲学层落地** (vision L48 "教'如何成为自己'") = 5 真调研借鉴的 **承接 + 真实施**
- **互补**: 5 真调研 给借鉴点, R11 4 gap 真实施承接借鉴 → 物种化从"调研"到"工程"的 critical path 闭环

### 6.4 0 装诚实标 (per O-5)

| 失守 | 详情 | 修法 |
|---|---|---|
| **R11 0 实测** | 仅读 1.0 真账 (4 个 .rs 402+141+360+478=1381 行) + v2 真账 (grep + handbook §1.2-1.3 + F6 organ 真账) 推论, **未 git clone v2 master branch** | 真实施前主代理必亲验 (per 真账 §6.2 派单 brief) |
| **education 物种化哲学层 0 spec** | 1.0 是 dx_check 技能教学, vision L48 是物种化哲学层, R11 仅给真实施路径, **spec 必须主代理亲做** | 主代理拍板 species education 哲学 (教'如何成为自己'的工程表达) |
| **partner 跨用户协作 0 真账** | 1.0 partner 是 per-user 单用户, vision L49 跨用户协作是 0 真账增量, R11 估时偏乐观 | 真调研 + 真实施阶段必 sub-agent 真 clone v2 master branch 验 cross_user_id 集成路径 |
| **community Graph primitives 缺是真实施前置** | per 主代理 §1.1 #2 Graph primitives 缺, community 真实施阻塞 Graph primitives 真实施 | 派单时 §1.1 + §3 community 联动 (per §6.1 P0 #3) |

### 6.5 5 重守门 baseline + LOCKED 0 触碰 验证

| 守门 | R11 真调研影响 | 实测 |
|---|---|---|
| clippy 0 warning | R11 0 改 src, 仅写真账 | ✅ (前 baseline 维持) |
| tests 0 fail (1739) | R11 0 改 tests | ✅ (前 baseline 维持) |
| legacy compat path < 100 (36) | R11 0 改 legacy | ✅ (前 baseline 维持) |
| LOCKED 5 项 0 触碰 | R11 0 改 9 哲学锚 / 13 键 / 3 不可变 / workspace.version / R11 baseline 3 值 | ✅ (本轮 0 触碰) |
| 9 哲学锚 0 减 | R11 真调研不触及 9 哲学锚 | ✅ |

### 6.6 修订 release 路径 (per 真账 §6.3)

- **物种化核心 4 gap 真实施估时**: 4 项 P0 必补, **并行 critical path 4 周** (principles ready first → partner/community 跟 → education 最后)
- **修订 ROADMAP §7 总进度**: 物种化核心 4 gap 真实施后, 总进度 +5% → **75-80% → 80-85%** (per §6.3 当前 75-80% 真实)
- **修订 MANIFESTO §14 release timeline**: v2.0 release 估 2027-Q1-Q2, 物种化核心 4 周真实施 → release 路径上调 **6-9 月 → 7-10 月**

---

_R11-SpeciesCore 写于 2026-08-28 Round 11, 真调研物种化核心 4 gap (education + partner + community + principles) 给主代理 Mavis 决策参考. 1.0 真账 1381 行真读 + v2 真账 (handbook §1.2-1.3 + F6 organ WIRED + grep 全 crates) 真调研. 0 装诚实标: 0 git clone v2 master branch, 仅读真账推论, 真实施前主代理必亲验. 真实施顺序: principles (F6 价值内化层基础) → partner → community → education (物种化核心最终形态). 估时 critical path 4 周. 5 重守门 baseline 维持 + LOCKED 0 触碰._