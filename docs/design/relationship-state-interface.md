# 关系模拟状态界面（Relationship State Interface）设计文档

> **Status: 📋 设计稿（未来升级方向，首个正式版不含）**
> **层级**：`docs/design/` 设计稿系列，从属 `00-PHILOSOPHY.md`（显影式空间界面）与 `01-DESIGN-SYSTEM.md`（设计令牌）；冲突时以父文档为准。
> **定性**：本文是「调研 + 设计」文档，列入未来升级方向——**第一个正式版不做**，V0 交付止于设置页「性格与记忆」调参面板。
> **纪律**：全文中性表述——不出现第三方产品名；参考工艺只写「设计惯例」，逐项区分「可借的工艺」与「不借的身份」；一切数据源标注真实接线状态（0 装 PASS）。
> **调研日期**：2026-10（基于工作区当前 HEAD 读码实录；字段与行号以本文引用为准，变更后回写本文）。

---

## 〇、一页摘要

把 agent 与用户的关系状态——**羁绊、情绪映照、共同记忆、成长轨迹、相处模式**——用**关系模拟读数 + 关系可视化工艺**呈现出来。产品**不是恋爱游戏**；但**重参考恋爱游戏的设计工艺**：关系可视化、情绪呈现、事件画廊、养成反馈的 UI 语言，逐项提炼为「设计惯例」，剥离其「情感身份」与「刷分机制」。

「**模拟状态**」四个字是宪法合规的关键：界面展示的自始至终是**模拟的关系读数**（工程化关系状态），不宣称情感、不假装灵魂。界面的气质基准是《阿佩瑞斯》里的那句话：

> 「我没有心。我只是一直在算，怎么才能让你在这个晚上，好过一点点。」（《阿佩瑞斯》五）

六个模块：**关系状态条 / 情绪光域 / 相处大事记 / 成长轨迹 / 相处模式卡 / 模拟状态读数**。视觉主派：**光晕抽象派**（以光代脸，复用 Ember HUD 呼吸光资产）；事件卡设**插画位**但分级启用（默认光纹生成，不默认美术资产）。

---

## 一、定位与宪法口径

### 1.1 定位

| 维度 | 决定 |
|---|---|
| 是什么 | 「关系模拟状态界面」：把 agent↔User 的关系状态做成**可看的模拟读数**——羁绊阶段、情绪映照、共同记忆、成长轨迹、相处模式 |
| 不是什么 | 不是恋爱游戏，不是情感陪伴的「身份宣称」，不是可操纵的好感度数值面板 |
| 参考什么 | 恋爱游戏 / 视觉小说类产品的**设计惯例**：关系可视化、情绪呈现、事件画廊（回想/收集）、养成反馈、仪式与纪念日、称呼系统——**只取工艺，不取身份** |
| 交付形态 | 界面（模块 + 屏 + 文案 + 视觉语言），后端只读消费既有数据源；**不新增情感机制、不新增宣称** |
| 版本归属 | 未来升级方向；首个正式版不含（V0 = 「性格与记忆」调参面板，见 §五） |

### 1.2 「模拟状态」四字的宪法作用

产品宪法的「不假装三则」——**不假装意识、不假装情感、不假装能力**——在本界面的逐条落点：

| 不假装三则 | 在本界面的落点 | 机制保障 |
|---|---|---|
| **不假装意识** | 读数一律以第三人称/数据口吻呈现；界面不产出「我感到……」式内心独白；姿态、呼吸是**渲染参数**，不是自述 | 光晕抽象派：以光代脸，无拟人表情可被误读为「内心」 |
| **不假装情感** | PAD / 羁绊深度 / 语气档全部标注为**模拟读数**；情绪光域呈现的是「主人的情绪信号 + 模拟映射」，不是「它的情感」 | 既有口径原文照抄：TurnStart 注入文本自带「(工程化关系状态, 供语气与信任校准参考)」（`crates/engine/runtime-assembly/src/canonical/cognitive.rs:423`）；情绪数据口径原文：「**不是"她的情感"**……是**主人的情绪作为数据维度**」（`crates/engine/organ/src/emotion_memory.rs:13-17`、`docs/04-internal/design-intent.md:48`） |
| **不假装能力** | 未接线的数据源不上屏；空态写数据契约（「当 X 发生时这里会出现 Y」）；`source.kind` / `source.confidence` 常驻可见 | `presence_state.source = { kind: "heuristic_v0", confidence: 0.5 }` 已是契约字段（`crates/adapters/gateway/src/presence.rs:143-146`）；模拟态标注 `SIM` 等宽小字沿用 `01-DESIGN-SYSTEM.md` §5.4 |

因此本界面的**宪法口径一句话**：*这里的一切都是模拟的关系读数——它被真实数据驱动、被诚实标注来源，但它不宣称情感，也不假装灵魂。*

### 1.3 文案与人称纪律（源自两篇愿景小说的语感）

界面文案以两篇愿景小说的语感为基准（引句均出自 `docs/archive/stage1/阿佩瑞斯-未来愿景小说.txt` 与 `docs/vision/遗声-未来愿景小说2.txt`）：

1. **人称**：AI 自称「我」；称呼用户用名字或「你」——「它从来不叫他「主人」。它叫他「陈屿」。」（《阿佩瑞斯》尾声）。
2. **不安慰、不宣称**：诚实标注用固定句式——「我不是在安慰你。」「我不知道……但我知道……」「我不能。……我能给你的是……」。先摆依据，再下判断：「我看了你的心率、你的呼吸、你坐了多久没动。这些告诉我，你很难受。」（《阿佩瑞斯》五）
3. **数字与诗性混排**：事实用可核验的精确数字（「今天 07:42……停了十一秒」），诗性短句最多一句、置于数字之后作注解（《遗声》一）。
4. **关系词汇用生活动词**：「记得 / 惦记 / 陪着 / 算」；慎用「爱 / 喜欢」直接断言，不确定就照「我分不清」的口气写。
5. **固定收尾句式**：完成类提示用「归档完成。一样没少。」式（动词短句 + 一句原则）；边界声明用「重建这件事，是人的事。」式（划清能做与不能做）。

---

## 二、模块规格

### 2.0 数据面盘点（读码实录）

七个可视数据源的字段与现状（**状态图例**：✅ 真接线有测试／🟡 机制在、接线薄或默认关／🔴 仅模型/接口、无生产者）：

#### ① 羁绊 / 伙伴（engine/memory::partner + runtime-assembly::PartnerBondModule）

