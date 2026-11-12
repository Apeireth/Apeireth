# 记忆质量与持久一致性

[Document-Meta]
Document: docs/design/memory-quality-hardening.md
Version: 1.1
Class: SPEC
Last-Modified: 2026-11-12
Status: 🟢 活跃
Author: lead

## 目标与范围

以本项目的用户需求、现有存储契约与可验证不变量补强长期记忆。
沿用 [D0–D5 设计](../01-architecture/memory-knowledge-depth-system-spec.md)
和[既有质量路线](../status/memory-quality-route-map-2026-10-27.md)，不新增平行记忆系统。
事实正确、作用域隔离和故障后的可解释状态优先于召回数量。
本批不导入外部项目代码、文档片段、提示词或评测数据；不改写既有研究历史。

验收沿用十八项质量指标，但必须区分算法库测试、真实装配路径与模型回答质量。
本地确定性测试不能证明真实模型的最终答题能力，也不能证明优于尚未运行的外部系统。

## 不变量

1. 持久写入失败不得把未提交记录发布为工作记忆；重试相同稳定 ID 不应挤占工作环。
2. SQLite 情节、出处及作用域元数据必须在同一事务提交；原文哈希链继续接受完整性验证。
3. 用户显式纠正应保留可追溯证据；模型回复、引用和低可信输入不能仅凭出现顺序覆盖事实。
4. 相同主题不等于相同事实键；主体、作用域、角色及版本必须有确定边界。
5. 未配置、无法测量或测试失败必须如实记录；不调整历史质量阈值来获得通过。

### 判定基础

身份按结构元组 `(class, scope, content)` 比较，显示标签仅用于诊断。
两个 SQLite 与 InMemory 组合入口中，同一稳定 ID、原文和归属的重放满足
幂等条件 `W(e, W(e, state)) = W(e, state)`，
其中 `state` 同时包含持久证据、元数据、链头及工作环；派生访问计数等既存字段保留。
一次组合写入失败时，SQLite 的上述受管持久状态保持提交前值，Working 不发布新项。
本批用明确反例、故障注入和软件回归验证这些性质；尚未执行全状态空间形式化模型检查。

## 批次与验收

| 批次 | 交付 | 验收 |
|---|---|---|
| Q1 持久一致性 | 复用 backend/coordinator，补原子存储与提交后发布 | 故障注入；SQLite 回滚；稳定 ID 重试；重启后元数据与哈希链一致 |
| Q2 中文生产路径评测 | 在既有 Runtime → SQLite → Provider 测试面补中文多轮场景 | 检查真实送达证据、来源和隔离；关键断言旧实现红、新实现绿 |
| Q3 事实纠错 | 基于 Q2 的失败证据选择最小修复 | 显式纠正、生效边界、旧证据保留、错误主体与角色对抗测试 |
| Q4 公平比较 | 固定语料、模型、预算、参数及重复策略 | 双方实例与配置齐备后才出比较；缺一方记未测 |

### 比较口径

比较前固定中文多轮样本与保留样本，按会话划分开发集和测试集；标签不得进入查询、索引或排序参数。
双方使用同一模型版本、输入与输出预算、记忆容量、召回名额、注入预算和可用工具，记录配置与语料摘要。
分别报告事实正确、纠错与时间正确、证据覆盖、噪声鲁棒、权限隔离、重启恢复、延迟与费用。
错误主体、已遗忘事实重新送达或来源伪造属于硬失败，不能被平均分抵消。
有模型随机性时报告多次配对结果及不确定性；未完成双方同条件测试，不发布领先结论。

确定性回归通过率只表示固定案例满足契约。RecordingProvider 只观测真实请求中的证据，
不生成被测答案；短案例也不构成饱和预算或大规模性能验收。

## 当前事实与边界

- `MemoryCoordinator::persist_episode` 在本批开始时先更新工作环，再分别写情节和元数据。
  这一顺序存在部分成功状态；本批通过故障注入验证具体后果后修复。
- 默认物化的一般事实调和未加载持久快照；承诺、人格、双时间关系和人工 CAS 已有实现。
  不把这些已有能力重复建设，也不把字符串相似度当作可信事实身份。
