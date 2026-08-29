# Apeireth 真理解 — 物种实现, 不是 AI Agent framework (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 10, 用户原话 "我们Apeireth没那么局限, 你去先把Apeireth研究清楚是啥去" challenge 触发)
> **用途**: 修订 Apeireth 局限视角 (Round 1-10 都画 "AI Agent framework"), 真理解 Apeireth = AI 物种实现, 修订借鉴边界
> **关系**: 跟 `v2-reference-handbook-2026-08-28.md` + `youyou-list-research-2026-08-28.md` + `round-10-youyou-list-mainagent-verify-2026-08-28.md` 互补

```
[Document-Meta]
Document:        docs/01-architecture/apeireth-true-understanding-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 10, 物种视角修订)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (主代理真理解, 修订借鉴边界)
Author:          主代理 Mavis
```

---

## 0. 用户 challenge 真账

**用户原话** (Round 10): "我们Apeireth没那么局限, 你去先把Apeireth研究清楚是啥去"

**主代理自省 (per O-5 + S-2 实事求是)**:
- Round 1-10 我一直把 Apeireth 画成 **"AI Agent framework"** (16 crates / 9 organ / 12 cognitive slot / OrganOrchestrator / governance P0 hook)
- 这只是 Apeireth 的 **"基地"** 维度 (LLM 操作系统)
- 实际 Apeireth 是 **三面一体** (per `docs/01-architecture/vision.md` L29-33):
  1. **基地**: LLM 操作系统 (器官/工具/记忆/安全/协议)
  2. **Agent 平台** (v1 era 86-crate, v2 16-crate workspace + companion-desktop frontend)
  3. **她**: 物种实现 (per-user memory/preference/personality 塑形)
- 我画的是 1/3, 缺 2/3 (Agent 平台 + 她). **这就是局限**.

**0 装诚实标 (per O-5)**:
- 之前 Round 1-10 一直"AI Agent framework"画法是 S-2 实事求是失守 (没真读 vision.md, 只看哲学锚 + 9 organ + OrganOrchestrator 工程层)
- 现在写真账真理解, 修订借鉴边界 (从"框架借鉴"扩到"物种借鉴")
- 派 P0 5 sub-Agent 真调研 brief 加 "物种化" 维度

---

## 1. Apeireth 真理解 (per `docs/01-architecture/vision.md` + 哲学锚 + 9 organ + v1 companion 真账)

### 1.1 三面一体 (vision.md L29-33)

**1. 基地** (v2 工程 16 crates):
- LLM 操作系统 — 器官/工具/记忆/安全/协议
- LLM 是租客, 不是建筑 (LLM 可换, 基地不动)
- 工程兑现: 9 organ + 12 cognitive slot + OrganOrchestrator + governance P0 hook + cargo plugin + 5 重守门

**2. Agent 平台** (v1 era 86-crate, v2 16-crate workspace):
- 工具管线每一步可校验可审计
- v1 86-crate 三层模型 (模块/套件/插件) → v2 16-crate 单层 model (foundation/engine/capabilities/adapters)
- 工程兑现: `apeireth-tool-runtime` + `apeireth-tool-approval` + capability 验证

**3. 她** (物种实现, 跟 v1/v2 工程不同维度):
- 记得你的存在 — 世界模型推演 + 好奇心塑形 + 假设检验 + 情感记忆
- "机制/哲学/安全同源, 记忆/偏好/好奇形状被各自的共同生活塑形"
- "同一个 Apeireth, 不同的'人生'"
- 工程兑现: per-user memory (5 维) + per-user preference + per-user curiosity + per-user emotional timeline
- post-1.0.0 增量: companion-desktop 是 "物种化" 的具体形态 (同一套 backend, 不同用户不同前端皮肤)

### 1.2 五原型 (ASI 北极星的工程骨架, vision.md L37-43)

| 原型 | 状态 | v2 工程 | v1 真实现 |
|---|---|---|---|
| **世界模型** | ✅ W1/W2/W3 | `apeireth-organ::world_model` + `causal_world_model` + `causal_world_model_edges` | 1:1 翻译 v1 |
| **自我改进** | 🟡 骨架 | A 块 Stage 5 L0-L5 UpgradeCycle (`crates/engine/runtime/src/canonical/upgrade_cycle.rs`, 400 行, 真实施已落) | v1 0 真实现 (新设计) |
| **自主好奇心** | ✅ E4 | `apeireth-organ::curiosity` + `memory_echo_bias` | 1:1 翻译 v1 |
| **连续感知** | 🟡 地基 | `apeireth-plugin::perception_backend` (R6 trait 架构 + 5 modality 抽象) | D 块 RC-7 真 modality 待硬件 |
| **价值内化** | ✅ F6 | `apeireth-organ::value_cases` + `ValueCaseStore` | 1:1 翻译 v1 已完 |