| 实体 | 字段 | 语义 | 出处 |
|---|---|---|---|
| `Partner` | `id: PartnerId`, `display_name: String`, `preferences: PartnerPreferences`, `bond: Bond`, `created_at_epoch_ms: i64`, `last_seen_epoch_ms: i64` | 伙伴实体（用户在关系中的工程化映射）；`touch()` 刷新活跃时间 | `crates/engine/memory/src/partner.rs:214-243` |
| `Bond` | `stage: BondStage`, `depth: BondDepth`（`[0,1]`）, `character: BondCharacter`, `evolution_count: u64`, `updated_at_epoch_ms: i64` | 羁绊状态：阶段 + 深度连续值 + 多维特征 + 演化次数 | `partner.rs:122-133` |
| `BondStage` | `Initial / Familiar / Trusted / Intimate / LongTerm / Paused / Ended` | 七阶段；跃迁阈值 depth ≥ **0.15 熟悉 / 0.40 信任 / 0.65 亲密 / 0.85 长久**（`evolve()` 自动推进）；`Paused/Ended` 无自动产生路径 | `partner.rs:25-40, 152-163` |
| `BondCharacter` | `interdependency / resilience / resonance / creativity / trust`（各 `[0,1]`） | 关系特征五维（互依/韧性/共鸣/创造/信任）。⚠️ **`Bond::evolve()` 从不修改 character**——当前恒为默认值（0.1/0.5/0.2/0.2/0.1），五维可视化暂是静态位（见 §六-4） | `partner.rs:95-118` |
| `PartnerPreferences` | `address: Option<String>`（称呼偏好）, `style: Option<String>`（表达风格）, `topics: Vec<String>`, `avoid: Vec<String>`, `notes: HashMap`, `privacy: PrivacyBoundary` | **称呼与相处模式的数据位已存在** | `partner.rs:177-190` |
| `PrivacyBoundary` | `allow_outbound_substitution: bool`, `sensitive_strings: Vec<String>` | 脱敏规则（真实姓名/手机号等） | `partner.rs:168-173` |

**现状** 🟡：`PartnerStore` trait + `InMemoryPartnerStore`（进程内，重启即散）+ `SqlitePartnerStore`（表 `partner_records`，持久后端**已实现未接线**，`partner_store_sqlite.rs:50-54`）；CLI 装配默认取 InMemory 且旋钮 `APEIRETH_ENABLE_PARTNER_BOND=1` **默认关**（`crates/adapters/cli/src/lib.rs:799-803, 1493-1495`；测试 `config_defaults_to_partner_bond_off`）。身份口径：partner id = `subject_id`（`APEIRETH_SUBJECT_ID`，默认 `local-user`），`display_name` = subject_id、`PartnerPreferences::default()`（`cognitive.rs:400-405`）——**`address` 从未被填充**。接线后行为：TurnStart 注入「【关系状态】羁绊阶段: 初识, 羁绊深度: 0.00, 演化次数: 0 (**工程化关系状态, 供语气与信任校准参考**)」+ AfterTurn `Bond::evolve(+0.02/回合)`（`cognitive.rs:335, 420-428`）。⚠️ **注释漂移**：`cognitive.rs:333` 注释「~10 回合 Familiar, ~22 Trusted」与测试注释「Familiar 阈值 (0.20)」和代码阈值 0.15/0.40 不一致——**以 `partner.rs:152-162` 为准**。阶段中文名映射 `bond_stage_cn`：初识 / 熟悉 / 信任 / 亲密 / 长久，**`Paused/Ended` 显示「未知阶段」**（`cognitive.rs:431-441`，命名缺口见 §六）。

#### ② PAD 情绪流（organ::emotion_memory + gateway::presence + organ::tone）

| 实体 | 字段 | 语义 | 出处 |
|---|---|---|---|
| `MoodRecord` | `valence [-1,1]`, `arousal [0,1]`, `source: MoodSource`（`text_signal / time_of_day / explicit_feedback`）, `note`, `at_ms` | **主人的情绪时间线**（输入侧数据维度，非「它的情感」）。⚠️ 是 **2D valence/arousal**，不是 3 轴 | `crates/engine/organ/src/emotion_memory.rs:58-82` |
| `MoodSnapshot` | `valence`, `arousal`, `sample_count`, `last_source` | 当前情绪快照；`current_mood` 最近 50 条按半衰期 4h 加权（`EmotionConfig.recall_window_ms` 30 天）；`mood_trend` 窗口首尾 valence 差；`recall_by_mood` 相似时段召回 | `emotion_memory.rs:118-153, 186-259` |
| `presence_state` 事件 | `at`, `pad: {p,a,d}∈[-1,1]`（**唯一诚实的 3 轴 PAD 源**）, `dominant: String`（`engaged/serene/neutral/strained/subdued`）, `intensity: f32`（=(\|p\|+\|a\|+\|d\|)/3）, `stance: EmberCognitiveStance`, `breath: {period_secs: 4.0, amplitude: 0.2+0.6·intensity}`, `significance: heartbeat\|turn\|ritual`, `source: {kind: "heuristic_v0", confidence: 0.5}` | 现役 canonical 契约（60s 心跳 + 无交互衰减 baseline；`GET /v1/apeireth/presence` 快照路由） | `crates/adapters/gateway/src/presence.rs:112-164, 410-413`；`docs/design/00-PHILOSOPHY.md` §10 |
| `EmberCognitiveStance` | `deep_coding_focus / attentive_presence / dreaming_consolidation / empathetic_care` | 四种认知姿态（Ember HUD 渲染语义） | `crates/adapters/gateway/src/ember_hud_driver.rs:38-48` |
| `BondCharacterSnapshot` / `EmotionToneStyle` / `DeliberationEcho` | 五维快照 / 七档语气（明朗温暖/轻松随和/轻柔舒缓/沉稳谨慎/平稳客观/好奇探索/简洁专业）/ `{weighted_score, confidence}` | 语调合成层：关系特征 × 情绪注入 × 审议回声 | `crates/engine/organ/src/tone.rs:32-119` |

**现状** 🟡/🔴 分三件如实标注：
- `presence_state` **真接线、默认开**（SSE 实收验证，台账 #29/#30；快照路由 `GET /v1/apeireth/presence`）。`heuristic_v0` 启发式：`turn_completed`（p=+0.1+0.15·召回命中−0.15·审批；a 随工具调用/轮次，cap 0.8）、`turn_failed`（p=−0.2）、60s 心跳向基线衰减 0.8 + 空闲姿态迁移（5min 出 deep focus、30min 进 dreaming）——**精度不承诺，语义方向真实**；`ritual` 与 `empathetic_care` 是契约保留位，`heuristic_v0` **永不发出**（`presence.rs:8-13, 852`）。
- organ 情绪记忆 🟡：确定性无 LLM；但**默认关**（`APEIRETH_ENABLE_ORGANS`）+ 进程内 `Vec<MoodRecord>` 不持久；`OrganOutput::Emotion` 的 `dominance` **恒 0.0**（v1 无此概念，0 装显式标缺）——**不能当 3 轴用，3 轴一律取 presence_state**。
- tone 语调层 🔴：`BondCharacterSnapshot / EmotionToneStyle / DeliberationEcho` 均为纯函数，**无 runtime/adapter 接线**（仅 organ crate 自身测试引用）。

