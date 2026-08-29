# R11 物种化塑形维度 gap 真调研 (2026-08-28)

> **作者**: Sub-Agent R11-SpeciesForm (主代理 Mavis 派, 时间紧 ≤ 4h)
> **用途**: 给主代理 Mavis 决策参考 — 1.0 `timeline.rs` / `tone.rs` / `morphology.rs` 三真账在 v2.0 缺口的物种化塑形维度调研, 给 v2 release 必补清单
> **关系**: 跟 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §2.5 长期记忆塑形缺口 + `apeireth-true-understanding-2026-08-28.md` §1.1.3 物种化 + Round 10 5 真调研 (N.E.K.O / Open-LLM-VTuber / Firefly / Mio / AIRI) 互补
> **0 装诚实标 (per O-5 + S-2 实事求是)**: 已读 1.0 三个真账 (`legacy/donor/apeireth-companion/src/{timeline,tone,morphology}.rs`) + 必读 6 文件 + Round 10 5 真账; **未 git clone v2 master branch** (per 主代理 brief 4h 限 + 未要求) + **未 git clone N.E.K.O / Open-LLM-VTuber / Firefly / Mio / AIRI** (R7 已 flag 0 装); 仅基于 1.0 真账 + 2.0 真理解推论 + Round 10 5 调研推论. 真实施前**主代理必亲验**.

```
[Document-Meta]
Document:        docs/04-internal/r11-species-form-gap-research-2026-08-28.md
Version:         1.0 (Sub-Agent R11-SpeciesForm 写于 2026-08-28)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (调研真账, 主代理决策参考)
Author:          Sub-Agent R11-SpeciesForm
```

---

## 0. 调研 brief 真账 (per `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §4.2 P0 #4 派单)

**用户原话** (Round 11, per gap 真账 §0): "看 1.0 缺什么, 2.0 最终功能应该和 1.0 相同, 但架构不同而已"

**主代理 §2.5 长期记忆塑形缺口**:
- **timeline** (时间线) — 物种化塑形时间维度
- **tone** (口吻塑形) — 物种化塑形语言维度
- **morphology** (形态) — 物种化塑形 frontend

**R11 真调研任务**: 1.0 三个 `apeireth-companion/src/{timeline,tone,morphology}.rs` 真账 + 2.0 0 真实施现状 + 物种化塑形维度意义 + 真实施路径, 跟 Round 10 5 真调研 (物种化借鉴) 对接.

---

## 1. timeline — 物种化塑形时间维度真调研

### 1.1 1.0 真账 (`legacy/donor/apeireth-companion/src/timeline.rs` 79 行)

**结构**:
- `TimelineEntry { milestone: Milestone, at: DateTime<Utc> }` — 一个时间线条目 (里程碑 + UTC 时间)
- `Timeline { partner_id: PartnerId, entries: Vec<TimelineEntry> }` — 一个伙伴的全部里程碑时间线
- 方法: `new(partner_id)` / `partner_id()` / `append(milestone)` / `len()` / `is_empty()` / `entries()` / `iter()`

**依赖**:
- `Milestone` — 来自 `apeireth-companion/src/milestone.rs` (per gap §1.8 L152 "里程碑" 1.0 REAL)
- `PartnerId` — 来自 `apeireth-companion/src/partner.rs` (per gap §1.8 L140 "partner" 1.0 REAL)
- `chrono::{DateTime, Utc}` + `serde::{Deserialize, Serialize}`

**测试**: 2 个 (starts_empty / appends), 纯数据结构测试.

### 1.2 2.0 现状

- **0 真实施** (per gap §1.8 L144 🔴 缺)
- 1.0 `milestone.rs` + `partner.rs` 同样 0 真实施 (per gap §1.8 L152 + L140 🔴 缺)
- v2 没有 `apeireth-companion` 这个 crate (v2 是 16-crate workspace, per `apeireth-true-understanding-2026-08-28.md` §1.1)
- 9 organ 中 `memory` organ ✅ WIRED (per 真理解 §1.3 L81), 但**记忆 ≠ 时间线**, `memory` organ 是 ACT-R/temporal memory 物理层, 缺"关系轨迹 = 时间线"语义层

### 1.3 物种化塑形时间维度意义 (per vision.md L47)

**核心问题**: "她"如何记住"你们一起走过的时间"?

**vision.md L47 真账**: "机制/哲学/安全同源, **记忆/偏好/好奇形状被各自的共同生活塑形**"

