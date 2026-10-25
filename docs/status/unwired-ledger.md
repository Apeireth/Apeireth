# 未接线台账（能力齐全但没接电）

> 纪律（四级口径）：本表是「已实现但没有生产调用点」的**唯一清单**。新增一条 = 必须在同一批里
> 写清 **owner / 决策 / 触发条件**；接线完成后把行改为「已接线 + 提交号」并保留原行。
>
> 生成辅助：`python scripts/audit_unwired.py`（孤儿 API 报告，**报告不阻塞 CI** ——
> 公开 builder API 与测试专用 API 天然无仓内生产调用，误报率高，必须人工复核）。
>
> 判据（四级口径）：**IMPLEMENTED = 库内可跑且测试通过；PRODUCTION WIRED = 生产装配有
> 构造/调用点且实测跑过**。两者混淆是本表存在的原因。

## 一、待接线（按风险排序）

| # | 符号 / 能力 | 现状（实证） | 决策 / 触发 | owner |
|---|---|---|---|---|
| W1 | `ImBridge` 生产构造 + 设备审批面注入 | 全仓仅 `gateway/tests/im_device_cards.rs` 调用；生产装配无构造点 ⇒ **money 档「必须 IM 卡」在生产发不出卡**。渠道/发送器/配置解析/四态闭合均已就绪，缺的是装配那一根线 | **owner 2026-10-23 裁定：暂不接**，作为「Apeireth 在社交软件里和人自由聊天」的**产品面模块**专项做（设计输入见 [im-channel-module-2026-10-23.md](../04-internal/backend-enhancement/03-designs/im-channel-module-2026-10-23.md)）。在此之前 money 档维持**缺卡即拒**（该档动作当前不可用，有意为之，不是故障） | 产品 owner |
| W2 | `verify_multisig` / `multi_sign` | 全库 `multi_sign = None`，零生产调用；高风险动作人审实际只有「字符串票」占位 | 触发：v2.1 真多签（`docs/architecture/r8-inventory-2026-10-20.md` G2） | 安全线 |
| W3 | `ScarLedger::learn` | 生产调用 0（仅测试）；无干净生产者、无订阅方 | **保留 NotWired**（`r1b-3-handoff` 结论）：裸 publish 属装，等真实消费方 | 安全线 |
| W4 | `SelfDisableAudit` | 装配面未见生产消费（drill 三路径只在测试内验证） | 触发：自停用演练进 CI 后随链接线（R8-D 转绿） | 安全线 |
| W5 | `ActionGuard::evaluate`（V1+V2 AND 门） | 不在执行主链（仅测试/示例调用）；13 键已降级。**2026-10-24 查证后改判**：阻塞**不是缺接线**，是缺「动作描述语言」—— `ActionTarget` 13 变体中 10 个是测试夹具（`Pretend*`），真实生产目标只有 3 个（改 L0 / 重组洋葱 / 改 Evolution L0），**而当前系统没有能表达这三件事的工具**；映射到 `NormalAction` 则 V1 永不触发 = 门形同虚设 | **不接**（owner 2026-10-24 授权「按你的建议来」）。直到两条同时成立：①出现需要这道门的**真实能力**（能改 L0 的工具）②有经产品确认的 `governance::Action → core::Action` 语义映射（风险等级 + target 规则，有人负责维护）。**防守对象尚不存在 ⇒ 现在接等于装饰**。详见 `w5-philosophy-gate-decision-2026-10-24.md` | 架构线 + 产品 |
| W6 | SDK `quota` | 永远返 `not_implemented`（D-05 阶段 6 stub） | 已知设计取舍，非缺陷；随 SDK v0 发布面补 | SDK |
| W7 | 教训对象 `Lesson` | L1 + L2 已接线（PRODUCTION WIRED；2026-10-23）：库级 `lesson.rs`/`V18`；CLI 出口 `apeireth lessons`；自述面 `memory_ledger.verified_lessons / candidate_lessons`（子字段，不动 21 节冻结合同）。为腾尺寸空间顺带拆出 `memory_ledger_view.rs`（tools）与 `self_status_probe.rs`（cli）。**写入路径 2026-10-24 接上**（此前三个写入 API 全仓零调用 ⇒ `apeireth lessons` 永远空表）：CLI `apeireth lessons add`，只写 `candidate`，**不开** `verify`/`retract`（不假装状态机能完整转）；硬约束不绕过（无出处直接拒；出处不存在由存储层给 `UnknownSource`）。仍缺：面板列表、反思步（L3）、上下文注入（L4） | **L4 已裁：暂不注入**（owner 2026-10-24：「按你的建议来」）。理由：注入是**行为变化**且消耗 token 预算，而当前**没有任何真实数据**说明「注入已验证教训」能改善什么 —— 先注入等于拿无数据的直觉赌预算。**解冻条件**：出现 ≥20 条 `verified` 教训（现在有多少看 `apeireth lessons`），且有可对照的评测手段能回答「注入后是否变好」。L3 反思步仍需与反思调度器同窗口 | 记忆线 |

| W9 | 场景预期数据与实现不一致 | 2026-10-24 第四轮接 `run_scenario_checked` 时暴露：`reconcile` 把**空 `expected_effects`** 当「预期无副作用」，而目录里**上千个场景实例**的预期为空或漏项（与实现的真实副作用集不一致）⇒ 未对账的场景此前**静默进入导出/训练集**，而导出物是门禁与 shadow 训练的证据来源。**本批不 fail-closed**（那只是把问题挪到 CI 失败里），改为：导出器逐场景跑 checked 版本，**未对账的场景计入 manifest 的 `reconciliation` 段** ⇒ 门禁与训练流程看得见这批数据不可信 | **待专项**：补全场景预期数据（需要逐个场景判断「应该有什么效果」，工作量以百计）。**解冻条件**：有一轮专门工作把 `expected_effects` 补齐到与实现一致，然后导出器才能从「记录」升级为「阻断」。同期修掉一个真 bug：`tool.filesystem` 的**只读调用**被算成有持久化副作用（静态描述符是读写合一的，已按参数收窄） | 架构线 + 记忆线 |
| W8 | 「未接线」机检守卫（`audit_unwired.py` + `unwired-ratchet.yml`） | 2026-10-24 四轮迭代后已可用，但**守卫自己也踩过坑**：①覆盖面从 3 个目录扩到全仓 30 crate（此前 `apeireth-memory` 根本没进视野 ⇒「教训写入 API 零调用」一直没人发现）；②`test-only` 不再计入基线（库 API 被集成测试覆盖是正常状态，混记会把数字从 164 虚报成 892）；③判据 v4：`use` 剔除改成**整块**（多行 `pub use` 只剔第一行会掩盖真孤儿）；④**工具漏提交事故**：`.gitignore` 的 `*.py` 静默吃掉三个新建脚本 ⇒ 本地全绿、提交里没有、CI 红 ⇒ 已改 `!scripts/*.py` 整目录放行，并新增 `scripts/check_commit_tree.py` **验证提交后的树** | 已解决（PRODUCTION WIRED；2026-10-24） | 架构线 |

## 二、测试专用但**有意保留**（不是缺陷；四级口径见上）

| 符号 | 为什么留着 |
|---|---|
| `build_production_governance`（`cli/src/lib.rs`） | 旧构造器：测试要构造「无 env 依赖」的 gate 做四态等价性对比；生产走 `build_production_governance_from_env` |
| `adapt_gate`（`cli/src/governance_chain.rs`） | 内存审计档的便捷包装，供测试与嵌入方用；生产走 `production()`（持久审计） |

## 三、死 API（无任何引用，待清理）

| 符号 | 处置 |
|---|---|
| ~~`as_session_store`~~（`runtime-assembly/src/sqlite_session.rs`） | ✅ 已删（早期批次；本行原为过期残留，2026-10-25 对账时发现该符号已不存在） |
| ~~`query_recent`~~（`memory/src/episode.rs`） | ✅ **已删**（2026-10-25）：`mvp/memory/store.py` 兼容层遗留，全仓零引用 |
| ~~`StreamHandle`~~（`memory/src/streams.rs`，含 `new`/`HistoryStream` 实现与专属测试） | ✅ **已删**（2026-10-25）：通用流句柄，`KIND` 还是 `Thought` 占位（自己都说「调用方应直接用具体流类型」），全仓零构造 |
| `append_only::kind_from_str` | 保留：`organ_kani_proofs` 在 `cfg(kani)` 下使用；stable 构建看不到使用但**不是死代码** |

**本节结论（2026-10-25）**：死 API 清理已收口 —— 现存零引用的公开 API 均为
`cfg(kani)` 证明面使用或测试专用（见 §二），不再有「无任何引用」的符号。

## 四、已接线（PRODUCTION WIRED；保留原行供审计）

| 符号 / 能力 | 何时接的 | 证据 |
|---|---|---|
| `memory.update`（`PanelData::update_episode` → `MemoryGovernanceStore::update_episode_content`） | 2026-10-22 | `PUT /v1/apeireth/memory/episodes/:id/content` + `panel_routes` 3 例；claim `gap.memory_update` 改判 |
| `ChainGovernanceHook`（R8-B1 适配器） | 2026-10-22（R8-B2） | `governance_chain::production` 进 CLI 装配根；claim `gov.chain_hook_adapter` 改判 |
| Recon 9 原生工具 + 10 外部适配器 | 2026-10-22 | `ProductionModulesConfig.recon/.external` + knob/face 测试 |
| 会话封缄（B6-2a） | 2026-10-22 | `SessionSealer` 进 `SqliteSessionStore`；11 个新单测 |

## 五、模块级「整块未接线」清单（2026-10-24 第四轮）

> 工具：`scripts/audit_unwired_modules.py`（三层判据：独特标识符零引用 + 限定路径未用 + 无私有 `use`）。
> **它是线索工具，不是判决** —— 产出必须人工过筛（已知局限见脚本 docstring）。
>
> 候选 92 → 人工判为整块未接线 50 → 按性质分类：

| 类别 | 数量 | 含义 |
|---|---|---|
| 重复实现 | 4 | 同能力另有主实现 ⇒ **不是债** |
| 有意不接 | 9 | 模块自述 default-off / 研究原型 / W5 哲学层 ⇒ **不是债** |
| 整族未接线 | 6 | `layered_memo` 整族（6 个模块同因） |
| 待判 | 31 | 逐条人工过筛中 |

### 5.1 重复实现（4 条，不是债）

| 模块 | 已被谁承担 |
|---|---|
| `memory_rank.rs` | 排序由 `retrieval_pipeline.rs` + `scope::MemoryRankingConfig` 承担 |
| `preference_store.rs` | 偏好已走 `SQLitePreferenceStore`（`cli/src/lib.rs:1858`） |
| `three_layer.rs` | 分层由 `layers.rs` + coordinator 承担（`production.rs:1435`） |
| `vector_distance.rs` | 向量数学归 `canonical::vector`（`coordinator.rs:613`） |

### 5.2 有意不接（9 条，不是债）

- `crates/engine/memory/src/dream_consolidation.rs`
- `crates/engine/memory/src/persistent_vector.rs`
- `crates/engine/memory/src/principles.rs`
- `crates/engine/memory/src/research_non_interference.rs`
- `crates/engine/memory/src/research_roaming_memory.rs`
- `crates/engine/organ/src/experience_growth.rs`
- `crates/engine/organ/src/goal.rs`
- `crates/engine/organ/src/motivation.rs`
- `crates/foundation/orchestration/src/research_vault_ftrl.rs`

### 5.3 整族未接线：`layered_memo`（6 个模块同因）

整个 `layered_memo` 在生产代码零消费（外部仅 `dream_consolidation.rs` 的 doc 链接），
所以是**一个缺口，不是六个**：记忆分层（L1→L4 晋级/降级、多路检索融合、分类树归置、
双轨合并去重、睡眠巩固周期）整族没有生产入口。

| 模块 | 缺的能力 |
|---|---|
| `crates/engine/memory/src/layered_memo/dual_track.rs` | — |
| `crates/engine/memory/src/layered_memo/enhanced.rs` | — |
| `crates/engine/memory/src/layered_memo/librarian.rs` | — |
| `crates/engine/memory/src/layered_memo/pipe.rs` | — |
| `crates/engine/memory/src/layered_memo/progression.rs` | — |
| `crates/engine/memory/src/layered_memo/sleep_cycle.rs` | — |

