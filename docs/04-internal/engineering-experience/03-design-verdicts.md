# 设计裁定矿 · 本战役口径裁决集（批 L2 沉淀）

> `Status: ✅ 批 L2 交付（2026-10-16，纯文档批 · 未 commit）`
> 一事：沉淀本战役各批交付文档里的**口径裁决**，每条 =【裁定 → 被拒案 → 理由链】三段——
> 不只留"结论是什么"，还留"当时拒了什么、为什么拒"，防未来把被拒案当新提案重提。
> 矿脉图：[L-experience-distillation.md](../backend-enhancement/L-experience-distillation.md) §二.3（本批挖第三矿）；
> 施工宪章：[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)；姊妹矿：[01-collaboration-patterns.md](01-collaboration-patterns.md)（协同面）/ [02-failure-anatomy.md](02-failure-anatomy.md)（死法面）。
> 出处标注：**★** = 仓内锚点（可链接复核）；**☆** = 会话记录侧（案例现场未落仓，不编造）。
> 素材口径：裁定与被拒案均取自仓内交付文档/契约/提交正文的明文记录；理由链是对明文依据的归纳，无明文依据处如实标注。

## 裁定 1 · 命名让位家族一致性（`max_cost_micros` 案）

【裁定】规格速记与家族命名冲突时，**命名让位家族一致性**：花费维上限的实现取家族口径 `max_cost_micros`（与 tokens/tool_steps 的 `max_*` 兄弟同形），规格速记 `cost_micros` 只留在**维度名**位（`QuotaDimension::CostMicros`，超限帧 `BudgetExhausted{dimension:"cost_micros"}` 帧名可复现）。env `APEIRETH_QUOTA_MAX_COST_MICROS`、`RuntimeConfig.quota.max_cost_micros`、`CognitiveQuota.max_cost_micros` 走同一条同名轴。

【被拒案】照规格速记把字段直接命名 `cost_micros`——拒：同族兄弟是 `max_tokens`/`max_tool_steps`，单个字段换词法会让 env→配置面→账本三面接线者各译一遍，同族解析纪律（正整数直通／越界钳制／0 非法=`None`／未设=不设限）也失去"同名即同纪律"的读法。

【理由链】①字段名的读者是三面接线者，同族同形 = 少一次翻译错误；②维度名（报帧用）与配置字段名（接线用）各司其职，不互相污染；③家族钳制锚点（`MAX_QUOTA_COST_MICROS`）与解析函数（`parse_quota_max_cost_micros`）随名走，"与 tokens/tool_steps 同口径"成为可逐项核对的承诺。

> 出处：★[A-quota-real-interface.md](../backend-enhancement/A-quota-real-interface.md) §P2 实施记录「新符号」行（"规格速记作 `cost_micros`，实现取家族口径 `max_*`，与 `CognitiveQuota.max_cost_micros`／env `APEIRETH_QUOTA_MAX_COST_MICROS` 同名轴"；`MAX_QUOTA_COST_MICROS` 家族钳制锚点、`parse_quota_max_cost_micros` 同族解析纪律四态）。

## 裁定 2 · 族集授权不绑目标 + 补偿控制

【裁定】族集授权（批"删除族" = 该族**所有形态**生效）在族内**不绑目标**——批 `Remove-Item x` 后 `rm -rf <y>` 不重批；授权面变宽的代价不靠缩小授权面消化，靠**补偿控制**显式定价：要目标级逐批走 `AllowedOnce`（一审批一动作恰好一次）或关闭会话 `approval_remember`（= 每次人批）；审批卡族标注随卡明示授权对象（人见族而批）；git 写 L2 人工审批地板不被免审记忆越过。

【被拒案】①族集授权绑目标（同族每目标重批）——拒：把族归一省下的重批又还回去，孪生形态同权的整改目标（G2 封堵"换形逃授权"）被抵消；②只放宽不配补偿——拒：授权面静默变宽 = 没有价格的扩权。

【理由链】①族门的目的是同族孪生换形同权，绑目标会让"同族换形不重批"红利消失；②宽面授权必须配窄面出口（一次性审批/人批地板），补偿控制是组合而非补丁；③边界如实进契约「已知边界」（族集授权不绑目标、审批记忆进程内），不假称覆盖。

