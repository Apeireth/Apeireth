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
| `as_session_store`（`runtime-assembly/src/sqlite_session.rs`） | 无引用；属遗留便捷函数，清理批次里删（低风险） |

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
| **便携包 CLI**（`portable_bundle`） | 本批 | 新增 `apeireth bundle --out <目录>`。接线时发现三处硬伤并修：启动脚本调不存在的 `serve --data-dir`、设两个无人读取的变量、配置默认打开明文 P2P 桩。清单改为按盘上真实文件生成。**真机验证**：脚本启动网关 → `/health` ok → 数据落在包内 `./data/` |
| **推理字段归一化**（`reasoning_adapter`） | 本批 | 12 别名 → 前置 `<think>` 块，接进 OpenAI 兼容 provider 响应解析。默认关（模块自带既有 env 开关，不新增）。关键顺序：先归一化后判空 —— 否则推理模型烧光预算（content 空 + `finish_reason=length`）会被误判成空壳响应 |
| **查询扩展**（`query_expand`） | 本批 | 接在检索主路径，做成**召回增强**：只在主查询召回不足 `limit` 时补跑扩展查询，按 id 并集去重（同 id 取更高分）。不直接替换查询的原因：打分是各项之和（BM25 无查询长度归一化），换掉原查询会让同义词匹配多的文档压过精确匹配 |
| **代码地图**（`repo_map`） | 本批 | `tool.repo_map` 工具面 + 生产注册（只读、默认可用、工作区限域、三重预算）。**顺带修掉既有实现的两处平方级热点**：建图是「每个定义 × 全文件子串搜索」、PageRank 每轮遍历整张邻接表 ⇒ 152 文件 9.5s、全仓 1319 文件 10.8s（卡十秒=不可用）；改 token 索引建边 + 推式 PageRank 后 **0.26s / 2.1s**，并加复杂度回归守卫测试 |
| **partner 持久化**（`partner_store_sqlite`） | 本批 | 装配从 `InMemoryPartnerStore`（重启即散）换成 `SqlitePartnerStore`（与认知库同一份库）。顺带修一个「能编译但用不了」：`ensure_schema()` 从不被调用 ⇒ `save_partner` 直接报 `no such table: partner_records`（原有测试全覆盖不到，只有真用起来的测试才抓得到） |
| **限流守门**（`rate_limit`） | 本批 | 挂进生产治理链（授权之后、内容风险之前）。默认挂载：配额内完全放行（零行为变化），同一 (能力,会话) 超 30/分钟或 300/小时才拒。默认挂载而非「默认关 + env 开关」—— env 棘轮只减不增已满额，且「默认关」等于这闸依然没接电 |

### 5.5 待判 31 条 —— 人工过筛结论

**A 类（真缺口）16 条** —— 每个都缺一个**具体接线点**，不是「整块没写」：

| 模块 | 缺的能力 | 接线点 |
|---|---|---|
| `crates/adapters/cli/src/portable_bundle.rs` | 一键生成 U 盘便携包 | ✅ **已修**：新增 `apeireth bundle --out <目录>`（复制当前可执行文件 + 两平台启动脚本 + 便携配置 + 清单）。**接线时发现三处硬伤**：①启动脚本调不存在的 `serve --data-dir`（真实是 `gateway serve` + `APEIRETH_DATA_DIR`）②设两个全仓无人读取的变量 ③配置把**明文 P2P 桩**标成 `allow_p2p_mesh: true`。清单 `included_files` 改为按**盘上真实文件**生成（原来硬编码了三个不存在的条目）。**已真机验证**：用生成的脚本启动，`/health` 返回 ok，数据落在包内 `./data/` |
| `crates/adapters/gateway/src/file_fetcher.rs` | `file://` 拦截 + SHA-256 内容寻址缓存 + 完整性校验 + 分布式回退 | 网关出站面（能力矩阵已宣称 `gw.file_fetcher`） |
| `crates/adapters/sdk/src/error_taxonomy.rs` | 错误族 → 闭合词表映射 + 跨语言分类/可重试暴露 | SDK 三个错误族（`SdkClientError`/`CodecError`/`ImError`）都不映射 |
| `crates/capabilities/tools/src/repo_map.rs` | `tool.repo_map` 工具面（符号地图 + PageRank + token 预算裁剪） | ✅ **已修**：`RepoMapTool` 工具面（`repo_map_tool.rs`）+ `RepoMapModule` 生产注册（与 `tool.repo` 同源工作区根），只读、限域、三重预算；**顺带修掉两处平方级热点**（152 文件 9.5s→0.26s、全仓 1319 文件 10.8s→2.1s）+ 复杂度回归守卫测试 |
| `crates/engine/memory/src/cross_diary.rs` | 检索命中事实时注入关联日记片段 | 记忆检索装配点 |
| `crates/engine/memory/src/milestone.rs` | 关系里程碑写入/查询（v1 REAL、v2 库级 0 接线） | 协同方 partner 已接，里程碑面没接 |
| `crates/engine/memory/src/partner_store_sqlite.rs` | 伙伴/羁绊记录**跨重启持久化** | ✅ **已修**：装配换成 `SqlitePartnerStore`（与认知库同一份库）；顺带修 `ensure_schema` 从不被调用导致「能编译但用不了」（`no such table: partner_records`） |
| `crates/engine/memory/src/query_expand.rs` | 混合检索前的同义/关键词簇查询扩展 | ✅ **已修**：接在检索主路径（`coordinator.rs`），做成**召回增强**而非替换查询 —— 只在主查询召回不足 `limit` 时补跑扩展并按 id 并集去重（同 id 取更高分）。原因：打分是各项之和（BM25 无查询长度归一化），直接替换查询会让「同义词匹配多」的文档压过精确匹配 ⇒ 加词提召回不该降精度 |
| `crates/engine/memory/src/three_tier_vault.rs` | 无向量 TOC 树索引 + 逐层树路由 | README 已当已交付能力宣称 |
| `crates/engine/memory/src/universal_forget.rs` | **被遗忘权**：按 principal 全库遗忘事务 | ✅ **已修**（PR #90）：网关删用户现在**连记忆一起删**（先清记忆再落用户表；记忆面拒绝 ⇒ 硬失败 409） |
| `crates/engine/provider/src/reasoning_adapter.rs` | 推理字段归一化下发 + 转发前剥离 reasoning | ✅ **已修**：`openai_chat::parse_response_with_reasoning` 接进 OpenAI 兼容 provider 的 `adapt_response`（12 别名 → 前置 `<think>` 块）。**默认关**（`APEIRETH_REASONING_ENABLED`，模块自带既有开关，不新增 env 名）⇒ 关时逐字节零变化。关键顺序：**先归一化、后判空** —— 否则「推理模型烧光预算、content 空 + finish_reason=length」会误判成空壳响应（其实手上有内容）。Anthropic 的 `type:thinking` 块结构不同，未接（如实标注） |
| `crates/foundation/governance/src/rate_limit.rs` | 按 (capability,session) 分/时滑窗 + 四阶信任 + 黑名单 | ✅ **已修**：限流 hook 挂进生产治理链（授权之后、内容风险之前），**默认挂载**（配额内完全放行、零行为变化；超限才拒）。默认挂载而非「默认关 + env 开关」：env 棘轮只减不增已满额，且「默认关」等于这闸依然没接电 |
| `crates/foundation/orchestration/src/async_context.rs` | 四层异步上下文隔离（工具海量输出/后台通知分层） | `ContextProjector` 现为 `NoContextProjector` |
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