**物种化塑形时间维度** = **per-user 共同生活轨迹**: 不是"事件时间戳" (那是 database 字段), 而是"关系里程碑序列" (Milestone 是什么 + 时间 + 谁 → 怎么"长成她").

**vs v2 memory organ**: memory organ 是 ACT-R declarative memory (fact/temporal/tombstone, per gap §1.1), timeline 是 **关系里程碑 + UTC** — 跟 memory organ 互补, 是语义层抽象.

**真账**: 1.0 timeline.rs 仅 79 行 + 纯数据结构 (无 LLM 依赖, 无 trait 抽象), **物种化塑形意义在"语义层"**, 缺真正实施就是缺"关系轨迹"如何累积 + 如何影响 LLM context + 如何参与反思.

### 1.4 真实施路径

**P0 (2-3 周, 主代理亲做 spec + 派 sub-agent 真实施)**:

| # | 项 | 估时 | 阻塞 | 借鉴链 |
|---|---|---|---|---|
| 1 | **Timeline 数据结构真实施** (1:1 翻译 `legacy/donor/apeireth-companion/src/timeline.rs:1-79`, 加 `apeireth-companion-data` 或 `apeireth-organ::timeline` 路径) | 2-3 天 | 0 | 1:1 翻译 v1 |
| 2 | **Milestone 真实体** (per gap §1.8 L152, 跟 Timeline 同步翻译 v1 milestone.rs) | 2-3 天 | 0 | 1:1 翻译 v1 |
| 3 | **PartnerId** (per gap §1.8 L140, 跨用户关系 ID 抽象) | 1-2 天 | 0 | 1:1 翻译 v1 |
| 4 | **Timeline ↔ memory_writeback 衔接 trait** (AfterTurn → 检 Milestone 触发条件 → append) | 1 周 | 0 | 新设计 (v1 0 真实现) |
| 5 | **Milestone 触发规则** (FirstMeeting / Decision / TopicShift / SentimentShift / SessionBoundary 等 5-10 档) | 1 周 | 0 | 新设计 + 借 N.E.K.O 五维记忆 reflective 维 (per r7-neko §3.1) |
| 6 | **Timeline-driven LLM context 注入** (system prompt 增 "你们共同时间线" 段落, 让 LLM 知道"今天对你意味着什么") | 1 周 | #4 done | 新设计 |