> 出处：★[E-approval-family-contract.md](../backend-enhancement/E-approval-family-contract.md) §6 重放语义表（同族不同目标行 + `AllowedOnce` 行 + git 写 L2 地板行）与「边界（如实）」段、§10 已知边界 3；★[E-threat-model.md](../backend-enhancement/E-threat-model.md) §5 G2 处置注（残余=族集授权不绑目标，补偿=AllowedOnce／关 approval_remember）。

## 裁定 3 · 迁移生效 vs 失效重批

【裁定】存量授权记录的兼容语义二选一钉死 = **迁移生效**：存量逐字条目（`tool.shell:<命令>`）与持久化老记录按"命令→族映射"解释后生效，不失效重批；词表外记录退回逐字精确语义（不静默扩权），未知族 id 不给族级语义（宁可无效不放宽）。

【被拒案】显式失效重批（老记录作废、逼运维重批）——拒：老记录直接作废会把运维存量名单变摆设；摩擦转嫁给运维，还会诱导"一键全批"的更糟习惯。

【理由链】①迁移生效 + 审批卡族标注（人见族而批）是更友好的口径；②方向一致更安全——族门比逐字门更宽 = 更多形态被管住，迁移不是放宽而是收紧；③不静默扩权是迁移的下限：映射函数对词表外命令回逐字对象，"解释"永不凭空造权。

> 出处：★[E-approval-family-contract.md](../backend-enhancement/E-approval-family-contract.md) §1 整改口径 4、§7「存量记录迁移语义（二选一钉死 = 迁移生效）」与「未选方案（显式失效重批）」段、§4 名单语法表；钉死测试 `approval_policy.rs::legacy_verbatim_entries_migrate_to_family_interpretation`、`approval_family.rs::legacy_verbatim_records_migrate_by_family_interpretation`（同文件 §9 测试表 9/16/21）。

## 裁定 4 · 构造式断言过护栏（`tool.shell` 案）

【裁定】测试夹具需要护栏全文扫描面所禁的字样时，断言走**构造式表达**（语义字段 / 代位 id / 运行时拼接），不落禁词字面量；护栏本体**零豁免、零放宽**。同族纪律 = 测试钉行为不钉字面（宪章 §一.4）。

【被拒案】①给扫描器加豁免/白名单放行夹具——拒：与豁免表滥用同病（02 矿死法 6），护栏变自我安慰；②摘护栏或把夹具挪出被扫面——拒：护栏名存实亡 / 测试与被扫面脱钩后漂移。

【理由链】①护栏以"字面量出现 = 执行核特判嫌疑"为判据（架构冻结全文扫描，去注释后扫执行核九个源文件，含测试面），夹具写禁词与真特判在扫描面不可区分；②被测语义不依赖那个字样——工具名字段（`name: "shell"`）+ 代位能力 id 完整承载断言，语义不损；③护栏严、测试全两头都真：扫描零命中是真零命中，断言覆盖是真覆盖。

> 出处：★[J-life-integration.md](../backend-enhancement/J-life-integration.md) 门禁证据注 1（在途夹具含字面量 `tool.shell` 撞既有架构冻结护栏全文扫描 → 其线已修夹具、隔离复跑转绿、非本批引入亦非本批遗留）；★护栏本体 [canonical_architecture_invariants.rs](../../../crates/engine/runtime/tests/canonical_architecture_invariants.rs)（`execution_core_does_not_special_case_standard_modules` 禁词表含 `tool.shell`）；★现行夹具形状 [approval.rs](../../../crates/engine/runtime/src/canonical/approval.rs)（`name: "shell"` + 代位 id，收卷提交 `bddc8aa4`）；☆会话记录（修夹具现场，未落仓）。

## 裁定 5 · 保守下界不冒充规格书（模型画像窗口）

【裁定】内置基线的上下文窗/输出取**保守下界**（routing floor：`BASELINE_CONTEXT_WINDOW_FLOOR=32768` / `BASELINE_MAX_OUTPUT_FLOOR=8192`），并在文档与注释明示「**不是**规格书」——下界是路由保守值；真规格数字只能来自画像数据面（数据目录/env 覆盖层）。

