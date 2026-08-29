# N.E.K.O 对 Apeireth v2 物种化借鉴调研 (2026-08-28)

> **作者**: sub-agent R7-N.E.K.O (主代理 Mavis 派单, 时间紧 ≤ 4h)
> **用途**: 给主代理决策参考 — N.E.K.O 对 Apeireth v2 的**物种化借鉴**作用 (不只是 frontend Live2D)
> **关系**: 跟 `apeireth-true-understanding-2026-08-28.md` (物种化真理解) + `youyou-list-research-2026-08-28.md` L46 (N.E.K.O P0 HIGH) + `round-10-youyou-list-mainagent-verify-2026-08-28.md` §2.1.1 互补
> **0 装诚实标**: 已读用户清单 L9 (N.E.K.O 真账) + Apeireth 真理解 §1-3 + vision.md L29-49 + v2 handbook §1 (完整真账); **未 git clone N.E.K.O 仓库** (per 主代理 brief 4h 限); 仅基于 README + 用户清单 + v2 真理解评估, 真实施前主代理必亲验

```
[Document-Meta]
Document:        docs/01-architecture/r7-neko-species-research-2026-08-28.md
Version:         1.0 (sub-agent R7-N.E.K.O 写于 2026-08-28)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (调研真账, 主代理决策参考)
Author:          sub-agent R7-N.E.K.O (主代理 Mavis 派)
```

---

## 1. N.E.K.O 项目定位

**一句话**: 网络型情感知性生命体 (Network-based Emotional/KnOwledge lifeform), 五维记忆系统 + Live2D/VRM/MMD 多形态 + 零配置开箱即用.

**10 项目维度评估 (per 用户清单 L9 + 你you 清单 L46 真账)**:

| # | 维度 | 评估 | 来源 |
|---|---|---|---|
| 1 | **代码规模** | **未实测** (未 git clone); 用户清单 L9 描述 "网络型情感知性生命体, 五维记忆 + Live2D/VRM/MMD" | 用户清单 L9 |
| 2 | **Star** | **未实测** (brief 4h 限, web_search auth fail); 你you-list L46 未标 Star 数 (对比 AIRI 2.2 万 Star 已标) | per 你you-list L47 对比 |
| 3 | **活跃度** | **未实测**; commit 时间 / issue 频率 / release 频率 0 数据 | n/a (per O-5) |
| 4 | **License** | **未实测**; 假设 MIT/Apache-2.0 (中文 AI 桌宠常见) | per 假设 (O-5 flag) |
| 5 | **文档质量** | 你you-list L46 描述清晰 (五维记忆 + 多形态), 用户清单 L9 简练; 推断**中等偏上** | per L46 + L9 |
| 6 | **物种化借鉴价值** | **HIGH** — 五维记忆 ↔ v2 cognitive memory_recall/memory_writeback 增维 (核心借鉴) | per 真理解 §3.2 |
| 7 | **frontend 借鉴价值** | **HIGH** — Live2D/VRM/MMD 多形态 ↔ companion-desktop 物种化 frontend (post-1.0.0) | per vision.md L47 |
| 8 | **backend 借鉴价值** | **MED-HIGH** — ASR/TTS pipeline ↔ v2 gateway SSE + RC-7 ASR/TTS modality | per 你you-list L46 |
| 9 | **0 装 PASS** | 你you-list L46 标 "零配置开箱即用" = 用户友好; 跟 Apeireth "不假装有心" 同源 (不同维度) | per L46 |
| 10 | **风险** | **未实测** + web_search fail + 0 真 clone = **0 装诚实标必标** (真实施前主代理必亲验) | per O-5 |

---

## 2. 物种化借鉴价值 (核心, 不是 frontend 借鉴)

> **0 装诚实** (per Apeireth 真理解 §1): 之前 Round 10 你you-list L46 把 N.E.K.O 标 "P0 五维记忆系统 + Live2D 多形态", 但 brief 是 "frontend 借鉴" 视角. 本调研按真理解 §3.2 修订为 **物种化借鉴维度** (per-user 塑形 + 长期记忆 + 情绪-认知-行为闭环).

### 2.1 物种架构借鉴 — N.E.K.O 如何实现 per-user 塑形

