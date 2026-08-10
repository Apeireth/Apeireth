# GLOSSARY spectrAI 词条草稿（R19+ 待拍板）

```
[Document-Meta]
Document: docs/stage4/glossary-spectrAI-additions-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + leader 复核)
```

> **性质**: 8 项 R19+ 新增术语的草拟词条——GLOSSARY.md 是 LOCKED 规范文档（属 APEIRETH-CONVENTIONS 不修改承诺），所以**先沉淀到 stage4**，等 Mavis 拍板后由 Mavis 批量插入 GLOSSARY.md（不是本任务的活）。
>
> **依据**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 集成映射 + ADR-0010（apeireth-mcp 来自 SpectrAI 翻译）+ ADR-0011（apeireth-team-lead 新 crate）+ `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` 13 项资产清单。
>
> **约束**: ❌ 不直接修改 GLOSSARY.md；本文件仅草拟词条，等 leader 复核后由 Mavis 合并。

---

## 🧩 1. spectrAI / SpectrAI

**定义**: Apeireth R19+ 集成蓝图的**源项目**——v0.9.21 版本的 TypeScript/Electron AI agent 桌面 app，由 minimaxi 生态出品。1:1 翻译成 Rust crate 并入 Apeireth 主项目，不再维护 TS 主线。