- 新物化 ID 的摘要输入改为结构化编码，既有存储 ID 不迁移；旧记录仍按原 ID 读取。
  本批未验证跨版本重新物化同一输入的 ID 等价性。
- 既有运行时测试已覆盖 SQLite 持久化、重启和 provider 请求中的实际记忆证据。
  新评测延伸这些入口，不另建排名器来替代生产路径。
- 本批不改并行工作线的 gateway 回合结果恢复文件，不实现 OS 驱动或原生桌面动作。

## 验收入口

```sh
cargo test -p apeireth-memory --test memory_writeback_consistency --locked -j 2
cargo test -p apeireth-memory --test memory_reconciliation_identity --locked -j 2
cargo test -p apeireth-memory --test conversation_memory_quality --locked -j 2
cargo test -p apeireth-runtime-assembly --test memory_provider_e2e --locked -j 2
python scripts/check_quality_metrics.py
python scripts/check_golden_baseline.py
```

Windows 跑小说基线时显式设置包含 `novels/` 的仓外数据根，保持 Python 与 Rust 评测器一致：

```powershell
$env:APEIRETH_DATA_DIR = Join-Path $env:USERPROFILE '.apeireth'
python scripts/check_golden_baseline.py
```

这些测试分别覆盖事务故障、身份边界、中文治理生命周期和真实 Provider 接线。
Provider 测试从独立 SQLite 读取发送 ID 对应的治理状态、作用域和出处；
不再把一个集合复制成“检索 / 选择 / 发送”三份后比较自身。
遗忘测试验证隐藏与不再送达，原始审计证据仍保留；它不证明物理销毁或备份清除。

## 进度与验收

### 2026-11-12 接续批次：自动纠错、失败恢复与真实回答

本轮在既有记忆协调器、治理层、AfterTurn 和 Runtime → Provider 路径补齐自动化，
不建立第二套记忆系统。自动纠错只能依据可归属的用户证据，不把模型回复、引用、工具结果或低可信文本当作覆盖权限。
恢复使用稳定身份与持久待办，写前登记、提交后确认；重启及重复处理必须保持幂等，未恢复项保留可观测状态。
真实回答评测复用实际运行链路，固定自有合成语料、模型、预算、配置和评分准则；标签不进入模型请求，
记录真实回答、错误、延迟及用量。临时密钥仅在本机调用时读取，不进入仓库、命令行参数、报告或日志。

| 批次 | 目标 | 验收 |
|---|---|---|
| Q5 | 自然语言纠错自动进入既有治理/调和路径 | 中文多轮新事实、明确纠正、有效时间、错主体/引用/模型伪造、歧义拒绝、重启与真实请求证据；旧接线红、新接线绿 |
| Q6 | 记忆写入失败自动恢复 | 持久待办、故障后重试、进程重启、重复消费、部分成功、作用域隔离及遗忘后不得复活；失败不发布未提交内容 |
| Q7 | 真实模型答题评测 | 本地确定性评测器验证后，再用用户临时 key 实际运行；保留逐例回答与失败，硬失败不得被均分抵消；无 key 记未测，不用录制 Provider 代替 |

当前状态：Q5/Q6 已实现并接入条件性生产组合，Beta 本地定向软件验收及下文红绿反证已通过；
完整工作区、真实模型复验及同提交 CI 尚未完成。Q7 首次真实调用未通过，原报告保留。
本地确定性 Provider 和回环 HTTP 仍只用于验证软件契约，不能代替真实答题结果。
当前无外部可运行实例，Q4 同条件比较仍独立记未测。

#### 本轮合同与限制

- Runtime 在接受输入及提交最终响应时记录版本化来源标记；`User` 角色本身不授予用户事实权限。
  读口按当前主体、行为模块、权限 epoch 和显式声明解析，不能合并不同 Profile 同名模块的权限。
  已遮蔽输入或输出、继承的 fork 记录和无来源标记的旧会话不成为新的覆盖证据。
  旧数据仍可读取，但不自动补造来源；新增事件需要支持该事件的 reader。