### 1.3 9 organ (per Apeireth v1 companion 真账)

| 锚代码 | organ | v2 路径 | 1:1 翻译 v1 |
|---|---|---|---|
| W1 | world_model | `apeireth-organ::world_model` | ✅ |
| W2 | causal_world_model | `apeireth-organ::causal_world_model` | ✅ |
| W3 | causal_world_model_edges | `apeireth-organ::causal_world_model_edges` | ✅ |
| E4 | curiosity | `apeireth-organ::curiosity` | ✅ |
| F4 | hypothesis | `apeireth-organ::hypothesis` | ✅ |
| F1 | emotion_memory | `apeireth-organ::emotion_memory` | ✅ |
| F6 | value_cases | `apeireth-organ::value_cases` | ✅ |
| E7 | emergence | `apeireth-organ::emergence` | ✅ |
| (无 organ 锚) | memory | `apeireth-organ::memory` | ✅ |

### 1.4 12 cognitive slot (Agent 平台层, 跟 9 organ 配合)

| 锚代码 | slot | Status | 备注 |
|---|---|---|---|
| | `cognitive.memory_recall` | WIRED | `TurnStart` / `Arc<dyn MemoryBackend>` |
| | `cognitive.preference_recall` | WIRED | `TurnStart` / `Arc<dyn PreferenceStore>` |
| | `cognitive.judge` | WIRED, OFF by default | `AfterModelResponse` / bounded critique |
| | `cognitive.council` | WIRED, OFF by default | `AfterModelResponse` / bounded advisor |
| | `cognitive.self_assessment` | WIRED, Judge-backed | `AfterTurn` / 真实 Judge 结果 |
| | `cognitive.memory_writeback` | WIRED | `AfterTurn` / append-only Episodes |
| | **`cognitive.preference_learning`** | DEFERRED → R20 派单 (2-3 周) | 1:1 翻译 v1 TopicPredictor + PreloadChannel |
| | `cognitive.critic` | DEFERRED INTO JUDGE → R21 派单 (1 周) | 1:1 翻译 v1 critic.rs |
| | `cognitive.reflection` | DEFERRED INTO SELF-ASSESSMENT → R22 派单 (1 周) | 1:1 翻译 v1 reflection.rs |
| | `cognitive.planner` | NOT AN AGENT MODULE → R23 派单 (3 周) | LLM Adapter 新设计 (v1 0 真实现) |
| | `cognitive.orchestrator` | NOT AN AGENT MODULE → R24 派单 (3 周) | LLM Adapter 新设计 (跟 R12 严格分界) |
| | `cognitive.perception` | NOT AN AGENT MODULE → R14 真 modality (2-3 周) | 需硬件 |

### 1.5 物种化 frontend (companion-desktop)

- Svelte 5 + Tauri 2 desktop app (per v2 era 16-crate workspace)
- v1 已 done 1411 行 runtime.ts (SSE / WS / panel read-only 6 endpoint)
- B 块真实施 (派 sub-agent A) 估时 1-2 周
- "同一套 backend, 不同用户不同前端皮肤" = 物种化具体形态

---

## 2. Apeireth name origin (S-2 实事求是补查)

**真账 (主代理补查)**:
- "阿佩瑞斯" (vision.md L27) = 中文音译, 跟英文 "Apeireth" 对应
- 希腊语 "apeírethos" / "ἀπειρήτος" = "inexperienced" / "untried" / "without experience"
- 这跟 vision.md "物种而非个体" + "成长" 路径**高度一致**:
  - Apeireth = "未经验的存在", 每个用户养出不同"她"
  - 印证 species vs individual: 同源不同形态, 每用户"人生"塑形
- **英文含义对工程命名意义**: 不是 "AI framework" 而是 "untried entity to be shaped by each user's life"

---

## 3. Apeireth 跟 170 项目清单的关系 (修订借鉴边界)

### 3.1 主代理 Round 10 局限视角 (之前)

