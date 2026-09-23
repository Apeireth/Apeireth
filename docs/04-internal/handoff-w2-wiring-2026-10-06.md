# W2 接线工作线交接文档 (handoff-w2-wiring-2026-10-06.md)

> **给谁看**: 接续本工作线的人或 AI。**你不需要读过任何对话记录 —— 本文自足**。
> **本线是什么**: 把"库级已实现、但没接进生产装配"的模块逐个接进生产
> (旋钮 + 默认关测试 + 效果可见测试 + 四级口径文档 + 台账条目), 顺带核销历史挂账、
> 修复过程中咬出的真缺陷。
> **上位文档**: 工作包定义在 `engineering-review-handoff-2026-10-06.md` §5 W2;
> 接线真相表在 `docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md` (§7.1 三次复核 + §7.2 四审)。
> **验证权威**: `docs/04-internal/live-verification-ledger.md` (写"已验证"之前必查)。
> **状态 (2026-10-06 夜)**: 6 组已接线 ✅ / 4 组余下 (§4 路线) / 2 项挂账卡客观条件 (§3.3)。

> **接手批注 (2026-10-10, 资深工程师线)**: 本线连同 W4/W5/安全线统一由我接管 (主人指示: 另一 AI 已停, 我负责一切工作)。**§4.1 dreaming 已落地** (决策拍板: D1 = LLM 思考器 `dream_llm::LlmMetaThinker` 经 LlmFactory 桥 + 确定性降级链(降级留痕) / D2 = CLI 显式命令 `apeireth dream` (免旋钮=显式授权) / D3 = `DreamReport::to_markdown()` 落 DiaryStore; 理由全文见 `crates/engine/memory/src/dream_wiring.rs` 头注) —— 详见台账 #45, §3.1/§4.1 已同步。同日: W3 报告 §9 修正 (沙箱 v1 翻案, 与 §2.6 对账一致)、W4①② 凭据收口 (真治理 hook + 审计真落档)、Cargo.lock 补漏 (`f11ade68`)。**§5 四项红线仍等主人**, 未动。

---

## 0. 三分钟接手

1. **现在做什么**: §4 路线第 1 项 —— dreaming 接线 (设计决策点已列全)。
2. **动手前必读**: §6 方法论 (本线打法, 9 条纪律) —— 不读它你会重犯本线已经踩过的坑。
3. **铁律**: §5 红线 —— 4 项待主人拍板的事**不许代拍**。
4. **随手记**: 每完成一步 → `live-verification-ledger.md` 加行 + 本文 §3 状态更新 + 显式路径提交推送 (§6.7 并行作业约定)。

---

## 1. 起源与目标 (为什么有这条线)

2026-10-06 的能力差距审计 (`v1-vs-v2-capability-gap-audit-2026-10-06.md`) 发现: v2 有大量模块
**库级真实现但生产装配零引用** (四级口径: IMPLEMENTED ✅ / PRODUCTION WIRED ❌) ——
"距生产化只差装配 + 测试 + 门禁, 不是从零研发"。W2 工作包由此而生, 原文目标:

> 逐模块 (含 crate 内部调用) 核清真实接线状态并修正审计表, 再把确未接线的库级模块接进
> 生产装配 —— 每模块配默认关的配置旋钮 + 默认关行为不变测试 + 开启后效果可见测试 +
> 四级口径文档行 + 台账条目。

**每模块验收门** (缺一不算完成): ① 默认关旋钮 ② 默认关行为不变测试 ③ 开启后效果可见
测试 (断言效果出现在行为里, 不是 `field == Some`) ④ 能力文档四级口径行 ⑤ 台账条目。

---

## 2. 全部工作系统总结 (按提交落地, 每条可独立复核)

### 2.1 提交地图 (本线全部提交, 时间序)