**缩写/全称**:
- `spectrAI`（全小写，作为来源项目代号；`.minimax-agent-cn\spectrai\spectrai-source\` 目录命名风格）
- `SpectrAI`（PascalCase，文档/讨论时的正式写法）

**命运**:
- v0.9.21 卡死在 mid-task bug 3 处
- m3 hallucination 多次复现不治
- 路线拍板：装进 Apeireth 主项目，不走 patch fork

**出处**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §1.1 + §3 现状

**关联**: `ARCHITECTURE.md` + `ADR-0010` + `ADR-0011` + `tauri-assets-from-spectrAI-2026-08-05.md`

**注意**: 不要写 `Spectrai` / `SPECTRAI` / `spectral`（避免拼写漂移）。

---

## 🧭 2. team-lead / apeireth-team-lead

**定义**: R19+ 新 crate，**1:1 翻译** SpectrAI `supervisorPrompt.ts` (808 LOC) 的 Rust crate。负责构造"团队 leader 角色"的 system prompt + 触发 7 advisor voting。

**缩写/全称**: `team-lead`（口语）/ `apeireth-team-lead`（crate 名全称）

**角色**: **团队 leader 角色**（agent-level）——构造 supervisor prompt + 7 advisor voting 触发。

**与同名概念严格区分**:

| crate | 层级 | 职责 |
|---|---|---|
| `apeireth-team-lead`（新） | Agent-level | 构造 leader 角色 prompt |
| `apeireth-supervisor`（已有 550 LOC） | OS-level | 进程树监督（PID 1） |
| `apeireth-council`（已有 2740 LOC） | Application-level | 7 强制 Advisor 平行审议 |

**估时**:
- 核心 lib: 600 LOC
- 14 工具 prompt 描述: 200 LOC
- 单元测试: 50 LOC
- **合计 ~850 LOC**

**出处**: `ADR-0011` A 方案（2026-08-05 13:34 主人拍板）

**关联**: `ADR-0010`（apeireth-mcp 集成路径）+ `ADR-0012`（team-lead 跟 council 协同）

---

## ⚖️ 3. council / apeireth-council / 七席审议庭

**定义**: R17 战略 0-4 收尾的 crate，2740 LOC。**7 强制 Advisor 平行审议**（safety/performance/philosophy/history/strategy/ethics/legal）+ 加权 synthesis + 拟人化辩论。

**缩写/全称**: `council`（口语）/ `apeireth-council`（crate 名）/ 七席审议庭（中文）

**角色**: **平行审议**（safety gate）——按风险等级差异化触发席位（critical 全 7 席、high 5 席、medium 3 席、low 1 席、info 0 席）。

**与同名概念严格区分**:

| crate | 职责 | 类比 |
|---|---|---|
| `apeireth-council` | **平行审议**（safety gate，横向 7 席） | 陪审团 / 安全审查委员会 |
| `apeireth-team-lead` | **团队 leader 角色**（纵向 leader-worker） | 项目经理 |
| `apeireth-supervisor` | **进程监督**（OS-level PID 1） | init / systemd |

**❌ 不要混淆**: council **不是** SpectrAI 团队功能！council 是 R17 自创的"7 强制 advisor 平行审议"机制，跟 SpectrAI 无关。

**出处**: `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` R11 LOCKED + `GLOSSARY.md`（已有"📊 风险分级 → 席位触发矩阵"词条）

**关联**: `GLOSSARY.md` 风险分级词条 + `ADR-0012`（team-lead 跟 council 协同规则）

---

## 🛡️ 4. supervisor / apeireth-supervisor

**定义**: R19 阶段 crate，550 LOC。**进程监督**（PID 1 / 5 sub-supervisor / actor + child + strategy）——监控 child actor 生命周期，tokio::process + nix 依赖。

**缩写/全称**: `supervisor`（口语）/ `apeireth-supervisor`（crate 名）

**角色**: **进程树监督**（OS 级别）——类比 systemd / launchd。

**与同名概念严格区分**:

| crate | 职责 | 类比 |
|---|---|---|
| `apeireth-supervisor` | **进程监督**（OS-level） | init / systemd |
| `apeireth-team-lead` | **团队 leader 角色**（agent-level） | 项目经理 |
| `apeireth-council` | **审议庭**（安全 gate） | 陪审团 |

**❌ 不要混淆**: supervisor **不是** 团队 leader！supervisor 是 OS 进程管理，跟 agent 角色无关。

**强约束**:
- `apeireth-team-lead` **不依赖** `apeireth-supervisor`（避免循环依赖风险，ADR-0011 §决策 4）
- 命名带"supervisor"≠ 命名带"lead"，未来 maintainer 一看就懂

**出处**: `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` R11 + `ADR-0011` §"apeireth-supervisor vs 新 crate 职责对比"

**关联**: `ADR-0011` + `ADR-0012`

---

## 📏 5. V1141 / V1131 / V1136（R-Measure baseline 3 值）

**定义**: R11 baseline LOCKED 的 3 个 R-Measure 指标值，R19+ 集成新功能时**必须不掉 baseline**。

**数值**:
- **V1141** = 0.8682
- **V1131** = 0.8532
- **V1136** = 0.9063

**用途**:
- R19+ 集成 `apeireth-team-lead` 新 crate
- R19+ 集成 `apeireth-mcp::team` 14 supervisor 工具
- R19+ 修 SpectrAI mid-task bug 3 处
- **以上任何变更不能掉 baseline**

**回归判定**: 任一指标 < baseline 值 → 立即回滚 + 走 R-Measure 复测。

**出处**: `APEIRETH-CONVENTIONS.md` §11（baseline LOCKED）+ R11 omnibus 文档

**关联**: `APEIRETH-CONVENTIONS.md` + `ADR-0010` §后果 + `ADR-0011` §决策（不假装掉 baseline 不发生）

---

## 🐛 6. mid-task bug / mid-task message bug

**定义**: SpectrAI v0.9.21 已知的"**团队 leader 发 mid-task 消息给子 agent 失败**" bug。3 处根因（非 1 处）一起改才能根治。

**真根因 3 处**:

1. **`SessionManagerV2.sendMessage:642`** — 终态用 `throw` 而非 `return`，导致失败路径抛异常而非返回错误对象
2. **`AgentManagerV2.sendToAgent:281`** — 用 `.catch()` 吞错 + 永远 `return success:true`，掩盖了失败
3. **child session 状态变化到 agent 状态变化的窗口期** — 异步 race，无锁保护

**修法**:
- 3 处一起改，合并到 `apeireth-mcp::team`
- 决策依据：`ADR-0010` §决策（"修一处 = 留 2 处 bug"）
- 不修 1 留 2 → 修 1 必暴露 2 → 必须 3 处一起改

**关联**: `ARCHITECTURE.md` §4 + `ADR-0010` §决策 + 蓝图 §B.3 3 处根因

**注意**:
- ❌ 不要在 R19 阶段"修第 1 处先上"——会暴露 2、3 处
- ✅ 3 处一起修，统一在 `apeireth-mcp::team` 实装

**出处**: 蓝图 §3.1（v0.9.21 卡死现状）+ 蓝图 §B.3（3 处根因分析）

---

## 📦 7. Tauri 资产沉淀

**定义**: R19+ 集成的**副产品**——SpectrAI Electron 特性沉淀成 Tauri 团队接手时的"项目资产"。

**13 项 T-001~T-013**:
- T-001 ~ T-013 涵盖 11 个 IPC handler + 1 个 UpdateManager + 1 个 OutputReaderManager 等
- 详细清单见 `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` §3

**文档位置**: `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md`

**原则（3 项）**:
- **透明**: 13 项资产全部文档化，不留"隐藏资产"
- **文档化**: 每个资产有 T-XXX 编号 + SpectrAI 来源文件 + LOC + 集成建议
- **不变成隐形资产**: Tauri 团队接手时能直接 grep 到 T-XXX 找到

**出处**: `tauri-assets-from-spectrAI-2026-08-05.md`（R19+ 战略拍板文档）

**关联**: `ADR-0010`（apeireth-mcp 跟 SpectrAI 来源映射）+ 蓝图 §5.2 第 7 行

---

## 🔌 8. 2 套 LLM 抽象

**定义**: R17 战略 1-4 之后，`apeireth-api` 存在 **2 套并行** 的 LLM 抽象层。第一套走 DEPRECATE 路径，第二套是当前主路径。

**第一套（DEPRECATE 路径）**:
- 抽象: `LlmProvider` trait
- 出处: 战役 0
- 实现: 4 concrete provider（`OpenAiProvider` / `AnthropicProvider` / `OllamaProvider` / `GeminiProvider`）
- 路由: `MultiLlmRouter`
- 状态: ⚠️ DEPRECATE（保留兼容，但不再新增功能）

**第二套（当前主路径）**:
- 抽象: `ProtocolRouter`
- 出处: 战役 1-4
- 实现: 4 zero-sized adapter（`OpenAiAdapter` / `AnthropicAdapter` / `OllamaAdapter` / `GeminiAdapter`）
- 优势: 编译期 hardcode + 零运行时开销

**5 Provider 全 base URL 配置**:
- minimaxi（默认）
- OpenAI
- Anthropic
- Ollama
- Gemini

每 Provider 都有独立 base URL 配置项（不 hardcode 在代码里）。

**关联**: `docs/stage4/apeireth-platform-modules-2026-08-05.md` + `apeireth-api` crate 源码

**注意**:
- ❌ 新功能**不要**走 `LlmProvider` trait（已 DEPRECATE）
- ✅ 新功能统一走 `ProtocolRouter` + 4 adapter
- ⚠️ 旧的 `LlmProvider` 代码会逐步迁移，R20+ 可能彻底移除

**出处**: R17 战略 1-4 文档 + `apeireth-api/src/protocol/` 目录

---

## 📋 拍板记录

| 时间 | 决策 | 影响 |
|---|---|---|
| 2026-08-05 13:34 | 主人拍板 `apeireth-team-lead` A 方案（ADR-0011） | 触发本词条草稿 |
| 2026-08-05 13:34 | 主人拍板 A 方案集成策略（蓝图 §1） | 触发 Tauri 资产沉淀 + 3 处根因一起修 |
| 待 Mavis 拍板 | 8 词条是否一次性插入 GLOSSARY.md | 由 Mavis 合并，不是本任务 |

---

## 🔒 不修改承诺

- GLOSSARY.md（LOCKED，本文件不直接改）
- APEIRETH-CONVENTIONS.md
- 阶段 1+2+3+4+5 LOCKED 文档
- v2/v4/v4.1 LOCKED
- 阶段 4 核心文档 LOCKED (6ca80776)
- workspace version 1.0.0 (semver 严格)

---

## 哲学 anchor（6 项穿透）

| 锚 | 来源 | 穿透点 |
|---|---|---|
| **S-1** | 22:33 | 6 anchor ASI 完整性 — 8 词条都属"成长阶段" |
| **S-2** | 17:43 | 6 anchor 实验室 — 词条可测、可追溯 |
| **O-5** | 17:58 | 6 anchor 12 急救 — 不假装词条已合并到 GLOSSARY |
| **O-2** | 19:33 | 6 anchor 4 分类 — 角色严格区分（3 个 supervisor/lead/council 词条） |
| **O-3** | 23:44 | 6 anchor 决策清单 — 8 词条对应决策锚点齐全 |
| **O-4** | 00:56 | 6 anchor 12 统一 — 跟 GLOSSARY.md 风格统一 |

---

## 关联文档

- `GLOSSARY.md`（目标合并位置，LOCKED 不动）
- `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md`（蓝图主文档）
- `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md`（13 项资产清单）
- `ADR-0010`（apeireth-mcp 来自 SpectrAI 翻译）
- `ADR-0011`（apeireth-team-lead 新 crate）
- `ADR-0012`（team-lead 跟 council 协同规则）
- `APEIRETH-CONVENTIONS.md` §11（baseline V1141/V1131/V1136）