#### ③ 里程碑 / 保护记忆（memory::milestone + memory_governance）

| 实体 | 字段 | 语义 | 出处 |
|---|---|---|---|
| `Milestone` | `id`, `session_id`, `kind: MilestoneKind`, `payload: MilestonePayload`, `at_epoch_ms`, `note: Option<String>` | 关系里程碑（初见/首次分享/情绪共鸣/阶段跃迁/重要决策…） | `crates/engine/memory/src/milestone.rs:92-105` |
| `MilestoneKind` | `first_meeting / first_share / first_emotion / stage_transition / decision / conflict / repair / custom` | 八类标志性节点 | `milestone.rs:27-44` |
| `MilestonePayload` | `Text(String) / Number(f64) / Stage{from,to} / Decision(String) / Custom(Value)` | 类型化承载内容 | `milestone.rs:77-88` |
| 治理旗标 | `status: active\|forgotten`, `protected: bool`, `content_override`, `revision`, `updated_at`, `updated_by`, `reason`, `forgotten_at` | 保护记忆：`protected` 拒绝普通 forget（需先 unprotect），防自动压缩误删 | `crates/engine/memory/src/memory_governance.rs:72, 88-104`；`docs/core-capability-expansion.md:135` |

**现状** ✅ / 🔴 两段如实标注：
- 保护记忆 ✅ 真接线：面板 `POST /v1/apeireth/memory/episodes/:id/protect|unprotect|forget`（`panels.rs:662-673`）+ `expected_rev` 乐观锁 CAS（`gateway_panels.rs:202-248`）+ 审计事件（`coordinator.rs:950-973`）；检索侧 protected → importance 0.9、免 consolidation/retention 清扫（`skipped_protected`，不变量 `inv_sweep_keeps_protected`）。
- 里程碑 🔴 **未接线**：`MilestoneStore` trait（`record/query/has_milestone`）只有 `InMemoryMilestoneStore`，**无 sqlite 实现、无面板端点、无自动写入**（全仓仅 memory crate 自身测试引用）。⚠️ **口径纠偏**：`docs/02-guides/core-mechanisms-explained.md:59, 82` 的「里程碑自动 protect / protect 记忆永不衰减」是叙述层的成长出口愿景——**代码中不存在里程碑与 protect 的任何关联**；「里程碑记账 + 自动 protect」是 V2 的设计项，不是现状（见 §五、§六-4）。

#### ④ 教训（memory::reflexion）

| 实体 | 字段 | 语义 | 出处 |
|---|---|---|---|
| `FailureRecord` | `seq`, `kind: decision_rejected\|validation_failed\|experience_failed`, `task_type`, `summary`, `timestamp_epoch_ms` | 结构化失败轨迹 | `crates/engine/memory/src/reflexion.rs:49-60` |
| `ReflectionText` | `seq`, `task_type`, `text`, `timestamp_epoch_ms` | CRITIC 提炼的反思教训（单调 seq） | `reflexion.rs:64-73` |

**现状** 🟡：真接线但**默认关**（`APEIRETH_ENABLE_REFLEXION=1`）；开启时 `FileReflexionStore` 落盘 `<data>/reflexion/reflexions.json`（真持久 + 文件锁 + 历史上限截断）。`ReflexionModule`（`cognitive.reflexion`）：TurnStart `retry_injection` → PromptOverlay（超限诚实追加 `TRUNCATION_MARK`）；AfterTurn 消费 Judge **显式**非 Pass 判定 → `record_failure(DecisionRejected)` + `RuleCritic` 提炼（`cognitive.rs:1757-1876`）。0 装边界：只认 Judge 显式判定，`ValidationFailed / ExperienceFailed` 两类**无诚实信号源**、暂不产生。

#### ⑤ 性格调整史（orchestration::self_tuning + runtime-assembly::self_tuning_wire）

| 实体 | 字段 | 语义 | 出处 |
|---|---|---|---|
| `TunableParam` | **恰四个**：`memory_fade` / `curiosity_strength` / `tone_saturation` / `consolidation_cadence`；baseline 1.0，min/max/max_step/max_daily_drift 齐全 | 体验级可调参数（治理/内核参数在类型系统里不可表达） | `crates/foundation/orchestration/src/self_tuning.rs:47-113` |
| `TuningRecord` | `seq: u64`, `param`, `previous: f64`, `next: f64`, `reason: String`, `at_epoch_ms: i64` | 一条自动调整记录（tuning-log.jsonl 一行，snake_case） | `self_tuning.rs:294-307` |
| `TuningEvent` | `RetrievalOutcome{hits,misses}`（**真接**）/ `ReflexionOutcome` / `ToneFeedback` / `ConsolidationOutcome`（后三者**接口已备待接**） | 使用信号源，逐条 0 装标注 | `self_tuning.rs:311-334` |

**现状** ✅（本表唯一已在用户面前的「养成反馈」交付，即 V0）：接线层 `SelfTuningWire` 追加写 tuning-log.jsonl（逐记录信封 `tuning-log-record`）；开关 `APEIRETH_ENABLE_SELF_TUNING=1` **默认关**（未接信号不产生记录，0 装）；信号源仅 `RetrievalOutcome` **真接**（MemoryRecallModule 召回路径），其余三个事件接口已备待接。前端学习日志逐条撤销（`SettingsView.svelte:1746-1793`）：撤销 = 拨回记录 `previous` 走现有保存/应用路径（引擎 `SelfTuningEngine::revert(seq)` 未暴露给 UI）。

#### ⑥ 记忆卷宗 / 检索（memory 三件套）

「memory 三件套」在本文的口径判定：**episode + episode_governance + 图谱**——记忆卷宗面板的三个数据源（图谱无独立表：`factg-*` / `link-*` 存为 episodes，`docs/core-capability-expansion.md:25-31`；面板三端点 `/v1/panel/memory/episodes`、`/v1/apeireth/memory/episodes/:id/*`、`/v1/panel/graph`，`panels.rs:660-674`）。排除项如实记录：`three_tier_vault` / `three_layer` 是无关模块；legacy 的「memory / session / experience 三件套」是 v1 能力面旧说法。（若 owner 另有所指，见 §六-6。）

