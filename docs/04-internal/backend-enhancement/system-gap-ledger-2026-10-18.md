# 系统级能力缺口台账（2026-10-18 立）

> `[Document-Meta]`
> Document:        docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md
> Status:          🟢 活跃（**缺陷/建议的唯一业务台账**，与 `00-backlog.md` 的批次账互补）
> 来源:            `docs/04-internal/ai-os-landscape-2026-10-18.md`（全景对照）+
>                  `artifacts/ai-os-market-scan-2026-10-18.md`（商业 15 样本）+
>                  `artifacts/agent-os-architecture-scan-2026-10-18.md`（学术 8 样本）+
>                  阶段 1/2/3 卷宗（`docs/archive/stage*/`）
> 用途:            **防止「调研结论聊完就忘」**——任何人接手，只看本文件就能知道：
>                  我们缺什么、为什么缺、先补哪个、补到什么程度算完、哪些明确不做。

---

## 零、使用规程（先读这 6 条）

1. **状态机**：`🟡 待拍板` → `🟢 已拍板（排期中）` → `🔵 进行中` → `✅ 已交付（提交号）`；
   不做的条目写 `⛔ 不做（日期 + 一句话理由）`，**保留在表里不删除**。
2. **拍板权**：只有 owner 能改「🟡 待拍板」的条目状态；Agent 可以加注调研/成本/验收判据，
   不得自行标「已拍板」。
3. **交付即销账**：每交付一条，补 `capabilities-matrix` 行 + 测试证据 + 提交号，并把状态改 ✅；
   **不许只写「做完了」不留证据**（0 装纪律正文版）。
4. **复核节奏**：每波总装时扫一遍本台账（新增/变更/延期各一行）；每季度全量复核一次排序。
5. **与其它账本的分工**：本台账 = 系统级能力缺口（长期、跨批）；`00-backlog.md` = 批次级待办与
   残余（D 编号）；`ROADMAP.md` §4 = 顶层优先级与安全缺口（G1–G10）；三者互链不互抄。
6. **数字口径**：成本为「1 名熟悉本仓的 Rust 工程师」估算（S < 2 周 / M 4–8 周 / L 8–16 周 / XL > 16 周），
   不是承诺工期；验收判据必须是可跑的命令/可观察的行为。

---

## 一、系统级能力缺口（SG 条目，按四轨编组）

> 依据：两家调研的交叉结论。**T1 内核健壮性 / T2 系统接口面 / T3 生态与多户 / T4 产品可见面**。
> 每条都带「现状证据 → 目标态 → 最小验收判据」，拍板后可直接转任务卡。

### T1 内核健壮性（长期伙伴的底座）

| ID | 缺口 | 现状证据 | 目标态 | 最小验收判据 | 成本 | 依赖 | 状态 |
|---|---|---|---|---|---|---|---|
| **SG-01** | **可取消的监督式 worker + 进程树** | 调度是**协作式抢占**（`preempt_active()` 只把任务重排队并标记 `is_preempted`，不中止执行中的代码）；`supervisor.rs` 仅 trait 骨架；无进程树快照/退出审计 | 任务可被真正取消/超时杀灭；worker 崩溃按重启策略恢复；副作用不重复 | ①P1 插话后 3s 内后台 dreaming/shell 任务被取消；②kill -9 一个 worker 后按策略重启且不重复副作用；③进程树快照可查 | **M** | 无 | 🟡 待拍板 |
| **SG-02** | **上下文语义分页 + 可恢复检查点** | 上下文有压缩/折叠/检查点文件（`context_fold/`、`compaction_checkpoint/`），但**无页表/换页语义**、无「中断后从检查点恢复」的端到端契约；来源调研见学术线（上下文当 RAM 的共识） | 上下文是「可寻址资源」：按主题/角色分页、可换入换出、检查点可恢复 | ①跨 3 次压缩后关键事实召回率不降（固定用例集）；②进程重启后从检查点恢复不重复副作用；③换页事件可审计 | **M**（6–10 周） | 可先做只读页表（S）再动写路径 | 🟡 待拍板 |

### T2 系统接口面（市场打穿风险最大）

| ID | 缺口 | 现状证据 | 目标态 | 最小验收判据 | 成本 | 依赖 | 状态 |
|---|---|---|---|---|---|---|---|
| **SG-03** | **GUI/UI 控制图（只读）**：Windows UIA / 无障碍树 / 窗口控件定位 | 感知面有截图（`perception::capture`）与窗口显著性，但**无控件树/无障碍驱动**，工具面停在 API 级 | 只读控制图：拿到窗口→控件树→稳定定位（含去重/回退），不执行动作 | 2–3 个常用 Windows 应用上输出控件树快照，与人工标注一致率 ≥ 90%；只读不写会话 | **L**（8–14 周，拆两步） | 截图为兜底；无 | 🟡 待拍板 |
| **SG-04** | **GUI 动作层 + 回放**：点击/输入/滚动/菜单，人批 + 审计 + 回放 | 无 | 动作经**同一套治理门**（观测门禁 + 人批 + 审计），可回放验证 | ①3 个应用上完成「读控件→生成动作→人批→执行→验证」闭环；②每步可回放（trace）；③越权/未批动作被拒并留痕 | **L**（依赖 SG-03） | SG-03 | 🟡 待拍板 |

### T3 生态与多户