- 自动纠错在 Recall 前请求受限语义提案，再确定性核验原始输入、目标列表、确切片段、范围、授权、版本和时间。
  支持有限的第一人称及所有格中文表达、ISO 和明确中文年月日；不确定主体、引用、转述、假设、未来日期或非法提案保守弃权。
  相对日历由 Host 选择的固定 UTC 偏移解释；CLI 采用装配时本机偏移，其他 Host 缺省 UTC。
  不宣称按地区时区规则重算所有历史夏令时边界。
  语义判定来自模型，不是数学证明，也不宣称通用自然语言纠错完备。
  `CorrectPrior` 保留原文和修订审计；`StateChange` 另生成可正常召回的有界历史状态，不能把原误值说成曾经正确。
  同主体、同属性的相邻声明已有明确变更日期时，模型不得只截取新值并以空日期退回当前时间；
  方案缺少所需日期时拒绝，不从证据片段外替模型补填日期。不同属性或他人日期不能借用。
  提案协议 v2 传入 Runtime 已提交的 UTC 时间及 Host 固定偏移，作为相对日期和未来日期的判断参考，
  不能替代用户原文中的明确生效日期；用户正文中的同名字段不覆盖这些元数据。
  更正模块通过固定原因及 `applied` / `no_mutation` 回执报告结果；回执不授予记忆引用资格，模型弃权也不触发无条件覆盖。
- 已验证纠错方案在修改前写入正式 SessionStore 的不可变操作证据，沿用会话封缄；正式存储迁移为 V4。
  认知库 V20 journal 保存冻结方案、依赖、进度及终态；情节依靠稳定身份、元数据及链的原子提交重放，
  可变累积 sink 的效果与其操作回执在同一事务提交。journal 在效果成功后确认进度，不宣称跨 sink 全局事务。
  typed 恢复须共享官方 journal 的连接池；不同数据库或不声明该能力的 store 显式保持未恢复。
  单条 episode 遗忘覆盖原文召回、派生 episode、Experience、Temporal 与未完成 typed 重放；
  已成功形成的承诺对象和 Persona 聚合有各自的治理生命周期，不由单条 episode 遗忘推定已逐项撤回。
  恢复发生在下一次获得授权的 `TurnStart` 和现行 `AfterTurn`，有界扫描和消费，无独立后台守护线程。
  尚有已接受纠错或无法确认来源已扫描完时，下一轮不能把旧值作为当前答案继续交付。
  普通 AfterTurn 故障保留当前已提交回答，待办及失败仍可观测。
- Profile 写入要求显式 Host 映射、可写私有 lease 和同一协调器；模型侧调用还要求独立的模块 Agent grant。
  未配置 journal 的兼容组合不增加纠错模型调用。自定义后端及兼容提取器必须分别验收，不由默认组合测试推定能力。
  遗忘以治理隐藏、撤销依赖及抑制重放为边界，原始 SQL 审计和哈希链保留；不等同物理抹盘或清除备份。
- 评测的个人数据根、会话库和认知库均隔离；不得从个人默认 wiki 读取未列入语料的内容。
  标签只进入评分，原始请求、所有调用尝试、错误、回答、用量与延迟留在仓外评测目录。
  真实 HTTP 复用生产协议转换；额外思考模式参数仅属于显式评测配置，不代表生产 Provider 已提供该配置口。
  临时 key 只进入交互启动器的子进程环境和授权头；没有隐式付费模型或凭据发现。

#### 首次真实评测记录与后续修复

首次真实运行（记为 live01）对应源码提交 `43c136d9d9ab646402524efa2215e5c02a26986a`，
执行文件 SHA256 为 `5028E6AD26144F226D6B22EE04FB6D899CC6654AE20C961C1E6A2CB350221ADE`。
仓外证据根为 `C:\Apeireth\logs\memory-autonomy-20261112`；汇总文件 `live-01-summary.json` 与
运行目录内的 `report.json`、逐次请求、响应及生命周期记录共同保留，不以修复后重评分覆盖原报告。
该轮 36 个槽位为 **14 passed、19 hard_failure、3 runtime_error**；
报告 `complete=false`、`live_quality_passed=false`，不能写成质量验收通过。

失败按实际边界分开记录，单个槽位可同时有多种问题：