【被拒案】①拿保守下界当规格书发布——拒：保守值冒充真规格，路由与容量预期被系统性误导；②编造精确规格填表——拒：与绝不编价同病（无来源不造数），且下界一旦被当规格就再也分不清哪边是真值。

【理由链】①下界的语义是"只低估不高估"（路由不超卖）；②精确规格无来源时 `None`（未提供）是诚实态，显式 null + 原因可上屏；③能力布尔取族声明照实口径（不拔高）与下界同构——**一切保守取值都要标明性质**（"floor"不是"spec"）；④基线口径由测试钉死（"基线 == 提供商默认模型表"逐 id 一致），保守值不漂移成事实。

> 出处：★[H-model-plane.md](../backend-enhancement/H-model-plane.md) §P1 配置面第 1 条（"上下文窗/输出取**保守下界**（…，routing floor，**不是**规格书）"）+ 消费面接线图注 + 测试证据「内置基线命中：保守下界+族能力布尔+单价未提供」。

## 裁定 6 · 绝不编价

【裁定】价格/成本类数字**绝不编造**：画像行有输入+输出双价才给价，缺一半/无价/未知一律回 `None`（缺一半也不补另一半）；无价轮不计入花费、只计 `unpriced_rounds`，自述如实标「部分计量」——绝不把部分当全量、绝不对无价轮编价、不伪造超限。

【被拒案】①缺价按默认价/行情估算补齐——拒：编价；②无价轮按 0 计入——拒：把"未知"冒充"免费"，总量失真且方向恒错；③部分计量当全量发布——拒：消费端拿失真账做预算决策。

【理由链】①钱的数字错一处即决策误导（配额烧钱闸、成本分级都吃这本账）；②`None` + 原因 = 可解释的诚实态，消费端渲染 "—"/「未提供」；③冻结文案「部分计量: N 个计量轮无单价未计入 (绝不编价)」把缺口写在数字旁边；④记账二分恒等式（每轮要么计入价、要么 `unpriced_rounds += 1`，不重不漏）让部分计量可审计。

> 出处：★[H-model-plane.md](../backend-enhancement/H-model-plane.md) §P1 画像结构（"**单价绝不编价**（缺一半也不补另一半）"）；★[A-quota-real-interface.md](../backend-enhancement/A-quota-real-interface.md) §H-P1 接头（"缺一半/无价/未知回 `None`——绝不编价"）+ §P2 口径备注（部分计量口径、记账二分恒等式）；★[frontend-data-contract.md](../../02-guides/frontend-data-contract.md)（`*.reason` 冻结文案「部分计量: N 个计量轮无单价未计入 (绝不编价)」）。

## 裁定 7 · 空态"—"语义

【裁定】空态 = 显式空 + "—"：无数据时列表显式空（`sessions: []`）、计数 0、分布/缓存/计价数值列 `null`→消费端渲染 "—"；**未知不用 0 冒充**；暂无接口的维**整行 "—"**；取不到/不设限的一侧显 "—"。"0"与"—"分工：0 = 测得为零，"—" = 无值/未知/暂无接口。

【被拒案】①未知渲染 0——拒：0 是数值断言，会冒充"测得为零"；②空列表静默降级成空——拒：消费端分不清"真空"与"坏了"；③无接口硬造端点/摆假数据——拒：假旋钮（宪章 §三.1）。

【理由链】①"—"是无值的专用记号，与 0 语义分离，仪表盘不会把"没接线"读成"没有消耗"；②空态即契约（设计原则：空态写"当 X 发生时这里会出现 Y"）；③显式空 + 计数对账 + 拒收侧车让"真空"可判；④读面缺席（501/404）→「暂无接口」诚实空态，其余错误抛错上屏，不静默降级成空列表。