| ID | 缺口 | 现状证据 | 目标态 | 最小验收判据 | 成本 | 依赖 | 状态 |
|---|---|---|---|---|---|---|---|
| **SG-05** | **插件隔离 + 签名**：插件/MCP server 供应链安全 | plugin 静态进程内（`apeireth-plugin` 无沙箱）；`vm_sandbox` 因缺 `libkrun` blocked；MCP server 外部进程治理未做 | 未签名/越权插件被拒；插件只调声明能力；插件崩溃不拖垮主进程 | ①未签名插件安装被拒且留审计；②插件调用未声明能力被拒；③插件 crash 后主进程存活 | **M–L**（6–12 周） | 与 SG-06 同批 | 🟡 待拍板 |
| **SG-06** | **技能包格式 + 注册/发现/分发** | 有技能沉淀（`skill_distill`/`skill_search`）与 MCP；**无技能包格式、无注册表、无签名分发** | 最小技能包：manifest + 工具声明 + 权限声明 + 签名；本地安装/校验/卸载 | ①技能包安装→签名校验→权限声明→运行 三条 e2e；②未签名/越权包拒绝；③卸载干净 | **M** | SG-05 | 🟡 待拍板 |
| **SG-07** | **记忆对象级共享语义（grant）** | 星群有每户隔离 + owner/member/guest 角色门；记忆有 `MemoryScope`，但**无「某条记忆共享给某成员/只读」的 grant** | grant = subject/object/access/expiry/issuer，与治理/审计同源 | ①户主把一条记忆只读共享给 member；②member 可读不可写；③撤销立即失效；④全程审计 | **M**（4–6 周） | 无 | 🟡 待拍板 |

### T4 产品可见面（用户立刻能感知）

| ID | 缺口 | 现状证据 | 目标态 | 最小验收判据 | 成本 | 依赖 | 状态 |
|---|---|---|---|---|---|---|---|
| **SG-08** | **记忆用户可控面板**（查看/纠正/删除/保护/导出） | 记忆后端完备（双时态/检索/遗忘/备份/导出），**UI 无记忆面板**（现有面板只有会话/设置类） | companion-desktop 记忆面板：时间线 + 检索 + 删改 + 保护 + 导出 | 查看/删/改/保护/导出 5 条 e2e，走既有 backend API，不新建旁路 | **S–M**（2–6 周） | 无（可立刻做） | 🟡 待拍板 |
| **SG-09** | **端侧推理接入**（本地小模型/OCR/嵌入/VAD） | provider 面是远端 API 归一化；无本地推理 provider 类型 | 「本地 provider」类型 + 生命周期管理；先接 1 小模型 + 1 OCR | 断网可用：OCR + 嵌入本地跑通；显式标注模型来源/许可 | **M** | SG-01 取消能力可先做 | 🟡 待拍板 |
| **SG-10** | **跨设备会话/记忆同步** | 本地优先，星群有域面隔离可复用；无同步协议 | 域内增量同步（先单向导出/导入 + 冲突策略），不承诺实时协作 | 双机同域同步一条 e2e；冲突策略可解释/可回滚 | **M–L** | SG-07 | 🟡 待拍板 |
| **SG-11** | **对外 SDK / OTA 灰度回滚 / 会话回放评估器** | 发布有签名+脚本+版本轴；无 OTA；SDK crate 未对外发布；trace 有但无回放 | SDK v0 可编译示例；更新可灰度可回滚；会话可回放给评估器 | ①SDK 示例工程编译运行；②更新失败回滚一条 e2e；③会话回放产出评估报告 | **M**（各） | 可拆三个小批 | 🟡 待拍板 |
| **SG-12** | **模型路由策略层**（按任务/隐私/成本自动选路） | 有 provider 抽象 + 成本路由（`cost_routing`），无「任务类型×隐私×成本」策略表 | 策略层：任务画像→候选模型→隐私/成本约束→选择解释 | 给定 10 个任务样本，选路结果可解释且成本下降 ≥ 20%（基线同批记录） | **M** | 无 | 🟡 待拍板 |

### 生物机制族（SG-13…SG-26，2026-10-19 owner 拍板「全都要」→ 🟢 已拍板排期）

> 来源：[06-biosphere-inspiration-2026-10-18.md](../../philosophy-study/06-biosphere-inspiration-2026-10-18.md)（14 条，全部采纳）。
> **纪律**：生物是灵感源不是保真模拟；每条落地必须先在代码里有可测试结构，再谈接线（0 装分级如实标注）。
> 波次：**W1（2026-10-19 2h 并行批）= 机制内核 v0**（纯模块 + 类型 + 测试，不做跨 crate 接线）；
> **W2** = 其余 5 条 + 全部接线（self_status 节/治理链/care 调度/矩阵行）。