- 更正场景的三次 `runtime_error` 均在模型发送前被来源审计拒绝；原审计将生产裁剪后的正文与完整持久正文比较，
  不能把这种拒绝计为模型收到问题后的答题失败，也不能因此绕过发送前审计。
- 状态变化场景两次模型提案主动弃权，属于语义判定未完成；另一次提案遗漏相邻声明的明确日期，
  旧实现以当前时间提交，导致历史区间错误。这是需要修复的产品接受边界，不能归为日期解析器不认识合法中文日期。
- 主答存在错误事实、状态、引用或无支持引用；时间输出还暴露了整数与字符串合同不清、个别年份错误。
  原始失败仍按当时判据保留，输出约定澄清不把错误年份、错误事实或无支持引用转为通过。

后续修复批（Beta）源码已冻结，本地定向软件验收通过；真实模型质量尚无通过结论。变更边界如下：

- 生产记忆正文增加机器可读的 `source_role`、`claim_kind`、`citable_as_fact` 与 `text` 字段，
  依据持久来源、派生关系和正式治理回执生成，不从评测答案或用户伪造标签授予资格。
  来源资格是数据归属与引用提示，不是执行授权，也不证明正文事实为真；保守分类不表示通用语义理解完备。
  模型与工具输出仍可保留为记录，但不能仅凭确认文字充当用户事实证据。
- 候选原始分值不因该批变更而抬高；来源覆盖规则会调整交付顺序，避免同请求的模型回声挤占可引用的独立来源。
  此行为属于生产召回变化，须重跑既有检索门禁，不由本轮案例推定一般排名收益。
  预算前候选池上限为 64，按独立可引用来源代表、其别名和其他记录分层，各层复用当前配置的 MMR。
  来源覆盖及各层 MMR 的定向回归已通过，候选原始分与 `min_score` 准入条件保持；不允许以来源资格越过最低分。
- 发送前审计复用生产正文投影函数，核对脱敏与字符裁剪后的确切可见正文，并独立核对持久来源资格。
  评分仍要求完整持久正文的预定支持哈希、真实送达且可引用的来源、正文实际可见的目标值以及精确事实元组同时成立；
  仅有同值子串不能证明引用正确，已裁去内容不能用来支持答案。
- 正式接线发现并修复类型化来源 ID 的存储身份解析与下游预算单位不一致。
  `typed:episode:` 是显示身份前缀，来源核验使用对应的实际存储 ID；缺少可信来源时角色仍为 `unknown`，不由前缀授予资格。
  Recall 从 Runtime 的通用读口获取实际预算，按现有每来源比例换算后编译完整记录；新来源格式按 Unicode 字符计费。
  认证模块可显式声明整块投影，跨来源披露与现有全局裁剪都不交付半条结构化数据；
  省略在派生 Tool 上标记 `context_budget`，持久原文不变，正文自报标记不能选择该策略。
  整块策略来自正式模块 ToolEvent 的外层元数据，来源正文仍是不可信数据，不因此获得执行权限。
  保留既有不可信包裹、安全转义、每来源硬限额及无 spill 时全局 advisory 语义，不宣称新建全局硬上限。
- 主答合同明确属性字段、裸来源 ID 及带三位毫秒的 RFC3339 UTC 字符串；不将整数时间戳或带显示前缀的 ID 自动改写为合格答案。
  新增诊断只分辨字段、引用和投影错误，不放宽通过判据。期望事实、日期与支持正文不因此更改，也不把评分标签发送给模型。
- 新运行必须保留新的源码、配置和提示词哈希，并使用新输出目录；此次合同澄清、来源呈现与产品修复并行发生，
  不能将前后通过率差异单独归因于某项记忆算法。完整工作区、真实模型复验和同提交 CI 仍待最终登记。

#### Beta 本地定向验收与反向验伪（2026-11-12）

本节只登记已读取的本地日志，不汇总为新的全仓测试数。以下定向命令对应的软件测试已通过：