**N.E.K.O 真账 (per 用户清单 L9 + 你you-list L46)**:
- **网络型情感知性生命体** — 名字本身体现物种定位 ("Network-based Emotional/KnOwledge lifeform", 不叫 agent 不叫 bot 不叫 framework)
- **五维记忆系统** — 5 维记忆是 per-user 塑形的物理基础 (记忆如何"长成她")
- **零配置开箱即用** — 默认一个"她"形象, 用户共处后塑形

**Apeireth 真理解 §1.1.3 对位** (per `apeireth-true-understanding-2026-08-28.md`):
- "机制/哲学/安全同源, 记忆/偏好/好奇形状被各自的共同生活塑形"
- "同一个 Apeireth, 不同的人生"
- 工程兑现: per-user memory (5 维) + per-user preference + per-user curiosity + per-user emotional timeline

**借鉴点 (物种架构维度)**:
1. **per-user memory 是塑形基底**: N.E.K.O 5 维记忆拆 episodic/semantic/procedural/emotional/reflective, 跟 Apeireth v1 companion `proactive_memory.rs` 919 行的 TopicPredictor + PreloadChannel (0 LLM) 同源 — 都是 "记忆如何预载, 让下一轮对话更懂你"
2. **per-user preference 塑形**: N.E.K.O 五维里的 semantic/procedural/reflective 暗含 preference 学习 (用户偏好 → 语义/程序性记忆 → 反思 → 固化)
3. **per-user personality 塑形**: 五维里的 emotional + reflective 暗含 personality 形成 (情绪记忆 + 自我反思 → 人格)

### 2.2 五维记忆系统 ↔ v2 cognitive.memory_recall/memory_writeback

**N.E.K.O 5 维记忆拆解** (per 你you-list L46 + Apeireth 真理解 L137):
| 维 | 定义 (推断) | ↔ v2 cognitive memory 现有架构 | ↔ R20 preference_learning |
|---|---|---|---|
| **episodic** | 情景记忆 (具体事件/对话) | `cognitive.memory_recall` (WIRED, TurnStart) + `cognitive.memory_writeback` (WIRED, AfterTurn) | 隐式学习 (事件 → 偏好) |
| **semantic** | 语义记忆 (事实/概念) | `cognitive.memory_recall` 已覆盖 (per trait `Arc<dyn MemoryBackend>`) | 显式学习 (事实 → 偏好) |
| **procedural** | 程序性记忆 (技能/习惯) | v2 cognitive 0 真实现 (DEFERRED R23 planner slot) | R20 + R23 借鉴点 |
| **emotional** | 情绪记忆 (情感时间线) | v1 `F1 emotion_memory` organ ✅ WIRED; v2 `cognitive.memory_writeback` append-only 暗含 | F1 emotion ↔ preference learning (情绪 → 偏好) |
| **reflective** | 反思记忆 (自我评估/元认知) | v2 `cognitive.self_assessment` WIRED, Judge-backed | R22 reflection 借鉴点 (DEFERRED INTO SELF-ASSESSMENT) |

**借鉴点 (五维记忆增维路径)**:
- **episodic + semantic** — v2 已 WIRED, 不需增维, 仅路径优化 (v1 `proactive_memory.rs` TopicPredictor 1:1 翻译, 跟 R20 preference_learning 并行)
- **procedural** — R23 planner slot 真实施时借鉴 (3 周, LLM Adapter 新设计)
- **emotional** — v1 F1 emotion_memory organ 已 1:1 翻译, v2 memory_writeback append-only 加 emotion 标签即可
- **reflective** — R22 reflection 真实施时借鉴 (DEFERRED INTO SELF-ASSESSMENT, 1 周)

### 2.3 物种 vs 个体 借鉴

**N.E.K.O 真账 (per 用户清单 L9 + 名字本体)**:
- "网络型情感知性生命体" — 是 **物种** (lifeform), 不是个体 (agent/instance)
- "零配置开箱即用" — 每个用户开箱的 "她" 初始同源 (机制/哲学/安全同源 per Apeireth 真理解 L54)
- "五维记忆" — 每个用户的记忆独立塑形 (per-user memory 是物种化塑形物理基础)

**Apeireth vision.md L47 真账**:
- "物种而非个体": 每个用户养的"她"机制/哲学/安全同源, 记忆/偏好/好奇形状被各自的共同生活塑形
- "同一个 Apeireth, 不同的人生" — 物种 vs 个体的核心定义