| ID | 机制 | 生物原型 | 落点 crate / 模块 | 最小验收判据（W1） | 成本 | 波次 |
|---|---|---|---|---|---|---|
| SG-13 | 神经调质慢变量 | 多巴胺(奖赏预测误差)/血清素(基线)/催产素(联结)/皮质醇(压力)/去甲肾上腺素(唤醒)/乙酰胆碱(学习增益) | peireth-organ::neuromodulation | 6 维状态带惯性与耦合、事件→调质→行为参数（语气/耐心/主动性）映射有界可解释、确定性测试 | M | **W1** |
| SG-14 | 异稳态 / 生命体征 | 预测性调节（越限前动作） | peireth-organ::homeostasis | 状态向量+设定值+调节动作（整编/节流/恢复），阈值边界与不动作态测试 | M | **W1** |
| SG-23 | 节律 + 睡眠压力 | 昼夜节律/超日节律/腺苷累积 | peireth-organ::circadian | 相位模型（注入时钟）+ 睡眠压力累积/清账 + 超日节律窗口；确定性 | M | **W1** |
| SG-15 | 免疫记忆 + 炎症收口 | 先天/适应免疫、免疫记忆、耐受、炎症可收口 | peireth-guard::immune_memory | 事件签名归一→命中复用；分级响应+resolution+冷却；self/non-self 容忍；store trait 可注入 | M | **W1** |
| SG-16 | 痛觉学习 | 痛觉=保护性学习信号；习惯化；慢性化 | peireth-orchestration::nociception | 失败分类（瞬时/系统性/外部）+ 习惯化 + 慢性检测（窗口内重复→升级）+ 修复后脱敏 | S-M | **W1** |
| SG-17 | 再生四期 | 止血→炎症→增生→重塑，留瘢痕 | peireth-orchestration::recovery_protocol | 四期状态机 + 每期动作约束 + 瘢痕（known-bad）记录；非法迁移拒绝 | M | **W1** |
| SG-19 | 依恋 / 分离-重聚 | 安全基地、分离-重聚脚本 | peireth-organ::attachment | 分离时长分类（小时/天/周）+ 重聚策略（不内疚、续线索）+ 线索选点规则；纯函数 | S-M | **W1** |
| SG-20 | 社会认知 / 交互修复 | 心智理论、共同注意、修复动作 | peireth-organ::social_repair | 不确定性/矛盾/歧义触发→修复动作（澄清/复述/给回退）；动作库与优先级 | S-M | **W1** |
| SG-24 | 群体感应升级门 | quorum sensing（阈值才集体行动） | peireth-orchestration::quorum | 信号累积/衰减/阈值/冷却→是否升级审议；确定性 | S-M | **W1** |
| SG-18 | 凋亡 / 神经发生 | 程序性死亡 + 终身神经发生 | 待定（工具/技能/记忆生命周期） | 生命周期四态 + 未用 prune + 墓碑可恢复 | M | W2 |
| SG-21 | 游戏 / 幼态延续 | play = 低风险练习；neoteny | 待定（练习场） | play session 生命周期 + 预算隔离 + 不回写主体记忆 | M | W2 |
| SG-22 | 仪式 | 低强度高频率关系维持 | 待定（care/仪式层） | 仪式模型（早/晚/周）+ 可静默、不可打卡 | S-M | W2 |
| SG-25 | 共生菌群 | 多样性=健康，dysbiosis | 待定（工具生态体检） | 工具清单→多样性/冗余/重叠指标 + 体检报告 | M | W2 |
| SG-26 | 承载力 / 演替 / 关键种 | carrying capacity / succession / keystone | 待定（容量与迁移治理） | 承载力上限告警 + 演替期灰度迁移 + 关键模块清单 | M | W2 |

> **W1 全部为「机制内核 v0」**：只交付模块 + 类型 + 单元测试 + 模块文档（生物原型/工程边界/未接线声明），
> 不改运行时装配、不改 self_status 合同、不碰 LOCKED；接线与矩阵行在 W2 统一收口。
>
> **W2 交付记录（2026-10-19，五线并行）**：剩余 5 条内核 v0 全部入库（**+91 新单测**：
> orchestration +40（lifecycle 21 + capacity 19）/ organ +33（play 18 + ritual 15）/ tools +18），
> scoped 门禁全绿（organ 320 / orchestration 437 / tools 585 passed；clippy 0；fmt 0 diff）。
> 提交：SG-18 0e3eb71｜SG-21 69038cde｜SG-22 6d0361e｜SG-25 283f37e｜SG-26 67dd06a。
> **⇒ 14/14 内核全部入库（W1+W2 合计 +228 新单测）。**
>
> **Wave 4 观测补格交付记录（2026-10-21）**：`bio_mechanisms` 观测面 9/14 → **12/14**（默认关零注册）。提交
> `73107ff0`（`runtime-assembly::canonical::bio_extra_probes`：调质 / 昼夜 / 仪式三 probe + 三个 `BioProbeSlots`
> 迟绑定槽 + 装配根 fill）与环境债批 `18691532`（D25/D26/D27 测试去抖；D18 取证 = `962f30a8` 已落地、0 改动）。
> **口径**：调质/昼夜 = 真实内存状态（`wired`）；仪式 = 本地状态文件只读快照（`wired`，无跨进程事件，写者仍是
> W3.7 工具面）；睡眠压力 = 父格 `wired` 下的 `not_wired` 子字段（原因：累加器由 CLI care 心跳持有，装配根无读取面）；
> 开关关 = 槽不武装 / 零 fill / 单格 `null` + 未接线真因。**门禁**：assembly 全目标 0 failed（lib 300；新集成
> `bio_observation_cells` 3）、core/tools lib 199/595 全绿、clippy/fmt 绿；`arch_guard --strict` 本批 0 违规（fast-gate
> 唯一失败项为他在途 cli lib 尺寸）。**未做（如实）**：观测面缺 homeostasis 行为接线（需注入真实 context budget）与
> social_repair 事件源；默认关；无 live E2E。

> **R1b-2c 交付记录（2026-10-21）**：审批社交信号经神经总线发布（默认关零行为变化）。提交：`f2dad300`。
> 新增总线信号类别 `NervousSignalKind::ApprovalSocial`（三态 `Approved`/`Rejected`/`Cancelled`；**取消不编事件**；拒绝理由文本不进信号，防 PII 进总线）；发布方 = `runtime-assembly::canonical::bio_approval_social::BioApprovalSocialSource`（`production.rs` 按调质开关交付句柄，开关关 = `None` = 零发布）；消费方 = 调质订阅者（`Approved`→`SocialPositive`、`Rejected`→`SocialNegative`，同一事件只推进一次）；CLI 审批在 `resolve` 成功后发布；删除 `bio_tone` 直写助手（发布 = 唯一真源）。
> **门禁**：assembly 62 套件 0 failed（lib 291）；CLI lib 56 + 集成套件全绿（跳过一条他线在途 R1b-3 测试，随后由他线 `0b71cd6f` 提交并复跑绿）；clippy `-D warnings` 0 / fmt 0 / `arch_guard --strict` VIOLATIONS 0；`verify_changed --arch-strict --no-test` FAST GATE PASSED；cli lib 3886→3878 守住既有 3879 棘轮。**未做**：无 live E2E；审批路径外无生产发布方；四档仍只有显式注入口。
> **R1b-3 状态**：由架构线 `0b71cd6f` 收口（ritual 文件通道端到端 + 三源如实 `NotWired`），与交接单 §R1b-3 结论一致。