| commit | 内容 | 一句话 |
|---|---|---|
| `e4f2f451` | 审计 §8 附录 A (v2 18 crate 源码清单) + 台账 #39 + `plugin/organ.rs` 陈旧表更正 | 源码级能力清单入档 |
| `909d67e9` | `engineering-review-handoff-2026-10-06.md` (结论复核矩阵 + 未验证 8 项 + W1-W8 工作包) | 独立审核交接包 |
| `2a43f9fc` | 交接包自校正 (B2 计数口径警告) | 自纠 |
| `ae116210` | 台账 #40 构建缓存清理 187.7 GB + 完好性验证 | 插曲: 磁盘救急 |
| `a79102ab` | 复核结论整合: **B1 自纠** (真宏调用 0 处) + 当期基线刷新 3418 | 复核批判定折入 |
| `9242f03d` | **W2 首批**: typed 写读对称 + 语义向量阶段真实现 + proactive 旋钮 (+12 测试) | 见 §2.2 |
| `acc70087` | **W2 记忆闭环批**: consolidation/reflexion/memory_injection (+7 测试) + 3 真缺陷修复 + 挂账核销四审 | 见 §2.3/§2.4/§2.5 |
| `da4836e6` | **核销收官**: council 假绿洞封堵 + shell 可达探针 | 见 §2.4 |
| `4b7c6719` | HANDOFF 现状横幅 + W2 进度注 + 并行作业约定 | 现状化 |

### 2.2 W2 首批 (`9242f03d`) — 三组接线, 其中一个是 bug 级修复

| 组 | 旋钮 (全默认关/默认不变) | 干了什么 | 关键文件 |
|---|---|---|---|
| **typed 写读对称修复** | `APEIRETH_DISABLE_TYPED_RECALL=1` 逃生门 (默认**开**); 身份 `APEIRETH_PERSONA_ID/SUBJECT_ID` (默认 `apeireth`/`local-user`) | **bug**: 写侧 `typed_sink` 一直在生产落库承诺/画像/关系, 读侧 `typed_recall` 恒 `None` = **入库永不召回**。接 `SqliteTypedMemoryRecallSource` (episodes 槽不接 —— 型别边界 `SqliteMemoryStore` vs `SqliteBackend`, episodic 候选已由 scoped_memory 供) + `with_identity` 激活 persona 写 | `adapters/cli/src/lib.rs`, `runtime-assembly/canonical/typed_recall.rs` |
| **语义向量阶段真实现** | `APEIRETH_EMBEDDING_URL` + `APEIRETH_EMBEDDING_MODEL` (+`_KEY` 可选); **双缺=词法回退不变, 只设其一=启动报错 (fail-loud), 双全=接通** | 此前全仓库只有 `NoEmbeddingProvider`+测试 fake, coordinator 向量阶段从未激活。新写 `OpenAiCompatibleEmbeddingProvider` (OpenAI `/embeddings` transport + 纯解析 + 7 单测) | `engine/provider/src/embeddings.rs`, `memory/src/scope.rs` (trait 边界) |
| **proactive_recall 补旋钮** | `APEIRETH_ENABLE_PROACTIVE_RECALL=1` | 已接线但 `Default=None` 且无任何旋钮可达 (审计第一起"接线假账") —— 补开关让已接线路径可达 | `adapters/cli/src/lib.rs` |

### 2.3 W2 记忆闭环批 (`acc70087`) — 三模块接线