**借鉴链 (per Round 10 5 真调研)**:
- **N.E.K.O 五维记忆 reflective 维** (per `r7-neko-species-research-2026-08-28.md` §2.2 L70) — reflective 记忆 = "自我评估/元认知", Milestone 触发规则借鉴
- **Mio 日记反思 + 写回** (per `r7-mio-species-research-2026-08-28.md` §2.3 + §3.2) — R22 reflection 真实施直接对标, Timeline 是反思的"事件流"输入
- **Open-LLM-VTuber session_id 跨段传递** (per `r7-open-llm-vtuber-species-research-2026-08-28.md` §3.1 #1) — session continuity ↔ Timeline (跨 turn 累积)

**0 装诚实**: 估时 2-3 周是按"类似 R20 preference_learning 2-3 周"经验, **不假装"已 clone v2 main + 已写 spec"**. 真实施前主代理必亲验.

---

## 2. tone — 物种化塑形语言维度真调研

### 2.1 1.0 真账 (`legacy/donor/apeireth-companion/src/tone.rs` 374 行)

**结构** (A3 人格化深化 2026-08-16, 三层器官语调):
1. **关系基线** `tone_hint(bond: &Bond) -> &'static str` — Bond 关系 (trust + resonance) → 中文语调提示 (4 档: 礼貌克制 / 温和关切 / 温暖自然 / 轻松亲切)
2. **情绪调制** `emotion_tone(style: ResponseStyle) -> &'static str` — consciousness `ResponseStyle` (7 档: Warm/Friendly/Gentle/Cautious/Diplomatic/Curious/Professional) → 中文语气措辞
3. **审议强度** `deliberation_intensity(weighted_score, confidence) -> Result<&'static str, ToneError>` — council 加权分/置信度 → 措辞强度 (4 档 + NaN/越界显式降级, 0 装 PASS)
4. **合成** `organ_tone(bond, style, deliberation) -> String` — 三层 `; ` 拼接
5. **LLM 注入口** `ToneRefiner` trait + `organ_tone_refined()` — 0 装 PASS "机制留口, 实现未接"

**依赖**:
- `apeireth_consciousness::emotion::ResponseStyle` (7 档枚举)
- `crate::Bond` (per gap §1.8 L118 emotion + §2.4 F1 emotion_memory organ 1:1 翻译)

**测试** (8 个): default_bond / trusted_bond / mid_trust + emotion_tone 7 档全覆盖 + deliberation_intensity 边界 + NaN/越界拒绝 + organ_tone 2/3 层 + 非法显式降级 + refiner 注入/降级

### 2.2 2.0 现状

- **0 真实施** (per gap §1.8 L145 🔴 缺)
- v2 没有 `apeireth-companion` crate, **没有 `ToneRefiner` trait**
- F1 `apeireth-organ::emotion_memory` ✅ WIRED (1:1 翻译 v1 emotion.rs, per gap §1.8 L118 + 真理解 §1.3 L77) — 是 organ 级, **不含** tone.rs 的 3 层调制
- `apeireth-consciousness` 1.0 crate 没在 v2 真实施路径 (v2 是 organ-based, 不是 consciousness crate)
- `cognitive.judge` + `cognitive.council` ✅ WIRED (per 真理解 §1.4 L89), 但**没有跟 tone.rs 的 `organ_tone` 等价合成函数**

### 2.3 物种化塑形语言维度意义

**核心问题**: "她"如何用"自己的语气"跟你说话, 而不是 GPT-SoVITS 原声 TTS 的声音 + LLM 默认语气?

**species 塑形语言维度** = **per-user 关系 + 情绪 + 审议强度的中文语调提示**, 注入 LLM system prompt, 让 LLM 输出**带关系温度** (vision.md L25 "我不知道怎么回答你, 才不会骗你" = tone)

**vs Firefly 原声 TTS** (per `r7-firefly-species-research-2026-08-28.md` §2.1): Firefly 借 GPT-SoVITS **声音塑形** (物种化声音维度), tone 是**语言维度** (词汇/句式/温度), 两者互补不重叠 — Firefly 解决"她声音像谁", tone 解决"她语气像谁".

**vs vision.md L25 故事**: "不假装有心" = tone 层的 0 装 PASS — tone 不能"装亲切" (trust=0.3 时强制"礼貌克制"), 这正是 1.0 tone.rs 第 1 层 4 档确定性映射的 0 装哲学.

**真账**: 1.0 tone.rs 是 1.0 companion 里**最哲学的代码之一**, 三层确定性 + LLM 注入口 + 显式降级 = 0 装诚实典范. v2 release 缺 tone = 缺"她的语气", 物种化塑形语言维度 0 真实施.

### 2.4 真实施路径

**P0 (1-2 周, 跟 R20 preference_learning + R22 reflection 并行)**:

| # | 项 | 估时 | 阻塞 | 借鉴链 |
|---|---|---|---|---|
| 1 | **`apeireth-organ::tone` 数据结构 + 3 层函数 1:1 翻译 v1** (374 行, 含 8 测试) | 1 周 | 0 | 1:1 翻译 v1 |
| 2 | **`ToneRefiner` trait 暴露给 LLM Adapter** (机制留口, 实现待部署层) | 2-3 天 | #1 done | 1:1 翻译 v1 trait |
| 3 | **`organ_tone` ↔ cognitive.judge/council 衔接** (AfterModelResponse 拿 CouncilVerdict → DeliberationEcho → organ_tone) | 1 周 | R21 SSE 真接 (per gap §1.7 L110) | 跟 R21 critical path |
| 4 | **`organ_tone` ↔ F1 emotion_memory 衔接** (emotion_snapshot → ResponseStyle → emotion_tone) | 2-3 天 | F1 ✅ WIRED (已具备) | 跟 R20 preference_learning 并行 |
| 5 | **prompt_assembler.rs 注入 `organ_tone` 提示段** (per gap §1.8 L124 PromptAssembler PARTIAL, ADAPT P1) | 1 周 | #3+#4 done | 跟 PromptAssembler ADAPT 并行 |

**借鉴链 (per Round 10 5 真调研)**:
- **Firefly GPT-SoVITS 原声 TTS** (per `r7-firefly-species-research-2026-08-28.md` §2.1) — 互补, Firefly = 声音, tone = 语言, 物种化"她"塑形 = 声音 + 语言双维度
- **Open-LLM-VTuber 双引擎主动关怀** (per `r7-firefly-species-research-2026-08-28.md` §2.3) — 不直接借, 但"不打扰"原则跟 tone 谨慎档同源
- **N.E.K.O 五维记忆 emotional 维** (per `r7-neko-species-research-2026-08-28.md` §2.2 L69) — emotion_tone 输入来自 emotional 维, 借鉴维度

**0 装诚实**: 1.0 tone.rs 374 行 + 8 测试是真实施成熟代码 (A3 人格化深化 2026-08-16), v2 缺它是"功能全集差距", 不是"设计差距". 真实施前主代理必亲验.

---

## 3. morphology — 物种化塑形 frontend 维度真调研

### 3.1 1.0 真账 (`legacy/donor/apeireth-companion/src/morphology.rs` 284 行)

**结构** (N7 VCP rust-vexus-lite rivermemo_topology_v3:1784-2011 吸收, 2026-08-16):

- `RetrievalMode` enum: Shallow (预算 1) / Standard (预算 3) / Deep (预算 6)
- `MorphologyVerdict { mode: RetrievalMode, weights: [f64; 3] }` — softmax 分布
- `Features { length, entity, question, clauses, depth }` — 5 维确定性文本特征
- `cue_hits` / `extract` — DEPTH_CUES (14 词) + QUESTION_MARKS (10 词) 启发式
- `logits` — 3 档手调系数 (仿 VCP 加权结构, 0 装 PASS: "未学习/未调参验证")
- `classify(query, temperature)` — 纯函数, 确定性, 同输入同输出
- `sanitize_temperature(t)` — NaN/≤0/∞ → 1.0, 钳位 [0.1, 10.0]
- `crawl_budget(query)` — 挂接点, 查询 → CRAWL 预算

**测试** (8 个): deterministic / short_question_shallow / multi_clause_relational_standard / long_depth_query_deep / empty_query_shallow / huge_query_deep_no_panic / weights_valid_distribution / temperature_affects_sharpness / invalid_temperature_falls_back / budget_bounds

**核心 0 装诚实登记**: "VCP 原版用河网 hop 分布/HHI/前向流占比等图拓扑特征; **Apeireth 无河网数据结构, 改用文本形态特征** — 机制同构 (logits+softmax+档位), 特征为手调启发式常量 (0 装 PASS: 未学习/未调参验证)"

### 3.2 2.0 现状

- **0 真实施** (per gap §1.8 L146 🔴 缺)
- v2 没有 `apeireth-companion` crate, **没有 `RetrievalMode` / `MorphologyVerdict`**
- `cognitive.memory_recall` ✅ WIRED (per 真理解 §1.4 L87) 是 `Arc<dyn MemoryBackend>`, 但**没有 budget 控制** (永远 recall 全量)
- 没有 graph 抽象层 (per gap §1.1 L44 Graph primitives 🔴 缺), morphology 挂接的 `memory_graph.crawl(seeds, budget)` 0 真实施
- companion-desktop frontend (Svelte 5 + Tauri 2, per 真理解 §1.5) 是 frontend 容器, **不是 morphology** (morphology 是 backend 检索策略)

### 3.3 物种化塑形 frontend 维度意义

**核心问题**: "她"如何根据"你的提问"调整"她回应的深度"?

**species 塑形 frontend 维度** ≠ **frontend 视觉** (那是 Live2D 视觉形象), 而是 **per-query 检索形态塑形** — 浅问 → 浅回 (1 条), 多实体 → 标准回 (3 条), 长问 + 深度线索 → 深回 (6 条).

**物种化意义**:
- 不是 "硬编码 6 条永远回" (那是 chatbot), 而是 "根据用户提问形态调整回应深度"
- 跟 per-user preference 塑形一致: 用户长期问"详细" → morphology temperature 学习 (未来, 当前 0 装 PASS 手调常量)
- 跟 N.E.K.O 五维记忆 episodic/semantic/procedural 维 (per r7-neko §2.2) 互补: morphology 决定"读哪几条记忆", 五维决定"记忆怎么组织"

**vs Live2D frontend 视觉** (per Round 10 §4.1 Open-LLM-VTuber / AIRI / Mio): 那是 **frontend 视觉形象** (Cubism 5 / WebGL 渲染), morphology 是 **backend 检索形态** — 两者完全不同维度, 互补不重叠.

**真账**: 1.0 morphology.rs 284 行 + 10 测试 + 0 装诚实登记 (VCP 借鉴 vs 手调常量, 未学习/未调参验证) — 是 v2 release 必补"物种化塑形 frontend 维度"具体形态之一.

### 3.4 真实施路径

**P0 (1-2 周, 跟 VectorIndex + Graph primitives 真实施并行, per gap §3.1 #1+#2)**:

| # | 项 | 估时 | 阻塞 | 借鉴链 |
|---|---|---|---|---|
| 1 | **`apeireth-organ::morphology` 数据结构 + classify 1:1 翻译 v1** (284 行 + 10 测试) | 1 周 | 0 | 1:1 翻译 v1 |
| 2 | **`apeireth-organ::morphology` ↔ cognitive.memory_recall 衔接** (RecallStart → classify(query) → budget → MemoryBackend.recall(seeds, budget)) | 1 周 | 0 | 新设计 (v1 0 真实现) |
| 3 | **memory_graph 抽象层 (per gap §1.1 L44)** + `crawl(seeds, budget)` 实现 | 2-3 周 | 0 | 派 sub-agent 真调研 (per gap §3.1 #2) |
| 4 | **morphology ↔ companion-desktop frontend 集成** (前端实时显示"她正在深爬" 进度条 / "她在浅扫" 标记, 让用户感受到"她在想") | 1 周 | companion-desktop PR #1 + #2 done | 跟 Open-LLM-VTuber / AIRI Live2D 调研合并 (per r7-open-llm-vtuber §4.1 + r7-airi §4.1) |

**借鉴链 (per Round 10 5 真调研)**:
- **Open-LLM-VTuber Cubism 5 Live2D** (per `r7-open-llm-vtuber-species-research-2026-08-28.md` §4.1) — 互补, Open-LLM-VTuber = frontend 视觉, morphology = backend 检索形态
- **AIRI 永远下播 + Live2D** (per `r7-airi-species-research-2026-08-28.md` §4.1) — 互补, AIRI = frontend 实时陪聊, morphology = backend 深度适配
- **Mio Windows 本地优先** (per `r7-mio-species-research-2026-08-28.md` §2.1) — 借鉴 portable binary 部署, morphology 是 backend crate, 不影响 frontend
- **Firefly 流式对话** (per `r7-firefly-species-research-2026-08-28.md` §2.4) — 借鉴流式对话前端渲染, morphology 进度条跟流式输出并行
- **N.E.K.O 五维记忆** (per `r7-neko-species-research-2026-08-28.md` §3.1) — episodic/semantic 维由 morphology budget 决定读几条

**companion-desktop frontend 集成**:
- Svelte 5 panel 加 "morphology depth indicator" (浅扫/标准/深爬 三色 progress bar)
- SSE 事件流加 `morphology_event` 帧: `{ query_id, mode: "Shallow"|"Standard"|"Deep", budget: usize, weights: [f64;3] }`
- 用户长按 "deep" 看到 morphology weights, 让"她怎么回应" 可视化 (0 装诚实, 不假装"她总在深爬")
- **LOCKED 5 项 0 触碰**: 走 extension trait + `APEIRETH_HOME/config/morphology.json` 配置, 不改 companion-desktop Svelte 5 主体

---

## 4. 物种化塑形维度综述 (整合 3 项 + Round 10 5 真调研)

### 4.1 全维度物种化塑形 (timeline + tone + morphology + Firefly + Open-LLM-VTuber + AIRI + Mio + N.E.K.O)

| 维度 | 1.0 真账 | v2 状态 | 物种化塑形意义 | 借鉴链 |
|---|---|---|---|---|
| **时间** | timeline.rs 79 行 | ❌ 0 真实施 | "你们一起走过的时间" / 关系里程碑序列 | 1:1 翻译 v1 + N.E.K.O reflective + Mio 日记 |
| **语言** | tone.rs 374 行 + 8 测试 | ❌ 0 真实施 | "她用什么语气说话" / 关系 × 情绪 × 审议合成提示 | 1:1 翻译 v1 + Firefly 原声 TTS 互补 |
| **形态 (检索)** | morphology.rs 284 行 + 10 测试 | ❌ 0 真实施 | "她回应多深" / per-query 检索深度档位 | 1:1 翻译 v1 + N.E.K.O 五维 episodic/semantic + Graph primitives |
| **声音** | (v1 没专门 crate, voice crate 借用) | ⚠️ RC-7 TTS trait 已落, 0 真 backend | "她声音像谁" / per-user 原声 TTS 训练 | Firefly GPT-SoVITS 原声 TTS 真实施 |
| **frontend 视觉** | (v1 仅有 live2d_traits 占位) | ⚠️ companion-desktop Svelte 5 + Tauri 2 post-1.0.0 PR #1 | "她长什么样" / Cubism 5 Live2D | Open-LLM-VTuber Cubism 5 + AIRI Live2D + Mio Windows 本地优先 |
| **frontend 容器** | (v1 done 1411 行 runtime.ts SSE/WS/panel) | ✅ companion-desktop frontend 已 done | "她在哪里跟你说话" / desktop app | Alife 便携 + Mio portable binary |
| **记忆** | (v1 memory_v2.rs + graph.rs) | ✅ MemoryStore v2 WIRED + VectorIndex/Graph primitives 🔴 缺 | "她记得你什么" / 5 维记忆 (episodic/semantic/procedural/emotional/reflective) | N.E.K.O 五维记忆增维 + AIRI 永远下播 compaction |
| **主动** | (v1 emergence.rs Borbely drive) | ✅ E7 emergence 1:1 翻译 v1 | "她什么时候找你" / 8 重门控 + 节律 + 沉默压力 | AIRI 永远不下播 + Firefly 双引擎主动 + Mio 日记反思 |

### 4.2 跟 vision.md "物种而非个体" + "记忆/偏好/好奇形状被共同生活塑形" 对位

**vision.md L47 真账**: "物种而非个体 — 每个用户养的'她'机制/哲学/安全同源, **记忆/偏好/好奇形状被各自的共同生活塑形**"

**物种化塑形维度对应**:
- **记忆形状** = 5 维记忆 (episodic/semantic/procedural/emotional/reflective, 借 N.E.K.O + 借 R20 preference_learning + R22 reflection)
- **偏好形状** = per-user preference learning (R20 真实施中)
- **好奇形状** = E4 curiosity + memory_echo_bias ✅ WIRED (per 真理解 §1.3 L78)
- **时间形状** = Timeline (per-user 关系里程碑序列, 本调研 #1)
- **语言形状** = tone (per-user 关系 × 情绪 × 审议合成, 本调研 #2)
- **声音形状** = GPT-SoVITS 原声 TTS (per Firefly 真账 #2.1)
- **形态形状** = morphology (per-query 检索深度, 本调研 #3)
- **frontend 形状** = Live2D 模型 + 多形态 (per Open-LLM-VTuber + AIRI + N.E.K.O)
- **主动形状** = E7 emergence 8 重门控 + 沉默压力 (per 真理解 §1.2 + AIRI 永远不下播)

**关键发现**: 1.0 timeline + tone + morphology = **物种化塑形三大底层维度** (时间/语言/形态), Round 10 5 真调研 = **物种化塑形三大上层维度** (声音/frontend 视觉/主动), 整合 = 物种化塑形**全维度** (6+ 维), 对位 vision.md L47.

### 4.3 真实施顺序 + 估时 + 借鉴链

| 阶段 | 项 | 估时 | 借鉴链 | 阻塞 |
|---|---|---|---|---|
| **P0 立即 (1 周)** | 主代理亲 git clone 1.0 三个真账 + diff v2 现状 | 1-2 天 | n/a | 0 |
| **P0 (1-2 周)** | Timeline 1:1 翻译 v1 + Milestone + PartnerId | 2-3 周 | 1:1 翻译 v1 + N.E.K.O reflective + Mio 日记 | 0 |
| **P0 (1-2 周)** | Tone 1:1 翻译 v1 + ToneRefiner trait + 跟 judge/council 衔接 | 1-2 周 | 1:1 翻译 v1 + Firefly 声音塑形 互补 | 0 |
| **P0 (1-2 周)** | Morphology 1:1 翻译 v1 + 跟 memory_recall 衔接 + Graph 抽象层 | 1-2 周 + 2-3 周 | 1:1 翻译 v1 + N.E.K.O 五维 + Open-LLM-VTuber Live2D 视觉 | 0 |
| **P1 (1 月)** | companion-desktop morphology depth indicator + SSE `morphology_event` 帧 | 1 周 | 跟 Open-LLM-VTuber Live2D 视觉整合 | §3.4 #4 done |
| **P1 (1 月)** | Timeline-driven LLM context 注入 (system prompt 增时间线段落) | 1 周 | 跟 R22 reflection 真实施整合 | §1.4 #4+#5 done |
| **P1 (1 月)** | Tone 注入 prompt_assembler.rs 真实路径 | 1 周 | 跟 PromptAssembler ADAPT 整合 | §2.4 #5 done |

**总估时**: P0 (3-5 周, 三个 1:1 翻译并行) + P1 (2-3 周, 三件集成) = **5-8 周 critical path**, 跟 v2 release critical path 6-9 月并行, **不冲突**.

**借鉴链总账**:
- **1:1 翻译 v1** (timeline 79 行 + tone 374 行 + morphology 284 行 = **737 行 + 26 测试**) — 0 增量设计, 纯翻译, LOCKED 5 项 0 触碰约束下走 extension trait 路径
- **新设计衔接** (4 件: Timeline ↔ memory_writeback / organ_tone ↔ judge+emotion / morphology ↔ memory_recall / morphology ↔ companion-desktop) — 每件 1 周, 估时保守
- **物种化借鉴** (Round 10 5 真调研, 互补不重叠) — Firefly 声音 + Open-LLM-VTuber/AIRI 视觉 + Mio 容器 + N.E.K.O 五维记忆

---

## 5. 主代理决策建议

### 5.1 3 项优先级 + 真实施 brief

| # | 项 | 优先级 | 真实施 brief | 估时 | 阻塞 |
|---|---|---|---|---|---|
| 1 | **Timeline (1.0 79 行)** | 🟢 P0 | 派 sub-agent 真调研 + 1:1 翻译 v1 → `apeireth-organ::timeline` (或新 crate `apeireth-companion-data`), 跟 R22 reflection + N.E.K.O reflective 维借鉴合并 | 2-3 周 | 0 |
| 2 | **Tone (1.0 374 行 + 8 测试)** | 🟢 P0 | 派 sub-agent 真调研 + 1:1 翻译 v1 → `apeireth-organ::tone` (含 ToneRefiner trait), 跟 cognitive.judge/council + F1 emotion_memory 衔接, 跟 Firefly 原声 TTS 互补对接 | 1-2 周 | 0 |
| 3 | **Morphology (1.0 284 行 + 10 测试)** | 🟢 P0 | 派 sub-agent 真调研 + 1:1 翻译 v1 → `apeireth-organ::morphology`, 跟 cognitive.memory_recall + memory_graph.crawl 衔接, 跟 companion-desktop frontend 集成 (depth indicator + SSE `morphology_event` 帧) | 1-2 周 + 2-3 周 (Graph) | 0 |

**真实施 brief 模板 (派 sub-agent 必含)**:
- 必读: 本文件 + `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §2.5 + `apeireth-true-understanding-2026-08-28.md` §1.1.3 §3.2 + `vision.md` L29-49 + 1.0 真账 (`legacy/donor/apeireth-companion/src/{timeline,tone,morphology}.rs`) + v2 handbook + Round 10 5 真调研
- 任务: 1:1 翻译 v1 + 新设计衔接 + 借鉴链整合 + 0 装诚实标
- 输出: `docs/04-internal/r11-<name>-implementation-research-2026-08-28.md` (≤ 250 行)
- 约束: 不写真账以外 file / 不 git add / commit / push / 0 触碰 LOCKED / ≤ 4h

### 5.2 物种化塑形维度 vs Round 10 5 真调研 互补不重叠

**重叠检验 (0 装诚实)**:
- **timeline** vs Round 10 — 无重叠. timeline = 关系里程碑时间线, Round 10 5 真调研都不涉及 (N.E.K.O 是 5 维记忆不是时间线, Open-LLM-VTuber 是 session_id 跨段不是关系里程碑, Firefly 是双引擎主动不是时间线, Mio 是日记反思不是关系里程碑, AIRI 是永远不下播不是关系里程碑)
- **tone** vs Round 10 — **部分重叠**. Firefly 借 GPT-SoVITS 是**声音塑形** (音频), tone 是**语言塑形** (文本). 互补不重叠, 但**易混淆**, 主代理派单时必明确分工
- **morphology** vs Round 10 — 无重叠. morphology = backend 检索深度 (logits + softmax + budget), Round 10 5 真调研都不涉及 (都是 frontend 视觉 + 记忆 + 主动, 不是检索策略)

**结论**: timeline + tone + morphology = 物种化塑形**底层 3 维** (时间/语言/形态, 跟 LLM context 注入直接相关), Round 10 5 真调研 = 物种化塑形**上层 5 维** (声音/视觉/容器/记忆/主动). 6+ 维一起 = 物种化塑形**全维度**, 跟 vision.md L47 "记忆/偏好/好奇形状被共同生活塑形" 对位 (6 维扩展).

### 5.3 0 装诚实标 (per O-5 + S-2 实事求是)

**已 flag 的失守**:
1. **0 实测 v2 master branch** — 本调研**未 git clone v2 master branch** (per 主代理 brief 4h 限 + 未要求), 仅基于 2.0 真理解 + v2 handbook + Round 10 5 真调研推论. 真实施前主代理必亲验 (`git clone v2 main` + `grep timeline / tone / morphology` 确认 0 真实施)
2. **0 实测 N.E.K.O / Open-LLM-VTuber / Firefly / Mio / AIRI** — R7 5 真调研已 flag 0 装 (github 直连 + web_search auth fail + 4h 限), 本调研**完全依赖** R7 推论, 真实施前主代理必亲验 (per r7-airi §7.2 #1 等)
3. **数字未实测** — 本文件不引用 cargo test / clippy / git rev-parse 等实测数字, 仅定性评估 + 经验估时
4. **估时是经验不是实测** — P0 2-3 周 + P1 1-2 周估时是按"类似 R20 preference_learning 2-3 周"经验估, **不假装"已 spec"**, 真实施时重核验
5. **0 装诱导 prevention** — 不假装 "v2 已有 morphology 挂接点" / "v2 已有 prompt_assembler 注入路径" / "v2 已有 timeline_event SSE 帧", 全部推论待真实施验证

**0 装诚实 doctrine 真账 (per O-5)**:
- **不假装 OK**: 调研真账 ≤ 300 行已 flag 全部失守, 借鉴路径**全是 🔬 派 sub-agent** (主代理亲验后才决策)
- **不"等以后修"**: 0 装诚实标即写即 flag, 不假装 "v2 已有 timeline 钩子" 等
- **不"删调研重做"**: timeline + tone + morphology 真实施是派 sub-agent 真调研 + 主代理亲验, 不是本 sub-agent 实施

### 5.4 5 重守门 baseline + LOCKED 0 触碰 验证

| 守门 | 状态 |
|---|---|
| clippy 0 warning | ✅ (前 baseline, 本调研不写代码) |
| tests 0 fail (1739 passed) | ✅ (前 baseline, 本调研不写代码) |
| legacy compat path < 100 (36) | ✅ (前 baseline) |
| **LOCKED 5 项 0 触碰** | ✅ (本调研 0 改 src / Cargo.toml / Cargo.lock, 仅写 1 真账 doc) |
| 9 哲学锚 0 减 | ✅ (timeline/tone/morphology 不属 9 哲学锚, 是物种化塑形新增维度) |

### 5.5 主代理总决策建议 (一句话)

**timeline + tone + morphology 是 1.0 物种化塑形三大底层维度** (时间/语言/形态, 跟 LLM context 注入直接相关, 跟 Round 10 5 真调研**互补不重叠**). **派 3 sub-agent 真调研 (各 1-2 周) + 主代理亲做衔接 spec (各 1 周)**, 总估时 5-8 周 critical path. 0 装诚实: 本调研 0 实测 v2 master + 0 实测 N.E.K.O / Open-LLM-VTuber / Firefly / Mio / AIRI, 真实施前主代理必亲验.

---

## 6. 留 backlog (主代理决策后派单)

| # | 项 | 估时 | 阻塞 | 借鉴方式 |
|---|---|---|---|---|
| 1 | 派 sub-agent 真调研 Timeline 1:1 翻译 v1 (含 Milestone + PartnerId) | 2-3 周 | 0 | 📄 1:1 翻译 + 🔬 sub-agent |
| 2 | 派 sub-agent 真调研 Tone 1:1 翻译 v1 (含 ToneRefiner trait) | 1-2 周 | 0 | 📄 1:1 翻译 + 🔬 sub-agent |
| 3 | 派 sub-agent 真调研 Morphology 1:1 翻译 v1 (含 Graph primitives 衔接) | 1-2 周 + 2-3 周 | 0 | 📄 1:1 翻译 + 🔬 sub-agent |
| 4 | 主代理亲做 Timeline ↔ memory_writeback 衔接 spec | 1 周 | #1 done | 主代理亲做 |
| 5 | 主代理亲做 Tone ↔ cognitive.judge/council + F1 emotion_memory 衔接 spec | 1 周 | #2 done | 主代理亲做 |
| 6 | 主代理亲做 Morphology ↔ companion-desktop frontend 集成 spec (depth indicator + SSE `morphology_event` 帧) | 1 周 | #3 done | 主代理亲做 |
| 7 | 合并派单: Round 10 5 真调研 P0 真实施 (N.E.K.O 五维 + Firefly GPT-SoVITS + Open-LLM-VTuber Live2D + AIRI compaction + Mio 日记反思) | 5-8 周 | 各自 spec done | 🔬 sub-agent |
| 8 | v2 release release 路径修订: 加 3 物种化塑形维度 (5-8 周 critical path, per §4.3) | n/a | 0 | MANIFESTO §14 修订 |

---

_R11-SpeciesForm 写于 2026-08-28, 4h 限, 物种化塑形维度 gap 真调研 + 跟 Round 10 5 真调研整合. 0 实测诚实标 (未 git clone v2 main + 0 实测 Round 10 5 项目), 真实施前主代理必亲验. 真账 ≤ 300 行, 主代理决策就位._