> **R1b-2a 交付记录（2026-10-21）**：调质事件面收敛到神经总线订阅（默认关零行为变化）。提交：`404919f6`。
> `bio_tone` 的生产事件源由「模块 `on_hook` 旁路扫 `AfterToolResult`/`OnError`」改为
> 「`nervous.bridge` → `NervousBus` → `BioToneBusSubscriber` 订阅」；模块 `on_hook` 只保留 `TurnStart`
> 语气装配；订阅者与模块共享同一 `Arc<ToneState>` 与同一首见集合（Novelty 语义不变）；开关关 = 不构造
> 订阅者 / 总线零订阅零发布 / 语气与叠加逐字节零变化。事件仍只推进一次（无 on_hook + 订阅双写）。
> **验收（R1b-2 handoff 口径）**：既有 tone 测试全绿 + 新「总线订阅与旧 hook 观测路径终态等价」测试；
> 开关开时 `NervousBus::published_count(ToolOutcome/ErrorObserved)` 生效；默认关总线零订阅。
> 测试：assembly 全目标 **62 套件 / 676 passed / 0 failed**（lib 281）；bio_tone 单测 16、
> `bio_tone_feedback` 10、`bio_tone_wiring` 8；clippy 0、fmt 0 diff、`arch_guard --strict` VIOLATIONS 0、
> `docs_truth --verify --check` OK。**未做（如实）**：R1b-2b（lifecycle 订阅 ToolOutcome + Lifecycle 探针
> 9/14；已由 `a71639f9` 补齐）、R1b-2c（`ApprovalSocial` 信号生产+消费同批）、R1b-3（ritual E2E + 3 源 NotWired 定性）；
> 权威全量仍未出数（§1 未替换）。

> **R1b-2b 交付记录（2026-10-21）**：生命周期工具结果面收敛到神经总线订阅 + Lifecycle 观测格接通（默认关零行为变化）。提交：`a71639f9`。
> `bio_lifecycle` 的 `AfterToolResult` 记账由「模块 `on_hook` 扫描」改为「`nervous.bridge` → `NervousBus` → `BioLifecycleBusSubscriber` 订阅」；订阅者与模块共享同一 `Arc<BioLifecycleState>`；`ToolOutcome` 写账（结果名→调用名回退），`ErrorObserved` 明确不写账；模块 `on_hook` 只保留 `AfterTurn` 评估。桥新增 `tool_call` 入参承接名字解析（结果名优先、空白/缺失回退调用名；tone/lifecycle 共用），无新信号变体、无 schema 变更。
> **观测格**：`BioProbeSlots` 增生命周期迟绑定槽（第 6 类），开关开时注册 `BioMechanismKind::Lifecycle` probe；`bio_mechanisms` 观测面 **8/14 → 9/14**（默认关零注册/零订阅/零 fill；槽空 = 单格 null + 真因）。自述 `nervous_system` 机制表同步：调质 5/9、lifecycle 总线订阅路径、仪式端到端归 R1b-3、观测面 9/14。
> **测试**：assembly 全目标绿（lib **284** passed；集成套件 0 failed；`bio_lifecycle_wiring` **11** passed，4 条事件链测试改由桥→总线驱动）；`nervous_snapshot` 28；clippy 0、fmt 0 diff、`arch_guard --strict` VIOLATIONS 0。
> **fast-gate 未通过如实登记（非本批代码）**：三跑均未出全链绿——①/② nextest 构建阶段 `link.exe` exit 1102 / 页面文件不足（D28）；③ check 阶段被在途未提交 governance 改动阻断（`crates/foundation/governance/src/chain.rs:372` E0277 `dyn ChainClock` 无 `Debug`）。scoped 门禁在阻断前已绿。**未做（如实）**：R1b-2c（`ApprovalSocial` 同批）、R1b-3（ritual E2E + 3 源定性）；权威全量仍未出数（§1 未替换）。