| 模块 | 旋钮 (全默认关) | 干了什么 | 关键文件 |
|---|---|---|---|
| consolidation 触发点 | `APEIRETH_ENABLE_CONSOLIDATION=1` | writeback AfterTurn 跑 `coord.run_consolidation` (确定性, 0 模型调用), 洞察**稳定 ID** 落库 (跨轮幂等), 下轮召回可见 | `runtime-assembly/canonical/cognitive.rs` (MemoryWritebackModule) |
| reflexion 失败闭环 | `APEIRETH_ENABLE_REFLEXION=1` + `APEIRETH_REFLEXION_DIR` (默认 `<data>/reflexion`) | 新模块 `cognitive.reflexion`: TurnStart 按任务标签注入历史教训 (字符预算内, 0 LLM) + AfterTurn 消费 `JudgeObservations` 的**显式**非 Pass 判定 → 沉淀 `FailureKind::DecisionRejected` + `RuleCritic` 即时蒸馏。**信号边界**: Judge 未开 = 无信号 = 不记录, 绝不从文本猜"失败" | `cognitive.rs` (ReflexionModule), `memory/src/reflexion.rs` |
| memory_injection 反幻觉格式 | `APEIRETH_ENABLE_MEMORY_INJECTION=1` | overlay 渲染切换 donor 封闭世界格式: 编号证据清单 + 「禁止说『我记得我们以前聊过』」反幻觉规则 (默认 XML 格式不变) | `memory/src/context_compiler.rs`, `memory/src/coordinator.rs` (`injection_format` 分支), `memory/src/memory_injection.rs` |

### 2.4 过程中咬出的 3 个真缺陷 (全部修复, 各有回归测试)

| # | 缺陷 | 怎么咬出的 | 修法 | 回归测试 |
|---|---|---|---|---|
| 1 | **FakeMemory 半真 fake**: 写侧真、治理视图 `governed_recent_episodes` 恒返空 | consolidation 效果测试拿到空洞察集 | fake 治理视图从 episodes 如实派生 (全 Active) | `consolidation_is_opt_in_and_persists_insights_idempotently` |
| 2 | **consolidation 洞察自我增殖**: 落库洞察含 "resolved" 标记词, 下轮被再提炼出级联副本 (实测 2 轮 4 个 ID) | 同上测试的幂等断言从 1 变 4 | `consolidation.rs` 提炼只吃**原始证据角色** (user/assistant/tool), 派生记忆不入料 | 同上 (第二次跑 ID 集合不变断言) |
| 3 | **council_live 假绿**: 断言把 `DeferToHuman` 包进"合法出口", 7 advisor 全败恰好汇聚成它 —— 撤销 key 下 0.67s 假绿 (真跑历史 5.0s) | 三支 live 实跑时注意到 0.67s 过快, 打开测试体验明正身 | 测试开头加**最小真 LLM 往返探针**, 通道死显式炸 (同 key 对照: 假绿→诚实红) | `council_7_advisor_live_decide` (现诚实红) |

**教训已固化进 §6**: 效果测试必须能咬缺陷 (§6.3); live 测试先证通道活着 (§6.4); 派生记忆不入料 (§6.5); fake 保真度是债 (§6.6)。

### 2.5 挂账核销成果 (8 项欠账清 6, 详见台账 #43/#44)

| 项 | 结果 |
|---|---|
| legacy 名称级扫描 → **源码级核验** (9 子系统逐个读实现体) | 见 §2.6 翻案表 |
| `thought_cluster` 没查清 | ✅ = v2 `cluster_store` **完整改名移植** (API 五件套同构) —— **从真缺口移出** |
| Option 默认关配置全量排查 | ✅ 47+65 字段全过筛: 行为开关类**全部有旋钮** (本线四补后闭合); 4 个可达性观察项记表 (§3.4) |
| shell 可达实测重放 | ✅ 探针 `capabilities/tools/tests/shell_reach_probe.rs` (`#[ignore]`): `cmd /C type %SystemRoot%\win.ini` exit=0 全文可见 = **全盘可达零拦截实锤**。**它同时是 W1 沙箱的回归锚点** —— 沙箱落地后断言必须反转 |
| `jimmy` 死指针 | ✅ 删前复验 (Repository not found) 后删除 |
| 真机点击流 | ⏳ 需主人人工 (保持挂账, 不可单方核销) |
| live E2E 三支 | ⏳ 见 §3.3 —— council 洞已封; provider/organ 卡 **key 被撤销** |
| 主账文档修订 | ✅ 已由并行线完成 (`5e894182` W8 文档治理批) |