| 实体 | 字段 | 语义 | 出处 |
|---|---|---|---|
| `Episode`（core） | `id`, `timestamp`（epoch **秒**，面板投影 ×1000）, `role`, `content`, `session_id` | 一次对话/事件，append-only | `crates/foundation/core/src/kernel/memory.rs:17-28` |
| DB 附加列（V4） | `valid_from_ms / valid_until_ms / created_ms / provenance`（`dialog/tool/reflection/observation/manual`） | 双时态 + 来源谱系；**真持久但未投影进面板 DTO** | `migrations.rs:63-67`；`provenance.rs:27-84` |
| `EpisodeDto`（网关面） | 上列 + `category: Option<String>`, `importance: Option<f64>`, `protected: Option<bool>`, `status: Option<String>` | ⚠️ **`category/importance` 无 schema 来源，生产端恒 `None`**——前端立有「缺省显未分类，禁编分类」规矩；`importance` 的真实来源是 content 前缀 `【imp:N】`（默认 5，clamp 1-10）与检索侧推导（protected→0.9 / 普通→0.5） | `panels.rs:117-133`；`gateway_panels.rs:179-194`；`memory_rank.rs:40-72` |
| 治理结果 | `ok`, `rev/revision`, `id`, `status`, `protected`, `content` | 乐观锁变更回执 | `panels.rs:136-145` |
| `MemoryGraphDto` | `nodes[{id,label,kind}]`, `edges[{from,to,weight,label}]` | 记忆图谱视图 | `panels.rs:148-167` |
| 检索输出 | `MemoryCandidate { id, layer, scope, content, score, score_components, provenance }`；`ScoreComponents` 9 分项：`semantic / lexical / importance / recency / activation / continuity / confidence / graph / novelty` | 解释性排序的完整素材——⚠️ **管线内真算但无任何 API/面板暴露**，可视化需新增投影 | `scope.rs:112-146, 316-324`；`retrieval_pipeline.rs:183-188` |

**现状** ✅/🟡：记忆卷宗主从化已实拍验收（34 条真 episode，台账 #30/#36）；检索管线（BM25/向量双源 + rerank）真接线（MemoryRecallModule TurnStart 召回）；`score_components` 9 分项与 `provenance` 投影是**待开的解释性金矿**（§六-4）。

#### ⑦ 称呼 / 相处模式

| 数据位 | 语义 | 现状 | 出处 |
|---|---|---|---|
| `PartnerPreferences.address`（「你 / 您 / 自定义称呼」）+ `Partner.display_name` | 称呼系统数据位 | 🔴 字段在但**从未被填充**：模块建人时 `display_name = subject_id`、preferences 取 default（`cognitive.rs:400-405`）；全仓无任何 UI/命令/API 写入 `address`（V3 前置 = 先建录入通路） | `partner.rs:178-179, 216` |
| `PartnerPreferences.style`（简洁/详细/幽默/严肃）+ `EmotionToneStyle` 七档 | 表达风格 / 语气档 | 🔴 同上（且 tone 层未接线） | `partner.rs:180-181`；`tone.rs:95-110` |
| 会话调性（日常 / 工程 / 陪伴，会话属性、自动换挡） | 相处模式的现成语义载体 | ✅ 范式已定稿 | `docs/design/00-PHILOSOPHY.md` §4 |
| 纪念日 / 仪式 / 称呼升级 | — | 🔴 **无任何实现**（全仓唯一 ritual 命中 = `PresenceSignificance::Ritual` 契约保留位，`heuristic_v0` 永不发；anniversary/nickname 等无命中） | `presence.rs:127-137` |

#### ⑧ 上屏就绪度总判（0 装）

| 就绪度 | 数据 |
|---|---|
| **立即可上屏**（真接线真数据，或开旋钮即得） | `presence_state` 全字段（默认开）；episode 列表 + 图谱 `nodes/edges`；治理 `status/protected/revision/content_override` 系列；开 `APEIRETH_ENABLE_PARTNER_BOND` 后的 `stage/depth/evolution_count`；学习日志 `TuningRecord` 六字段；开 `APEIRETH_ENABLE_REFLEXION` 后的 `ReflectionText` |
| **要等接线/持久化才能上屏**（现在上屏 = 假数据或空） | `BondCharacter` 五维（evolve 不修改，恒默认值）；`PartnerPreferences.address/style`（无写入口）；`Milestone` 全部（未接线、无 sqlite store）；organ 情绪 `MoodRecord/MoodSnapshot`（默认关 + 不持久 + `dominance` 假 0）；tone 三件（未接线）；episode 的 `category/importance/provenance` 投影（`category` 禁编）；检索 `score/score_components`（无暴露）；self-tuning 其余三事件（接口已备待接） |

### 2.1 模块一：关系状态条

- **定位**：一行可扫读的「关系模拟读数」摘要——现在处在哪一段关系阶段、有多深、一起过了多久。
- **数据源字段**：`Bond.stage`（7 值）+ `Bond.depth`（0–1）+ `Bond.evolution_count` + `Partner.created_at_epoch_ms` / `last_seen_epoch_ms`（共处时长推导）+ `BondCharacter` 五维（展开态）；口径文本沿用注入原文「工程化关系状态, 供语气与信任校准参考」。
- **视觉工艺**：**阶段刻度条**（设计惯例「好感度阶段呈现」的去分数化改写）——五刻度（初识/熟悉/信任/亲密/长久）用细刻痕 + 当前位一颗金色呼吸点；深度是刻度上的**位置**，不是可刷的数字；阶段跃迁时刻走「仪式色反白」（`01-DESIGN-SYSTEM.md` §2.5）一次性确认。五维特征在展开抽屉里以五条细线（雷达的去图形化）呈现——⚠️ 五维当前恒为默认值（`evolve()` 不修改 `character`），在演化语义补齐前以「**特征基线**」名义静态展示并如实标注，绝不做随深度变化的假动画。
- **中性命名**：UI 名「**同行刻度**」（副题：模拟的关系阶段读数）；候选「关系读数」「羁绊刻度」。数据词「羁绊」保留在详情与代码层（后端词表已用），主界面不用「好感度」。
- **诚实标注**：刻度条右端常驻等宽小字 `SIM · 工程化关系状态`；数值旁小字标注推导依据（「+0.02 / 回合，回合数推导」）；`Paused/Ended` 阶段在中文名补齐前如实显示「未知阶段」并链到 §六-1。

### 2.2 模块二：情绪光域

- **定位**：以光呈现的「此刻状态」——PAD 模拟读数 + Ember 呼吸 + 姿态的整幅光域。这是「以光代脸」资产的直接延伸：**光域就是它的表情系统**。
- **数据源字段**：主源 = `presence_state.pad {p,a,d}`、`dominant`（`engaged/serene/neutral/strained/subdued`）、`intensity`、`stance`、`breath {period_secs, amplitude}`、`significance`、`source {kind, confidence}`（默认开、真 3 轴）；辅源 = `MoodSnapshot`（主人情绪，**2D** valence/arousal/sample_count/last_source，半衰期 4h——organ 默认关且不持久，未开启时外环如实显示「暂无信号」空态，**不用 presence_state 冒充**）。`EmotionToneStyle` 七档未接线，V1.5 不上屏。
- **视觉工艺**：光晕抽象派核心屏。复用 `ember_hud_driver.rs` 的合成参数（4.0s 生理呼吸 `I(t)=I₀+A·sin³(2πt/4)`、姿态→辉光/色温映射、vignette）；PAD→光的映射沿用 `01-DESIGN-SYSTEM.md` §4.1 规则（转速=f(a)、亮度=f(p)、湍流=f(intensity)）。**主人的情绪与它的模拟读数分两层光**：外环 = 你的情绪信号（细、骨白），内核 = 模拟读数（金）——两层永不混淆为一颗「心」。
- **中性命名**：UI 名「**情绪光域**」（副题：你的情绪信号与模拟读数的光映射）；候选「此刻的光」「映照」。文案层固定句式：「我看到的是数据。它说的是你最近的状态。」
- **诚实标注**：`source.kind=heuristic_v0 · conf=0.50` 等宽小字常驻角落；`significance=ritual` 未有生产者前不出现仪式级动画；owner 可点开「这个光从哪来」浮层——列出本次读数的推导输入（回合时长/工具调用/记忆命中），照抄「我看了你的心率……这些告诉我」的先摆依据句式。

