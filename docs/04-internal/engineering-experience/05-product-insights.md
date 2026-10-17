# 产品洞察矿 · 内测判词与产品级洞察（批 L3 沉淀）

> `Status: ✅ 批 L3 交付（2026-10-17，纯文档批 · 未 commit）`
> 一事：沉淀产品面的**洞察**——内测/owner 的判词如何变成制度，每条 =【洞察 → 制度化落点 → 可复用判据】三段；
> 判词的价值不在"说得好听"，在于它指向的病灶能被制度化封死、判据能被下一个施工者直接执行。
> 矿脉图：[L-experience-distillation.md](../backend-enhancement/L-experience-distillation.md) §二.5（本批挖第五矿，末矿）；
> 施工宪章：[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)；工程总纲：[ENGINEER-MANIFESTO.md](../ENGINEER-MANIFESTO.md)。
> 姊妹矿：[01-collaboration-patterns.md](01-collaboration-patterns.md)（协同面）/ [02-failure-anatomy.md](02-failure-anatomy.md)（死法面）/ [03-design-verdicts.md](03-design-verdicts.md)（裁定面）/ [04-methodology.md](04-methodology.md)（活法面）。
> 出处标注：**★** = 仓内锚点（可链接复核）；**☆** = 会话记录侧（判词原话与现场未落仓，不编造）。
> 素材口径：制度化落点全部可指回仓内文档/测试；判词原话多在会话记录（☆），仓内只存其制度化产物——本矿如实区分两者。

## 洞察 1 · 内测判词的价值："开关不均匀生效" → 开关权力审计表制度化案

【洞察】内测用户一句"开关不均匀生效"，是任何内部审计都很难自发产生的判词。用户是**按开关的承诺**使用产品的：承诺范围与实际权力的任何错位（有的开关即效、有的悄悄没接线、有的名字管的事实际没管住），在用户侧第一时间显形——而在工程侧，同一个错位分散在注册层/授权层/生效时机三层里，逐层看都"没错"。判词的价值 = 用户把三层错位**压成一句话**还给了工程侧。

