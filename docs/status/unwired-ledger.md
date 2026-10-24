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
| `crates/foundation/orchestration/src/continuation.rs` | 断点续行快照（进程重启后带未决工具调用续跑） | runtime 会话存储只有内存实现 |
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