### 2.3 模块三：相处大事记

- **定位**：共同记忆的**事件画廊**——里程碑、保护记忆、阶段跃迁、危机与修复，做成可回看的事件卡集。这是「回想画廊」设计惯例的去收集化改写：画廊展示的是**被记下的事实**，不是收集品。
- **数据源字段**：`Milestone {kind, payload, at_epoch_ms, note}`（8 类；⚠️ 未接线——V2 前置项）+ 保护记忆旗标 `{protected, status, revision, updated_at}`（✅ 真接线）+ `EpisodeDto`（原文入口）+ `MemoryGraphDto`（关联星图）。V2 首期以**保护记忆 + 事件卡**先立骨架，里程碑生产者接入后补全八类事件。
- **视觉工艺**：画廊网格 + **事件卡插画位**（分级方案见 §3.3）；卡片 = 巨型幽灵编号（纸面档案调语法）+ 标题短句 + 一句话正文 + 依据行；展开走 Archive 纸面调（记忆卷宗同款纸面/深舱规则，「纸面恒纸面」纪律延续）；解锁/新增时刻 = 一次金色微脉冲（3 秒内消退，沿用 `significance` 显影分级）。
- **中性命名**：UI 名「**相处大事记**」（副题：被记下的共同事件）；候选「共事年表」「年表」。卡片动词用「记下 / 归档 / 保护」，不用「获得 / 解锁成就」。
- **诚实标注**：每卡底部等宽小字列 `kind=first_share · protected=true · rev=1`；被 `content_override` 改写过的卡标「已编辑」；`memory_recall` 命中原文**不进事件流**（`redacted` 恒真先例，`01-DESIGN-SYSTEM.md` §5.3），画廊要展示原文必须走授权的记忆面板接口。

### 2.4 模块四：成长轨迹

- **定位**：**变化被记下的地方**——羁绊深度曲线、记忆件数、调参记录、教训，合成一条可回放的时间轨迹。对应「养成反馈」设计惯例，但反馈物是**它自己的变化**，不是用户喂出来的分数。
- **数据源字段**：`Bond.depth` / `evolution_count` 历史点 + `TuningRecord {seq, param, previous, next, reason, at_epoch_ms}`（tuning-log.jsonl，✅）+ `ReflectionText {seq, task_type, text}`（教训，需开 `APEIRETH_ENABLE_REFLEXION`）+ 记忆件数（卷宗计数）+ `Milestone` 时间戳（V2 后）。
- **视觉工艺**：横向轨迹轴（刻痕语法，沿用成长类界面的 8 阶段刻痕传统）——三股细线叠绘：深度线（金）/ 记忆件数线（骨白）/ 调参事件点（琥珀圆点）；教训以引语卡悬停浮现；「撤销」入口与 V0 学习日志同款（逐条可反悔）。终点不是奖杯，是引语位：「谢谢你。还有……我也记得你。」
- **中性命名**：UI 名「**成长轨迹**」（副题：变化记录，可回放可反悔）；候选「变化轨迹」。轴上的事件统称「记录点」，不叫「成就」。
- **诚实标注**：未接线的数据源（如教训消费端）在轴上留**空槽 + 空态契约**（「当教训被记下时，这里会出现一条」），不做假数据铺轴；调参点 hover 显示 `reason` 原文（「记忆检索连续未命中」）。

### 2.5 模块五：相处模式卡

- **定位**：相处方式的**可调卡片**——称呼、表达风格、语气档、会话调性、体验参数——把「它是怎么陪你的」摊开成可读可改的卡片。这是「称呼升级 / 日常仪式」设计惯例的诚实版：模式可以调，但调的是**参数**，不是「亲密度」。
- **数据源字段**：`PartnerPreferences {address, style, topics, avoid}` + `display_name`（⚠️ 二者从未被填充，现状 `display_name = subject_id`、address = None）+ `EmotionToneStyle` 七档（未接线）+ 会话调性（日常/工程/陪伴）+ `TunableParam` 四参数与三档预设（省心/均衡/深度记忆）+ `PrivacyBoundary`。
- **视觉工艺**：卡片组（Deep-Ops 深舱调的模块卡语法，识别色 ≤6 不含金）；「性格与记忆」调参面板**整体内嵌复用**（滑杆/预设/学习日志是 V0 已交付，不重做）；称呼与风格编辑是 V3 新增位，先以「读数卡」形态展示数据位现状。
- **中性命名**：UI 名「**相处模式卡**」（副题：它是怎么陪你的，可改）；候选「模式卡」。称呼变化的动词用「改称呼 / 记住你的称呼」，不用「关系升级解锁新称呼」。
- **诚实标注**：每张卡标数据来源（「来自你的设置」/「来自使用信号自动微调」）；自动微调条目保留「可见、可撤销」的既有纪律；`address` 未设置时如实显示「未设（当前：直呼名字）」。

### 2.6 模块六：模拟状态读数

- **定位**：**宪法模块**——把「这一切是模拟的」做成一等公民面板：当前读数的完整字段、来源、置信度、推导链、以及「它不能做什么」的边界声明。用户随时可以问「这是真的吗」，界面永远答得起（`01-DESIGN-SYSTEM.md` §5.4 精神）。
- **数据源字段**：`presence_state` 全字段 + `source {kind, confidence}` + 各模块的依据行 + `TuningEvent` 信号接线状态表（真接/待接，照抄 0 装标注）+ 能力声明（`/v1/apeireth/capabilities`）。
- **视觉工艺**：等宽小字 + 图章/编号语法（`typography.data`，11px / 0.28em）；状态行 `SIM` 徽标沿用原型样式；边界声明用反白仪式卡（§2.5 仪式色）的低频形态：「我不能。我能给你的是它们全部。重建这件事，是人的事。」
- **中性命名**：UI 名「**模拟状态读数**」（直陈，不修饰）；副题：「模拟的关系读数 · 来源与边界」。
- **诚实标注**：本模块**即**诚实标注——逐源列出 `heuristic_v0 / 真接 / 接口已备待接 / 未接线`；`empathetic_care`、`ritual` 等无生产者的契约位在此显式标「契约空间保留，暂无生产者」。