> **W3.6 交付记录（2026-10-20，两批）**：观测面与持久化收尾（**+31 新测**）。
> 提交：probe 迟绑定生产接入 `f7a436a9`｜生命周期账本持久化 `8c4fa5c2`。
>
> **观测面**：`capabilities.bio_mechanisms` 已接 **7/14 格**（容量 + 练习场 + 迟绑定 5：免疫账/
> 心跳状态/审议门账/痛觉/再生）；槽空 = 单格 null + 真因；`capabilities.nervous_system.observed`
> 按渲染帧自动计数（7/14）。pain/再生读取口只在 chat/approve 与常驻网关两条 `Arc` 化路径绑定
> （session/域伙伴运行时如实未接线）。
>
> **持久化**：生命周期账本 `lifecycle_state.json`（StoredDoc v1、原子写、坏档拒开不覆盖、启动重放、
> 回合收口写一次；关/目录缺省=纯内存）；单写者、无 WAL/schema 迁移（如实）。
>
> **W3.7 交付记录（2026-10-20，三批）**：tone 事件源两档补真实源 + SG-22 仪式
> 完成事件源落地 + SG-25 生态位观测面 v0（scoped **+~30 新测**）。
> 提交：`53e7aa09`（Novelty 首见 + 人批/人拒社会结论 + 审批分派拆模块）｜`35c4b24a`（架构守卫切片终点
> 跟随拆分修正）｜`7e98744d`（仪式完成/跳过工具面，默认关，装配级注册）｜`9133c7bb`（SG-25 生态位观测面 v0：策展表 + 生态格迟绑定，观测面 8/14）。
>
> **tone 事件源 5/9 已接**：TaskSuccess/TaskFailure（W3.5 工具结局 + 运行时错误）、Novelty（工具名
> 进程内首见，`AfterToolResult`，不读文本不推断）、SocialPositive/SocialNegative（CLI 人批/人拒，
> 仅 `resolve` 成功后注入，不改审批结论、不参与权限判定）；**其余 4 档**
> （RewardPrediction/Threat/Quiet/UserCorrection）仍仅显式注入口，如实。首见集进程内去重、无
> TTL/无淘汰（重启归零）；无 live E2E；网关审批面板未接（如实）。
>
> **SG-22 仪式完成事件源已接（默认关）**：`tool.ritual_done` / `tool.ritual_skip` 两模型面工具注册于
> production 装配（复用 `APEIRETH_ENABLE_BIO_CARE_CANDIDATES` 总闸 + care 数据目录；只认精确 `"1"`，
> 缺目录 = 拒绝注册），调用即写 `<care_dir>/ritual_state.json`（StoredDoc v1 原子写、坏档 fail-closed）。
> **事件语义 = 模型在用户明确确认后调用，不做自动推定、不打卡**（SG-22 底线保持）；消费侧 = care 心跳
> 宿主 `ritual_inputs` 读同一文件；纯 CLI chat 无心跳 = 写入暂无候选消费（如实）；单写者设计，显式
> `record_ritual_done/skip` 注入口保留（与工具面并用无跨实例锁 = 具名缺口）。
>
> **SG-25 生态位观测面 v0 已交付（默认关，观测面）**：`tool_taxonomy` 40 条工具 id/名 → 13 类标签策展表（代码内常量快照真源；未覆盖工具如实计 untagged，覆盖数上报，不从描述推断）+ `tool_ecosystem_probe` 经 `BioProbeSlots` 迟绑定接入既有 `APEIRETH_ENABLE_BIO_SELF_STATUS`（不新增 env）；观测面 **7/14 → 8/14**，槽空/关 = 单格 null + 真因。**W3.5 盘点里的 IMPLEMENTED 1 条（SG-25）据此升为 PRODUCTION WIRED（默认关）**；完整 taxonomy 合同（工具声明携带标签）仍 B 类 ADR 前置，未做。
>
> **W3.7 剩余**：①SG-25 v0 观测面已交付（完整 taxonomy 合同/工具声明携带标签仍 B 类 ADR 前置）；②R1b 订阅迁移（进行中：R1b-2a `404919f6` 调质 + R1b-2b `a71639f9` 生命周期已收敛到总线，`nervous_system.observed` 8/14 → 9/14；余 R1b-2c + R1b-3）
> ③发布收口 + 全量基线统一复跑（当前 §1 口径受其它线
> R4b/R7 归档在途影响，本批只追加证据注记、不替换 §1）。
>
> **W3.5 交付记录（2026-10-20，四批并行 + 总装）**：剩余接线收口（**+~100 新测**）。
> 提交：生命周期接线 `ae594c6c`｜仪式存储 `5068ad34`｜装配瘦身+生命周期注册+tone 事件源 `2ceee2ef`｜
> 观测扩展+ADR-0003 `77b1c1fb`｜自述 probe 生产接入 `151e9aea`。
>
> **接线盘点（14 条，更新 W3.4 记录）**：**PRODUCTION WIRED（默认关）13 条**（新增 SG-18 生命周期、
> SG-22 仪式状态存储）；**IMPLEMENTED 1 条** = SG-25 生态健康（**无真实信号源**：运行时无工具能力标签，
> 拼标签=伪接线；需 taxonomy，B 类 ADR 前置）。**观测面 2/14 格已接**（容量 + 练习场；6 条 probe 实现
> 就绪，免疫/心跳/审议/痛觉句柄需迟绑定，下一批接）。
>
> **ADR-0003（R1a 神经系统只读聚合）已交付**：`capabilities.nervous_system` 只读子对象（schema v1：
> version/summary/14 条 mechanism 的 level/enabled_env/signal_sources/gaps，last_observation 恒 null）；
> **顶层 21 节冻结合同不变**（ADR「21→22 节」措辞按子对象口径落地）；`arch_guard --strict` 0 违规
> （STALE 2 = 可缩表基线，待效率线 `--update-baseline`）。
>
> **W3.6 / 后续**：①6 条 probe 的迟绑定生产接入（免疫/心跳/审议/痛觉/再生）；②仪式完成事件源、
> 生命周期持久化、tone 其余 7 档事件源；③SG-25 工具能力 taxonomy（B 类 ADR）；④`nervous_system.observed`
> 作为 R1b 订阅迁移验收指标（当前 2/14）；⑤两小时总装复验 + 发布收口。
>
> **W3.4 交付记录（2026-10-20，两批 + 矩阵收口）**

> **W3.4 交付记录（2026-10-20，两批 + 矩阵收口）**：收口接线入库（**新增 ~130 新测**：
> recovery 11 模块 + 2 集成 / 节律门生产 8 装配 + 5 cli）。提交：再生协议接失败链 `007aff0f`｜
> 节律门生产装配 `2b0a7844`。
>
> **接线盘点（14 条，更新 W3.3 记录）**：**PRODUCTION WIRED（默认关）11 条** = SG-13 调质→tone｜
> SG-14 稳态→策略｜SG-15 免疫→hook｜SG-16 痛觉→失败链｜SG-17 再生→失败链（recovery 状态机同开关）｜
> SG-19 依恋→care｜SG-20 修复→prompt｜SG-21 play｜SG-23 节律→主动消息闸（心跳最前，显式时区配置）｜
> SG-24 审议门｜SG-26 容量→自述；**半程 1 条** = SG-22 仪式→care（**仪式状态存储未接 ⇒ 0 候选**）；
> **未接线 2 条** = SG-18 生命周期｜SG-25 生态健康（内核已入库）。**14/14 内核 + 11 条生产装配（全部
> 默认关）**，剩余半程/未接均有明确缺口登记（矩阵 §1.1 生物机制 14 行 + §0.2 开关行）。
>
> **W3.3 交付记录（2026-10-20，六批并行 + 装配根总装）**