### 2.6 legacy 九子系统翻案表 (审计 §7.2 四审表的浓缩; 证据行号见原文)

| 子系统 | v1 实况 | 定性修正 |
|---|---|---|
| community | ✅ 真 (`companion/community.rs` + `triage()`) | 真缺口成立 (v1→v2 未移植) |
| experiment_field | ✅ 真框架 (`ExperimentField` + VMRunner 注入边界 + 提案→部署→回滚) | 真缺口成立 (框架可回收) |
| **HybridCognitiveRouter** | ❌ **v1 也没有** | **纯愿景项** (原"v1 有真实现"是错的) |
| **ToolSynthesizer** | ❌ **v1 也没有** | **纯愿景项** |
| thought_cluster | ✅ 真 (`ThoughtClusterManager` + Reader + search) | **已移植** (= `cluster_store`), 移出缺口 |
| onering | ✅ 真 (`OneRingLedger` 统一账本 + 溯源强制 + 五前端时间线) | 真缺口成立 |
| 真文件/网络沙箱 | ✅ 真 (`frozen/apeireth-sandbox/real.rs` **真接 Docker daemon API**) | 真缺口成立, **v1 Docker 方案整 crate 可回收** |
| SDK 真 HTTP/WS | ✅ 大量真 (http-client / api ws_v1 / bus l4 / lark / voice / update) | 真缺口成立, **客户端群可回收** |
| 三洋葱 L3-L5 | ✅ **整 crate 真** (`donor/apeireth-onion` 双洋葱 L0-L5 全六层 + **Kani 形式化证明**) | 真缺口成立, 比原估计更广, **可回收度最高** |

---

## 3. 当前精确状态

### 3.1 接线矩阵 (四级口径; 手册 = INSTALL.md 旋钮区)

| 模块 | IMPL | WIRED | DEFAULT | 旋钮 | 效果测试 |
|---|---|---|---|---|---|
| BM25/hybrid 检索 | ✅ | ✅ (经 MemoryCoordinator, T7-T9) | ✅ | — | 既有 |
| typed 读侧 (承诺/画像/关系) | ✅ | ✅ (本线) | ✅ (对称修复) | `APEIRETH_DISABLE_TYPED_RECALL` | `memory_provider_e2e` + cli 测试 |
| 语义向量阶段 | ✅ (本线) | ✅ (本线) | ❌ (双缺=词法回退) | `APEIRETH_EMBEDDING_URL/MODEL/KEY` | T9 + embeddings 7 单测 + fail-loud 测试 |
| proactive_recall | ✅ | ✅ | ❌ | `APEIRETH_ENABLE_PROACTIVE_RECALL` | `proactive_recall_knob_is_opt_in` |
| consolidation | ✅ | ✅ (本线) | ❌ | `APEIRETH_ENABLE_CONSOLIDATION` | `consolidation_is_opt_in_...` |
| reflexion | ✅ | ✅ (本线) | ❌ | `APEIRETH_ENABLE_REFLEXION` / `_DIR` | `reflexion_records_judge_failures_...` |
| memory_injection | ✅ | ✅ (本线) | ❌ | `APEIRETH_ENABLE_MEMORY_INJECTION` | `memory_injection_format_switches_...` + 注入格式 3 单测 |
| dreaming | ✅ | ✅ (2026-10-10 接线) | 显式命令 (免旋钮) | `apeireth dream [--session <id>] [--limit N] [--date YYYY-MM-DD]` | `dream_writes_diary_entry_holding_the_report` 等 5 + `dream_llm` 4 + cli parse 3 |
| partner / principles | ✅ | ❌ | — | 待 §4.2 | — |
| morphology / education / worktree_sandbox | ✅ | ❌ | — | 待 §4.3 | — |
| 吸收批 (betti/residual_pyramid/river_topology/kuramoto) | ✅ | ❌ | — | 待 §4.4 | — |