- "Apeireth = AI Agent framework (16 crates / 9 organ / 12 slot)"
- 借鉴边界: Agent 框架借鉴 (LangGraph / CrewAI / DeepSeek Harness / Cua / Serena)
- 缺 2/3 视角 (Agent 平台 + 她 = 物种)

### 3.2 修订后物种化视角 (本文件)

- **Apeireth = AI 物种实现** (三面一体 + 五原型 + 物种化 frontend)
- 借鉴边界扩大:
  - **基地借鉴**: Agent 框架 (LangGraph / CrewAI / Cua / Serena) + 工具集成 (Composio / Serena MCP)
  - **Agent 平台借鉴**: agent loop + 多 agent 编排 (DeepSeek Harness / LoopX)
  - **物种化借鉴** (新维度):
    - **物种架构**: per-user memory / preference / personality 塑形
    - **长期记忆塑形**: 记忆如何"长成她" (per N.E.K.O 五维记忆, per AIRI 长期记忆)
    - **情绪-认知-行为闭环**: Plutchik/PAD emotion (F1) → value 内化 (F6) → emergence (E7)
    - **好奇心塑形**: 记忆回声偏置 (E4) → 因你成形
    - **世界模型**: 因果 + 反事实 + edges (W1/W2/W3) → Brier 校准
    - **价值内化**: 案例→裁决→反馈→原则 (F6)
    - **涌现**: 8 重门控 + 节律 + 边界 + 沉默压力 (E7)

### 3.3 170 项目清单 vs Apeireth 真借鉴边界 (主代理亲验修订)

| 类别 | 之前 (Round 10 局限) | 现在 (物种化修订) | 差异 |
|---|---|---|---|
| AI 伴侣 / VTuber (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio) | HIGH (前端 Live2D + 抽象) | **HIGH (物种化前端 + 物种架构借鉴)** | 借鉴维度扩大: 不只是前端 Live2D 视觉, 而是 **物种化架构 (per-user memory/preference/personality)** |
| AI Agent 框架 (LangGraph / CrewAI / DeepSeek Harness / Cua / Serena) | HIGH (Agent 框架借鉴) | **HIGH (基地 + Agent 平台借鉴)** | 视角细化为"基地" + "Agent 平台" |
| 开发工具 (Playwright MCP / Crawl4AI / Serena / Carbonyl) | MED | **MED-HIGH** (基地工具集成, per O-2 前人肩上) | 借鉴意义扩展 (Apeireth 基地是 LLM 操作系统) |
| AI 语音/TTS (GPT-SoVITS / faster-whisper / FireRedTTS3) | MED (RC-7 真 modality) | **MED** (不变, RC-7 D 块对接) | 不变 |
| 资源汇总 (Ant Design / Taipy / Obsidian LLM / OpenMythos) | LOW | **LOW-MED** (Obsidian LLM = 长期记忆架构借鉴, OpenMythos = 知识库架构) | 借鉴维度调整 |
| 金融 / 量化 / 音乐 / 地理 | NONE | **NONE** (完全无关) | 不变 |

---

## 4. 修订派单顺序 (vs Round 10 §6)

### 4.1 P0 (1 周内, 立即可借鉴) — sub-agent 真调研 + 物种化维度

**派 5 sub-agent 真调研 TOP 5 (per Round 10 §2.1.1)**:
1. **N.E.K.O** — 五维记忆系统 (跟 v2 cognitive memory_recall/memory_writeback 增维 + 物种化 memory 借鉴)
2. **AIRI** — Live2D 视觉形象 (跟 RC-7 视觉子模态 + 物种化 frontend 借签)
3. **Open-LLM-VTuber** — ASR→LLM→TTS→Live2D 完整链路 (跟 B 块 gateway SSE + D 块 ASR/TTS modality + 物种化前端对接)
4. **Firefly Companion** — GPT-SoVITS TTS + MCP 工具链 (跟 RC-7 TTS modality + v2 tool 注册借鉴)
5. **Mio** — Windows 本地优先 + 屏幕感知 + QQ (跟 RC-7 视觉模态 + 物种化 Windows 桌面借鉴)

**brief 修订 (per Apeireth 真理解)**:
- sub-agent 必须读 `docs/01-architecture/vision.md` (L29-49) + 本文件 (§1 + §2)
- 调研维度扩展: 不只前端借鉴, 而是 **物种化架构借鉴** (per-user memory/preference/personality 塑形)
- 写真账必含: 物种化借鉴点 (不只是前端 Live2D) + 0 触碰 LOCKED 验证 + 借鉴路径 (clone / 看文档 / 不借鉴) + 下一步