> **W3.3 交付记录（2026-10-20，六批并行 + 装配根总装）**：收尾接线入库（**+~100 新测**：
> care 10 / play 25 / pain 21 / quorum 免疫源 12 / tone 16 单元 + 8 装配 / circadian 9）。
> 提交：care 数据供给 `6c65e46a`｜play 端到端 `83484a80`｜痛觉失败链 `178c46de`｜
> quorum 免疫源 `e9e95730` + cli 挂载 `d1cb750c`｜tone+节律门 `74a5850d`｜
> 装配根总装（Lead）= production 注册 `BioToneModule`（懒加载）+ `canonical.rs` re-export +
> cli 痛觉开关（`APEIRETH_ENABLE_BIO_PAIN`）。
>
> **接线盘点（14 条，更新 W3.2 记录）**：
> - **PRODUCTION WIRED（默认关）9 条**：SG-13 调质→tone（模块已注册；真实反馈事件源未接=边界）｜
>   SG-14 稳态→上下文策略｜SG-15 免疫→guard hook（cli 挂载）｜SG-16 痛觉→失败链（只观测记账）｜
>   SG-19 依恋→care（真实 presence/会话数据供给已接）｜SG-20 修复→prompt｜SG-21 play（四工具面）｜
>   SG-24 审议门（含 guard 免疫外部 Conflict 源）｜SG-26 容量→自述；
> - **半程 2 条**：SG-22 仪式→care（管道已接，**仪式状态存储未接 ⇒ 0 候选**）｜
>   SG-23 节律门（内核 + 心跳闸已实现，**生产装配缺时区/睡眠压力配置源**）；
> - **未接线 3 条**：SG-17 再生（`recovery_protocol` 内核已入库未接）｜SG-18 生命周期｜
>   SG-25 生态健康。
> - **W3.4** = 14 条能力矩阵行 + self_status 机制格补接线 + 生产装配缺口（节律门配置源、仪式存储、
>   tone 事件源、recovery 联动）+ 两小时总装复验 + 发布灰度。
>
> **W3.2 交付记录（2026-10-19，五线并行 + 装配根总装）**

> **W3.2 交付记录（2026-10-19，五线并行 + 装配根总装）**：装配面接线入库（**+68 新测**：
> 新装配测试文件 4 个共 37 + cli 7 + tools 7 + 单元 17）。提交：cli 免疫挂载 3f255505｜
> care 候选 9a11ea6｜修复/上下文策略 627b002d｜审议门 8c506173｜bio_mechanisms 85984bdb｜
> **装配根总装（Lead）** = production.rs 注册 BioAssemblyModule + 自述源挂 io_mechanisms
> + 承载力 probe 读同一任务队列 + 矩阵 §0.2 开关行 + 名册布尔断言豁免。
>
> **接线盘点（14 条）**：
> - **PRODUCTION WIRED（默认关）5 条**：SG-14 稳态→上下文策略｜SG-15 免疫→guard hook（cli 挂载）｜
>   SG-20 修复→prompt 组装｜SG-24 quorum→审议门｜SG-26 容量→自述 bio_mechanisms；
> - **半程 4 条**：SG-13 调质→tone（映射已备，装配未持 NeuromodulatorState）｜
>   SG-19 依恋 + SG-22 仪式→care（管道已接，**生产数据供给 presence/memory/仪式状态未接 ⇒ 0 候选**）｜
>   SG-23 节律门（装配未接心跳主动消息闸）；
> - **未接线 5 条**：SG-16/17 痛觉/再生、SG-18 生命周期、SG-21 练习场、SG-25 生态健康（内核已入库）。
> - **W3.3** = 数据供给 + 事件源（会话纠正/外部异常）+ 失败链/play 端到端 + 剩余装配。
>
> **W3.1 交付记录（2026-10-19，六线并行，crate 内接线）**：六条「内接线」API 全部入库（**+77 新单测**：
> guard +10 / organ +57（care 24 + context/circadian 14 + tone 10 + prompt 9）/ orchestration +10），
> scoped 门禁全绿（organ 377 / guard 61 / orchestration 447 passed；clippy 0；fmt 0 diff）。
> 提交：SG-15→hook 3703055｜SG-20→prompt 53ba705b｜SG-24→council 2a0858b9｜
> SG-19+22→care 7559ca01｜SG-14+23→context 8a0725d1｜SG-13→tone 3f86bfd6。
> **接线进度 = 6/14 的 crate 内接线（全部默认关、可回退）；PRODUCTION WIRED 待 W3.2 装配面**（注入点已由各批列明：
> cli hook 挂载 / runtime-assembly context-cognitive-orchestrator 三处 / production.rs 开关表）。
>
### W3 接线计划（2026-10-19 定，14/14 内核入库后）

> 目标：把 14 条内核从「库级 IMPLEMENTED」接到「PRODUCTION WIRED / DEFAULT ENABLED（默认关）」，每接一条：
> ①接线代码 + 装配级测试；②capabilities-matrix 行（四级口径）；③self_status 观测（方式见下）；④台账状态改 🔵→✅。
> **self_status 合同策略（先定，避免接线批各自为政）**：不逐个加节（21 节合同已冻结，逐节扩会让合同碎片化），
> 统一走「**观测聚合**」：在既有 capabilities 节内新增只读子对象 io_mechanisms（每机制一栏：stage/值/阈值/最近动作），
> 合同节数不变、字段缺省 
ull + reason；若 owner 要独立节，另立「合同扩展批」评估。