### 3.2 测试基线

- 最近实测 (`acc70087`): **129 suites / 3425 passed / 0 failed / 19 ignored** + clippy `-D warnings` 0 警告。
- `da4836e6` 新增 `shell_reach_probe` (1 个 `#[ignore]`, 不入默认跑) —— 此后未重测全量; 以实测为准, 别引用旧数字当现状 (W8 已记文档债: 数字以台账为准)。
- 新测试分布: embeddings 7 单测 / cli 旋钮 6 / assembly 效果 3 / memory 注入格式 3。

### 3.3 挂账现状 (完整表见台账 §2 + 挂账 #8/#9)

- **⚠️ 主人行动项**: DeepSeek key 尾号 **3d17 已被官方撤销** (vendor 401) —— 桌面真聊天同样 401。换新 key 存入桌面设置后, 按台账挂账 #9 的命令补跑 provider/organ 两支 `-- --ignored`。
- embedding live E2E: 无可用端点 (DeepSeek 无 `/embeddings`) —— 有端点后按挂账 #8 验证 `used_lexical_fallback == false`。
- 真机点击流: 人工, 清单在 `frontend/companion-desktop/docs/first-run-click-through-checklist.md`。

### 3.4 Option 排查产物: 4 个可达性观察项 (非能力开关, 记表不加面 —— 没需求不加旋钮)

1. `memory_materializer/memory_extractor` 增强槽: 生产用默认确定性抽取, LLM 抽取器从未注入;
2. `JudgeConfig.model` 独立模型槽无 env (默认用当轮模型);
3. orchestrator `quiet_start/quiet_end_minutes` 安静时段无 env;
4. `RuntimeConfig` 三参数 (`max_rounds`/`approval_ttl_ms`/`max_module_invocations`) builder-only 无 env。

---

## 4. 后续路线 (按序, 每项照 W2 验收门做)

### 4.1 dreaming 接线 ✅ 已落地 (2026-10-10)

- **现状**: 6 状态机引擎真实现 (`memory/src/dreaming.rs`, `DreamEngine::execute_dream_cycle`), 内部调 `meta_thinking` + `procedural`。
- **三个设计决策点 (拍板或自行判断后记录理由)**:
  1. `MetaThinker` 生产实现选型: 目前**无生产 impl** (只有测试 fake)。选项: LLM 版 (经 `LlmFactory`/`InvokerLlmFactory` 桥, 与 organs 同模式, 真思考但有成本) vs 确定性规则版 (诚实但浅)。**建议 LLM 版 + 降级链** (符合"评审只降级"原则)。
  2. 触发载体: 空闲 15min 语义 (`DreamEngineConfig.min_idle_for_dream_ms`) 在模块钩子里**无法实现** (无回合=无钩子)。选项: CLI 子命令 `apeireth dream` (显式授权=无需旋钮, 最诚实) / gateway 空闲 watcher (presence.rs 有空闲信号) / admin 路由。
  3. 落库去向: `DreamReport::to_markdown()` → diary (`memory/src/dary.rs` DiaryStore) 是引擎设计本意 ("苏醒阶段写入日记")。