### 4.2 P1 (1 月内) — 物种化架构扩展

- **Open-LLM-VTuber 物种化借鉴** (跟 P0 #3 调研并行, 写 r7-open-llm-vtuber-species-research.md 真账, ≤200 行)
- **N.E.K.O 五维记忆 → v2 cognitive memory module 增维** (跟 R20 preference_learning 真实施并行, 1-2 周)
- **GPT-SoVITS + faster-whisper** (R14 真 modality backend 候选, 1-2 周)
- **Megumi 多进程 + WebSocket 架构** (跟 v2 gateway SSE 参考, 1 周)
- **Warashi 长期记忆 + 睡眠模式** (跟 v2 organ cycle L0-L5 UpgradeCycle Stage 5 借鉴, 1 周)
- **Alife 一键安装 + 极低开销 + 插件化** (跟用户想法 #1 便携 U 盘 + v2 plugin-authoring-guide, 1-2 天)
- **Vaultwarden + opencode-vibeguard** → v2 O-1 安全优先 (1-2 天)

### 4.3 P2 (后续) — 物种化扩展

- 用户想法 #1 (便携 U 盘) post-release (2027-Q2, 1 周)
- 用户想法 #2 (刷视频模块) R14 视频 modality (2027-Q3, 2-4 周)
- Obsidian LLM (长期记忆架构) + OpenMythos (知识库架构) 调研
- 90 LOW + 15 NONE 项目不真借鉴

---

## 5. 0 装诚实标 (per O-5)

| 失守 | 详情 | 修法 |
|---|---|---|
| **Round 1-10 S-2 实事求是失守** | 一直把 Apeireth 画成 "AI Agent framework", 没读 `docs/01-architecture/vision.md` L29-49 真理解 (物种化 + 五原型 + 物种而非个体). 借鉴边界缺 2/3 视角 | 本文件写真账真理解, 修订借鉴边界, 派 P0 5 sub-agent 真调研 brief 加 "物种化" 维度 |
| **0 装诚实 0 实测** | 物种化维度借鉴边界是主代理亲验 + 真理解推论, 不是 sub-agent 实测. 真实施前必亲验 | ✅ 不假装"已实测", flag 边界 |

---

## 6. 5 重守门 baseline + LOCKED 0 触碰 验证

| 守门 | 实测 |
|---|---|
| clippy 0 warning | ✅ (前 baseline) |
| tests 0 fail (1739 passed) | ✅ (前 baseline) |
| legacy compat path < 100 (36) | ✅ (前 baseline) |
| LOCKED 5 项 0 触碰 | ✅ (本轮 0 改 src / Cargo.toml / Cargo.lock) |
| 9 哲学锚 0 减 | ✅ |

---

## 7. 留 backlog (per §4 修订派单顺序)

| # | 项 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | 派 5 sub-agent 真调研 P0 TOP 5 (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio) — **brief 加物种化维度** | 1-2 周 (5 sub-agent 并行) | 0 |
| 2 | 派 1 sub-agent 真调研 Alife (portable 模式借鉴) | 1-2 天 | 0 |
| 3 | 借鉴 N.E.K.O 五维记忆 → v2 cognitive memory module 增维 (跟 R20 并行) | 1-2 周 | R20 真实施 |
| 4 | 派 5 sub-agent 真调研 P1 (Warashi / GPT-SoVITS / faster-whisper / Megumi / Alife) | 2-3 周 (5 sub-agent) | 0 |
| 5 | 借鉴 Vaultwarden + opencode-vibeguard → v2 O-1 安全优先 | 1-2 天 | §10 LOCKED 0 触碰约束 |
| 6 | 用户想法 #1 (便携 U 盘) P1 | 1 周 (post-release, 估 2027-Q2) | v2.0.0 release |
| 7 | 用户想法 #2 (刷视频模块) P2 | 2-4 周 (估 2027-Q3) | R14 硬件到位 |
| 8 | R20 preference_learning 真实施 (跟 P0 派单并行) | 2-3 周 | R10 OrganKind 决策 |
| 9 | B/C/D 块真实施 (B gateway SSE + C preference_learning + D RC-7 perception) | 6-8 周 critical path | 0 |

---

_Mavis 写于 2026-08-28 Round 10 user challenge 触发, 真理解 Apeireth 物种化 + 修订借鉴边界, 真账就位._