【制度化落点】三件套封死这类病灶：
1. **开关权力审计表**（[capability-switch-power-audit.md](../../02-guides/capability-switch-power-audit.md)）：核查命题「**开关名 = 开关权**」——摆设开关（名称管的事实际没管住）与幽灵权力（名称外的实际权力）都是病灶；逐行给"声称范围/实际范围/处置"，整改先例 = file_write 从"只管补丁面"整改为**唯一写总闸**（shell 写命令同闸），名册按**实际注册条件**取值（依赖图谱槽的开关报 false 而非照抄 config）；
2. **宪章条款**：命名即承诺（[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §一.3：符号/开关/字段的名 = 它的全部权力）；
3. **生效时机契约**：哪些旋钮即效、哪些必须重启按 spawn 快照**分类声明**（热应用/重启契约），"热更假生效"这类真 bug 修掉后有回归锚；桌面侧另有旋钮四方对账审计（后端语义/桌面旋钮/文档口径逐行对账，"UI 有旋钮但后端不生效"即判病）。

【可复用判据】新增任何开关/旋钮三问，逐问可执行：
1. **名实**：名称承诺的范围 == 实际权力范围？依赖主开关（单独设=无效果）、fail-closed 子依赖、恒授权无开关面（如只读工具）都要在名册/描述里如实声明；
2. **时机**：生效时机（即效/需重启）有声明且与实现一致？
3. **名册**：名册/描述按**实际注册条件**取值，不照抄配置值（缺依赖时报 false 是诚实，报 true 是假旋钮）。
守住三问的落法 = 审计表逐行 + 行级锁定测试（表结构与全责文案各有复查脚本锁死），不是一次性检查。

> 出处：★[capability-switch-power-audit.md](../../02-guides/capability-switch-power-audit.md) 抬头（开关名=开关权、摆设开关/幽灵权力）+ §一（file_write 唯一写总闸整改行、名册按注册条件取值行）+ §三/§四（全责文案三落点、行级锁定测试指针）；★[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §一.3（命名即承诺）；★[live-verification-ledger.md](../live-verification-ledger.md) #2（"Shell 开关即效（开→可执行、关→工具消失，双向实证）"与 6 个真 bug 含"热更假生效"）+ #24 ③（热应用/重启契约：按 spawn 快照分类）；★[capability-knob-audit-2026-09-12.md](../capability-knob-audit-2026-09-12.md) §1（"UI 有旋钮但后端不生效"对账判定）；☆内测判词原话（"开关不均匀生效"，会话记录，未落仓）。

## 洞察 2 · "工具在清单≠能产出结果" → 能力四级口径与证据链制度化案

【洞察】内测用户判词"工具在清单 ≠ 能产出结果"：清单/设置页/名册是**声明面**，能产出结果是**证据面**，两者之间隔着"接线、默认开、真机验"三道门。工程侧的同构病灶是"写完代码就算交付"——库级实现了、没进生产装配；接了线、默认关；默认开了、没真机验过。用户不看过程，只看结果，所以这条判词天然就是验收口径。

【制度化落点】四级口径阶梯 + 证据链三件：
1. **四级口径**（[system-capabilities.md](../../01-architecture/system-capabilities.md) 头注）：`IMPLEMENTED（代码存在）≠ PRODUCTION WIRED（接入生产主路径）≠ DEFAULT ENABLED（无需 opt-in）≠ HARDWARE VALIDATED（真机验证）`——一切能力自述先报自己在哪一级；
2. **证据链**：[claims-evidence-matrix.md](../claims-evidence-matrix.md)（每条对外声明 → 证据 → 等级）、[live-verification-ledger.md](../live-verification-ledger.md)（"什么测过、什么没测"唯一权威，写"已验证"前先查）、差距审计两级化（"库级已实现但未接生产装配"与"真缺口"分开记账）；
3. **抓假账先例**：接线假账靠**生产调用点实证**抓出（某输出脱敏函数全仓只有定义+导出、零生产调用——IMPLEMENTED ≠ PRODUCTION WIRED 当场现形，随后真接线）。

【可复用判据】
1. 写"可用/已实现"之前先答**哪一级**，答不出就不写；
2. "在清单"的真值看**名册按实际注册条件取值**（洞察 1 判据 3），"能产出结果"的真值看**调用链实证**——注意双向陷阱：名字扫描"0 引用"也会误判（crate 内部互调也是生产路径，已有三审推翻旧结论的先例），判据是调用链走到生产主路径，不是名字计数；
3. 每个"✅"能指回证据（测试名/台账行/真机记录），指不回的降级为"未实测"。

> 出处：★[system-capabilities.md](../../01-architecture/system-capabilities.md) 头注（四级口径阶梯原文）；★[claims-evidence-matrix.md](../claims-evidence-matrix.md)（声明→证据→等级矩阵）；★[live-verification-ledger.md](../live-verification-ledger.md) #48（`scan_and_sanitize_output` 零生产调用——接线假账现形案）/ #41（"0 引用=未接"是名字扫描陷阱，三审推翻两条旧结论）；★[v1-vs-v2-capability-gap-audit-2026-10-06.md](../v1-vs-v2-capability-gap-audit-2026-10-06.md) §7.1/§7.2（"库级已实现但未接生产装配" vs "真缺口"两级化）；☆内测判词原话（"工具在清单≠能产出结果"，会话记录，未落仓）。

## 洞察 3 · 模型的实测精神：实测而非推断的文化

【洞察】"模型的实测精神"是本战役被 owner 点名的工程气质：模型（与任何施工者）最强的可信度来源不是推理流畅，而是**实测**。本仓为此摔过两跤：一次是调研线六个子代理写"真账"全程零实测、纯凭账面推断，被 owner 当场抓出（定性为 O-5 失守）；一次是安全告警归因靠推断、把不在名单里的条目推断进去，API 实取后推翻——自己都标注了"推断"，仍被定性为"**叙事先于实测**"。结论：推断不是罪，**推断冒充实测**才是罪；文化靠"实测/推断/未验"三分标注落地。

【制度化落点】
1. **0 装 PASS**（[AGENTS.md](../../../AGENTS.md) §1）：未接线写「未接线」，未实测写「未实测」，双向性（不许把"已实现未接线"说成 0 装，也不许反向）；
2. **验收权威**：[live-verification-ledger.md](../live-verification-ledger.md) 是"什么测过、什么没测"唯一权威——写文档/注释/提交前先查它，别把挂账的当已验的；
3. **数字纪律**：无脚本不发布数字（每个数字可复核 = 命令 + 日期 + 工具链）；
4. **实测文格式**：结论三分标注（实测过 / 推断 / 未验），所有结论附可复现命令（既有交接文的抬头口径先例）。

【可复用判据】
1. 任何结论出门前打三个标签之一：**实测**（附命令+日期）/ **推断**（明写"推断"，不得升级为事实）/ **未验**（明写"未验"）；
2. 发现推断错误**立即纠正并留档**（含"我当时怎么错的"），不悄悄改口——"叙事先于实测"是要被点名的病；
3. 悬案裁决用实取/实跑（示踪、探针、API 实取），不用第二层推断去补第一层推断。

> 出处：★[round-13-1-0-maturity-audit-2026-08-28.md](../round-13-1-0-maturity-audit-2026-08-28.md)（"R11 6 sub-agent 写真账时…0 实测 1.0 真代码…这是 O-5 失守，用户 catch 对"）；★[handoff-18crate-ci-fix-2026-09-23.md](../handoff-18crate-ci-fix-2026-09-23.md) 头注（"只写实测过的事实；推断一律标'推断'，没验的一律标'未验'"）+ 推断错误勘误段（"仍应视为一次'叙事先于实测'"）；★[AGENTS.md](../../../AGENTS.md) §1（0 装 PASS 与"每个数字必须可复核"）；★[live-verification-ledger.md](../live-verification-ledger.md)（验收权威定位）；★[ENGINEER-MANIFESTO.md](../ENGINEER-MANIFESTO.md) §13（"`git fetch` 失败 ≠ `git push` 失败…不要凭 fetch 失败推断 push"同构教训）。

## 洞察 4 · "AI 先用、人后用"第一律的诞生现场

【洞察】一条产品第一律的诞生方式本身就是方法论：它不是设计文档里推导出来的，而是**吸收战役现场 owner 一句裁定**（2026-10-13）——「Apeireth 是给 AI 用的」；裁定当天进吸收候选池立为**设计第一律**，随即回写施工宪章 §〇 成为一切能力设计的裁定顺序，并成为吸收任何外部项目的统一提问法。判词→裁定→条款→日常判据，一条链在同一波吸收战役里走完——洞察的保质期取决于**制度化的速度**。

【制度化落点】
1. **宪章 §〇**（[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §〇，2026-10-13 立）：能力先做成**工具**（模型可调用、过治理、入名册）；人类界面是同内核的**包装出口**（不另起炉灶、不两套逻辑）；判据：删掉人类界面后，该能力对 agent 仍然完整可用；
2. **吸收裁定顺序**（[absorption-candidates-2026-10.md](../absorption-candidates-2026-10.md) §〇）：吸收任何项目先问『AI 拿到它能干什么』，再问『人怎么玩』；
3. **落地实形**：一个引擎两个出口（备份面：模型面工具先有、CLI 面同内核包装），后续能力照此成形。

【可复用判据】设计新能力先过三问，第三问是硬判据：
1. 它是**工具**吗（模型可调用、过治理、入名册）？
2. 人类界面是同内核的**包装出口**吗（有没有第二套逻辑）？
3. **删掉人类界面后，该能力对 agent 仍然完整可用**吗？——删不删都一样的能力，才叫过了第一律。

> 出处：★[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §〇（设计第一律全文与三步裁定顺序）；★[absorption-candidates-2026-10.md](../absorption-candidates-2026-10.md) §〇（2026-10-13 owner 裁定原文与吸收提问法）；★[G5-backup-restore-delivery.md](../backend-enhancement/G5-backup-restore-delivery.md) 头注（"一个引擎, 两个出口: 模型面 `tool.backup_export` (AI 先用) + CLI 面"落地实形）；★[philosophy-study/00-master-synthesis.md](../philosophy-study/00-master-synthesis.md)（"吸收纪律照旧：设计第一律（AI 先用、人后用）"）。

## 洞察 5 · "用户觉得重复 = 分层没说清"案（本会话）

【洞察】用户读文档面时判"重复"，病灶往往**不在内容重复，而在分层没说清**：同一事实按层各放一份（宪章 / 经验矿 / 对外指南 / 手册），若读者无从判断"哪层是权威、哪层是转述、我该读哪层"，重复感就是导航缺失的症状——删内容是错药，补分层声明才是对药。本会话（批 L3）当场验证：对外指南一旦声明"独立可读 + 指向两份宪章"，重叠内容即变为**分层分工**而非重复。

【制度化落点】
1. **链接不重抄**：小文件族"一矿一文件"，重叠处互相链接不重抄（[04-methodology.md](04-methodology.md) 读法注先例）；
2. **单一真源**：口径只允许一个"抄写点"，其余皆引用（测试数字 → [test-baseline.md](../../03-reference/test-baseline.md) 只引用不抄写，[AGENTS.md](../../../AGENTS.md) §3）；
3. **文档头读法注**：每份文档头写"一事 + 读法 + 上位文件"，让读者三秒定位本层职责（本矿族与对外指南均照此）；
4. **历史口径注记**：带日期的历史记录保留原值 + 「历史口径（日期）」标注，不靠重复解释解决冲突。

【可复用判据】读者报"重复"时，先查分层声明再动内容：
1. 每层职责有一句话吗（这层写什么、不写什么、权威在哪）？
2. 重叠内容是**链接**还是**抄写**？（全库同一口径只允许一个抄写点）
3. 新读者能顺着"读法注 → 上位文件"三跳找到真源吗？
三问都"是"仍有重复感 → 才考虑合并内容；否则补分层声明即可。

> 出处：★[04-methodology.md](04-methodology.md) 读法注（"重叠处互相链接不重抄"）+ §四（口径纪律细则）；★[AGENTS.md](../../../AGENTS.md) §3（测试数字单一真源——只引用不抄写；历史不追溯改写）；★[docs/README.md](../../README.md)（分层导航树：01-architecture / 02-guides / 03-reference / 04-internal 分层目录）；★[02-guides/multi-ai-construction-guide.md](../../02-guides/multi-ai-construction-guide.md)（本批对外指南"独立可读 + 指向宪章与 MANIFESTO"收口实形）；☆判词与现场（"用户觉得重复"，本会话，未落仓）。

## 洞察 6 · 分诊与吸收的方法论：随记 60 项 → 真金提炼案

【洞察】owner 随手记录的 60 项（视频随手记录习惯触发）经分诊变成真金：随手记的杂音量不是负担，是**未分诊的资产**；价值不在记了多少，而在"分诊 → 调研 → 裁定 → 落地 → 回写台账"这条流水线有没有建成。流水线建成后产出惊人：五类分诊总表、5 条机制级 TOP 候选全部进入施工序、并且被产品化为它自己的能力（随手记入口 + 书→技能沉淀）——**吸收优秀实践从一次战役动作变成产品本能**。

【制度化落点】
1. **吸收候选池**（[absorption-candidates-2026-10.md](../absorption-candidates-2026-10.md)）：分诊总表（五类、每项"一句话假说（它教我们什么）"+ 核实状态，未核实标"待研"不望文生义）+ TOP 候选 + 调研队列（"一项目一小节、机制假说+实证+吸收裁定三段式"，见 [04-methodology.md](04-methodology.md) 配方 3）+ 落地追踪；
2. **吸收台账**（[absorption-ledger.md](../absorption-ledger.md)）：吸收后回写台账可追账（哪批验收、证据在哪）；
3. **产品化回环**：随记习惯 → 随手记工具（[quick-capture-delivery.md](../backend-enhancement/quick-capture-delivery.md)：一键记一条→未整理原料入记忆流）；吸收战役 → 书→技能沉淀（[I-absorption-automation.md](../backend-enhancement/I-absorption-automation.md) P 轴分期，文件名保留旧名防断链）——判词与方法论最终都变成产品能力。

【可复用判据】面对一批杂乱输入，五步判据逐步可执行：
1. **只分诊不改写**：原料原样保留，按类归堆（分诊总表式）；
2. **每项一句假说**：它教我们什么？说不出假说的先标"待研"，不望文生义；
3. **吸收裁定过第一律**（洞察 4 三问）；
4. **落地必回写台账**：吸收后能追账（验收批 + 证据指针）才算吸收完成；
5. **中性收口**：借鉴思路、产物不留痕迹（宪章纪律，表述扫描零命中）。

> 出处：★[absorption-candidates-2026-10.md](../absorption-candidates-2026-10.md) 头注（"owner 随记 60 项（视频随手记录习惯触发）"）+ §一五类分诊总表 + §二 TOP 候选 + §三调研纪律 + §三之半落地追踪 + §四登记/回写；★[absorption-ledger.md](../absorption-ledger.md)（吸收台账回写先例）；★[quick-capture-delivery.md](../backend-enhancement/quick-capture-delivery.md) 头注（"owner 随记习惯的产品化"）；★[I-absorption-automation.md](../backend-enhancement/I-absorption-automation.md)（P 轴分期：吸收自动化）；★[04-methodology.md](04-methodology.md) 配方 3（一项目一小节、三段式）。

## 附：洞察 ↔ 制度化落点 ↔ 对外指南对照速查

| # | 洞察（判词/现场） | 制度化落点 | 可复用判据（一行） | 对外指南落点 |
|---|---|---|---|---|
| 1 | "开关不均匀生效" | 开关权力审计表 + 命名即承诺 + 生效时机契约 | 名实/时机/名册三问，锁定测试钉死 | [指南](../../02-guides/multi-ai-construction-guide.md) §口径纪律 |
| 2 | "工具在清单≠能产出结果" | 四级口径阶梯 + 证据链三件 | 报能力先报哪一级；✅ 必能指回证据 | [指南](../../02-guides/multi-ai-construction-guide.md) §门禁三级 |
| 3 | 模型的实测精神 | 0 装 PASS + 验收权威 + 无脚本不发布数字 | 实测/推断/未验三分标注 | [指南](../../02-guides/multi-ai-construction-guide.md) §口径纪律 |
| 4 | "AI 先用、人后用"第一律 | 宪章 §〇 三步裁定 + 吸收提问法 | 删掉人类界面后能力对 agent 仍完整可用 | [指南](../../02-guides/multi-ai-construction-guide.md) §进场第一课 |
| 5 | "用户觉得重复=分层没说清" | 链接不重抄 + 单一真源 + 文档头读法注 | 先查分层声明再动内容 | [指南](../../02-guides/multi-ai-construction-guide.md) §去哪读更深 |
| 6 | 随记 60 项→真金 | 吸收候选池 + 台账 + 产品化回环 | 五步：分诊→假说→第一律→回写→中性收口 | [指南](../../02-guides/multi-ai-construction-guide.md) §接棒遗嘱模板 |