> 出处：★[gateway-api-contract.md](../../gateway-api-contract.md) §8b（"空态 = 显式空 + "—"：…未知不用 0 冒充"）；★[frontend-data-contract.md](../../02-guides/frontend-data-contract.md)（空态 = "—" 语义、`*.display` 取不到/不设限侧显 "—"、暂无接口整行 "—"、`SKILL_SHELF_NO_FACE` 诚实空态）；★[G-observability.md](../backend-enhancement/G-observability.md) §指标空态（空态显式 "—"、null 不冒充 0）；★[A-quota-real-interface.md](../backend-enhancement/A-quota-real-interface.md)（不设限显示 "—"、深度维继续整行 "—"）。

## 裁定 8 · 错误帧报语义不回显值

【裁定】错误/拒绝帧报**语义**（原因码、存在性、脱敏描述），**绝不回显值**（凭据/令牌/密钥材料本体）；值与报错文本分离（"值自由报错"）。报语义面包括：`reason=provider_not_configured` 这类原因码、present/absent 这类存在性布尔、`（无族·逐操作授权）` 这类对象标注。

【被拒案】①错误帧带原值（把凭据泄漏进日志/审批卡/前端屏）——拒：值进帧 = 多处泄漏面；②只报"失败"不报原因——拒：丢语义没法排障，与"错误即帧报真因"（宪章 §三.3）冲突。

【理由链】①错误帧的读者包括日志、审计、审批卡、前端，是全链最宽的广播面；②语义足够排障：原因码 + 存在性 + 脱敏命令串已可定位问题；③脱敏走统一函数全链过（命令面、输出、审批理由、审批卡命令文本）；④安全向不对称：宁可信息少一点也不回显值；⑤报语义还有"不冒充裁定"子条——提示帧刻意避开 401（不冒充服务端鉴权裁定）、拒绝帧不回显凭据本体。

> 出处：★[W2-2-remote-access-auth-delivery.md](../backend-enhancement/W2-2-remote-access-auth-delivery.md)（"401 帧不回显任何令牌值"）；★[server-deployment.md](../../02-guides/server-deployment.md)（令牌在否只报 present/absent，永不回显值）；★[self-insight-channel.md](../../02-guides/self-insight-channel.md)（凭据只回存在性布尔，本体绝不回显）；★[git-write-tiering-delivery.md](../backend-enhancement/git-write-tiering-delivery.md)（`redact_secret_surfaces` 全链脱敏）；★[H-model-plane.md](../backend-enhancement/H-model-plane.md)（误粘贴凭据字段拒开且错误帧不回显值（值自由报错））；★[audit-gap-behavior-fixes-2026-10-16.md](../backend-enhancement/audit-gap-behavior-fixes-2026-10-16.md) §1（503 刻意避开 401，绝不冒充服务端鉴权裁定）；★[J-life-integration.md](../backend-enhancement/J-life-integration.md)（拒绝帧不回显凭据本体）。

## 裁定 9 · 审计误报的勘误姿势（C-05 案）

【裁定】审计清单条目复核为**误报**时，姿势 = **不硬改 + 勘误注记 + 补证据测试 + 如实报告**：逐行读码复核（对源码不对历史文档）→ 未改任何默认值 → 写勘误注记（可整段落回清单对应节；无法就地写入时注明原因与替代载体）→ 补测试把真实行为钉住，让误报有终结物。

【被拒案】①为消清单条目硬改默认值（把对的实现改错去迁就误报）——拒：误报驱动的"修"是真回归；②只口头否认不补证据——拒：下轮审计再误报，悬案变死循环；③照单全收把误报当真缺口排期——拒：浪费施工力且掩盖真缺口。

【理由链】①判决必须对源码（文档写 X、源码是 Y 时以源码为准）；②误报根因常是审计者没看到生产组合根（内核 builder 的惰性缺省 ≠ 生产路径）——勘误注记要把"生产不经过它"的完整链路写清（组合根显式组装生产存储 → 网关复用同一条装配线 → 桌面侧车为其子进程并注入绝对路径）；③补证据测试（跨"重启"存活等）让真实行为可复核，勘误不再靠口供；④原条自带"未二次核验"标记的，先复核再定罪，定罪结论与证据测试同批交付。