### 5.4 已修（A 类逐步清零中）

| 缺口 | PR | 内容 |
|---|---|---|
| **被遗忘权**（`universal_forget`） | #90 | 删用户 ⇒ 连记忆一起删。先清记忆再落用户表（反了会留下「用户已删、记忆还在」的中间态）；记忆面拒绝 ⇒ 硬失败 409；未注入 ⇒ 响应如实说清。生产入口 `UniversalForgetFacade::forget_principal`（此前只有 pool 版，生产不可达） |
| **关系里程碑**（`milestone`） | 本批 | 新增 SQLite 持久实现（关系史必须跨重启）+ 幂等记录面 + 接进羁绊模块：TurnStart 记初见、AfterTurn 阶段跃迁时记一条。未注入存储 = 零记录 |
| **错误闭合词表映射**（`error_taxonomy`） | 本批 | `ClassifyError` 此前零实现；现覆盖 SDK 三个错误族（10+8+4 个变体），并新增跨语言 `ErrorFrame`。三处显式重试覆盖：限流可重试带 `retry_after_ms`、**配额不可重试**（退避无用）、**5xx 覆盖为可重试**（瞬时故障） |
| **便携包 CLI**（`portable_bundle`） | 本批 | 新增 `apeireth bundle --out <目录>`。接线时发现三处硬伤并修：启动脚本调不存在的 `serve --data-dir`、设两个无人读取的变量、配置默认打开明文 P2P 桩。清单改为按盘上真实文件生成。**真机验证**：脚本启动网关 → `/health` ok → 数据落在包内 `./data/` |
| **推理字段归一化**（`reasoning_adapter`） | 本批 | 12 别名 → 前置 `<think>` 块，接进 OpenAI 兼容 provider 响应解析。默认关（模块自带既有 env 开关，不新增）。关键顺序：先归一化后判空 —— 否则推理模型烧光预算（content 空 + `finish_reason=length`）会被误判成空壳响应 |
| **查询扩展**（`query_expand`） | 本批 | 接在检索主路径，做成**召回增强**：只在主查询召回不足 `limit` 时补跑扩展查询，按 id 并集去重（同 id 取更高分）。不直接替换查询的原因：打分是各项之和（BM25 无查询长度归一化），换掉原查询会让同义词匹配多的文档压过精确匹配 |
| **代码地图**（`repo_map`） | 本批 | `tool.repo_map` 工具面 + 生产注册（只读、默认可用、工作区限域、三重预算）。**顺带修掉既有实现的两处平方级热点**：建图是「每个定义 × 全文件子串搜索」、PageRank 每轮遍历整张邻接表 ⇒ 152 文件 9.5s、全仓 1319 文件 10.8s（卡十秒=不可用）；改 token 索引建边 + 推式 PageRank 后 **0.26s / 2.1s**，并加复杂度回归守卫测试 |
| **partner 持久化**（`partner_store_sqlite`） | 本批 | 装配从 `InMemoryPartnerStore`（重启即散）换成 `SqlitePartnerStore`（与认知库同一份库）。顺带修一个「能编译但用不了」：`ensure_schema()` 从不被调用 ⇒ `save_partner` 直接报 `no such table: partner_records`（原有测试全覆盖不到，只有真用起来的测试才抓得到） |
| **限流守门**（`rate_limit`） | 本批 | 挂进生产治理链（授权之后、内容风险之前）。默认挂载：配额内完全放行（零行为变化），同一 (能力,会话) 超 30/分钟或 300/小时才拒。默认挂载而非「默认关 + env 开关」—— env 棘轮只减不增已满额，且「默认关」等于这闸依然没接电 |

### 5.4p 审计锁文件在正常退出后永久残留 —— 已修（2026-10-25）

**缺陷（我自己 §5.4j 修法带来的副作用）**：审计 sink 存在**进程级注册表**
（按路径单例）里，不随栈析构 ⇒ 进程正常返回时 `Drop` 不跑 ⇒
`governance-audit.lock` 每次运行都在用户数据目录里留一个。陈旧锁接管能兜住
（不会锁死），但那是**垃圾文件**，且 pid 复用时理论上会误判「已被占用」。

**修法**：`main` 包一层——`run_cli()` 跑完显式调用
`governance_chain::release_audit_lock()`（只删本进程记录过的锁）。

**过程中又抓到自己一个 bug**：第一版把「记录已取锁」与「释放」写成**两个函数
各自的 `static`** —— 函数内 `static` 是**不同的变量**，记录与释放看到不同容器，
实测锁仍残留。改成模块级单一 `static HELD_AUDIT_LOCKS` 后真机验证：
正常退出后锁残留 **0**（连跑两次都是 0）。

**真机验证**：

```
① 正常退出后锁残留数 = 0（修前 1）
② 再跑一次：无「已被另一个进程持有」告警、无链校验失败
```

### 5.5p D-4 裁定落地：`cross_diary` 从「零消费方」变**有生产消费方**（2026-10-25）

**裁定**：owner 选 D-4 **A**（`cross_diary` 先做；`three_tier_vault` 按 B 后置）。

**接线点**：`MemoryCoordinator::execute_hybrid_retrieval`（`recall()` / `recall_async()`
**共用**的检索执行尾段）—— 命中事实挂日记片段；片段进两种 overlay
（XML 封闭世界 / **生产默认**的反幻觉编号证据清单）。

**读取面**（新模块 `cross_diary_wiring.rs`）：**只读**、有界（≥2 共享词才算关联、
每事实 ≤3 段、单段 ≤120 字、最多扫最近 14 天、overlay 内 ≤3 行且记忆正文优先）、
确定性排序、坏文件优雅降级。`CrossDiaryIndex::build_recent` 让 `cross_diary`
模块**第一次有了生产消费方**（此前零引用 ⇒ A 类缺口）。

**开关口径**：**不新增 env**（预算 160/160 满）⇒ **存在性即开关**
（`<data>/diary` 目录存在才挂只读日记面；不存在 ⇒ 检索路径压根不碰日记，
`diary_snippets` 恒空、overlay 与接线前**逐字节一致**，集成测试用 JSON 全等钉死）。

**真机证据**：真 CLI 跑 `apeireth dream --date 2026-10-25` 落盘 `<data>/diary/2026-10-25.json`
（`source="dream"` + markdown body + `timestamp_epoch_ms`）；生产检索路径指到同一真目录
⇒ 命中事实确实挂上梦境日志摘要（共享词证据齐全）。
**未验**：「片段出现在真实模型提示词里」只有代码级证据（需真 LLM 回合）。

### 5.5o D-6 裁定落地：后果追踪的**显式绑定账** + `failed` 自动回填（2026-10-25）

**裁定**：owner 选 D-6 **A 的第一步**（`confirmed` 仍人工/外部）。

**最关键的查证结论**（决定了实现形态）：egress 的 `action_id`（`act-{now_ms}`，墙钟派生）
与工具调用的 `ToolCall.id`（provider 赋的不透明 id）**不同源、不可推导** ——
出网闸在 *provider 请求* 上判定，影响声明在工具调用存在**之前**就落账；边界也不是同一件事。
⇒ **不能自动归属**。按纪律**不编造归属**，落地为**显式绑定账**：

- `ActionToolBindingLedger`（append-only `action-tool-bindings.jsonl`，零真值）：
  `bind(action_id, tool_call_id)` **两侧各 1:1**（破坏即拒、同一对幂等）；
- 登记入口两处同语义：库级 `OutcomeBackfiller::bind` + CLI `apeireth egress bind <ACTION_ID> <TOOL_CALL_ID>`
  （**只指向已声明的影响**，查不到即拒、明说「不编造归属」）。

**`failed` 自动回填**：`backfill_from_tool_result_at` —— 成功 ⇒ **第一步就不动**
（填 `confirmed` 的代码路径**根本不存在**）；未登记 ⇒ 不动；已是 `failed` ⇒ 幂等；
**已是 `confirmed` ⇒ 不降级**；其余 ⇒ `Failed` + `locally_observed`。
生产触发面只听 `AfterToolResult`，装配根按**已有 env** `APEIRETH_ENABLE_EGRESS_GATE` 注册
（**默认关 = 零注册**，符合「未启用 = 不注册」纪律）。

**顺带修的真缺陷**：`act-{now_ms}` 是**毫秒级** ⇒ **同一毫秒两次出网会共用一个 `action_id`**
（两条影响声明一个 id）⇒ 源头就破坏 1:1。已改为 `act-{now_ms}-{序号}`（进程内单调），
并加回归测试钉死「同毫秒两次出网 ⇒ 不同 id」。

**诚实边界**：`bind`/`impact` **真机跑过**；但「工具失败 → 账本变 failed」的**完整真机链路未跑**
（需真实 provider 回合 + 工具失败，本环境无凭据）—— 钩子路径、账本写入、报告面读取三段
各有集成测试钉死，但未在真机串起来。
**待定**：「为工具动作落影响声明」的生产者仍缺位（真正闭环需要指定谁在何时落）。

### 5.5q orphan 专项（cli/sdk/memory/provider 区 47 条）：真死 6 / 设计如此 41

判据口径（沿用仓库既有先例 `append_fact` / `as_session_store` / `stats_ref`）：
**「与另一公开 API 同签名纯改名」且零引用 ⇒ 删**；**签名不同的便利包装**、
**私有字段的唯一注入点**、**公开字段上的语法糖** ⇒ 保留（属公开 API 表面）。

**已删 6 条**（零能力损失、零测试需删）：`events_with_ids` / `select_candidates` /
`query_facts` / `traverse_from` / `retract_relation` / `get_history` ——
全部是同签名纯别名，主路径在生产在用。**基线 149 → 143**（G6 78→72）。

**另更正 2 处过期 doc**：`consolidate_governed` 自称「production entry point」实为**未接线**；
`put_episode_full` 的「与 `put_episode` 不同」自 2026-10-24 起**已不成立**（现已一并写全 4 列）。

**判据盲区（未改判据，报备）**：裸 token 计数会**同名遮蔽** ——
`TemporalGraphStore::facts_as_of` / `::history` 是**真死但被同名方法遮蔽**
（`BitemporalGraph::facts_as_of` / `TuningEngine::history` 在生产出现）⇒ 不在基线里。
**不改判据缩表**（那是 cosmetic 抵扣），单独立条目待处理。
另：`without_as_of` 的归组理由**指向另一个孤儿** ⇒ 理由循环；`with_len` 被 `with_` 前缀
误扫进 G4（它是便利构造，不是注入点）。

**整族未接线（模块级，建议单独立条目）**：`memory/src/scope.rs` 的 `RecallPolicy`、
`dailynote/` 的 `DailyNote`、`online_calibration.rs` 的 `AdaptiveBaseline`/`DriftDetector`、
`cluster_store.rs` 的 `ClusterStore` —— 各自那几条基线条目是**症状**，不是独立债。

**「零引用 ≠ 没接电」的反例**：`apeireth_sdk_py`（PyO3 `#[pymodule]`）由 **Python import
机制**调用，Rust 侧零引用是正常态 —— 判据看不见。

### 5.5n D-3 裁定落地：`care_potential_field` 有了**真实信号源**并接进生产（2026-10-25）

**裁定**：owner 选 D-3 **A 的 ①②**（纯确定性、可测、不猜语义），③ 时区信号后置。

**之前的状态**：`CarePotentialField`（关怀势能场）有完整实现、有势能公式、有三级
关怀动作，但**零生产调用** —— 缺的不是算法，是**输入**：五个因子
（nocturnal/distress/fatigue/silence/flow_resistance）没人算。

**本轮落地（三层）**：