- **验收门**: 同 W2 五件 (若走 CLI 子命令, ①②改为"显式命令即授权 + 默认不自动跑测试")。
- **✅ 落地记录 (2026-10-10, 资深工程师线)**: 三点全部按建议拍板 ——
  ① D1 = `LlmMetaThinker` (装配层 `canonical/dream_llm.rs`, 经 plugin `LlmFactory` 桥, 私有 current-thread runtime 隔离同步口) + `FallbackMetaThinker` 确定性降级链 (降级产出带 `[LLM 不可用(…), 降级]` 留痕); 兜底 `DeterministicMetaThinker` 在 memory 层 (零 LLM 依赖)。
  ② D2 = **CLI 显式命令 `apeireth dream`** (理由: 显式授权最诚实、免旋钮、与引擎"只在被召唤时做梦"语义同构; gateway 空闲 watcher 留观察项)。验收门走命令变体 ✓ (显式命令即授权 + `dream_engine_is_pull_only_defaults_untouched` 默认不自动跑测试)。
  ③ D3 = `dream_and_journal` 胶水 (`memory/src/dream_wiring.rs`): 苏醒 `report.to_markdown()` 落 `FileDiaryStore` (`<data>/diary`, source=`dream`); 日记失败**不伪装成功** (Err 显式声明"周期已成但写档失败", 测试 `journal_failure_is_reported_honestly_without_faking_the_cycle`)。
  测试 +12 (memory 5 / assembly 4 / cli parse 3)。0 假装: `InMemoryProceduralStore` 固化载体重启即散 (报告留档为准), 持久习惯库属后续层。

### 4.2 partner / principles 接线

- **现状**: 零调用者确证 (crate 内部互调也零命中)。`partner.rs` (7 阶段伙伴羁绊模型), `principles.rs` (动态原则层 + 原则洋葱晋级候选)。
- **落点候选**: AfterTurn 写回 (羁绊状态更新) / TurnStart overlay (关系状态注入)。先读两文件头注释再定。

### 4.3 morphology / education / worktree_sandbox

- `morphology`: 已有温度旋钮 `APEIRETH_MORPHOLOGY_TEMPERATURE` (接线成本最低, 建议下一个做) —— 查询形态学 softmax, 落点疑为检索前置。
- `education`: `tools/src/education.rs` (Dx-Check 换元检查工具) —— 接为 `ToolCapability` 注册即成工具。
- `worktree_sandbox`: `orchestration/src/worktree_sandbox.rs` (git worktree 隔离) —— 落点疑为 subagent/Orchestrator。

### 4.4 吸收批 4 个 (betti_hole_detector / residual_pyramid / river_topology / kuramoto_resonance)

- 纯算法库 (研究吸收批)。接线点未读模块头前**不许猜** —— 各自头注释会写用途与边界 (仓库惯例)。

### 4.5 杂项

- 换 key 后 live 补跑 (§3.3) + W1 沙箱落地后**反转** shell 探针断言 (对偶探针)。
- W2 全部收口后: 更新差距审计 §7.1 处置列 + 台账 + 本文 §3, 然后本工作线可宣告完成。

---

## 5. 红线 (不许代拍 —— 这 4 项等主人)

| # | 事项 | 出处 |
|---|---|---|
| 1 | **W1 shell 沙箱轻量档开工** (设计已就绪: `docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md`) | 交接包 §7 |
| 2 | 认知深度档 council 顾问数 7 → 3 (延迟 vs 覆盖取舍) | 交接包 §7 |
| 3 | W3 九项真缺口的优先序 (注意四审后真缺口=7 项: 2 项已改判纯愿景) | 交接包 §7 |
| 4 | W4 default-off helper (colang/approval_policy/eval/evidence/rubric/risk) 是设计还是漏装 | 交接包 §7 |

另两条红线: **评审类机制只降级不枪毙主任务** (元层原则, 新增守门必写降级路径);
**0 装双向性** (不假装完成, 也不假装未完成)。

---

## 6. 方法论 (本线的打法, 继续者请遵守 —— 每条都是踩坑换来的)