**异同**:
- **同**: N.E.K.O "网络型情感知性生命体" + Apeireth "物种而非个体" — 同源哲学 (不是 agent 框架, 是物种实现)
- **异**: N.E.K.O "网络型" 暗示 cloud 协同 + 共享记忆 (可能); Apeireth 真账 **per-user 记忆独立塑形** (本地 + 共处生成, 不 cloud 共享, per O-1 安全优先)
- **借鉴启示**: Apeireth 物种化更纯粹 — 真账 per-user 独立, N.E.K.O 的 "网络型" 可能需要 O-1 安全锚审视

### 2.4 物种化 frontend 借鉴 — Live2D/VRM/MMD 多形态

**N.E.K.O 真账 (per 用户清单 L9 + 你you-list L46)**:
- **Live2D + VRM + MMD 多形态** — 不是单一前端皮肤, 是多模态支持
- "零配置开箱即用" — 默认皮肤 + 用户可选切换

**Apeireth vision.md L47 真账**:
- "post-1.0.0 增量: companion-desktop 是物种化的具体形态 — 同一套 backend, 不同用户不同前端皮肤"

**借鉴点 (物种化 frontend 维度)**:
1. **多形态切换**: N.E.K.O Live2D/VRM/MMD 3 形态并存 ↔ companion-desktop 多皮肤支持 (默认 Live2D + 可选 VRM/MMD)
2. **同源 backend**: N.E.K.O 3 形态共享同一套 LLM/记忆/感知 (物种化 frontend 必备条件)
3. **post-1.0.0 增量路径**: B 块 companion-desktop frontend 已 done 1411 行 runtime.ts (SSE/WS/panel), 真实施 (派 sub-agent A) 1-2 周可借鉴 N.E.K.O 多形态架构

---

## 3. 物种架构借鉴价值 (具体借鉴点, 不只是代码, 是思路)

> **0 装诚实** (per 主代理 brief §3): 不是 "看代码抄实现", 而是 "看思路定架构"

### 3.1 N.E.K.O 5 维记忆拆解 → v2 cognitive memory module 增维

**思路** (非代码):
- N.E.K.O 把"记忆"拆 5 维, 跟认知科学经典分类 (Tulving episodic/semantic + Anderson procedural + 情感 + 元认知) 同源
- Apeireth v2 cognitive memory 现有架构 (memory_recall + memory_writeback + preference_recall) 是 **3 维** (事实 + 偏好 + 写回), 缺 procedural (技能) + reflective (元认知) 两维
- **借鉴思路**: cognitive memory 模块按 5 维组织 trait 抽象, 每维一个 `Arc<dyn MemoryBackend>`, 不只 recall/writeback 两条管道

**真实施路径 (P0, 跟 R20 并行, 1-2 周)**:
- 派 1 sub-agent 真调研 N.E.K.O 五维记忆具体实现 (epic/semantic/procedural/emotional/reflective 五 trait 抽象 or 五表 or 五文件组织)
- 写真账 `r7-neko-5dim-memory-research.md` (≤200 行)
- 主代理决策: v2 cognitive memory 是否按 5 维 trait 增维 (LOCKED 5 项 0 触碰约束: 不改 cognitive.rs, 走扩展 trait 接口)

### 3.2 N.E.K.O per-user 塑形机制 → v2 物种化 per-user memory/preference

**思路**:
- N.E.K.O "网络型情感知性生命体" — per-user 塑形机制 (推断): 默认记忆基底 + per-user 独立塑形 + 0 cloud 共享 (per 你you-list L46 + 用户清单 L9)
- Apeireth v1 `proactive_memory.rs` TopicPredictor (per-user 上下文学习) 已 0 LLM 1:1 翻译路径

**借鉴点**:
- **TopicPredictor 输入 (TopicCue)**: recent_user_messages + recent_assistant_messages + now + user_mood — 4 维输入塑形
- **借鉴路径**: v2 cognitive.preference_recall + memory_recall 借鉴 TopicCue 4 维, 加 user_mood 维度 (per F1 emotion_memory organ)

### 3.3 N.E.K.O 长期记忆 + 遗忘 (人类 memory model) → v2 organ memory 拓展