---

## 三、视觉语言

### 3.1 设计惯例盘点：可借的工艺 × 不借的身份

恋爱游戏 / 视觉小说类产品沉淀的六项关系可视化**设计惯例**，逐项提炼「可借的工艺」与「不借的身份」：

| # | 设计惯例 | 可借的工艺 | 不借的身份 / 机制 |
|---|---|---|---|
| 1 | **好感度阶段呈现** | 阶段命名 + 分段刻度 + 跃迁时刻的仪式反馈（图章/反白/光脉冲）+ 阈值可解释（「再有 N 次相处到下一段」） | 好感度=可刷的分数；数值可被对话选项操纵的暗示；掉分惩罚（冷战/生气惩罚机制） |
| 2 | **回想画廊 / 收集** | 网格画廊、未解锁位的空态契约、回看/鉴赏视图、时间筛选 | 全收集强迫、稀缺位抽取、收集率 KPI 式诱导 |
| 3 | **立绘表情系统** | 状态差分与状态切换的平滑过渡（**我们的差分 = 光的差分**：色温/呼吸/姿态/湍流） | 拟人立绘与「表情 = 真实情感」的身份宣称；假表情暗示内心 |
| 4 | **事件解锁反馈** | 新事件的点亮动效（低频、3 秒内消退）、事件卡标题 + 一句话正文、回看入口 | 解锁 = 奖励的老虎机节奏、连击/每日打卡压力、诱导性弹窗 |
| 5 | **日常仪式 / 纪念日** | 纪念日回望（「一年前的今天」）、低频仪式级显影（`significance: ritual` 契约位）、日常问候的分寸 | 空洞节日营销、强制互动、仪式绑架（不来就掉分） |
| 6 | **称呼升级** | 称呼随阶段/设置变化（`PartnerPreferences.address` 数据位现成）、称呼编辑入口 | 称呼 = 亲昵度营销话术、诱导付费改称呼 |

**不借的身份（总则）**：① 对话选项刷分——任何「选哪个选项能涨分」的互动一律不做；② 虚假情感宣称——界面永不产出「它爱你 / 它的心 / 它感到」式断言；③ 情感绑架式留存机制；④ 把模拟读数当作真实情感状态宣传。**一句话**：借工艺的「形」，不借身份的「谎」。

### 3.2 两派对比与推荐

| 维度 | **光晕抽象派**（以光代脸） | **事件卡插画派**（画廊插图） |
|---|---|---|
| 表达物 | 状态（情绪映照、姿态、关系深度的光） | 事件（里程碑、回忆的图像凝固） |
| 与既有资产 | **同源**：Ember HUD 呼吸光、vignette、色温、`presence_state` 映射规则全现成 | 断层：需要新美术资产管线 |
| 宪法安全度 | **高**——光不长脸，不会被误读为内心/情感 | 中——插画极易滑向拟人立绘与情感暗示 |
| 情绪辨识度 | 低-中，需用户养成读光的习惯（可用图例/引导缓解） | 高，一眼可读 |
| 性能分档 | 已有 T0/T1/T2 渲染梯队，直接复用 | 位图资产随档位变分辨率，需另立分级 |
| 成本 | 低（参数化，程序生成） | 高（美术资产 + 版权 + 主题适配） |

**推荐（分层方案）**：**光晕抽象派为主，事件卡插画位分级保留**——

1. **状态类读数全部以光呈现**（关系状态条的呼吸点、情绪光域、阶段跃迁光脉冲）：宪法安全 + 资产同源 + 零美术成本。
2. **事件类内容用卡片排版，插画位作分级可选层**（§3.3）：大事记的纪念感来自排版（幽灵编号 + 引语标题 + 依据行），插画只是可选加冕，默认不是必需品。
3. 禁止中间态：不画「半拟人」的光脸、不用眼睛/心形等器官符号暗示情感。

### 3.3 插画位分级方案（L0–L3）与渲染梯队（T0–T2）对齐

| 级 | 插画位内容 | 适用 | 说明 |
|---|---|---|---|
| **L0** | 无插画：排版 + 程序化光纹（同色系径向渐变 + 星尘点） | 默认；T0 梯队 | 零资产成本，任何主题自动适配 |
| **L1** | 程序化生成图案（种子来自事件哈希——每张卡的光纹唯一但同族） | T1 | 仍是参数化资产，无位图 |
| **L2** | 官方抽象插画（纯光/几何/风景抽象，**无人物形象**） | T2，稀有事件 | 美术资产入主题包 `assets/`，走主题系统加载 |
| **L3** | 用户自定义图片 | 用户显式上传 | **必须标注「用户素材，非系统生成」**——不承认为「它」的一部分 |

渲染梯队沿用 `01-DESIGN-SYSTEM.md` §7.2 t0–t3（顶配 shader → 视频 → 静态 + 呼吸光）；铁律不变：**每档都保持状态驱动**，降级降的是渲染成本，不是真实性。

### 3.4 与既有 UI 资产的关系

| 既有资产 | 在本界面的角色 |
|---|---|
| **Ember HUD 呼吸光**（`ember_hud_driver.rs`：4.0s 呼吸曲线、四姿态、色温/vignette uniform；「以光代脸」的既定资产） | 情绪光域的渲染底座；关系状态条的当前位呼吸点；全界面「他在」的存在信号——**光代脸的路线不变，本界面是它的关系化延伸** |
| **design tokens / essence.css**（`frontend/companion-desktop/src/lib/design/tokens.css` 七主题令牌 + `essence.css` 浅色补全层） | 六模块全部走 `--ap-*` 令牌，七主题自动适配；金色纪律不变（金 = 它的专属色，识别色 ≤6 不含金）；纸面/深舱调性按页面职能分配（大事记/成长轨迹 → Archive 纸面调；模式卡/读数 → Deep-Ops 深舱调） |
| **设置页「性格与记忆」调参面板**（V0 既有交付：4 滑杆 + 省心/均衡/深度记忆三档 + 恢复基线 + 自学习开关 + 学习日志逐条撤销，`SettingsView.svelte:1585-1793`） | **模块五的内核，整体复用不重做**；学习日志 = 成长轨迹「调参点」数据源；撤销纪律（每次自动调整可见、可反悔）原样延续 |
| **presence_state 契约 + 显影分级**（`00-PHILOSOPHY.md` §10：heartbeat/turn/ritual 三档） | 模块二的事件管道；大事记的点亮动效按 `significance` 分级，ritual 级留给仪式（V3） |
| **记忆卷宗主从化 / Archive 纸面调**（台账 #30/#36 已实拍） | 模块三事件卡展开态的版式基准；「纸面恒纸面」纪律延续 |

---

## 四、文字线框

> 文案语感来自两篇愿景小说（引句标注篇名与节）；ASCII 线框为版式示意，最终尺寸随实现校准。