| 层 | 内容 | 真实性 |
|---|---|---|
| ① 信号源 | `BioLifecycleState` 新增 `consecutive_tool_failures`（连续失败数，**成功即清零**）+ `last_tool_success_ms`（最近工具成功时刻） | **真实** `ToolOutcome` 总线回调维护，非采样非估算 |
| ② 因子映射 | 新模块 `care_signals.rs`：`distress` = 连续失败/3（钳 [0,1]）、`fatigue` = 无成功小时数/6（钳 [0,1]）、`silence` = 同源秒数；`nocturnal`/`flow_resistance` **固定 0**（时区债未收 / 无心流检测面） | 纯函数、确定性、时钟回拨不误剪 |
| ③ 生产接线 | `CarePotentialDriver` + `CarePotentialFeed`（实现 `ReminderFeed`）接进**常驻关怀心跳**（CLI `care_heartbeat`） | 势能突破**只产一条提醒信号**，之后与天气提醒/守护告警**同管道同纪律**消费（勿扰/专注/频率/预算/发起闸**一道都不绕**） |

**开关与时基**：
- **不新增 env**（env 预算 160/160 已满）⇒ 复用 `APEIRETH_ENABLE_BIO_LIFECYCLE`
  （信号源本身的存在条件）；关怀心跳另有既有的双闸
  （`APEIRETH_ENABLE_CARE` + `APEIRETH_ENABLE_CARE_HEARTBEAT`）。
- 时基统一用 `state.now_ms()`（与「最近工具成功时刻」同一时钟）⇒ 时钟回拨/注入时钟下两者同轴。
- 构造即锚定起始时刻 ⇒ **首拍** dt = 构造到首拍的真实间隔（生产 ≈ 一个心跳间隔），
  不是 0（否则势能永远起步为 0、永远不突破）。

**证据**：`care_signals.rs` 内 11 个测试（含「未突破无信号 / 突破产一条 / 未消费重投不
重复 / 消费后清位 / 闸关恒静默 / 势能信号必须走标准纪律管道」）。

**口径**：**装配面已接、默认关**（`APEIRETH_ENABLE_BIO_LIFECYCLE=1` + 关怀双闸开才生效）。

### 5.5m `tool.repo_map` 的 HTTP 级注册证据（2026-10-25，四层证据齐）

代码地图这条线此前有「工具面测试 + 装配级注册条件测试」，但**缺 HTTP 级证据**
（生产运行时它的名字到底出不出现）。本轮真机补上：

```
GET /v1/tools/list →
  ["filesystem","search","repo","repo_map","trait_report","self_status"]
```

⇒ `repo_map` **确实被生产注册**。注意：**HTTP 工具面用裸名**（`repo_map`），
而守卫/能力注册表用 `tool.repo_map`；我第一次按 `tool.*` 全名查所以误判「缺失」——
**同一批工具在两个面上用两套命名**，查证据时要按面选口径。

至此 `tool.repo_map` 的证据链四层齐：
① 工具面实现与单测（`repo_map.rs` / `repo_map_tool.rs`）；
② 装配级注册条件（`startup_instrumentation.rs::repo_map_registers_only_when_the_workspace_root_is_provided`）；
③ **HTTP 可见（本轮）**；
④ 并行派发白名单成员（`is_provably_read_only` 派生）。

### 5.5l 自审我自己的并行派发白名单：安全假设成立（2026-10-25）

并行派发的白名单是**从守卫安全注册表派生**的「可证明只读」工具
（`tool.repo` / `tool.repo_map` / `tool.self_status` / `tool.trait_report` / `tool.search`）。
既然它们会被**并发调度**，就必须验证两件事——否则「只读」只是描述符上的说法：

| 检查 | 结果 |
|---|---|
| **生产段是否有写操作**（`fs::write` / `File::create` / `write_all` / `OpenOptions` 写模式） | 五个工具**全无写作**；`search.rs` 唯一 `OpenOptions` 是 `read(true)` |
| **是否有全局可变状态**（`static mut` / `OnceLock` / `Mutex` / `RwLock` / `thread_local`） | 五个工具**全无** |

⇒ 「可证明只读」不是纸面判断：**并发执行这五个工具不会写盘、也不会争同一把锁**。
（`self_status` 内部探针已有 `try_lock` + 如实报错纪律，见 `accountability_probe`。）

**为什么值得单独记一笔**：我给白名单加登记时，证据只有「能力描述符写着 ToolOutput/只读」；
真并发起来会不会写缓存、会不会锁竞争，是另一回事。这次把「描述符声明」升级为
「代码事实核对」，白名单的安全假设才算落地。

### 5.5k 自我审计（73 commit 全量过一遍）—— 又清掉一个误提交 + 披露跨区改动（2026-10-25）

**自我审计方法**：`git diff --name-status origin/main..HEAD` 全量分类（20 新增 / 83 修改 / 0 删除），
对新增文件逐个判「源码/测试/文档 vs 生成物」，并核对是否越出协作红线段。

**① 又发现一个误提交**：`artifacts/guard-dataseton`（guard 数据集清单）也是**我的分支新增**的
（源头同 §5.5j：a4903b92 里 `git add -A` 带进来的；main 里本无此文件）。
它与 4 大 jsonl 同属「导出再生件」——CI 每次先跑导出、再读它做断言。
已 `git rm --cached` + `.gitignore`；**验证**：导出后 CI 同款断言（`dataset_version`、
4 件存在性、隐私键全 False）通过，且导出后 `git status` 干净。

**② 跨区改动主动披露**：审计发现我动过 `docs/status/`（协作纪律里写的对方区）——
- `unwired-ledger.md`：+909 行（本轮缺陷记录，我理解它是清债轮台账）；
- `claims.yaml`：15 条 `verified_at` + `updated_at`（**不改 CI 会红**：`--block-stale` 报 stale=15）；
- `STATUS.md` / `wiring.yaml`：`--gen` 重生成（生成物禁手改）。
已在看板 §二十五 逐项列出并请对方确认「可接受/要改方式」，**没有静默占用**。

**③ 审计结论**：0 删除文件、0 越界到 `crates-ui`/`frontend`/`fuzz`/`benches`/`.github`；
所有新增文件均为源码/测试/文档（除上述 2 类生成物，已处理）。

### 5.5j 我自己把 22MB 生成物误提交进仓库 —— 已取消跟踪 + ignore（2026-10-25）

**怎么发现的**：跑「仓库卫生」核对（跟踪文件里有无构建产物/大文件）时发现
`scripts/guard_ml/` 下有 4 个共 **22MB** 的 `.jsonl`，而 `scripts/guard_ml/README.md`
明确写着它们**已在 F-02 执行批外置、库内不存放**（"库内不存放不影响 CI"）。

**根因（我的失误）**：第 20 轮我为了镜像 CI 门禁，本地跑了
`cargo run -p apeireth-guard --example export_guard_scenarios` —— 该导出会**重新生成**
这 4 件到 `scripts/guard_ml/`；它们当时**没有被 `.gitignore` 覆盖**，于是
`git add -A` 把 22MB 一起提交了（commit `a4903b92` 里能看到 2850/2835/2835/127 行新增）。

**修法**：
1. `git rm --cached` 4 件（**git 历史不动**，符合 F-02 原话）；
2. `.gitignore` 加 4 条精确规则（下次本地导出不会再被 `git add -A` 带走）；
3. 验证：重新跑导出 + CI 同款断言（存在性 + 隐私键黑名单）→ **通过**；
   导出后 `git status` 仍干净（被 ignore）。

**顺带**：export 还会重写被跟踪的 `artifacts/guard-dataseton`（证据清单，格式已随版本演进）
—— 我已**还原**、未并入提交；该类文件是否也该外置由 guard/评估线决定（同 `README` 那一类的治理）。

**教训**：跑「会写文件」的门禁镜像前，先 `git status` 看它写了什么；
`git add -A` 是我的常用动作，也是把生成物带进仓库的最短路径。

### 5.5i 跟踪文件凭据扫描（2026-10-25，未发现泄漏）

对 `git ls-files` 的 **5686 个跟踪文件**扫 5 类模式（GitHub token `gh[pousr]_` /
OpenAI `sk-` / AWS `AKIA` / PEM 私钥头 / Slack `xox[baprs]-`）：

- 原始命中 **75 处**；
- **逐条核实全部为假**：
  - AWS 官方文档示例键 `AKIAIOSFODNN7EXAMPLE`；
  - `sk-1234567890abcdef…` / `sk-xxxxxxxx…`（明显占位）；
  - `ghp_abcdefghijklmnopqrstuvwxyz`（字母序示例）；
  - `crates/adapters/gateway/tests/fixtures/tls/*.pem`（**测试专用**自签 TLS 私钥）；
  - `crates/foundation/governance/src/input_security.rs` / `tools/src/{guardrail,redaction_bridge}.rs`
    里的命中是**凭据检测/脱敏器自身的模式定义**（它们就该长得像凭据）；
  - `legacy/{donor,frozen}/` 为捐赠代码的示例常量。

⇒ **跟踪文件里 0 真实凭据泄漏**。（CI 另有 `pii-leak-detection.yml` + `.gitleaks.toml`
门禁；本批是全量人工复核，不是替代门禁。）

### 5.5h 管理面 HTTP 全链真机核对（2026-10-25，未发现缺陷）

把此前未测的 `/v1/admin/*` 面逐操作跑了一遍（token 门 + 面板族列表交叉核对）：

| 操作 | 结果 |
|---|---|
| `POST /v1/admin/users`（bob/member） | **201** + 正确 DTO |
| `POST /v1/admin/domains`（bob 域） | **201** + `user_count:1` |
| `PATCH /v1/admin/users/bob`（→ guest） | **200**；非法角色 → **400**（列冻结词表 owner\|member\|guest）；不存在 → **404** |
| `DELETE /v1/admin/users/bob` | **200**，响应含 **`memory_forgotten:true`** + 逐面 `memory_note`（episode/preference/commitment/temporal/persona…）⇒ **「被遗忘权」级联经 HTTP 面确实生效**（此前只有代码级证据） |
| 重复创建同一 user | **409 conflict** |
| 删除最后一个 owner | **409 conflict**（「must keep at least one owner」） |
| `POST /v1/admin/shutdown` | **200** `{"status":"shutting_down","trigger":"first"}`，进程**优雅退出**（日志 `server stopped`） |
| `GET /v1/admin/{users,domains}` | **405** —— 正确：这两个是 POST-only 创建面，列表在面板族（代码注释同口径） |
| 无令牌访问 | **401**（三态门） |

⇒ 管理面实现与宣称一致，**无缺陷**；顺带把「被遗忘权 = PRODUCTION WIRED」补上 HTTP 级证据。

### 5.5g `8-promise-audit.sh` 两项判据是坏的（反转 + 路径过期）—— 已修（2026-10-25）

跑 `scripts/audit/8-promise-audit.sh` 得到 **2/8 未严守、exit 1**。逐条查证后确认：
**两项都是脚本判据坏了，仓库没问题**（所以不能「调阈值凑绿」，要修判据语义）。

**① [1/8] 不假装已实现：判据反转**

原逻辑：`TODO|unimplemented!()|skeleton` 命中数 **> 50 才 PASS**。
⇒ **代码越干净，审计越 FAIL**（当前 32 处 → FAIL）。这是**反向激励**。

真实语义应是「**生产代码里没有伪实现**」。改为：统计**非注释行**的
`unimplemented!()/todo!()`，必须为 0；诚实标记数量只作信息报告。
（顺带修掉一个 `pipefail` + `|| echo 0` 交互 bug：值会变成 `0\n0` 导致判据恒假。）

**② [7/8] 不重复造轮子：排除路径过期 + 误算字样**