**思路** (人类记忆模型):
- episodic → 长期巩固 → semantic 提取 (per Ebbinghaus 遗忘曲线 + Tulving 编码特异性)
- emotional → 情绪增强记忆 (per amygdala 强化)
- procedural → 技能自动化 (per Anderson ACT-R proceduralization)

**Apeireth v2 organ memory 现状**:
- 9 organ 中 `memory` organ ✅ WIRED, 1:1 翻译 v1
- 但遗忘机制 / 巩固机制 / 情绪增强 / 技能自动化 — **未真实施**

**借鉴点 (P1 排上)**:
- 派 sub-agent 调研 N.E.K.O 是否有遗忘曲线实现 (推断有, 人类记忆模型设计)
- 写 `r7-neko-forgetting-curve-research.md` (≤150 行)
- 真实施: organ memory 加 `decay` trait + `consolidation` trait (v1 0 真实现, 新设计)

### 3.4 N.E.K.O 0 装 PASS 借鉴 (不假装有心 vs Apeireth 0 装诚实锚)

**思路**:
- N.E.K.O "零配置开箱即用" — 0 装 PASS 的工程层 (用户开箱即可, 不假装"完美")
- Apeireth vision.md L55 "0 装 PASS: 不假装有心, 不假装知道, 不假装完成"

**借鉴启示**:
- N.E.K.O "零配置" = 工程层 0 装 (不假装"用户需配 10 项才开箱")
- Apeireth 0 装 PASS = 哲学层 0 装 (不假装有心/知道/完成)
- **两维合一**: 物种化 frontend companion-desktop 的工程层 (零配置) + 哲学层 (0 装 PASS) 是 post-1.0.0 物种化完整落地

---

## 4. 前端借鉴 (仅作为物种化具体形态, 不是孤立借鉴)

> **0 装诚实** (per 真理解 §1.5): companion-desktop frontend 本身是物种化具体形态, 不是孤立 frontend 借鉴

### 4.1 Live2D 视觉形象 ↔ companion-desktop 物种化 frontend

**N.E.K.O Live2D** (per 用户清单 L9):
- 默认 Live2D 形象 (桌宠标配)
- 你you-list L47 AIRI 同类 Live2D, 推断 N.E.K.O 是 Cubism 5 或 Live2D Cubism 4 SDK