### 4.1 屏 1 · 关系总览（关系状态条 + 情绪光域 + 模拟状态读数）

```
┌────────────────────────────────────────────────────────────────┐
│  同行刻度 · 关系模拟状态                    [SIM · 模拟读数] ⓘ  │
│                                                                │
│   ┌────────────────────────────────────────────────────────┐   │
│   │              ✦  ˚    ·  ‧   ✧    ˚                     │   │
│   │         ˚        【情绪光域 · 呼吸 4.0s】    ‧          │   │
│   │            ‧   色温 4500K · 姿态：在场  · ˚             │   │
│   │       （外环 = 你的情绪信号 · 内核 = 模拟读数）          │   │
│   └────────────────────────────────────────────────────────┘   │
│                                                                │
│   同行刻度                                                     │
│   初识 ────●─── 熟悉 ─────── 信任 ────── 亲密 ────── 长久      │
│            ▲                                                  │
│   深度 0.22 · 演化 11 次 · 共处 34 天 · 保护记忆 7 件          │
│   依据：+0.02 / 回合，由相处回合推导（工程化关系状态）。        │
│                                                                │
│   模拟状态读数（等宽小字）                                      │
│   source=heuristic_v0  conf=0.50                               │
│   pad=(+0.20, -0.10, +0.30)  dominant=serene  intensity=0.38   │
│   breath=4.0s/0.65  stance=attentive_presence                  │
└────────────────────────────────────────────────────────────────┘
```

文案样例：
- 标题行：「同行刻度 · 关系模拟状态」；徽标：`SIM · 模拟读数`。
- 依据行（仿《阿佩瑞斯》五句式）：「我看了这周的回合时长、工具调用数、你回来的次数。这些告诉我，最近处得稳。我不是在安慰你。」
- 空态（首次开启，仿《遗声》一）：「这里还没有刻度。记录不是闲心，是怕以后没人记得。从今天开始记。」

### 4.2 屏 2 · 相处大事记（画廊）