| 仓外日志 | 实际覆盖 |
|---|---|
| `beta-memory-final-targeted.log` | Memory 库测试及 `memory_source_evidence`；包括预算前来源覆盖、最低分保护、来源分层保留配置 MMR、确切来源及历史核验 |
| `beta-assembly-eval-v4.log` | `memory_automatic_correction`、`memory_autonomy_production`、`memory_writeback_recovery`、`profile_memory_evidence_authority`、`session_operation_evidence` 与回答评测 example 软件测试 |
| `beta-runtime-budget-evidence.log` | Runtime `committed_evidence` 与 `context_budget` 的实际接线；新增省略元数据还由下述 atomic 恢复绿色日志覆盖 |

测试修正按正式合同保留了更强的可判真断言：

- 双时间查询将有效时间与获知时间分开：历史与现行状态查询显式设置记录时刻的 belief cutoff，
  另外保留默认 belief 为事件时刻时仍返回旧值的断言；没有把晚报状态伪装成当时已经获知。
- 隔离 `SingleShot` 子调用不执行父 Runtime 行为钩子；恢复验收重新装配 Host 后，
  通过实际授权的新 `TurnStart` 消费冻结意图，不以直接模块调用冒充生产恢复。
- 实际送达可引用经 SQL 核验父正文、提交来源和治理回执的派生别名；
  不要求预算选择必须同时送达全部原文 ID，但现行正文、实际 ID、来源支持及授权断言保持。

`reverse-wiring-beta.json` 记录下表四项均发生真实断言失败（退出码 101），
随后恢复原源码逐字节一致并退出 0；已复核四处当前源码散列与该记录一致。
各 `reverse-<control>-red.log` / `reverse-<control>-green.log` 的实际测试行如下：

| control | RED 实际输出 | GREEN 实际输出 |
|---|---|---|
| `runtime-post-read-lease` | `test revocation_during_the_owning_store_read_cannot_release_committed_input ... FAILED` | `test revocation_during_the_owning_store_read_cannot_release_committed_input ... ok` |
| `production-source-port` | `test production_source_option_delivers_verified_user_and_nonfactual_assistant_records ... FAILED` | `test production_source_option_delivers_verified_user_and_nonfactual_assistant_records ... ok` |
| `correction-required-date` | `test a_live_proposal_that_omits_the_adjacent_change_date_cannot_silently_write_today ... FAILED` | `test a_live_proposal_that_omits_the_adjacent_change_date_cannot_silently_write_today ... ok` |
| `authenticated-atomic-policy` | `test atomic_per_source_overflow_is_omitted_but_body_cannot_select_atomic_policy ... FAILED` | `test atomic_per_source_overflow_is_omitted_but_body_cannot_select_atomic_policy ... ok` |

这些证据支持本地定向软件验收，不能转写成真实模型 Beta 成功、全仓通过或适用 CI 已完成。

#### 本轮验收入口

```sh
cargo test -p apeireth-runtime --test committed_evidence --test context_budget --locked -j 2
cargo test -p apeireth-memory --lib context_compiler --locked -j 2
cargo test -p apeireth-memory --test memory_source_evidence --test memory_automatic_correction --test writeback_recovery --locked -j 2
cargo test -p apeireth-runtime-assembly --test memory_automatic_correction --test memory_writeback_recovery --test memory_autonomy_production --test profile_memory_evidence_authority --test session_operation_evidence --locked -j 2
cargo test -p apeireth-runtime-assembly --example memory_answer_eval --locked -j 2
cargo run -p apeireth-runtime-assembly --example memory_answer_eval --locked -j 2 -- offline-check --output NEW_DIRECTORY
```

真实运行须显式指定服务地址、模型、采样/思考模式、输出和总调用上限、重复数及新输出目录。
启动器为 `crates/engine/runtime-assembly/examples/memory_answer_eval/run-live.ps1`；密钥不能作为脚本参数。
未运行、失败、无效设置或未接线的槽位不算答题通过；软件契约通过也不转写成模型质量分。

#### 资源变更

本轮新增可信提交证据、纠错事务、恢复 journal 和生产行为，确有源文件及行数增长。
最终按实际变化更新相关非 LOCKED 尺寸预算，提交信息说明增量；不搬移测试或压缩注释掩盖增长。
仅增加已有工作区 Provider 的开发依赖以复用正式协议转换，不引入外部包版本。
历史质量阈值、小说卷、检索基线和 LOCKED 五项均不因本轮新增功能变动。