原逻辑 grep 所有 `wmic|ioreg|dmesg` **字样**，排除目录名 `apeireth-machine-id`。
但该实现**早已迁到** `crates/engine/storage/src/machine_id.rs`（且改为**纯解析**、不再起子进程）
⇒ 注释/测试夹具/**安全黑名单**（`approval_family.rs` 里把 `wmic` 列为进程族词）里的字样
全被误算成违规（25 处）。

真实语义是「**不为平台信息去 spawn 系统命令**」。改为统计真正的
`Command::new("wmic"|"ioreg"|"dmesg")` 子进程调用（允许 machine-id 实现文件例外）——
实测 **0 处**，承诺其实守得比 1.0 时代更严（纯解析替代了 shell 调用）。

**修后**：`8/8 严守`、`exit 0`。

**顺带**：`run-cargo-audit-deny.sh` 本地跑会在 `reports/` 留两份未跟踪报告
（`reports/r20-cargo-{audit,deny}-stdout-*.txt`，CI 上是 artifact）⇒ 已加 `.gitignore` 模式，
本地跑不再弄脏工作区（`audit-report.json` 本就在 gitignore）。

### 5.5f 打包/安装脚本的版本系统性漂移 —— 已统一从 Cargo.toml 派生（2026-10-25）

**缺陷**：15 个打包/安装脚本各自硬编码版本默认值，其中多数**已过期**：

| 脚本 | 修前默认 | 实际产品版本 |
|---|---|---|
| `scripts/install/install-brew.sh` | `1.0.0` | 2.0.0-rc.4 |
| `scripts/install/install-deb.sh` | `2.0.0~rc1` | 2.0.0-rc.4 |
| `scripts/install/install-rpm.sh` | `2.0.0` | 2.0.0-rc.4 |
| `scripts/install/install-tarball.sh` | `2.0.0-rc.4` ✓ | 2.0.0-rc.4 |
| `scripts/install/install-scoop.ps1` | `1.0.0` | 2.0.0-rc.4 |
| `packaging/deb/build.sh` / `packaging/deb/install-deb.sh` | `2.0.0~rc1` | 2.0.0-rc.4 |
| `packaging/rpm/{build,install-rpm}.sh` | `2.0.0` | 2.0.0-rc.4 |
| `packaging/brew/{build,install-brew}.sh` | `1.0.0`（安装面 `1.0.0`） | 2.0.0-rc.4 |

⇒ 照文档跑 `sudo ./scripts/install/install-deb.sh` 会**找错版本号的包**（找不到即报错），
且注释里还写着「编译期 hardcode VERSION=1.0.0」（**注释与代码也不一致**）。

**修法（系统性，非逐个改数字）**：15 个脚本统一改为**从 workspace `Cargo.toml` 派生**
（`APEIRETH_VERSION` 仍可覆盖），并补上「先 `cd` 到仓库根」保证与调用处 cwd 无关
（`packaging/rpm/install-rpm.sh` 的 `cd` 原本在版本行之后、`packaging/tarball/install.sh`
与 `packaging/brew/install-brew.sh` 原本**没有** `cd` —— 从别处调用会解析不到 `Cargo.toml`）。
PowerShell 版（`install-scoop.ps1`）用等价的正则解析。注释里的「hardcode VERSION=1.0.0」
一并改成派生口径。

**实测**：
- 15 个 bash 脚本 `bash -n` 全过；
- 从 `/tmp`（非仓库目录）调用 `packaging/tarball/install.sh` 与
  `packaging/brew/install-brew.sh` → **都正确解析到 `2.0.0-rc.4`** 并进入各自的下游检查
  （分别报「未找到二进制」「brew 未装」，均为如实失败、无副作用）；
- `grep 'APEIRETH_VERSION:-[0-9]' scripts/install/ packaging/` → **零残留**。

**有意保留未改**：`scripts/release/cosign-sign-all.sh` 与
`scripts/release-1.0-checklist.sh` 仍用 `1.0.0` —— 那是**产品轴 tag**（1.0.0），
与 workspace 轴（2.0.0-rc.4）本就不同轴；贸然统一反而错。已在台账标注待 owner/发布线确认。

### 5.5e 两个 release 脚本的安全缺陷：`rollback.sh` 静默忽略 `--dry-run`、upgrade dry-run 缺密钥即死（2026-10-25）

跑 CI release-prep 里的三个 dry-run 脚本时发现：

**① `scripts/upgrade/rollback.sh`（31 行破坏性脚本）零参数解析**

- 传 `--dry-run` → **被静默忽略并真执行**（`systemctl stop/start` + 覆盖二进制/配置）；
- 非 root 跑 → 部分命令失败后仍打印 **「Rollback done」**（误导成功）；
- 无备份完整性校验。

⇒ 这是**破坏性脚本的失败模式**：任何自动化传了 `--dry-run` 期望模拟，实际会回滚生产。

**修法**：加参数解析（`--dry-run` / `--help` / 位置参数备份目录 / **未知参数 exit 2**）、
非 dry-run 强制 root、`run()` 包装所有变更命令、dry-run 输出逐条 `[DRY-RUN] would` 并以
「**未执行任何变更**」收尾。四态实测：dry-run=0（零变更）/ 未知参数=2 / 非 root=1（清晰拒绝）/ --help 正常。

**② `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh --dry-run` 直接 exit 1**

配置期用 `${POSTGRES_PASSWORD:?...}` 展开 ⇒ **dry-run 在跑到 dry-run 分支之前就死了**：

```
scripts/upgrade/...: 行 74: POSTGRES_PASSWORD: set POSTGRES_PASSWORD (no weak default)
退出码 1
```

而 release 清单第 5 项声称「upgrade dry-run 0 错」，CI 又用 `|| true` 吞掉错误
⇒ **这一项一直是假绿**。（CI 的 `|| true` 属 `.github/workflows/**`，不在我改动范围，只记录。）

**修法**：凭据改**惰性**（dry-run 不需要密钥，仅非 dry-run 要求），并把校验块移到
工具函数之后（第一版放在 `log_info` 定义前，实测 `command not found`——顺序错也是 bug）。
两态实测：dry-run=0（日志明确「模式: DRY-RUN」）/ 非 dry-run 无密钥=1（清晰提示）。

**顺带**：`uninstall.sh --dry-run` 原本就正常（5 步、诚实跳过项）。

### 5.5d 主 CLI `--help` 不提同 workspace 的其它二进制 —— 已修（2026-10-25）

**缺陷**：README 把 `apeireth-tui` 列为产品前端面（「faces / Adapters」），
但 `apeireth --help` **一个字都不提** —— 用户从 CLI 帮助看不到终端驾驶舱的存在
（发现性缺口；与第 6 轮「help 漏 8 个命令」同一类，只是跨二进制）。

**修法**：`HELP_TEXT` 加一节「其它二进制」：

```
其它二进制（同一 workspace 的其它入口，不属 `apeireth` 子命令）:
  apeireth-tui [--endpoint URL] [--token TOKEN]   终端驾驶舱（全屏 TUI 客户端）
  apeireth-gateway 形态由 `apeireth gateway serve` 提供；SDK 见 crates/adapters/sdk
```

帮助防漂移守卫仍绿（它只校验「分发块里的命令都在帮助里」，新增条目不影响）。

### 5.5c SDK 与本地网关端点不对齐（2026-10-25 实测，登记为产品决策）

**实测**（真机起 `apeireth gateway serve`）：

```
POST /v1/tools/web_search/invoke  → 404     ← SDK 的 6 个工具方法走这里
GET  /v1/tools/list               → 200     ← 网关实际提供的工具面
```

**结论（比「缺一条路由」更彻底）**：SDK（`ApeirethClient`）的「HTTP 真传输」是
**客户端实现真实**，但其对端是 **R18/R20 规划中的平台 API**：6 个工具名
（`web_search` / `calendar` / `code_exec` / `file_ops` / `git_ops` / `message`）
与 `/v1/tools/{tool}/invoke` 路由，在本仓**只作为协议契约**出现在
`crates/foundation/protocol/src/ws_v1.rs` 的文档/测试里 ——
**工具面没有这些工具、网关没有这条路由**（实测 404）。⇒ **SDK 对本仓产品无可对接面**，
不是「少一个端点」，而是「两边的契约还没交汇」。

**为什么登记成决策而不是直接补**：给本地网关加「任意工具 HTTP 执行」是**产品面扩展** ——
必须回答：走不走治理链/审批？是否受 `file_write`/`egress` 等总闸约束？
面板已有 `safety.guard.evaluate`、`workbench.turn` 等受控面，工具执行是否也开一个？
这些是产品语义与安全边界决策（object ⑥），不擅自动手。

**已做**：SDK 模块头补一句**对端边界**说明（说清「客户端真接 ≠ 本仓网关有该端点」），
并把选项写进 owner 决策请求书 D-7。

### 5.5b `apeireth-tui` 在非 TTY 环境只回裸 OS 错误 —— 已修（2026-10-25）

**缺陷**：在脚本/管道/CI 里跑 `apeireth-tui`：

```
终端驾驶舱异常退出: No such device or address (os error 6)
```

exit code 正确（1），但**提示不可行动** —— 用户看不出「因为不是终端」，
更不知道该怎么替代。根因：`enable_raw_mode()` 在非 TTY 下直接失败，
错误只被原样打印。

**修法**：入口**前置 TTY 检查**（`stdin`/`stdout` 任一不是终端即拒启动），
并给出可行动提示 + exit code 2：

```
apeireth-tui 需要真实终端（stdin/stdout 必须是 TTY）。
当前 stdin/stdout 不是终端 —— 脚本、管道、CI 里请改用：
 · apeireth gateway serve        （起服务，走 HTTP API / 桌面端）
 · apeireth chat "你的问题"    （单轮对话，无需全屏界面）
```

判定抽成纯函数 `should_refuse_non_tty(stdin, stdout)` + 单测
（三条「任一非 TTY」+ 一条「都是 TTY 则正常」），`--help` 不受影响（先于检查返回）。

**端到端补证（同一轮）**：用 `script -qec "stty rows 40 cols 120; apeireth-tui ..."` 分配
伪终端真跑 TUI 连真实网关 —— 渲染出「会话/对话/记忆账本/上下文波形/用量」等面板、
显示「**后端已连接: http://127.0.0.1:…**」、空数据处如实标「未探到 / 不猜数」，
两次 Ctrl-C **干净退出（exit 0）**。
（注：不设窗口尺寸时 PTY 为 0×0，TUI 只进备用屏不画字 —— 那是 PTY 特性，不是缺陷；
    一开始差点误判为「TUI 不渲染」，设 `stty rows/cols` 后正常。）

**顺带核对（无缺陷）**：README/INSTALL 里出现的 9 条命令全部被 CLI 识别；
`egress impact --json` 的后果叠加生效（`unknown` → `confirmed`）；
claims.yaml 的 `sources`/`tests` 路径 **0 条不存在**。

### 5.5a 必需检查「cargo deny / 8 硬墙」也补进本地镜像（2026-10-25）

继续把 CI 的**必需检查**逐条对到本地命令：

| 必需检查 | 本地命令 | 结果 |
|---|---|---|
| `cargo deny check` | `cargo deny check` | ✅ `advisories ok, bans ok, licenses ok, sources ok`（仅 deny.toml 里 3 条 ignore 的「advisory-not-detected」提示，非失败） |
| `8 硬墙守门` | 直接跑 workflow 不变量 | ✅ ① LOCKED 源码 logic 0 触碰（3 个 `[locked]` 文件 word-diff 无逻辑变更）② `workspace.version` 未变 ③ R11 三值各 1 处 ④ `ALL_THIRTEEN_KEYS: [PhilosophyKey; 14]` |
| `cargo kani` | — | ❌ 需专用工具链（CI only） |
| `cargo audit` | 尝试后放弃 | ⚠️ 需克隆 RustSec 库（网络/本地库状态）；**advisories 面已由 `cargo deny check` 覆盖并全绿** |

**结论**：必需检查 6 项里，除 `Kani` 外全部**本地可镜像且全绿**；
`Kani` 覆盖的证明对象我这批未改动。

### 5.4z `docs_truth --block-stale` 会红：15 条 claim 的 `verified_at` 过期 —— 已修（2026-10-25）

**发现方式**：补跑 CI 门禁时发现 CI 的 docs-truth 用的是
`python3 scripts/docs_truth.py --verify --block-stale`（**STALE 即 exit 1**），
而我此前只跑 `--verify --check`（不阻塞 STALE）。本地照 CI 口径一跑：

```
claims=29 fail=0 stale=15 review_due=0   → 退出码 1
```

**根因**：我这一轮改了大量源码（`cli/lib.rs`、`runtime.rs`、`panels.rs`、
`chain.rs`、`nociception.rs` …），而 claims 的 `verified_at` 停在 `af0c3852`。
claims 的纪律是：**源码动过而 `verified_at` 未更新 = STALE = 过期**。

**处置**：逐条核对 15 条 claim 的 `level` / `default` / `switch_env` **是否仍准确**
（多数改动是注释/测试；行为面变化只有 egress 报告读取侧与审计链单写者，
都不改这些 claim 的等级）—— 确认无误后把 `verified_at` 统一更新到当前 HEAD
`a4903b92`（含全部改动、已在远端可达），`updated_at` 同步 2026-10-25。

**连带修**：`docs/status/STATUS.md`（`--gen` 重生成）与
**`docs/status/wiring.yaml`**（`wiring_report.py --gen`）—— 后者的
`claims_sha256` 随 claims.yaml 变化必须重生成，否则 `wiring-ledger` 那条 CI 也会红
（**生成物禁手改**）。复核后 `--verify` 退出码 0。

**纪律沉淀**：跑文档门禁要跑**CI 那一档**
（`--verify --block-stale`，不是 `--check`）；生成物改动必须走 `--gen`/`--gen` 再 verify。

### 5.4y CI 门禁本地镜像补齐（2026-10-25）：从「部分验证」到「可镜像的全跑」

上轮发现我跑的门禁比 CI 少两档（`clippy --all-targets` / `doc tests`），补完后继续把
**剩余 CI 脚本**也逐个在本地跑了一遍：

| CI 项 | 本地结果 |
|---|---|
| Docs 链接（`docs_links.py`） | 1466 个 md，**断链 0**（已知缺失 44 条为政策保留） |
| Legacy 依赖（`check_no_legacy_deps.py`） | 30 成员，path 违规 0、传递遗留包 0 |
| Ledger 验证（`verify_ledger.py`） | errors=0 warnings=0 |
| 提交树（`check_commit_tree.py`） | 5681 文件完整 |
| **Guard 3.3**（`export_guard_scenarios` + `guard_ml` 三段） | 2835 场景导出；`py_compile` OK；`train_eval` 退出码 0；`analyze_shadow --self-test` ok |
| **O-6 锚 5 重守门** | 全过（`RUNTIME_ENFORCED=false` / 9 锚 / rc.4 / R11 三值 / 9 表头） |
| Self-status vocabulary | 退出码 0（报告项；10 处体验性词在债栏，非阻塞） |

**仍只有 CI 能跑的四项**（本地不镜像，已在验收摘要 §6.3.1 如实标注）：
`Kani` / `Miri` / `Fuzz` / `Coverage (tarpaulin)` —— 但它们覆盖的代码面我这批**未改动**
（无新增 `unsafe`、未改证明对象、未改 fuzz 目标解析器）。

**纪律沉淀**：「本地门禁全绿」必须说清是**哪一档**。现在我能说：除四项重型/需专用
工具链的 CI 项外，其余全部在本地跑过并通过。

### 5.4x CI 的 clippy 门禁是 `--all-targets`，我此前只跑 `--lib` —— 三处测试代码会打红（2026-10-25）

**发现方式**：本轮第一次在本地跑 `cargo clippy --all-targets`（此前每批只跑
`cargo clippy --lib` + CLI bin 档）。**测试代码（`#[cfg(test)]`）在 `--lib` 下根本
不编译**，所以我的测试里三类 lint 一直没被看到：

| 位置 | lint | 修法 |
|---|---|---|
| `guard/src/decision_log.rs`（我本轮新写的测试） | `cast_lossless`：`index as f64` / `step as f64`（i32→f64 可 infallible） | 改 `f64::from(index)` |
| `memory/src/milestone_store_sqlite.rs`（我本轮新写的测试） | `clone_on_copy`：`SessionId` 是 `Copy` 却 `.clone()` | 直接传值 |
| `gateway/src/chain_audit.rs`（我本轮新写的测试） | `single_match_else` → 建议 `let...else` | 改 `let Err(error) = ... else { panic!() }` |

这三处**都会让 CI 的 clippy 门禁变红**（CI 那档是 `--all-targets`），而我此前的
本地门禁看不见它们。修完后 `cargo clippy --workspace --all-targets -- -D warnings`
退出码 **0**（唯一排除 `apeireth-tools-canonical`：本机既有
`CARGO_BIN_EXE_sandbox-test-child` 编译期问题，干净 main 上同样存在、CI 上正常）。

**顺带验证两个此前从未本地跑过的 CI 门禁**（都绿）：

```
cargo test --doc --workspace        → 29 个 crate doc 测试全过，0 失败
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace → 干净
```

**流程修正（写进我的批处理纪律）**：每批本地门禁从
`clippy --lib` → **`clippy --all-targets`**。测试代码也是代码。

### 5.4w 能力矩阵把已可用的 `memory.update` 写成「未实现未接线」—— 已修（2026-10-25）

**缺陷（保护路径文档，产品能力宣称过期）**：`docs/03-reference/capabilities-matrix.md`
§0（自称「唯一事实源」的 `/v1/apeireth/capabilities` 镜像）写着：

```
| `memory.update` | false | false (`reason=not_implemented`) | Declared in the universe, not wired |
```

且「未接线（不是默认开关问题）」清单里也列了它。**真机事实**：

```
$ GET /v1/apeireth/capabilities
{"id":"memory.update","operations":["update"],"read":false,"supported":true,"write":true,"available":true}
$ PUT /v1/apeireth/memory/episodes/<id>/content
→ 200（内容已更新）；过期 expected_rev → 409 revision_conflict
```

即 `memory.update` 早已 **IMPLEMENTED + PRODUCTION WIRED**。矩阵说它「不存在」，
会让集成方**不去用**这个能力（与「文档低估实现」同一类，且这条在保护路径上）。

**修法（只动两处 + 一条对账注记）**：

1. §0 表该行 → `取决于 governed memory ports / 同 supported / GovernedMemoryService（纠正内容；乐观并发 expected_rev，过期提交 = 409）`；
2. 「未接线」清单**删掉** `memory.update` 一项（同行 `DslOnionLayer` / `Ws` 等已核对**仍成立**）；
3. 表格下方补「对账注记（2026-10-25）」写明原值与真机值 —— 保留改前事实，可审计。

按协作纪律，**改保护路径前已在看板 §二十二 登记**（单行范围、不碰表格结构）。

### 5.4v 面板能力清单的 `not_assembled` 易被读成「能力不存在»—— 已加口径（2026-10-25）

`GET /v1/panel/capabilities` 里两条 `not_assembled`：

- `voice.duplex` —— 真未装配（无输入可仲裁）✅ 准确；
- `subagents.orchestration` —— **只表示网关没有 spawn/coordinate 端点**，
  而 CLI 侧 `apeireth subagent <标题>` **是可用的**。

前端若把该字段读成「产品没有子代理能力」就会误判。已在清单分隔注释写明口径：
**本清单只描述网关/面板面；`not_assembled` = 该 HTTP 面未装配，不等于能力不存在；
判断产品能力看 capabilities-matrix 的四级口径**。

（`reason` 字段值不动 —— 它是机器字段且有测试钉死；只在人读的注释里补口径。
尺寸仍精确等于基线 1623。）

### 5.4u 一批「IMPLEMENTED（未接线）」自述已过期 —— 已修（2026-10-25）

上一轮修的教训是「功能接上了、叙述没跟上」。顺着这条线全仓搜「未接线/未接电/
NOT WIRED」，把**已经不成立**的逐处改成准确状态：

| 位置 | 原文 | 事实（2026-10-25 核对） |
|---|---|---|
| `governance/src/chain.rs` `ChainGovernanceHook` | **NOT WIRED: no production assembly constructs this type yet (R8-B2)** | ❌ **直接错误**：它就是生产治理入口 —— `cli::governance_chain::production()` 构造它并交给 `with_governance`（`cli/lib.rs:1570` 调用） |
| `orchestration/src/quorum.rs` | IMPLEMENTED，未接线 | 装配面已接：生产经 `OrganModule`（`APEIRETH_ENABLE_ORGANS` 开启时）调用，`last_quorum_trace()` 供自述 SG-24 格 |
| `orchestration/src/nociception.rs` | IMPLEMENTED（未接线） | 装配面已接：`RuntimeBuilder::with_bio_pain`（`APEIRETH_ENABLE_BIO_PAIN` / `bio-mechanisms.json`，默认关） |
| `orchestration/src/recovery_protocol.rs` | IMPLEMENTED（未接线） | 装配面已接：与痛觉同一 `PainSnapshotReader` → 自述 SG-17 格 |
| `orchestration/src/lifecycle.rs` | IMPLEMENTED（未接线） | 装配面已接：`NervousSwitchboard::BioSwitch::Lifecycle`（9 开关默认全关） |
| `orchestration/src/capacity.rs` | IMPLEMENTED（未接线） | 装配面已接：`capacity_bio_probe` 注册进自述 SG-26 格（需自述开关 + 任务板） |
| `orchestration/src/durable/{replay,mod}.rs` | 存储路径已证明但未接线 | 存储路径**已接**（`cli::continuation_guard` 回合级中断检测）；未接的是**重放**（产品语义待定） |

**统一的准确表述**：「装配面已接，**默认关**（开关名）」——既不夸大成「已生效」，
也不贬低成「没人用」。原文的「未接线」会让读者以为**根本没有装配面**，
从而误判为「要新建」而不是「打开开关」。

**纪律**：`IMPLEMENTED` / `PRODUCTION WIRED` / 「未接线」这些词必须与**装配事实**
一致；只读内核 = 「纯内核」，有 env 开关 = 「装配面已接，默认关」，
恒被装配 = 「PRODUCTION WIRED」。

### 5.4t 守夜人把「接线后无数据」误报成「没接线」—— 已修（2026-10-25）

**缺陷**：判定落盘接线完成后（§5.4 那批），守夜人仍在报

```
- [gap] audit chain 未持久化供闲时读取 —— no-degrade 复盘段空转 (接线项)
```

—— 让人以为接线没做。真实情况：这个数据根**还没有守卫判定记录**（日志文件要等
第一条判定才出现）。「接好了但没数据」与「根本没接线」共用一句文案是误报。

**修法**：`NightwatchInputs` 加 `audit_source_wired: bool`（默认 `false` 保持旧语义）：

- `false` + 空 ⇒ 旧文案（具名接线缺口，仍如实）；
- `true` + 空 ⇒ **「审计链为空：本数据根还没有守卫判定记录 —— no-degrade 复盘段本轮
  无料可查（不是接线缺口）」**。

生产侧恒传 `true`（装配根 `attach_decision_log` **无条件**调用）；按「日志文件是否存在」
判断会误报 —— 实测跑过带治理的回合后 `guard-decisions.jsonl` 仍不存在（它记的是**守卫
判定**，不是治理链审计）。

**真机验证（两态）**：

```
① 已接线但无数据 → 「审计链为空：…（不是接线缺口）」
② 注入 2 条正确 schema 的判定（deny 0.9 → allow 0.1）后：
   守夜复盘 0 episodes / 2 audit: 1 findings
   - [medium] NoDegrade: 审计链风险静默降级: `critical` → `info`
   （「未持久化」缺口消失，只剩 council ballot 那条真缺口）
```

**测试**：`not_wired_and_empty_reports_the_wiring_gap` /
`wired_but_empty_does_not_claim_the_wiring_is_missing`（断言收窄到「审计链」那条，
不误伤 `council ballot 未持久化`）。

### 5.4s 并行派发在**网关形态**同样生效（2026-10-25 补证）

CLI 对话路径与 `gateway serve` 是**两条装配入口**（前者
`build_canonical_runtime_from_env`，后者 `build_canonical_runtime_assembly_from_env`）。
分开的装配根最容易出现「CLI 有 X、网关没有 X」这类隐性缺口，
所以并行派发接线后必须**两条都钉**：

- `production_runtime_installs_a_conservative_parallel_scheduler`（CLI 路径）；
- `gateway_assembly_shares_the_same_composition_root`（**网关路径**）：
  断言网关入口装出的运行时同样有调度器、且白名单仍从守卫注册表派生。

两条入口现在都**公开**（`build_canonical_runtime_assembly_from_env` 加 `pub`，
理由是它本身就是网关形态的装配根、测试要从同一入口验证），
`lib.rs` 尺寸仍精确等于基线（注释压缩，非行数位移）。

### 5.4r `continuation` 安全子集接线（PRODUCTION WIRED：回合级中断检测，2026-10-25）

**做了什么**（`cli::continuation_guard` + `run_chat` 接线）：

- 回合开始前落快照（`ContinuationSnapshot`，`pending_tool_call: None` = 常规轮次断点，
  这正是该字段设计里写明的用法）；**正常收尾即删**（成功 / 业务错误 / 待审批挂起都算
  「没崩」）；
- 下次同会话启动时快照还在 ⇒ **上次回合没跑完**（崩溃 / 强杀 / 断电）⇒ 打印
  「上次回合未正常结束（第 N 轮，约 M 秒前）…本次是新回合，**不会自动重放**上次的工具调用」；
- **报一次即清**（同一中断不反复打扰）。

**三条纪律（写进模块头）**：

1. **不存正文**：快照 `messages` 恒为空 —— 检测中断不需要用户说了什么（少落一份正文 =
   少一份泄漏面），有测试钉死；
2. **不自动重放**：只报，重试由用户决定（自动续跑会重放副作用，属产品语义决策）；
3. **不用 `consume`**：`consume` 会留「已消费声明」，而本轮快照 id 按会话固定
   （`turn-<session>`）⇒ 声明会把后续回合的 `save` 永久挡成 `AlreadyConsumed`。
   故用 `load` + `delete`（读后清，不写声明）。

**真机验证**：

```
① 正常（缺凭据）回合后：快照文件数 = 0（不误报）
② 手工造一个「上次未收尾」快照（turn=7）再跑 chat：
   [续行] 上次回合未正常结束（第 7 轮，约 887 秒前）—— 可能崩溃或被中断；
          本次是新回合，不会自动重放上次的工具调用
   报完后快照文件数 = 0（报一次即清）
```

**剩余（明确是决策项）**：**自动续跑**（读快照后重放未决工具调用）。技术上要动
`execute.rs` 的 turn-loop，语义上要定「重放副作用」的产品规则 ⇒ 保留在待拍板清单。

### 5.4q 阻塞理由复核（2026-10-25）：一条前提已过期，四条仍成立

**为什么要复核**：停滞理由会随代码演进过期，而过期的理由会让「不接线」显得
合理（实际上是没人再看）。逐条查证如下，**证据即命令**。

| 项 | 原登记理由 | 2026-10-25 复核 | 结论 |
|---|---|---|---|
| `continuation` | 「runtime 会话存储只有内存实现」 | ❌ **前提已过期**：生产用 `SqliteSessionStore`（`cli/lib.rs` 两处构造，含封缄装配）—— 会话/审批历史**都已持久化** | 真阻塞换成了**新的一条**（见下） |
| `cross_diary` | 「日记不在检索路径」 | ✅ 成立：`memory/src/coordinator.rs` 里**没有任何 diary 相关代码**（`grep diary` 为空）；日记只有 `dream` 的写入面 | 仍阻塞（缺读取方） |
| `three_tier_vault` | 「缺 markdown 语料面」 | ✅ 成立：`ThreeTierVault` 需要**三层语料**（raw 只读源 / Living Wiki markdown / Schema Contract），全仓**零生产者**（只在 `memory/lib.rs` re-export） | 仍阻塞（缺语料） |
| `care_potential_field` | 「全仓无 distress/fatigue 信号源」 | ✅ 成立：只有 `orchestration/lib.rs` 的定义与 re-export，**零调用方、零生产者** | 仍阻塞（缺信号） |
| `speech_arbiter` | 「全双工未装配」 | ✅ 成立：`gateway/src/panels.rs:1612` 把 `voice.duplex` 硬编码为 `not_assembled`（有测试钉死） | 仍阻塞（上游面缺失） |

**`continuation` 的真阻塞（更正后）**：

1. **不是存储问题**（会话已持久化），而是 **turn-loop 集成**：需要 `execute.rs`
   在「发起工具调用前」落快照、在「进程启动时」读回；
2. **更关键的是重放安全语义**：进程在工具调用中途崩溃时，续跑意味着**重放该调用**
   —— 副作用类工具（写文件/发消息）会**执行两次**。`ContinuationStore::consume`
   的「一次性销毁」只防重复**消费快照**，不防重复**执行副作用**。
   ⇒ 安全设计应是「**消费快照 + 报告，不自动重放**」，把「要不要重试」交给人；
   这个语义需要产品确认（自动续跑 vs 报告后人工决定），属**决策项**不是工程项。

**未变的一条纪律**：宁可写「理由已过期、现阻塞是真阻塞 X」，
也不写「已完成」—— 待办条目的状态必须与代码事实一致。

### 5.4o 两处「报错说错事/没有下文」—— 已修（2026-10-25）

**① `backup verify/restore` 路径打错 ⇒ 报「备份清单损坏」**

```
$ apeireth backup verify /tmp/nope
backup operation refused: backup manifest is corrupt: /tmp/nope/manifest.json unreadable: No such file or directory
```

路径打错是**客户端输入问题**，说成「清单损坏」会让人以为自己的备份坏了。
已加独立变体 `BackupError::NotABundle`（与 `ManifestCorrupt` 分开），并区分
「包不存在」与「目录在但不是包」：

```
backup operation refused: 备份包不存在: /tmp/nope （路径打错？先用 `apeireth backup export <DIR>` 导出）
backup operation refused: 目录 /tmp/b 里没有清单 manifest.json —— 这不是一个备份包；用 `apeireth backup export <DIR>` 导出
```

**② `crypto unlock` 缺文件 / 坏件 ⇒ 只回裸 `os error 2` / 无路径**

```
$ apeireth crypto unlock --keys /tmp/nokeys.json
crypto operation refused: No such file or directory (os error 2)     ← 哪个文件？下一步做什么？
```

已带**路径 + 下一步动作**：

```
crypto operation refused: 密钥文件不存在: /tmp/nokeys.json （先用 `apeireth crypto wrap --partition <ID> --keys <FILE> --recovery-out <FILE>` 创建）
crypto operation refused: 密钥文件解析失败 /tmp/bad.json: crypto json error: expected ident at line 1 column 2
```

`--recovery` 恢复件读取同样补上路径（此前也只有裸 io 错误）。

**顺带核对（无缺陷）**：`bundle --out` 对已存在目录**不覆盖**（文本产物跳过并逐一
报出，有测试钉死）；`bio enable` 拼错会**列出全部 14 个合法机制名**；
`user add` 坏角色、`egress outcome` 坏标签都给出词表与用法。

### 5.4n 主入口 `chat` 缺凭据时不告诉用户怎么配 —— 已修（2026-10-25）

**缺陷**：用户跑的第一条命令 `apeireth chat "你好"`，在未配模型时只回：

```
canonical chat failed: provider.minimax: authentication failed: missing API key for provider.minimax.api_key
```

—— **没说怎么配**。对比 `council` / `subagent` 同一处境都有可行动提示，主入口反而最差。

**修法**：在 CLI 出口给**可自救错误族**补一句下一步动作（不猜内部错误，只认
`missing api key` / `authentication failed` 与连不上两族），提示里点名真实 env。

**过程中的两个自我纠错（都值得记）**：

1. 我第一版提示写了 `APEIRETH_OPENAI_API_KEY` / `APEIRETH_OPENAI_BASE_URL` ——
   **两个 env 都不存在**（真实是 `OPENAI_API_KEY` / `APEIRETH_OPENAI_URL`）。
   给错 env 名与给错建议同样有害（用户照做仍失败）。现已按实现核对改正。
2. 改正后 `arch_guard` 仍报 3 条违规：**我在注释里写下了那两个不存在的 env 名**，
   env 计数守卫把它当作「新增未登记开关」（162 > 160 预算）—— 连注释都算。
   ⇒ 提示里不得出现伪造 env 名字面量，反向断言改用运行时拼接。

**新增两条机检守卫**：

- `chat_error_hints_only_cite_registered_apeireth_envs`：提示里每个
  `APEIRETH_*` 名字必须在 `docs/architecture/env-registry.toml`（单一真源）里存在；
- `chat_error_hint_uses_the_real_openai_key_name`：必须点名 `OPENAI_API_KEY`，
  且不得出现带前缀的错误拼法。

### 5.4m `council` 缺 LLM 工厂时只回一行英文 —— 已修（2026-10-25）

**缺陷**：`apeireth council <主题>` 在未配 LLM 时回 `no llm factory configured`
—— 英文、无行动指引，用户不知道要设哪个 env。对比 `subagent` 同一处境给出的是
「请设 `APEIRETH_OPENAI_MODELS` (+URL/KEY) 或 `APEIRETH_API_KEY`」**同一口径的
可行动提示**（真机对照发现的质量不一致）。

**修法**：提示抽成常量 `LLM_FACTORY_MISSING_HINT`（点名两个 env），
并加守卫测试 `missing_factory_hint_names_the_envs_to_set`
（脱 env 直接钉住「提示必须点名要配的 env」）。

**顺带真拆分**：工厂/旋钮的 env 解析整块搬到 `cli/src/llm_factory_env.rs`
（`lib.rs` 有尺寸棘轮：3885 → **3812**，基线 3878 内）。

### 5.4l 认证/权限错误的「建议」指向错误的东西 —— 已修（2026-10-25）

**缺陷（真机 HTTP 抓到的两处，同一个病根：一个错误码被多种原因共用）**：

1. `auth_missing_key`（网关 401，message 明写 `gateway token required for this endpoint`）
   的 `solution` 却是「在「设置 → 模型」中保存 **API 密钥**」—— 把**网关访问令牌**
   说成了**模型密钥**，照做也解决不了；
2. `permission_denied`（网关 403，message 明写 `the calling client is not bound to
   any user`）的 `solution` 只讲「角色不足去找 owner 提权」—— 真正该做的是
   **把令牌绑定到用户**（`apeireth user add --clients <令牌名>`）。

**根因**：`ErrorCode` 是**码 → 固定建议**的单射表，但同一码实际覆盖两种原因：
- `auth_*`：模型 provider 密钥 **或** 网关访问令牌；
- `permission_denied`：角色不足 **或** 客户端未绑定用户。

**修法**：把两条建议改成**覆盖两种原因**（都说清怎么做），不动码表结构、
不破坏 wire 契约（码不变，只有提示文本更准确）。

**顺带验证（未发现缺陷）**：令牌→用户→角色整条链真机跑通 ——
`APEIRETH_GATEWAY_TOKENS` + `user add --clients tui` 后，带令牌访问
`/v1/panel/users` = **200 + owner 视图**；伪令牌 = 401 `auth_invalid_key`；
无令牌 = 401；逃生阀与令牌表互斥（配置即拒启动，符合设计）。

### 5.4k CLI 帮助漏掉 8 个可用命令 —— 已修（2026-10-25）

**缺陷**：`apeireth --help` 是 `main.rs` 里手写的一段（32 行），**漏掉**了
`dream` / `nightwatch` / `lessons` / `egress` / `bundle` / `bio` / `council` /
`subagent` 八个**真实可用**的命令，还有缩进不一致（`crypto` 多缩进、
`exposure` 少缩进）。用户从 `--help` 根本发现不了这些能力 ——
**帮助低估产品，与「文档夸大」一样违反 0 装**（方向相反而已）。

**修法**：把帮助集中成单一真源 `HELP_TEXT`（按用途分组：会话/审批/服务/
身份/记忆/治理/数据），并加**防漂移机检测试**
`help_covers_every_dispatched_command`：从 `include_str!("main.rs")` 的
`fn main` 分发块里提取所有 match 臂命令名，断言每个都在帮助文本中出现 ——
**新增命令忘了写帮助 = 测试红**（不再靠人盯）。

真机：`--help` 32 → **55 行**，20 个顶层命令 + 各命令子面全部在列。

### 5.4j 审计哈希链在多写者下分叉 ⇒ 审计永久降级内存 —— 已修（2026-10-25）

**真实事故（本机数据根上发生）**：启动时打印

```
[governance-audit] 持久审计账本不可用: governance audit record 181 hash mismatch (tamper detected); 降级内存账 (审计不持久, 如实)
```

现场证据：记录 180/181/182 的 `previous_hash` **全是同一个值** `59acafa08dee`
—— 链条**分叉**了（三条记录都挂在同一链尾上，1ms 内写入）。

**根因**：`chain_audit` 的「单写者」只是**模块文档里的假设**，没有被强制：
`production()` 每次装配都新建一个 `FileChainAuditSink`，每个 sink 各持一条内存
链、共用同一文件 ⇒ 后写者基于**过期链尾**追加 ⇒ 分叉 ⇒ 下次 `open()` 逐条重放
比对必然失败 ⇒ **审计永久降级内存账**（不再持久），且报错文案误说成
「tamper detected」（不是篡改，是并发写）。

**修法（三层，缺一层就仍会复发）**：

1. **强制单写者**：`open` 抢 `<ledger>.lock`（`create_new` 原子 + 写 pid）；
   第二个写者得到**如实错误**（「已被另一个进程持有」）而不是静默分叉；
   **陈旧锁**（pid 已不存活）自动接管 —— 一次崩溃不会永久锁死审计；
2. **进程内按路径单例**：`cli::governance_chain::shared_audit_sink`，同一进程
   多个 runtime 共用**一条**链（否则同进程多 sink 一样分叉）；
3. **坏链隔离恢复**：分叉历史文件改名 `governance-audit.forked-<ts>.jsonl`
   保留证据，随后重建空链 ⇒ 审计**恢复持久化**而不是永久降级。

**真机验证**（拿真实坏链文件）：

```
① 启动 → 报「审计链分叉（多写者历史，非本进程造成）」+ 隔离到
        governance-audit.forked-1792865394.jsonl（保留证据）+ 新建空链
② 再启动 → 无告警，链可校验（审计保持持久化）
```

**测试**：`single_writer_tests` 3 例（第二写者被拒 / drop 后释放 / 陈旧锁接管）
+ `audit_sink_sharing_tests` 3 例（同路径共用 sink / 不同路径独立 / 坏链隔离
+ 恢复可追加）。

**遗留（如实标注）**：`process::exit` 会跳过析构 ⇒ 锁文件在进程退出后可能残留；
靠**陈旧锁接管**兜底（已测），不构成阻塞。彻底清理需要退出钩子。

### 5.4i 网关输入错误被报成 500 + 误导性建议 —— 已修（2026-10-25）

**缺陷（真机 HTTP 冒烟发现）**：`POST /v1/memory/append` 传 `session: "s1"` 返回

```json
{"error":{"code":"runtime_error","message":"invalid SessionId \"s1\": ...",
 "solution":"重启 Companion 后重试, 若复现请提交日志"}}
```

这是**客户端输入错误**（`SessionId` 只接受 UUID），却被 `panel_error` 全兜底映射成
**500 + 「重启 Companion 后重试」** —— 重启治不了格式错误，那是误导。

**修法**：在边界**快速失败**（与 `session_settings.rs::invalid_request` 同口径）：

```
HTTP 400
{"error":{"code":"invalid_session_id",
          "message":"session 必须是 UUID 形式，收到 \"s1\"",
          "solution":"用 GET /v1/panel/sessions 查看现有会话 id；新建会话后再追加"}}
```

合法 UUID 仍 `201 CREATED`（真机两态都验过）。同步更新 `panel_routes.rs`
（原测试用 `"s1"` 断言 201 —— 那是因为 mock 适配器宽松，掩盖了生产上的 500）。

**同一族的第二处（已修）**：乐观并发冲突（过期 `expected_rev`）此前也返回
`500 runtime_error` + 「重启 Companion 后重试」—— 而重启治不了 rev 冲突。
现 `panel_error` 按**窄而稳定的标记**分类：认出 `revision conflict`
（`memory_governance` / `session_lifecycle` / persona 三处同族文案）⇒ **409
`revision_conflict`** + 「重新读取最新 revision 后再提交（不是故障，不需重启）」；
**认不出的一律仍 500（宁严不宽：把真故障报成 4xx 更危险）**。
分类器连同测试真拆到 `gateway/src/panel_errors.rs`（尺寸棘轮不给 panels.rs 余量）。

**遗留（未改，记录在案）**：`panel_error` 的其余分支仍是全兜底（认不出 → 500）。
彻底修需要给端口加类型化错误（`MemoryCommand` 等 trait 现在只回 `String`），
那是跨 crate 的接口改造 —— 记为后续项，不在本批硬做。

### 5.4g 问责闭环「回填只写不读」—— 已修（2026-10-25）

**缺陷**：`apeireth egress impact` 只读 `impact-ledger.jsonl`，
**从不读 `consequence-ledger.jsonl`** ⇒ `apeireth egress outcome ...` 的回填
**在 CLI 上完全看不出效果**（报告仍显示「后果未知」）。自述面用的是
`ConsequenceTracker`（读得到真值），CLI 报告用的是原始声明 —— 两者分叉。

**修法（两处）**：

1. `run_impact` 叠加后果账（与自述面**同一个** `ConsequenceTracker`，文件即共享真源），
   且**只升级不降级** —— 后果账无该行动记录时保留声明自带 outcome，
   否则会把已知后果降级成未知（把真信息变假信息，比不叠加更糟）；
2. **计数按叠加后重算**：`ledger.counts()` 读原始账，会与叠加结果打架
   （实测回填后「超期未回填=0」却仍写「后果未知=1」）。

**真机闭环验证**：

```
① 回填前            → 不可逆=1 影响他人=1 后果未知=1 超期未回填=1 ⚠
② egress outcome a1 confirmed
③ 回填后            → 不可逆=1 影响他人=1 后果未知=0 超期未回填=0（0 个超期标记）
```

### 5.4h 问责闭环「补救建议基于过期后果」—— 已修（2026-10-25，与 §5.4g 同族）

**缺陷**：`apeireth egress notify <fact> <severity>` 的**补救建议**由
`assess_correction` 生成，而它只拿得到 ledger（原始声明、后果恒 `unknown`）⇒
即使后果已经回填为 `failed`，建议里仍会出现：

> 先确认这些行动的结果（当前状态未知，系统不知道是否真的生效）

—— 而结果**其实已知**。基于过期状态给错指引，比不给指引更糟。

**修法**：

1. `notification_duty` 拆出 `assess_correction_from(&[ImpactStatement], ...)`
   作为真源（旧 `assess_correction(ledger, ...)` 委托它，行为不变）；
2. CLI 的 notify 路径**先叠加 consequence 账**（同一 `ConsequenceTracker`、
   同一「只升级不降级」口径），再据此评估补救。

**真机验证（两态都对齐）**：

```
未回填   → 后果=unknown + 「先确认这些行动的结果（当前状态未知）」   ← 正确
回填 failed → 后果=failed + 该条建议**消失**（只剩撤回/重审）      ← 已修
```

### 5.4f 问责闭环「后果追踪」缺自动回填面（2026-10-25 查证，判定为**设计项**）

ADR-0014 §5 的问责五步里，第 ③ 步「后果追踪（`unknown` 一等公民）」目前**只有手动入口**
（回填读取侧的断裂已在 §5.4g 修好；本节的「自动回填」是另一件事）：

```
apeireth egress outcome <ACTION_ID> <confirmed|failed>
```

`AccountabilityLedger` 的生产写入方只有 egress 闸（发信时落影响声明）；
行动真正跑完后，**没有任何东西把结果回填**，于是账面永远停在 `Outcome::Unknown`。

**为什么没有直接补自动回填（查证结论）**：

1. **`confirmed` 不能自动填**：它的语义是「影响已按声明发生」，需要**外部证据**
   （对方是否真收到 / 副作用是否真出现）。本地工具调用成功 ≠ 影响已确认 ——
   自动填 `confirmed` 就是替外部背书，违反 0 装。
2. **id 空间不对齐**：影响声明的 `action_id` 来自 egress 闸，而工具调用带的是
   `ToolCall.id`；两者**没有既有映射**。要自动回填，先得定「一次工具调用如何
   归属到一个已声明的影响」（1:1？1:N？跨会话怎么算？）。
3. **可自动的那一半**：`failed` 是**确定的**（工具调用出错 ⇒ 影响没发生）。
   但即便只填 `failed`，也依赖上面的 id 映射。

⇒ 归 **设计项**（需要 owner 或架构线定「行动↔影响声明」的归属语义），
不是纯接线。本批只把现状与约束写清，**不硬接**（硬接等于编造 `confirmed`）。

### 5.4d 并行工具派发：生产永不并行（2026-10-25 修）

运行时**完整支持**并行派发（`plan_parallel_window` 找窗口 / `dispatch_parallel_window`
滚动池 + 模型顺序提交），但整条链挂在 `RuntimeBuilder::with_call_scheduler` 上，
而**生产装配从不构造 `CallScheduler`** ⇒ 窗口恒为 1 ⇒ 并行派发形同不存在。

**修法（不拍脑袋列白名单）**：从**守卫的能力安全注册表**派生「可证明只读」白名单
（`is_provably_read_only`：known + 无持久化效果 + 无破坏性 + 无外部副作用 + 不联网 +
运行类只含 Read/Compute；**未知一律不放行**）。为此先给守卫注册表补登记了 5 个
真实在生产的只读工具（`tool.repo` / `tool.repo_map` / `tool.self_status` /
`tool.trait_report` / `tool.search`）—— 它们此前在安全注册表里是 `unknown`。

**过程中抓到并修掉一个真缺陷（我自己的）**：给 `tool.trait_report` /
`tool.self_status` 写的 `SourceClass::PrivateMemory` 被 `FastGuard` 折算成
风险 0.40 ⇒ 判定升为 `RequireApproval`，直接让既有契约测试
`trait_report_is_granted_by_default_with_zero_approval` 变红。
两工具读的是**派生聚合计数**（不是私密原文），改为 `SourceClass::ToolOutput` 后恢复。
⇒ 教训：**安全描述符不是文档，是会改判定的输入**；登记一个字段前要看它在
`FastGuard` 里折算多少风险分。

**装配级端到端验证**：`cli/tests/parallel_dispatch_production.rs` 从
`build_canonical_runtime_from_env()` 出发断言生产运行时**真的装上了**调度器
（为此给运行时加了只读访问器 `Runtime::call_scheduler()`），且白名单只放行只读、
一律不放行写/执行/联网/未登记能力。

### 5.4e SDK 文档对账：HTTP 早已真接，文档还在说 stub（2026-10-25 修）

`adapters/sdk/src/client.rs` 模块头写着「不实接 HTTP，走 `unimplemented!()` 守门」，
但 `invoke_tool` 自 **W5（2026-10-10）**起就是 reqwest POST + Bearer + 有界超时 + audit；
全文件也没有任何 `unimplemented!()` / `todo!()`。**文档在低估实现**（对外发布说
「SDK 不能用」），同样违反 0 装。

已按事实改写：HTTP 半场 = **PRODUCTION WIRED**；WS 流（`invoke_stream`）与
quota（`check_quota`）仍是桩，但都返**显式错误码**（不 panic、不假装支持）
⇒ 口径 **IMPLEMENTED（未接）**。`STUB_MODE` 常量语义也写清：它**只守 WS 层**。

### 5.4c 编译器本来能抓到、但被全局 `allow(dead_code)` 挡住的缺陷

`Cargo.toml` 的 `[workspace.lints.rust]` 里 `dead_code = 'allow'`，
注释写明原因是「当时只有 1 处 dead_code 警告，在别的 AI 正在改的文件里」——
于是**整个 workspace 的这道 lint 被关掉**。代价是「私有字段/函数存了不用」
这类缺陷对编译器隐身。本批临时打开 lint 拿到 60 条线索清单，逐条核查后：

| 线索 | 结论 | 处理 |
|---|---|---|
| `crates/engine/memory/src/five_dimensional.rs` `recent_memories` 从不读写 | **真缺陷**：`MemoryDimension` 有 5 个变体，但 `export_browser_entries` 只产出 4 个 —— 「Recent（24h 滑动情境窗口）」**从未产出**，矩阵宣称的「5 维时空记忆」对 Memory Browser 实际只有 4 维 | ✅ 已修：字段改为带时间戳的 `RecentMemory` + `push_recent`/`prune_recent`/`get_recent`（24h 滑动窗，边界含等号、时钟回拨不误剪）+ 导出补 `Recent` 段 + 「每个维度都必须可导出」守卫测试 |
| `crates/engine/organ/src/emergence.rs` `initial_depth` 从不读 | 非缺陷但是**冗余副本**（同值已在 `LocalRelationship` 里） | ✅ 已删 |
| `crates/foundation/plugin/src/llm_factory.rs` `role`/`model` 从不读 | 待人（副总指挥 crate） | 已在看板同步 |
| `crates/capabilities/tools/src/doc_ops.rs` `obj_ref` 从不读 | 待人（副总指挥 crate） | 已在看板同步 |
| 其余 ~55 条 | 多为测试夹具字段 / `_` 前缀保留字段 / v2.1 占位（`_llm_factory` 等已用下划线表意） | 不动 |

> **建议（不擅自执行）**：`dead_code = 'warn'` + 基线棘轮能把这类缺陷长期挡住。
> 但它会改动 workspace 共享 lint 策略并让 CI 立刻红 60 处，
> 且 CI workflow 属副总指挥范围 ⇒ 记在这里等 owner 拍板，本批不动策略。

### 5.4b 守夜人自报缺口（系统自己报告的未接线项）

`apeireth nightwatch` 会在数据不足时**如实报缺口**（这是它的设计优点 ——
不用人工审计就能发现「分析段空转」）。本批处理了两条中的一条：

| 缺口 | 状态 |
|---|---|
| `audit chain 未持久化供闲时读取 —— no-degrade 复盘段空转` | ✅ **已修**：守卫判定（decision/risk_score/reasons）此前只在内存 `Mutex<VecDeque>`，重启即散 ⇒ 新增 append-only `guard-decisions.jsonl`（`GuardDecisionLog`）+ nightwatch 读取映射。**真机验证**：无账时报缺口 → 有账时 `2 audit` 且报出 `[medium] NoDegrade: critical → info`。风险档映射是本仓**首个** score→档定义（此前不存在），已用 `governance::risk` 闭合词表交叉校验 + 单调性/6 档全覆盖测试钉死 |
| `council ballot 未持久化 —— 立场平衡复盘段空转` | 🔶 **E 类待裁定（深查后确认不能硬接）**：`council` **已在生产装配**（`config.council` + env 开关 + 后端注入），所以不是「council 没接」。真缺口是：`rubric::Ballot` 需要 `{advisor_id, stance: StanceKind, confidence, weight, domain}`，而 council 只产出 `AdvisorEvaluation { advisor, kind, verdict: AdvisorVerdict }` —— **没有 confidence / weight**，且 `Ballot::new` **全仓从未被构造过**（只有它自己的测试）。硬接必须**编造** confidence 与 weight ⇒ 按「不编造信号」纪律**停下**。待裁定：一个顾问评价如何折算成一张票（stance 从 verdict 怎么来、confidence/weight 谁给、或干脆改 nightwatch 的输入类型）—— 这是评分口径决策 |

### 5.5 待判 31 条 —— 人工过筛结论

**A 类（真缺口）16 条** —— 每个都缺一个**具体接线点**，不是「整块没写」：

| 模块 | 缺的能力 | 接线点 |
|---|---|---|
| `crates/adapters/cli/src/portable_bundle.rs` | 一键生成 U 盘便携包 | ✅ **已修**：新增 `apeireth bundle --out <目录>`（复制当前可执行文件 + 两平台启动脚本 + 便携配置 + 清单）。**接线时发现三处硬伤**：①启动脚本调不存在的 `serve --data-dir`（真实是 `gateway serve` + `APEIRETH_DATA_DIR`）②设两个全仓无人读取的变量 ③配置把**明文 P2P 桩**标成 `allow_p2p_mesh: true`。清单 `included_files` 改为按**盘上真实文件**生成（原来硬编码了三个不存在的条目）。**已真机验证**：用生成的脚本启动，`/health` 返回 ok，数据落在包内 `./data/` |
| `crates/adapters/gateway/src/file_fetcher.rs` | `file://` 拦截 + SHA-256 内容寻址缓存 + 完整性校验 + 分布式回退 | 🔶 **半修（实现补全，消费点待协调）**：查证发现 `cache_dir` 是**死字段** —— 文档宣称的「原子入库 / 磁盘内容寻址缓存」**根本不存在**（缓存只在内存、重启即散）。已真正实现：两级内容寻址（`<url_key>.meta.json` 记 URL→内容哈希、`<sha256>.bin` 存不可变字节，同内容只落一份）+ **每次命中重算哈希校验**（篡改/截断的坏条目自动清理并回源）+ 原子写（tmp+rename，先内容后元数据）+ 写失败计数可见。测试 7 条（跨重启命中/篡改拒绝/缺 blob 回退/内容去重/远端回填离线可用）。**剩余**：`TransparentFileFetcher` 仍零生产消费；消费点（MCP 资源 `file://` URI 读取、IM 附件）在副总指挥的 crate 内 ⇒ 已上看板 §四 请他指定可动文件或由他接 |
| `crates/adapters/sdk/src/error_taxonomy.rs` | 错误族 → 闭合词表映射 + 跨语言分类/可重试暴露 | ✅ **已修**：`ClassifyError` 此前**零实现**（词表有测试但没任何真实错误走进去）。现为 `SdkClientError` / `SdkError`+`SdkErrorCode` / `CodecError`+`CodecErrorKind` 全部实现映射，并新增跨语言 `ErrorFrame`（`category`/`retryable` 恒在场）。**如实标注**：语言绑定（node/python）目前只暴露 `count_tokens`/`hash_request`，还没有错误面 ⇒ `ErrorFrame` 暂无可用的跨语言出口，待绑定补错误 API |
| `crates/capabilities/tools/src/repo_map.rs` | `tool.repo_map` 工具面（符号地图 + PageRank + token 预算裁剪） | ✅ **已修**：`RepoMapTool` 工具面（`repo_map_tool.rs`）+ `RepoMapModule` 生产注册（与 `tool.repo` 同源工作区根），只读、限域、三重预算；**顺带修掉两处平方级热点**（152 文件 9.5s→0.26s、全仓 1319 文件 10.8s→2.1s）+ 复杂度回归守卫测试 |
| `crates/engine/memory/src/cross_diary.rs` | 检索命中事实时注入关联日记片段 | 记忆检索装配点 |
| `crates/engine/memory/src/milestone.rs` | 关系里程碑写入/查询（v1 REAL、v2 库级 0 接线） | ✅ **已修**：新增 `SqliteMilestoneStore`（关系史跨重启存活，此前只有内存版）+ `MilestoneRecorder`（**幂等**记录面）+ 接进 `PartnerBondModule`：`TurnStart` 记**初见**（幂等）、`AfterTurn` 阶段**真跃迁**时记一条（`Bond::evolve` 的阈值推进是确定性的）。未注入存储 ⇒ 零记录（零行为变化）。其余类型（首次分享/情绪共鸣/决策/冲突/修复）需语义判定，本批**不猜** |
| `crates/engine/memory/src/partner_store_sqlite.rs` | 伙伴/羁绊记录**跨重启持久化** | ✅ **已修**：装配换成 `SqlitePartnerStore`（与认知库同一份库）；顺带修 `ensure_schema` 从不被调用导致「能编译但用不了」（`no such table: partner_records`） |
| `crates/engine/memory/src/query_expand.rs` | 混合检索前的同义/关键词簇查询扩展 | ✅ **已修**：接在检索主路径（`coordinator.rs`），做成**召回增强**而非替换查询 —— 只在主查询召回不足 `limit` 时补跑扩展并按 id 并集去重（同 id 取更高分）。原因：打分是各项之和（BM25 无查询长度归一化），直接替换查询会让「同义词匹配多」的文档压过精确匹配 ⇒ 加词提召回不该降精度 |
| `crates/engine/memory/src/three_tier_vault.rs` | 无向量 TOC 树索引 + 逐层树路由 | README 已当已交付能力宣称 |
| `crates/engine/memory/src/universal_forget.rs` | **被遗忘权**：按 principal 全库遗忘事务 | ✅ **已修**（PR #90）：网关删用户现在**连记忆一起删**（先清记忆再落用户表；记忆面拒绝 ⇒ 硬失败 409） |
| `crates/engine/provider/src/reasoning_adapter.rs` | 推理字段归一化下发 + 转发前剥离 reasoning | ✅ **已修**：`openai_chat::parse_response_with_reasoning` 接进 OpenAI 兼容 provider 的 `adapt_response`（12 别名 → 前置 `<think>` 块）。**默认关**（`APEIRETH_REASONING_ENABLED`，模块自带既有开关，不新增 env 名）⇒ 关时逐字节零变化。关键顺序：**先归一化、后判空** —— 否则「推理模型烧光预算、content 空 + finish_reason=length」会误判成空壳响应（其实手上有内容）。Anthropic 的 `type:thinking` 块结构不同，未接（如实标注） |
| `crates/foundation/governance/src/rate_limit.rs` | 按 (capability,session) 分/时滑窗 + 四阶信任 + 黑名单 | ✅ **已修**：限流 hook 挂进生产治理链（授权之后、内容风险之前），**默认挂载**（配额内完全放行、零行为变化；超限才拒）。默认挂载而非「默认关 + env 开关」：env 棘轮只减不增已满额，且「默认关」等于这闸依然没接电 |
| `crates/foundation/orchestration/src/async_context.rs` | 四层异步上下文隔离（工具海量输出/后台通知分层） | 🔁 **A→B 更正（重复覆盖，非缺口）**：原判据是「`ContextProjector` 现为 `NoContextProjector`」——**前提不成立**。生产 `cli/src/lib.rs:1609` **无条件**调 `cognitive.register_context_projection(builder)`，注入的是 `MemoryContextProjector`（`ContextWindowManager::default()`，production.rs:1790/111），它做的正是「按预算压缩投影 + 工具海量输出用 `continuity_summary` 桩替代」（有测试 `compaction_only_changes_provider_projection`）。⇒ 生产不存在「扁平无界 transcript 直送 provider」这回事。`async_context` 的独特部分（HUD 通知层 / summary-status 微记录）在**生产也无产出方与消费方** ⇒ 归重复实现 |
| `crates/foundation/orchestration/src/care_potential_field.rs` | 疲劳/深夜挫败 → 三阶克制关怀 | **先补信号源**（distress/fatigue/流保护阻力全仓无源） |
| `crates/foundation/orchestration/src/continuation.rs` | 断点续行快照（进程重启后带未决工具调用续跑） | ~~runtime 会话存储只有内存实现~~（**前提已过期**，见 §5.4q）；**部分接线（2026-10-25，见 §5.4r）**：回合级中断检测已接进 `apeireth chat`；剩余 = 自动续跑语义属产品决策 |
| `crates/foundation/orchestration/src/speech_arbiter.rs` | 多主体同台发言权仲裁（排队/TTL/抢占打断/时长平衡） | 全双工未装配（`voice.duplex = not_assembled`） |

**B 类（重复实现）6 条** —— 不是债：同能力另有主实现。
`std_sub_supervisor`（桌面侧车真监督器承担）、`stealth_crawler`（web_fetch 工具族承担且信封治理更强）、
`chronicle_crystallizer`（consolidation + dreaming 承担）、`hallways`（`AssociationStore` 已生产接线）、
`history_streams`（`append_only` + `SqliteMemoryStore` 承担）、`wiki_fs`（`WikiEntryStore` 已有生产实现）。

**C 类（有意不接）7 条** —— 模块自述 default-off / 研究原型 / roadmap 档期。
`admission_gate`、`backend/in_memory`、`intent_brier`、`online_calibration`（终局台账裁定「维持未接」）、
`context_fold/fold_block`（家族自述 DEFAULT OFF）、`lineage_spawning`（roadmap 2027-01-15）、
`p2p_mesh`（明文桩，接线即泄密）。

**E 类（待 owner 裁定）2 条**：

| 模块 | 缺什么裁定 |
|---|---|
| `crates/engine/memory/src/semantic_axis.rs` | 归属哪个消费点（并入 absorption_insight 补第 5 段，还是删） |
| `crates/foundation/core/src/eight_anchors.rs` | 9 锚要不要有执行面（接进 council 提示词/哲学门禁，还是删登记册） |

> **D 类（真死代码）0 条** —— 没有任何模块同时满足「无重复实现 + 无设计意图 + 零引用」。
> 这本身是个结论：**模块级债的主体是「接线缺口」与「有意不接」，不是死代码。**