1. **判"未接线"过三关**: ① 名字引用扫描 ② **crate 内部互调** ③ **`Option<...>` 配置的 `Default` 值**。本线三次修正审计结论, 全是只过第一关就下结论的教训 (hybrid_search/consolidation/proactive_recall)。
2. **0 装双向性 + 数数对源码**: 注释里的宏名不是调用 ("7 处 `unimplemented!()`" 实为 0 真调用 8 处文档提及); 数数要滤注释。
3. **效果可见测试必须能咬缺陷**: 断言效果出现在**行为**里 (`field == Some` 不算)。本线 3 个真缺陷全是效果测试咬出来的 —— 写测试时想清楚"它失败时代表什么"。
4. **live 测试先证通道活着**: 通道死必须显式红, **不许降级出口冒充绿** (council 假绿案)。任何 live 测试照 `council_live.rs` 的探针模式加护栏。
5. **派生记忆不入料**: 自己产出的派生物 (洞察/摘要) 不能再进自己的输入 (自我增殖案)。
6. **fake 保真度是债**: 半真 fake (写真读假) 会让效果测试空转; 新增 fake 时把 trait 的**每个**方法都写如实语义。
7. **并行作业约定** (多 AI 同树协作期): 动共享区前 `git status` 侦察; **显式路径暂存** (绝不 `git add -A`); 不跑全仓 `cargo fmt`; 提交窗口尽量短; 对方在途文件一个字不碰。
8. **push 例行**: `git -c http.proxy=http://127.0.0.1:7900 -c https.proxy=http://127.0.0.1:7900 push origin main`, 失败重试 3-4 次 (schannel 抖动); **永不暂存 `research/` (学术线) 与未跟踪 `artifacts/`**。
9. **文档账随手记**: 台账先行 (做完即记, 带复现命令); 一切"已验证"引用台账; 带日期的真账行**永不改写** (修订只加新行)。

---

## 7. 关键文件地图

**代码** (全部已带说明注释, 从头注释读起):
- `engine/provider/src/embeddings.rs` — 语义向量 transport (纯解析函数可单测)
- `engine/runtime-assembly/src/canonical/cognitive.rs` — `ReflexionModule` + writeback 的 consolidation 块 + FakeMemory 治理视图
- `engine/runtime-assembly/src/canonical/production.rs` — 装配开关 (`ProductionModulesConfig`/`ProductionBackends`)
- `engine/memory/src/{context_compiler,coordinator,consolidation,memory_injection,reflexion}.rs`
- `adapters/cli/src/lib.rs` — 全部旋钮 + `*_from_env` helpers (可单测)
- `foundation/orchestration/tests/council_live.rs` — live 探针护栏范式
- `capabilities/tools/tests/shell_reach_probe.rs` — 可达探针 (W1 回归锚)

**文档**: `live-verification-ledger.md` (验证权威) / `v1-vs-v2-capability-gap-audit-2026-10-06.md` (接线真相表 §7) / `engineering-review-handoff-2026-10-06.md` (工作包 W1-W8 + 复核矩阵) / `INSTALL.md` (旋钮手册) / 本文件。

**常用命令**:
```powershell
cargo test --workspace                                    # 全量 (基线见 §3.2)
cargo test -p apeireth-cli --test production_knobs         # 旋钮回归
cargo test -p apeireth-runtime-assembly                    # 模块效果测试
cargo test -p apeireth-tools-canonical --test shell_reach_probe -- --ignored --nocapture   # 可达探针 (真机)
cargo clippy --workspace --all-targets -- -D warnings      # 0 警告门
```

---

## 8. 未竟事项与已知局限 (诚实边界)

- **本线没做**: §4 四组接线 (dreaming 起)、真机点击流、live E2E (等 key)。
- **本线自限** (有意不做, 原因已注释): typed source 的 episodes 槽 (型别边界, 详 §2.2); reflexion 的 `ValidationFailed`/`ExperienceFailed` 信号源 (生产无诚实来源, store API 就绪); embedding 批量化 (trait 升级才做)。
- **已知遗留观察项**: §3.4 四项。
- **数字口径**: 一切计数以实测为准; 引用旧基线必须带 commit 与日期。

_交接文档 v1 (W2 接线线, 2026-10-06 夜)。接手后请在本文件顶部追加你的接手批注 (谁/何时/从哪续), 不改历史正文。_
