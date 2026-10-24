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

### 5.4 待判（31 条）

逐条人工过筛中，结论待补。