### 前一批 Q1–Q4 历史快照

以下状态与本地验收属于先前已合并的一致性批次，不作为本轮 Q5/Q6/Q7 的验收结果。

| 项目 | 当前状态 | 证据 / 还差什么 |
|---|---|---|
| Q1 | PRODUCTION WIRED | 默认协调器调用组合写；两 SQLite 原文、sidecar、链头同事务，成功新插入后发布 Working；既存 ID 不回填工作环。故障注入、身份冲突、派生元数据保留、淘汰后重试及重开检查通过。legacy 默认顺序写仍可能留下原文，不承诺原子性；尚无 AfterTurn 自动恢复队列接线。 |
| Q2 | 软件回归通过 | 自有中文案例经真实 SQLite/协调器及 Runtime → RecordingProvider；覆盖显式治理纠错、旧 revision 拒绝、角色/主体隔离、重复、重启与遗忘隐藏。尚未测真实模型答案和饱和预算压力。 |
| Q3 | 部分完成 | Working 已应用治理后的正文；调和与物化 ID 使用结构化身份，不再由显示分隔符混淆主体；结构化事实/多值关系保留独立断言，临时 ID 不覆盖快照。一般自然语言自动纠错仍未实现，不能把本批当作完成该能力。 |
| Q4 | 未测 | 暂无外部可运行实例；先完成自有实现与评测 |

### 2026-11-12 前一批本地验收

工具链：`rustc 1.97.1` / `cargo 1.97.1`，Windows；所有 Cargo 命令清空 `RUSTC_WRAPPER`。
`cargo test -p apeireth-memory -p apeireth-plugin -p apeireth-runtime-assembly --all-targets --locked --no-fail-fast -j 2`
退出 0；新增场景均无忽略，现有忽略设置未修改。
`pwsh -NoProfile -File scripts/fmt-check.ps1`、`python scripts/check_crate_budget.py --check`
及 `python scripts/arch_guard.py --strict` 退出 0；未增加依赖、预算或基线。
`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 退出 0。
显式设置仓外数据根后，`python scripts/check_golden_baseline.py` 退出 0：历史臂与生产臂均实际跑到语料，
检索指标未低于既有基线，证据送达观察项无变化；基线文件与阈值未修改。
首跑目录不一致导致的失败已保留，不计为通过。
跨平台全仓测试与合并裁决以同一 PR 提交的远端适用检查为准；全部通过才允许合并。

内部知识卷的 `ig-v1-027/028/030/121/123/127` 随本批源文档纠正而同步修订依据。
其中一题去掉了错误的历史前提，另两题改指向实际承载依据的章节；题号、题数及严格子串校验保留。
该内部卷修订与源语料变化需独立记录，旧内部卷分数仅保留作历史，不能据此声称算法收益。
小说卷、其基线及全部验收阈值未因这次修订而变化。

反向验伪的有效输出：

```text
# 身份修复前：新身份边界测试真实断言失败；修复后通过
test display_labels_do_not_merge_distinct_persona_identities ... FAILED
test display_labels_do_not_merge_distinct_persona_identities ... ok

# 仅恢复旧协调器接线：会返回旧事实且留下部分写入；恢复新接线后通过
test governed_chinese_correction_reaches_every_layer_and_survives_restart ... FAILED
test sqlite_sidecar_failure_rolls_back_evidence_and_head_before_working_publication ... FAILED
test governed_chinese_correction_reaches_every_layer_and_survives_restart ... ok
test sqlite_sidecar_failure_rolls_back_evidence_and_head_before_working_publication ... ok

# 首版环内去重不足：淘汰后重试仍失败；加入存储真实写入结果后通过
test replay_of_an_evicted_stable_id_does_not_reenter_or_displace_working_memory ... FAILED
test replay_of_an_evicted_stable_id_does_not_reenter_or_displace_working_memory ... ok
```

反证只计实际运行到断言的失败；在途源码造成的编译失败不计为红证据。
这些是固定反例与回归结果，不是外部系统比较分数。
全量测试数字仍以[测试真源](../03-reference/test-baseline.md)为准。