| 批次 | 内容（SG） | 目标缝（代码） | 验收判据 |
|---|---|---|---|
| **W3.1 crate 内接线（6 批并行）** | 免疫→guard hook（SG-15）｜社交修复→prompt_assembly（SG-20）｜仪式→care reminder（SG-22）｜quorum→审议触发（SG-24）｜依恋→care 投递/会话恢复（SG-19）｜稳态+节律→care/dreaming 闸（SG-14/23） | peireth-guard::hook / peireth-organ::{prompt_assembly, care} / peireth-orchestration（council 触发） | 装配级测试（crate 内可跑）+ 行为开关默认关 + 回退路径 |
| **W3.2 跨 crate 接线（4 批并行）** | 生命周期→工具/技能/记忆清理（SG-18）｜承载力→上下文/记忆配额（SG-26）｜生态健康→self_status 只读（SG-25）｜神经调质→tone/人格缩放（SG-13） | 
untime-assembly + 各 face 注入点 | cargo test --workspace 零回归 + 自述新字段 + 默认关 |
| **W3.3 跨 crate 接线（2 批）** | 痛觉/再生→失败处理链 + harness_patch（SG-16/17）｜游戏→opt-in 卡 + 调度器沙箱（SG-21） | 失败处理链 / gateway 审批卡 / orchestration 调度 | 端到端一例（失败→诊断→修复→瘢痕；play opt-in→沙箱→产出） |
| **W3.4 合同/文档面** | 14 条矩阵行 + capabilities 节 io_mechanisms 子对象 + CHANGELOG + 台账收口 | matrix / self_status / CHANGELOG / ledger | 口径守门 0 新增违规、SSoT 数字同步 |

> 接线纪律：**默认关 + 灰度**；每条接线必须保留「关掉即回原行为」的回退路径；不碰 LOCKED；
> 接线批各自带装配级测试，总装批跑全量并刷新基线。

> **W1 交付记录（2026-10-19，七线并行，2h 窗口）**：9 条全部入库，**137 个新单测**
> （organ +79 / guard +18 / orchestration +40）；scoped 门禁全绿（organ 287 / guard 51 / orchestration 397 passed；
> clippy 0 警告；fmt 0 diff；中性/secret 扫描干净）。提交：SG-13 192cc327｜SG-14+23 1544e54｜
> SG-15 7021225d｜SG-16+17 5d4df2eb｜SG-19 0d136b4｜SG-20 3cd2779c｜SG-24 87172d90。
> 全部 IMPLEMENTED **未接线**（生产 0 调用点），W2 接 self_status 节/治理链/care 调度/矩阵行后转 ✅。

---

## 二、本轮其它现存缺口索引（细节在真源，不在此重抄）

| # | 缺口 | 真源（改状态请改真源并在此行同步） | 现值状态 |
|---|---|---|---|
| 1 | 安全缺口 G1–G10 | `ROADMAP.md` §4 安全缺口账 + `E-threat-model.md` §5 | G4/G10 ✅；其余 🟡/未销（以 ROADMAP 为准） |
| 2 | D20 三族诚实残余（**IM 桥整座无生产构造点（`ImBridge::new`/`im_router` 全仓仅 tests 引用，非仅触发器未接）** / live IM 未实测 / xlsx 写出**已交付 fb35148b** / DOCX **已有 numbering.xml 一级 bullet 00484bf7**，仍无主题、无字体美化 / 图核换档偏差登记） | `00-backlog.md` D20 | 🟡 部分收口（xlsx 写出、DOCX 编号外观已销；余 IM 链路与图核偏差） |
| 3 | D21 文档面债（口径漂移 26 处 / rc.1 时代规格无历史横幅 / 11 crate README 陈旧 / matrix **device_control 行已补 8b31abeb（§1.1 独立行仍缺）** / test-baseline 表头 / CHANGELOG broken link） | `00-backlog.md` D21 | 🟡 部分修（28 文件 deef3bed + matrix 8b31abeb），余待各归口线 |
| 4 | D11 客户端令牌管理界面（后端令牌表/门已好，UI 缺） | `00-backlog.md` D11 | 🟡 未开工 |
| 5 | D8 治理流程整机合练（智囊团审议 + 物理多签从未真跑通过一次；multi-sig 无密码学效力） | `00-backlog.md` D8 / G9 | 🟡 设计在，合练未做 |
| 6 | K 轴 L3/L4 IM 审批链的运行时触发器接线 | `00-backlog.md` D20① + `K-device-approval-im-push.md` | 🟡 显式提交口已有，生产桥未接 |
| 7 | M 轴 3D 在场（侦察两轮结论：候选仓库不建议搬；暂不立项） | `docs/design/m-axis-3d-recon-*.md` + 账本 | ⛔ 暂不立项（2026-10-18） |
| 8 | Q27 守门级数「7 重/9 重」两处 LOCKED 文本不一致（代码实证倾向 7 重 v7，改 LOCKED 行需 owner 明示解锁） | `docs/design/founder-ai-qa.md` Q27 | 🟡 待 owner 一字 |
| 9 | 三段式遗忘默认关闭；`acp.rs` 仅 schema；`p2p_mesh.rs` 明文非加密；`vm_sandbox` blocked | 各交付卷宗 / 学术线扫描 §五 已标注 | 🟡 已知未接线（0 装已标） |
| 10 | Docx/xlsx 写出（xlsx 写面后置，唯一表格写出口 = CSV） | D20② | 🟡 后置 |
| 11 | **依赖告警 10 条（Dependabot open，2026-10-18 实查；前端 4 条已清，余 6 条非产品面）**：1 high + 5 medium + 4 low。前端 `frontend/companion-desktop`：source-map-js **high**（fix 1.2.2）、katex low ×2（fix 0.18.2）、dompurify low ×2（fix 3.4.16）——**均已清（7c07566a）**；`legacy/archived/**` 5 份 Cargo.lock + `research/llm_judge/Cargo.lock`：rustls medium（fix 0.23.45） | GitHub Dependabot alerts API（open 状态，2026-10-18 核查） | 🟡 部分处置：①前端 4 条**已清**（7c07566a：dompurify 3.4.16 / katex 0.18.2 / source-map-js 1.2.2，`pnpm audit` 4→0，check/build/test 82/82 全绿）；②legacy/archived 与 research 锁文件属非产品面 → 由 owner 定口径（dependabot.yml 忽略 或 删除归档锁文件），不要让告警长期挂着污染视线 |