> 出处：★[audit-gap-behavior-fixes-2026-10-16.md](../backend-enhancement/audit-gap-behavior-fixes-2026-10-16.md) §0 三件一览（C-05 行："审计误报…不硬改：…未动任何默认"）+ §3（读码实况逐行坐标、结论、2 个证据测试、勘误注记全文与"可整段抄回清单 C-05 节"附注）；★[engineering-review-handoff-2026-10-06.md](../engineering-review-handoff-2026-10-06.md)（"判决必须对源码，不对历史文档"）。

## 裁定 10 · 网关持形状+端口（skills 读面 O-6 案）

【裁定】网关对下游能力面**只持 DTO 形状 + 限界查询端口**（`SkillShelfQuery` bounded-context 端口），实现由组合根适配（`CliSkillShelf` 读 `SkillLibrary::scan`，StoredDoc 拒开语义原样沿用）；网关不直接依赖技能库实现面。形状即诚实面：七字段元数据、**无 `body` 字段**（卡体是 L2 封套面独占）、坏卡进 `refused` 侧车（只回文件名不泄路径）、未接线回 501 不假装。

【被拒案】网关直连技能库实现（在网关内直接取库类型/扫库目录）——拒于依赖方向与新增依赖边：adapters 层被能力面实现形状绑架，库换实现则网关重编；O-6 三阶审查正文见提交记录，被拒案细节的会话现场未落仓（☆），仓内可复核的拒证据 = 依赖方向单向 + `Cargo.lock` 零行 diff（零新依赖边）。

【理由链】①总体最优：读面（元数据七字段）与卡体（L2 封套面）分离，读面不回卡体，披露面与读面各归其位；②系统最优：网关持形状后前端契约与库内部解耦（前端 DTO 契约独立演进，`version` 缺省显「—」不编版本）；③架构最优：端口即接缝——网关持形状与端口、组合根持适配，依赖方向保持单向，改实现不动契约；④诚实面随形状走：501/404 = 读面缺席 →「暂无接口」诚实空态，不硬造端点、不摆假数据。

> 出处：★git log `dc92da9f`（提交正文 O-6 案："O-6：网关持形状+端口、组合根适配，Cargo.lock 零行 diff"）；★[gateway-api-contract.md](../../gateway-api-contract.md) §7d（只读面合同 + 实现注："DTO + `SkillShelfQuery` bounded-context 端口、组合根适配"）；★[frontend-data-contract.md](../../02-guides/frontend-data-contract.md) §10.8（DTO/端口/handler 出处注 + `SKILL_SHELF_NO_FACE` 诚实空态 + 不编版本）；☆会话记录（O-6 三阶审查的拒案细节，未落仓）。

## 附：裁定 ↔ 一行速查

| # | 裁定 | 一行口径 | 被拒案（一行） |
|---|---|---|---|
| 1 | 命名让位家族一致性 | 同名轴 `max_*` 优先，规格速记留维度名位 | 字段照规格速记命名（三面各译一遍） |
| 2 | 族集授权不绑目标+补偿控制 | 族内不绑目标，宽授权配窄出口 | 绑目标重批 / 只放宽不配补偿 |
| 3 | 迁移生效 | 存量命令→族映射后生效，词表外退逐字 | 失效重批（存量名单变摆设） |
| 4 | 构造式断言过护栏 | 构造式表达过扫描面，护栏零豁免 | 给扫描器加豁免 / 摘护栏 |
| 5 | 保守下界不冒充规格书 | floor 标明性质，真规格只来自数据面 | 拿下界当规格 / 编精确规格 |
| 6 | 绝不编价 | 缺价回 `None`，无价轮标部分计量 | 编默认价 / 无价按 0 / 部分当全量 |
| 7 | 空态"—"语义 | 0=测得为零，"—"=无值/未知/暂无接口 | 未知渲染 0 / 静默空 / 硬造假数据 |
| 8 | 报语义不回显值 | 原因码/存在性/脱敏描述，值不进帧 | 帧带原值 / 只报失败不报因 |
| 9 | 审计误报勘误姿势 | 不硬改+勘误注记+证据测试+如实报 | 硬改迁就误报 / 口头否认 / 照单全收 |
| 10 | 网关持形状+端口 | DTO+限界端口在网关，适配在组合根 | 网关直连能力面实现 |