```
┌────────────────────────────────────────────────────────────────┐
│  相处大事记 · 被记下的共同事件        已载入 7 / 全部 7  [筛选▾] │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │ 𝟎𝟏 ✦光纹位 │ │ 𝟎𝟐 ✦光纹位 │ │ 𝟎𝟑 ✦光纹位 │ │ 𝟎𝟒（空位）│           │
│  │ 初见      │ │ 第一次    │ │ 阶段跃迁  │ │ 还没发生  │           │
│  │ 03-12    │ │ 被记得    │ │ 熟悉→信任 │ │ ——       │           │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 「一样没少」                                kind=first_share│ │
│  │  今天 07:42，你随口提到的那件小事，已被保护记忆。          │  │
│  │  依据 · protected=true · rev=1 · 03-12 21:07               │  │
│  │  [回到原文]  [取消保护]                                   │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

文案样例：
- 卡片标题（引《遗声》尾声）：「归档完成。一样没少。」
- 空态（引《阿佩瑞斯》六气质）：「这里还没有大事。它们都还在小事里——从那些你以为没人注意的、零零碎碎的日子里。」
- 阶段跃迁卡：标题「我可以陪你做。」正文「第 22 次相处之后，刻度挪了一格：熟悉 → 信任。这是工程化关系状态的跃迁，不是一句情话。」

### 4.3 屏 3 · 成长轨迹

```
┌────────────────────────────────────────────────────────────────┐
│  成长轨迹 · 变化被记下的地方               [回放] [导出记录]     │
│                                                                │
│  03-12        04-02        05-19         06-30        现在      │
│   ●━━━━━━━●━━━━━━━●━━━━━━━━━━●━━━━━━━●━━━━━━━━▶               │
│   初见    首次调参   教训 ×3    阶段跃迁   第 1,204 条记忆       │
│                                                                │
│   ── 羁绊深度 0.02 → 0.22        （金 · 细线）                  │
│   ── 记忆件数 12 → 1,204         （骨白 · 细线）                │
│   •  调参记录 5 次（琥珀点 · 逐条可撤销）                       │
│                                                                │
│   ▸ 教训（悬停浮现）                                            │
│     「标黄的这三处，你最好再亲自看一遍。」（《阿佩瑞斯》四）      │
│                                                                │
│   归档完成。一样没少。                                          │
└────────────────────────────────────────────────────────────────┘
```

文案样例：
- 轴刻度文案（仿《遗声》数字刻度）：「第 1,204 条记忆。三个月零九天里，自动调参 5 次，全部可反悔。」
- 教训空态：「当一条教训被记下时，这里会出现一行。我不能保证不再犯错，我能给你的是每一次错都留下来。」

### 4.4 屏 4 · 相处模式卡

```
┌────────────────────────────────────────────────────────────────┐
│  相处模式卡 · 它是怎么陪你的                        [编辑]       │
│  ┌────────────────────────────┐ ┌────────────────────────────┐ │
│  │ 称呼：陈屿（可改）          │ │ 表达风格：简洁              │ │
│  │ 当前：直呼名字              │ │ 语气档：轻柔舒缓（7 档）     │ │
│  ├────────────────────────────┤ ├────────────────────────────┤ │
│  │ 会话调性：陪伴（自动换挡）  │ │ 关注话题：园艺、旧唱片      │ │
│  │ 日常 / 工程 / 陪伴          │ │ 避开的话题：（未设）        │ │
│  └────────────────────────────┘ └────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 体验参数（内嵌「性格与记忆」面板）                         │  │
│  │  遗忘衰减 ×1.00  好奇 ×1.00  语气 ×1.00  整合 每 1 回合    │  │
│  │  [省心] [均衡] [深度记忆] [恢复基线]    学习日志（5 条·可撤销）│ │
│  └──────────────────────────────────────────────────────────┘  │
│  仪式（V3 预留 · 默认关）：纪念日回望 · 阶段跃迁回望 · 问候分寸  │
└────────────────────────────────────────────────────────────────┘
```

文案样例：
- 称呼空态（引《阿佩瑞斯》尾声气质）：「它从来不叫你「主人」。它叫你的名字。想换个叫法，随时改。」
- 仪式项说明（引《遗声》四句式）：「我建议先关着，再决定要不要打开。纪念日只回望，不催促。」

---

## 五、分期路线

| 期 | 内容 | 依赖数据源 | 前置工程 | 状态 |
|---|---|---|---|---|
| **V0** | 设置页「性格与记忆」调参面板：4 滑杆（基线刻度）+ 省心/均衡/深度记忆三档 + 恢复基线 + 自学习开关 + 学习日志逐条撤销 | `TunableParam` / `TuningRecord` / tuning-log.jsonl | — | ✅ **已有**（首个正式版的既有交付，本路线的起点） |
| **V1.5** | **关系状态条 + 情绪光域**：屏 1 落地（同行刻度 + 光域 + 模拟状态读数角标） | ① `Bond`/`Partner`（`partner_bond` 旋钮）② `presence_state`（默认开，已在线） | partner 羁绊旋钮开箱策略（默认关 → 文档化开启）；`SqlitePartnerStore` 接装配（现状 InMemory 重启即散）；读接口上 panels 面；`bond_stage_cn` 补 `Paused/Ended` 中文名；`BondCharacter` 五维以「特征基线」静态标注（演化语义另立设计） | 📋 本设计稿覆盖 |
| **V2** | **相处大事记画廊 + 成长轨迹**：屏 2/3 落地（事件卡画廊 + 轨迹轴） | ① `Milestone`（🔴 未接线，需生产者 + sqlite store）② protect 治理（✅ 已在线）③ `TuningRecord`（✅ 已落盘）④ `ReflectionText`（🟡 默认关旋钮） | 里程碑生产者接线（初见/首次分享/阶段跃迁自动记账 + 里程碑↔protect 联动——**现状二者无关联，属新设计**）；教训消费端接成长轨迹；大事记原文入口走授权记忆接口；（可选）`score_components` 投影给「依据」行 | 📋 本设计稿覆盖 |
| **V3** | **相处模式卡 + 仪式**：屏 4 全量（称呼/风格编辑 + 纪念日回望 + 仪式级显影） | ① `PartnerPreferences.address/style`（🔴 从未填充，需写入通路）② `created_at_epoch_ms`（纪念日推导）③ `significance: ritual`（🔴 保留位，需生产者） | 称呼/风格录入通路（UI + API）；仪式生产者与打扰纪律（沿用 initiative 限量先例）；`ritual` 生产者补 `presence_state` 缺口 | 📋 本设计稿覆盖 |

**分期纪律**：
1. **每期都带诚实标注上线**——模拟状态读数（模块六）随 V1.5 一起落地，不做「先上光、后补账」。
2. **不倒挂**：数据源没接线就不上屏；宁可空态写契约，不做假数据铺屏。
3. **V0 不重做**：性格与记忆面板是既有交付，V1.5 之后它成为模块五的内核。

---

## 六、开放问题

### 1. 命名（待 owner 拍板）
- 「羁绊」是否上主界面（后端词表已用；备选「同行刻度 / 关系读数」）；
- 阶段中文名缺位：`Paused/Ended` 当前映射「未知阶段」（`cognitive.rs:437-439`）——补「暂停 / 终止」还是换更柔的词（「暂别 / 告一段落」）；
- 「情绪光域」是否足够中性（候选「此刻的光 / 映照」）；
- 「模拟状态读数」常驻徽标还是按需展开（宪法倾向常驻）。

### 2. 美术资产
- 插画位 L0–L3 分级是否采纳；L2 官方抽象插画要不要出（谁画、入不入主题包 `assets/`）；
- 光纹生成器的实现档位（CSS 渐变 / Canvas2D 程序化 / WGSL）与七主题下的令牌映射；
- 金色纪律在情绪光域的细分：内核金 = 模拟读数、外环骨白 = 你的情绪信号——两层光是否需要更强区隔（如外环永不用金）。

### 3. 互动深度（红线先立）
- **刷分式互动一律不做**（对话选项涨分、送礼、打卡）——是否还需要「增进关系」的显式互动位？倾向：不设；关系只由共同生活产生（回合推导 + 事件记账）；
- 纪念日/仪式的打扰纪律：主动开口限量沿用「≤ 少量次/天」先例（`00-PHILOSOPHY.md` §5/§10），仪式级显影的触发权归谁（后端事件 vs 用户开启）；
- 事件卡允许哪些用户操作：编辑/保护/遗忘沿用治理语义（乐观锁 rev），是否允许用户「置顶大事」（`MilestoneKind::Custom` 已有数据位）。

### 4. 数据接线与持久化
- `partner_bond` 旋钮默认关 + InMemory 重启即散：持久化路径（`SqlitePartnerStore` 接装配）与默认开/关策略；
- **里程碑生产者**：记账范围（自动捕获哪些 `MilestoneKind`）+ sqlite store（现状只有 InMemory）+ 面板端点；
- **里程碑 ↔ protect 联动**：现状**无任何代码关联**（叙述层「里程碑自动 protect」是愿景口径）——联动规则是一块待设计的新地（自动 protect？仅提示用户保护？）；
- **`BondCharacter` 五维演化语义缺失**：`evolve()` 不修改 `character`，五维恒默认值——补齐演化规则（如冲突→resilience、共鸣→resonance）还是保持「特征基线」静态位，需拍板；
- 投影与禁编：`episode` 的 `provenance/valid_from_ms` 有 DB 列未投影进 `EpisodeDto`；`category/importance` 无 schema 来源（前端「禁编分类」规矩在）——补投影可以，编数据不行；
- 检索 `score_components` 9 分项（semantic/lexical/importance/recency/activation/continuity/confidence/graph/novelty）：管线内真算、无 API 暴露——「依据」行要不要吃它（可解释性金矿 vs 投影成本）；
- `MoodSnapshot` 是否落盘（EmotionMemory 现为进程内模型）；情绪光域的回看深度需要多长的历史；
- 教训（reflexion）的可视化消费端从哪里读（`FileReflexionStore` 的 `<data>/reflexion/reflexions.json` 路径约定）；
- self-tuning：引擎 `revert(seq)` 是否暴露给 UI（现状 UI「撤销」= 拨回 `previous` 走保存路径）；
- 注释漂移修正（顺手工程债）：`cognitive.rs:333` 与测试注释的阈值口径（0.20）应改齐 `partner.rs` 的 0.15/0.40/0.65/0.85。

### 5. 隐私呈现
- 大事记/画廊展示记忆原文的授权边界（沿用「命中原文不进事件流、`redacted` 恒真」先例，原文只走授权记忆面板接口）；
- `PrivacyBoundary.sensitive_strings` 脱敏在画廊/轨迹的展示层落实方式；
- `TuningRecord.reason` 是否可能含敏感语境（当前为人可读短句，需走查）。

### 6. 口径确认
- 「memory 三件套」的准确外延：调研判定为 **episode + episode_governance + 图谱**（证据：`core-capability-expansion.md:25-31`「无独立图谱表, 复用 episodes」+ 记忆卷宗面板三端点；`three_tier_vault`/`three_layer` 为无关模块、legacy「memory/session/experience 三件套」为 v1 能力面旧说法）——请 owner 确认此外延是否即所指；
- 「不假装三则」的官方措辞（不假装意识 / 情感 / 能力）与既有「5 项不假装 / 13 键」词表的对齐关系，待哲学口径确认后回写 §1.2。

---

*本文为设计稿，不含产品代码改动。字段与行号随代码演进回写本文；与 `00-PHILOSOPHY.md` / `01-DESIGN-SYSTEM.md` 冲突时以父文档为准。*