> 本索引只维护「指针 + 现值状态」；**改细节改真源**，避免双写漂移（改完回来同步这一行）。

---

## 三、明确不做（NT 条目，保留理由防复燃）

| ID | 不做的事 | 理由（本轮证据） |
|---|---|---|
| **NT-01** | 计费 / 账号运营 / 订阅 / 应用商店分成 | C 端订阅在 AI OS 赛道已被证伪（两家头部先后停服/取消订阅）；与「数据主权 + 不中心化」冲突；可作为外部商业层，永不进内核 |
| **NT-02** | 自研基础大模型训练 / parameter memory 微调固化 | 十年投入的军备竞赛，不在本地伙伴主路径；端侧只接推理（SG-09） |
| **NT-03** | 通用 syscall ABI + 第三方 OS 应用生态 | 会把现有工具异构 schema 压平，收益 < 复杂度；`CapabilityKind` + MCP + manifest 已够用 |
| **NT-04** | 远程 kernel / 多设备 Galaxy 式 DAG 编排 / 跨品牌硬件互联 | 多租户 OS 的解法；星群在数据面做多户隔离，跨设备更适合「连接器/同步」（SG-10） |
| **NT-05** | KV-cache / activation / logits 级内存层级 | 闭源 provider 拿不到；用结构化摘要 + 语义页 + 检查点（SG-02）达到 80% 效果即可 |
| **NT-06** | 对外文案自称「AI OS」 | 名词通胀 + 会把「伙伴」误读成工具；对内路线叫「系统级能力」，对外继续「数字伙伴 + 认知微内核」 |

---

## 四、形态不同（NF 条目：不是差距，禁止当技术债追）

| ID | 形态差异 | 说明 |
|---|---|---|
| **NF-01** | 多租户 OS vs 每户独立伙伴实例 | 我们做「一个伙伴一个域」，不做「一个 OS 服务多租户」；星群是**更强隔离**不是落后 |
| **NF-02** | 系统级 AI 层 vs 本地优先单机能力 | 默认数据不出户，不拿云端换能力；端侧推理（SG-09）是补强不是转向 |
| **NF-03** | 工具型助手 vs 数字伙伴 | 15 商业样本 + 8 学术样本无一把「伙伴」当产品形态——这是空位，不是差距 |
| **NF-04** | 单进程微内核 vs 进程树/服务树（v1 设计遗产） | v2 主动简化（单进程 + SQLite），工程取舍；若 SG-01 需要进程树，按需局部引入，不恢复 v1 拓扑 |

---

## 五、复核与销账规程（机器可查）

- **每波总装**：跑 `git log --oneline origin/main..HEAD` 核对本波是否动了本台账；新增缺口当场加行。
- **每次交付**：状态 → ✅ + 提交号 + 测试证据（套件名/命令/数字）+ `capabilities-matrix` 行号。
- **每季度**：全量复核一次「🟡 待拍板」排序（成本/依赖/打穿风险重估），复核结论写进本文件末尾
  「复核记录」小节（日期 + 结论 + 谁）。
- **禁写**：不许把未拍板项写成「已排期/将做」；不许删除 ⛔ 条目；不许在本台账复制
  `00-backlog`/`ROADMAP` 的细节正文（只放指针）。

---

## 六、证据与诚实边界

- **外部证据为宣称级**：两份 `artifacts/*` 报告均未安装/未运行/未逆向第三方产品；厂商与论文数字为自报；
- **样本缺口**：市场线缺 Flyme AIOS 等；学术线只读摘要与选定章节；GitHub 元数据受匿名限流影响；
- **我们侧「有」的证据**：以 crate/模块存在 + 既有文档为准，未逐项复跑测试；四级口径以
  `capabilities-matrix` 与 `production.rs` 实况为准；
- **成本估算非承诺**：S/M/L 为相对量级；
- 本台账只做**缺陷/建议登记与排序依据**，不替 owner 拍板，不改变任何能力默认开关。

---

## 复核记录

| 日期 | 结论 | 执行 |
|---|---|---|
| 2026-10-18 | 初版建立：四轨 12 条 SG + 10 条现存缺口索引 + 6 条 NT + 4 条 NF；全部 🟡 待拍板 | 总装主线 |
| 2026-10-18 | 小批总装复核：D23 前端 4 条销账（7c07566a）、D20 xlsx 写出（fb35148b）与 DOCX numbering（00484bf7）收口、D21 matrix device_control 行补齐（8b31abeb）；余 6 条 rustls 告警待 owner 口径 | 总装主线 |
| 2026-10-19 | 生物机制族 owner「全都要」进排期；W1 九条内核 v0 七线并行交付（137 新测，提交 192cc327…87172d90），全部 IMPLEMENTED 未接线，W2 接线 | 总装主线 |