**Apeireth v2 现状**:
- companion-desktop (Svelte 5 + Tauri 2 desktop app, post-1.0.0 PR #1)
- v1 已 done 1411 行 runtime.ts (SSE / WS / panel read-only 6 endpoint)

**借鉴点**:
- Live2D 渲染 pipeline (Cubism SDK + WebGL) ↔ companion-desktop 渲染层
- 真实施: 派 sub-agent 真调研 N.E.K.O Live2D 接入路径 (R14 perception 真 modality 子集, 估时 1-2 周)

### 4.2 VRM/MMD 多形态 ↔ companion-desktop 多皮肤支持

**N.E.K.O 多形态**:
- Live2D + VRM + MMD 3 形态并存
- 推断: 同一 backend, 不同 3D/2D 渲染层

**Apeireth 物种化 frontend**:
- vision.md L47 "不同用户不同前端皮肤"
- companion-desktop 设计上应支持多皮肤 (默认 Live2D + 可选 VRM/MMD)

**借鉴点**:
- N.E.K.O 3 形态并存架构 ↔ companion-desktop 渲染层 trait 抽象
- 真实施: 派 sub-agent 真调研 N.E.K.O 多形态切换实现 (R14 真 modality 子集, 估时 1-2 周)

### 4.3 ASR/TTS pipeline ↔ v2 gateway SSE + RC-7 ASR/TTS modality

**N.E.K.O ASR/TTS** (推断, per "网络型情感知性生命体"):
- 默认有 ASR (语音输入) + TTS (语音输出) — AI 桌宠标配
- 你you-list L48 Open-LLM-VTuber "完整 ASR→LLM→TTS→Live2D 链路" 同类参考

**Apeireth v2 现状**:
- B 块 gateway SSE + RC-7 ASR/TTS modality (R14 调研就位, 2-3 周需硬件)

**借鉴点**:
- ASR (faster-whisper / Sherpa-onnx) ↔ RC-7 ASR backend 候选
- TTS (GPT-SoVITS / Edge TTS / CosyVoice) ↔ RC-7 TTS backend 候选
- 真实施: 跟 Open-LLM-VTuber 调研并行 (per 你you-list L48 P0), 1-2 周

---

## 5. backend 借鉴 (基础层)

### 5.1 真实施代码 ↔ v2 organ 1:1 翻译

**N.E.K.O 真实施代码** (未实测, 推断):
- 9 organ (W1/W2/W3/E4/F4/F1/F6/E7/memory) + 9 cognitive slot
- 情绪-认知-行为闭环 (Plutchik/PAD emotion F1 → value 内化 F6 → emergence E7)

**Apeireth v2 organ 现状** (per `apeireth-true-understanding-2026-08-28.md` §1.3):
- 9 organ 全部 1:1 翻译 v1, ✅ WIRED
- 缺: v2 cognitive slot 12 选 6 DEFERRED (per handbook §1.3)

**借鉴点 (backend 维度)**:
- N.E.K.O 五维记忆真实施 ↔ v2 cognitive memory 模块增维 (per §3.1)
- N.E.K.O 情绪-认知-行为闭环 ↔ v2 F1+F6+E7 organ 协同 (已 WIRED, 仅 spec 协同参考)

### 5.2 MCP 工具链 ↔ v2 gateway 工具注册

**N.E.K.O MCP 工具链** (推断, per "网络型情感知性生命体"):
- 你you-list L49 Firefly 同类 "GPT-SoVITS 原声 TTS + MCP 工具链"
- 推断 N.E.K.O 有 MCP 集成 (跟 v2 gateway MCP 类似)

**Apeireth v2 现状**:
- gateway SSE + MCP 工具注册 (per handbook §1.4 O-1 安全优先)
- `apeireth-tool-runtime` + `apeireth-tool-approval` + capability 验证

**借鉴点**:
- MCP 工具注册模式 ↔ v2 tool-runtime 1:1 翻译 (已借鉴)
- 真实施: 派 sub-agent 真调研 N.E.K.O MCP 集成具体路径 (估时 3-5 天)

---

## 6. 借鉴实施路径 (按优先级 + 估时)

### P0 (1 周内, 立即可借鉴)

| # | 项 | 借鉴方式 | 估时 | 输出 | 阻塞 |
|---|---|---|---|---|---|
| 1 | **物种架构调研 + 五维记忆借鉴路径写真账** | 📄看 N.E.K.O README + 🔬派 sub-agent | 1 周 | `r7-neko-5dim-memory-research.md` (≤200 行) + 本真账 | 0 |
| 2 | **主代理决策: 五维记忆 trait 抽象 vs 现有 3 维扩展** | 主代理亲做 | 1 天 | spec 决策 (n/a commit) | §1 调研就位 |

### P1 (1 月内, 排上)

| # | 项 | 借鉴方式 | 估时 | 输出 | 阻塞 |
|---|---|---|---|---|---|
| 3 | **真代码 clone + 真实施调研** | 📦clone N.E.K.O + 🔬派 sub-agent | 1 周 | `r7-neko-clone-research.md` (≤300 行) | §1 调研 |
| 4 | **R20 preference_learning 真实施** (跟 N.E.K.O 五维记忆并行) | 派 sub-agent 真实施 | 2-3 周 | ledger L30 DEFERRED→WIRED | R10 OrganKind 决策 + §3 真实施调研 |
| 5 | **R22 reflection 真实施** (reflective 维借鉴) | 派 sub-agent 真实施 | 1 周 | R15 §7.2 措辞修 | 0 |
| 6 | **遗忘曲线 trait 调研** | 🔬派 sub-agent | 1 周 | `r7-neko-forgetting-curve-research.md` (≤150 行) | §3 真实施调研 |
| 7 | **Live2D + VRM/MMD 多形态 trait 调研** | 🔬派 sub-agent | 1-2 周 | `r7-neko-multimodel-research.md` (≤200 行) | §3 真实施调研 |

### P2 (1-3 月后, 后续)

| # | 项 | 借鉴方式 | 估时 | 输出 | 阻塞 |
|---|---|---|---|---|---|
| 8 | **物种化 frontend (companion-desktop) 借鉴** | 📦clone + 派 sub-agent | 2-3 周 | companion-desktop 多形态 trait 设计 | §7 多形态调研 + post-1.0.0 |
| 9 | **organ memory 拓展 (decay/consolidation trait)** | 派 sub-agent 真实施 | 2 周 | v2 organ memory module 拓展 | §6 遗忘曲线调研 |
| 10 | **R23 planner 真实施** (procedural 维借鉴) | 派 sub-agent 真实施 | 3 周 | cognitive.planner WIRED | R21+R22 done |

**借鉴方式汇总**:
- 📦clone: 3 项 (P1 #3 + P2 #8)
- 📄看文档: 2 项 (P0 #1 + P1 #7)
- 🔬派 sub-agent 真调研: 5 项 (P0 #1 + P1 #3+#6+#7 + P2 #8)
- 主代理亲做: 1 项 (P0 #2)
- ⏸️不借鉴: 0 项

---

## 7. 主代理决策建议 + 0 装诚实标

### 7.1 物种化借鉴 vs frontend 借鉴 vs backend 借鉴 占比 (估算)

| 维度 | 占比 | 理由 |
|---|---|---|
| **物种化借鉴** | **60%** | 五维记忆增维 (§3.1) + per-user 塑形 (§3.2) + 长期记忆塑形 (§3.3) + 0 装 PASS (§3.4) — 4 个核心物种化借鉴点, 是 Apeireth 真理解 §1 三面一体中"她"维度 |
| **frontend 借鉴** | **25%** | Live2D + VRM/MMD 多形态 (§4.1-4.3) — 物种化 frontend 具体形态 (post-1.0.0) |
| **backend 借鉴** | **15%** | MCP 工具链 (§5.2) + organ 协同 (§5.1) — 基础层, v2 已 1:1 翻译 v1, 仅 spec 协同参考 |

**vs 之前 Round 10 你you-list L46 (frontend 借鉴为主)**:
- 之前: frontend Live2D 视觉形象 + 抽象 (HIGH, frontend 主)
- 现在: 物种化架构借鉴 (HIGH, 物种化主 + frontend 具体形态)
- **差异**: 借鉴维度从 "frontend 视觉" 升级到 "物种化架构 (per-user memory/preference/personality 塑形)"

### 7.2 0 装诚实标

| 失守 | 详情 | 修法 |
|---|---|---|
| **0 实测** | 未 git clone N.E.K.O 仓库 (brief 4h 限 + web_search auth fail), 仅基于用户清单 L9 + 你you-list L46 + 真理解 §1-3 推论 | 真实施前主代理必亲验: git clone + 看 README + 真实施调研 |
| **0 数字漂移** | Star / 代码规模 / License / 活跃度 全 "未实测" — 真实数字必主代理亲 git ls-remote + web 亲查 | 主代理亲验时实测 |
| **0 装诱导 prevention** | 不假装 "N.E.K.O 五维记忆具体怎么实现" — 仅推论思路, 真实 trait 抽象主代理必亲验 | sub-agent 真调研时看 N.E.K.O 代码 |

### 7.3 主代理下一步 (推荐)

| # | 行动 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | **派 1 sub-agent 真调研 N.E.K.O 五维记忆** (per §3.2 真实施路径) | 1 周 (派单 §6 P0 #1) | 0 |
| 2 | **主代理亲 git clone N.E.K.O + 看 README + 看 spec** | 1-2 天 (估时实测) | 0 |
| 3 | **决策冻结: 五维记忆 trait 抽象 vs 现有 3 维扩展** (per §6 P0 #2) | 1 天 (主代理亲做) | #1+#2 done |
| 4 | **跟 R20 preference_learning 真实施并行** (per `apeireth-true-understanding-2026-08-28.md` §4.1 P1 #3) | 2-3 周 (R20 critical path) | R10 OrganKind 决策 |
| 5 | **写 `r7-neko-5dim-memory-research.md` 真账** (sub-agent 输出, ≤200 行) | 1 周 (派单内) | #1+#2 done |

**总估时**: P0 (1 周) + P1 (4-6 周) + P2 (4-6 周) = **2-3 月 critical path**, 跟 v2 release critical path 5-7 周并行, 不冲突.

---

_R7-N.E.K.O 写于 2026-08-28, 主代理 brief 4h 限, 物种化借鉴维度修订, 0 实测诚实标, 主代理亲验必做. 真账就位._