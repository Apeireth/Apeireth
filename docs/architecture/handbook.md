# 架构线续作手册（Handover / Continuity）

> `Status: ✅ 活跃（2026-10-20；v1.2，Wave 3 收口版）`
> 目的（O-4 任何人都能接手）：任何 AI 线或人类接手架构线时，只读这一页 + 四个入口就能继续，不需要重新读 2.7MB 卷宗。
> 证据基线：2026-10-20 实测（crates=30 / edges=111 / env=154 / max_src_loc=6150 / 基线条目见 `scripts/arch-baseline.txt`）。

## 1. 四个入口（按顺序）

1. [架构契约](contract.toml) —— 唯一可执行真源（结构规则/extension points/预算/例外参数）；
2. [ADR-0001](../adr/0001-adopt-architecture-contract.md) —— 规则体系为什么存在、例外预算怎么定；
3. [ADR-0002](../adr/0002-target-architecture-v1.md) —— 目标架构 v1（R0-R8 收敛 + 波次路线），**Accepted（2026-10-20 owner 拍板）**；
4. [三线协作契约](line-coordination.md) —— 文件所有权/提交纪律/波次日历。

运行工具：[arch_guard.py](../../scripts/arch_guard.py)；基线：[arch-baseline.txt](../../scripts/arch-baseline.txt)；准入：[intake-checklist.md](intake-checklist.md)。

## 2. 一分钟健康检查

```powershell
$env:PYTHONIOENCODING='utf-8'; [Console]::OutputEncoding=[System.Text.Encoding]::UTF8
python scripts/arch_guard.py --strict        # 期望 VIOLATIONS 0, exit 0
python scripts/docs_truth.py --verify        # 期望 fail=0 stale=0
git log -1 --format='%h %s'                  # 确认 HEAD 与协作日历一致
```

## 3. 当前状态（接手人先看这里）

| 项 | 值 |
|---|---|
| 规则体系 | **已生效**（ADR-0001，commit `8bfefb1b`）；灰度 report-only 至 **2026-11-03** |
| 三线协作契约 | **已生效**（commit `8e9dd3ae`） |
| 目标架构 v1 | **Accepted**（ADR-0002，2026-10-20 owner 拍板）；Wave 2 单项 ADR：0003 R1a / 0004 R3 / 0005 R4a；Wave 3：0007 R7 / 0008 R2 均 **Accepted 且交付**；Wave 4：0009 R1b / 0010 R8 / 0011 R6 均 **Accepted（2026-10-20，owner 授权 Wave 4 按建议推进）** |
| 架构波次 | `current_wave = 5`（Wave 4 于 2026-10-21 收口；Wave 5 = R5 取消可恢复 worker + 收口，待 owner 启动授权）；活跃例外 `expires_wave = 6` |
| Wave 4 已收口（2026-10-21） | **R1b** 全交付：1a `10ec68ae` 总线/桥/开关表、1b `6837d9b7` 开关收敛+台账校验、2a/2b/2c 机制订阅迁移与 `ApprovalSocial` 发布（生物线 `404919f6`/`326e1542`/`a71639f9`/`0f4ef4dc`/`f2dad300`/`815d85e7`）、3 `0b71cd6f` 三缺源如实定性、4a schema v2 additive；**R8**：A `fd74a669` / B1 `b6d902ce` / C `da7e224f` / D 部分 `60627d8e`（B2/B3 待安全线）；**R6**：A/B `5858fc1f`（gate_full 唯一执行器 + CI 守卫改读契约）、C/D 载体 `530611dc`/`391cc8ec`（台账/边界/夜间载体；夜间未运行、四层签核 0 条） |
| Wave 4 已落 | **R7-2.1**：`ContextAssembler` 单一管线（orchestration 唯一实现 + organ 策略桥，concept known_debt 清零，`c8cb34a4`）；**R1b-1a**：神经信号总线内核/唯一钩子桥/开关单入口（`10ec68ae`，默认关、生产暂无订阅者）；**R1b-1b**：生物开关读取收敛到装配根单入口 + 台账 4 行更正 + 3 条结构校验（`6837d9b7`，默认关零行为变化）；**R1b-4a**：`nervous_system` schema v2 additive——总线聚合计数观测面（subscribers/published/delivered，`last_observation` 恒 null；未绑总线 = null；桥/模块构造走装配根）；**R8-A**：`foundation/governance/src/chain.rs` 治理链骨架——`GovernancePhase` 七段 + `ChainOutcome`/`ChainRecord` + 四注入端口（`MultiSignVerifier` 占位 fail-closed / `AuditSink` / `ReflectionScheduler` / `ChainClock`），10 条链测试 |
| Wave 3 已落 | **R7**：`ContextBlock` 单一化（R7-2a `0bdd9bb4` / R7-2b `a93fdbb3`）、`SessionStore` 三义→`SessionLifecycleStore`/`SessionNoteStore`（R7-3 `1cbaee95`/`3c0bcde1`）、dormant scheduler 归档（R7-4b，含 bench 同步 `2481200b`）、`PromptCacheStabilizer` 删除（R7-4a `51bb6cfa`）、Heartbeat 复核保留 active（`b36ee05e`）；**R2**：Extension ABI 家族（R2-2 `56ae816b`）、PluginManager/BehaviorRegistry 双 catalog（R2-3a/b `911e2f49`/`b5f75ba9`）、plugin 内部索引 `CapabilityIndex`（`d8aa87af`）、命名迁移 BehaviorModule/BehaviorRegistry（R2-4 `efcce6c4`） |
| 待 owner 裁决 | **R4a-2 条件性 workspace 提取**（触发≈200k LOC，当前约 178k 行，未触发）；**R6-C/D 正式验收**（四层签核需真实人类，当前签核 0 条）；**Wave 5 启动授权**（R5） |
| 在途违规（本波） | 无未登记在途违规；`arch_guard --report` VIOLATIONS 0 / exceptions=2（`EXC-0002` cognitive.rs 4763、`EXC-0003` tool_modules.rs 1216，weight=1、expires_wave=6，NOTE 抑制）/ STALE **0**（本批缩表 2 条） |
| 效率线 R3a 现状 | `env-registry.toml` 155 条；R3b foundation 批验收（ADR-0004 §7.6）；R3c report-only 至 2026-11-03 后 review |
| 基线 | `scripts/arch-baseline.txt`（340 条；Wave 4 收口缩表 2 条 = `play_tools.rs` 1414→1412、`cli/lib.rs` 3879→3878；只减不增） |
| 已登记的已知债务 | `contract.toml [flagged_edges]` 3 条；`[extension_debt]` **0**（R2 收口清零）；`[concepts].known_debt` **0 组**（R7-2.1 已合并）；CI 注释残留旧口径（`24 LOCKED` 见 cosign/dependabot/o6-anchor 等非 active guard 注释，待清）；`exceptions.toml` 活跃例外 **2** 条（EXC-0002/EXC-0003，均 size 类 weight=1、expires_wave=6，清偿任务见例外条目） |
| 未完成项 | Wave 5+：R5 取消可恢复 worker（未开工，待 owner 启动）；R8-B2/B3（CLI 装配根切换 / 持久 `AuditSink` + IM 审计对，待安全线）；R8-D wiring 台账治理面已由效率线 E2 登记（ADR-0010 §6 已勾选；CI 演练步骤仅本地，远端未运行）；R6-C/D 正式验收（夜间 full **NOT MET**：owner 2026-10-21 裁决不推送、全本地；本地全量证据 341/8570；四层签核 0 条）；R3c 2026-11-03 后 review；R4a-2 条件性提取（owner 触发） |

## 4. 每波收口 SOP（架构线主持）

1. `python scripts/arch_guard.py --report` → 看违规/可缩表；
2. 对照记分牌（ADR-0002 §4）逐项记录"变好/持平/变差"；变差项必须有例外或 ADR；
3. 缩表：逐条人工只删/改小（`--update-baseline` 会按现状整体重建、可能带入增长项——仅在 diff 确认只减后使用，正常收口直接删失效行/改小尺寸）；CI 会校验 baseline diff 只减；
4. `contract.toml` `current_wave += 1` + 写 `wave_note`；
5. 出波次复盘一页（问题/决策/下一步），引用 ADR 编号。

## 5. 裁决位（不能代替 owner 拍板的）

- 目标架构后续单项 ADR（Wave 4+：R5；R1b 已 Accepted ADR-0009、R8 已 Accepted ADR-0010、R6 已 Accepted ADR-0011）；
- **14 键表述**已正式化（ADR-0006），后续变更仍须 owner 授权；
- R4a-2 条件性 workspace 提取的触发阈值（members 下降节奏由 owner 决定）；
- 产品形态（单进程永久 vs 多进程阶梯）——决定 R5 边界；
- 冻结窗口宣布（owner/release authority，架构线不单方面冻结）。

## 6. 已知陷阱（接手人别踩）

1. **同工作树并行线**：共享 git index → 只 `git add` 本线文件、提交前核对 staged、同一时刻只一条线 commit（见协作契约 §3）。
2. **全量门禁会抢构建锁**（>18 分钟）：架构批不跑全量；`arch_guard` 只读 metadata+文本，3-5 秒。
3. **`--update-baseline` 是后门**：只在波收口用，且 diff 必须只减；否则棘轮失效。
4. **不改 src**：架构线只碰契约/守卫/文档/CI 文件；R1/R2/R7 的实施归对应线，架构线出 spec + review。
5. **不碰 LOCKED 5 项**（9 锚本体 / 键 / 3 脊柱 / workspace.version / R11 baseline）；14 键表述修正需 owner 授权。
6. **报告只引用真源数字**：测试数字 = `test-baseline.md`，能力状态 = `claims.yaml`，结构 = `contract.toml`；不手抄。
7. **例外不要口头承诺**：任何"先这样，下波清理"都必须进 `exceptions.toml`（权重/owner/到期/移除计划），否则 strict 会红。

## 7. 文件地图

| 文件 | 用途 |
|---|---|
| `docs/architecture/contract.toml` | 结构契约（改它 = 结构决策） |
| `docs/architecture/README.md` | 体系说明与用法 |
| `docs/architecture/exceptions.toml` | 例外登记簿 |
| `docs/architecture/intake-checklist.md` | 功能准入八问 |
| `docs/architecture/integration-plan.md` | 三处接入与记分牌 |
| `docs/architecture/line-coordination.md` | 三线协作契约 |
| `docs/adr/0001-*.md` | 规则体系裁决 |
| `docs/adr/0002-*.md` | 目标架构 v1（**Accepted**） |
| `docs/adr/0007-*.md` / `docs/adr/0008-*.md` | R7 上下文/会话单一化、R2 扩展 ABI 统一（均 **Accepted 且交付**） |
| `scripts/arch_guard.py` / `scripts/arch-baseline.txt` | 守卫与棘轮基线 |
| `scripts/gate_full.ps1` | 全量门禁（主 workspace 排除 vendored 图像家族 + 图像独立门禁，清单读契约；R6-A：进程锁 + 失败分类） |
| `docs/02-guides/verification-architecture.md` | 三层门禁语义（fast / arch / full）、full 唯一执行器与失败分类、CI 对应表（R6-A） |
| `docs/architecture/r8-b-entry-convergence-design.md` | R8-B 入口收敛设计候选（未实施）：两级治理面、三候选方案、批次与未决问题 |
| `scripts/arch_doc.py` / `ARCHITECTURE.generated.md` | R0 架构依赖图生成物（契约+metadata，CI `--check` 防漂移） |
| `docs/architecture/env-registry.toml` | 开关登记表（R3a，格式由架构线定、效率线播种） |
| `.github/workflows/arch-guard.yml` | 架构守卫 CI（report-only 至 2026-11-03） |
| `.github/workflows/image-family.yml` | 图像家族独立 CI（R4a-1） |

## 8. 波次日志

| 波 | 收口 | 结果 |
|---|---|---|
| **W1 规则落地** | 2026-10-20 | 契约/守卫/基线 356 条/ADR-0001/协作契约落地；`arch_guard --strict` 0 违规；进入灰度（report-only 至 2026-11-03） |
| **W2 架构手术** | **2026-10-20 收口** | ADR-0002 Accepted；ADR-0003 **R1a 交付**（生物线 `151e9aea`/`77b1c1fb`：nervous_snapshot 647 行 + probes 578 行 + 测试 1,205 行，默认关；`self_status_source.rs` −173 行）；R4a-1 门禁隔离；R0 完成（依赖表生成物 + CI）；R3b 主体完成（env 登记 155 条，TODO-OWNER 0、fragments 0，验收见 ADR-0004 §7.7）；缩表 2 条、守卫 0 违规 |
| **W3 架构手术** | **2026-10-20 收口** | R7 交付：ContextBlock 单一化（`0bdd9bb4`/`a93fdbb3`）+ SessionStore 三义收敛（`1cbaee95`/`3c0bcde1`）+ dormant scheduler 归档（`14c0e9a0`/`d5a4c537`，Heartbeat 复核保留 `b36ee05e`）+ PromptCacheStabilizer 删除（`51bb6cfa`）+ bench 同步（`2481200b`）；R2 交付：Extension ABI 家族（`56ae816b`）+ 双 catalog（`911e2f49`/`b5f75ba9`）+ plugin 内部索引更名（`d8aa87af`）+ 命名迁移（`efcce6c4`）；R4b 四岛收口（23,649 行归档 + lark 解耦）；R3b 主体完成（155 条 TODO-OWNER=0）；R3c 待 2026-11-03 后 review；缩表 3 删 1 缩、守卫 0 违规（生物线在途 1 条除外） |
| **W4 架构波** | **2026-10-21 收口** | R1b 全交付（1a/1b/2a/2b/2c/3/4a 交付、4b 收口）/ R8 A-B1-C-D 部分 / R6 A-B + C-D 载体；全量 `gate_full` **341 套件 / 8570 passed / 0 failed / 51 ignored**（26.8 min，exit 0，日志 `artifacts/wave4-close-full-2026-10-21.log`）；基线缩表 2 条；guard VIOLATIONS 0 / exceptions=2；carries 如实（R8-B2/B3、R8-D wiring、R6-C/D 签核、夜间 full 未运行、R5 未开工） |
| **W5 阶段 6 收口** | **2026-10-31 反思归档** | 阶段 6 三段闭环：**清单 ✅ + 签核 ✅ + 反思归档 ✅**（本节 §10）⇒ ADR-0011 R6-C「至少 1 条 claim 走通四层」达成（claim `governance.approval_path`；四层条目 `V-formal-003` / `V-scenario-004` / `V-bench-002` / `V-signoff-001`）；bench 另一条 `V-bench-001`（claim `verification.kani_approval_sm`）；挂账表 §2 六行核销/已决并**保留编号**（便于他文回引） |

## 9. 变更记录

| 日期 | 变更 | 证据 |
|---|---|---|
| 2026-10-31 | **W5 阶段 6 第三段「反思归档」**（ADR-0011 D4 的最后一段）：手册新增 §10「波次复盘 · 阶段 6 反思归档」，**逐条引用台账 id**（`V-formal-003` / `V-scenario-004` / `V-bench-002` / `V-signoff-001` / `V-bench-001`）；同批把挂账表 §2 的 **#1 / #2 / #4 / #6 / #8 / #9** 六行按 2026-10-30 裁定核销/已决（**编号保留**以便 `engineering-review-handoff-2026-10-06.md` 等他文回引）、补 **§2 标题**（此前正文引用的「§2 挂账 #2」指向空处）、新增 **#10**（#2 残余 2 小项）、并把 **#11**（自动化 HTTP 闭环回归）**删掉** —— 裁定核实仓内已有`canonical_entry_e2e.rs::http_pending_approval_can_be_approved_without_double_execution` 覆盖完整闭环（202 执行 0 → resolve 200 执行**恰好 1** → 重复 resolve **400** 执行仍 1）⇒ 属重复登记 | 本节 §10 + `docs/04-internal/live-verification-ledger.md` §1.5/§1.6/§2 |
| 2026-10-20 | 手册 v1（规则落地同日） | commit `8bfefb1b` / `8e9dd3ae` |
| 2026-10-20 | 手册 v1.1：Wave 2 单项 ADR + R4a-1 门禁隔离（gate_full/image CI）+ arch_guard 例外抑制/登记表双通道/`--print-vendored` | commit `5e68f644` |
| 2026-10-20 | Wave 2 收口（R1a 交付 + R3b 主体完成 + 缩基线 2 条）；R4b 执行：B 归档干净、A 内容全落地但遭**共享 index 抢占**拆成 `df78542f`(混) + `6dccb1d2`(净)，裁决 **R2 接受现状不重写历史**（并行线 5 秒级连提，重写会二次事故）；事故登记入 ADR-0005 §3.2 | `22771a50` / `e8e7db33` / `6dccb1d2` |
| 2026-10-20 | **R4b 四岛收口**：lark 解耦（im 自有 wire 原语，行为保持）+ 归档，SDK 四孤岛 42 文件/23,649 行全部迁 `legacy/archived/sdk-islands/`；mutation 验证 = lark 代码引用 0、guard crates=30/edges=112/env=154/VIOLATIONS 0；遗留 = 5 依赖清理批 + c 头重生成 + 文档对齐 + STALE 11 缩表（Wave 3 收口） | `29387d78` / `23cae7cc` |
| 2026-10-20 | **Wave 3 收口**：R7 全批交付（ContextBlock 收敛 / SessionStore 改名 / dormant scheduler 归档 / PromptCacheStabilizer 删除 / Heartbeat 复核保留）+ R2 全批交付（Extension ABI 家族 / 双 catalog / plugin 内部 CapabilityIndex / 命名迁移 BehaviorModule·BehaviorRegistry）；契约 `current_wave=4`、extension_debt 清零、concepts known_debt 收敛为 1 组；基线收口实测 **340 条**（env 153 / edge 111 / size 65 / ext 6 / concept 4 / devcycle 1；删失效条目 3 条 + orchestration/lib.rs 1432→1427 缩 1 条）；Wave 4 = R1b + R8 + R6 | `efcce6c4` / `d8aa87af` / `096f1266` / `2481200b` |
| 2026-10-20 | **Wave 4 波中交付**：R7-2.1 `ContextAssembler` 单一管线（orchestration 唯一实现 + organ 策略桥，concept known_debt 清零）；R1b-1a 神经信号总线内核 + 唯一钩子桥 + 开关单入口 + 装配根接线（默认关、生产暂无订阅者）；已知 W1 待收口重钉 | `c8cb34a4` / `10ec68ae` |
| 2026-10-20 | 波中事故与修复：R2-3c 漏检（Debug impl 调 `PluginManager::capabilities()`）由全 workspace check 暴露并修复 `ae1f8779`；R7-4b-2 归档 scheduler 后 bench 未同步（E0432）→ bench 归档 `2481200b`；两起均记入 r2-/r7-inventory §7 教训（0 调用结论必须编译兜底；跨目标漏检） | `ae1f8779` / `2481200b` |
| 2026-10-20 | **R1b-1b 交付**：生物开关读取收敛到装配根单入口（删除 6 个 env 读取构造，4 个安装函数收 `NervousSwitchboard`/显式 bool；CLI 侧读取点如实保留待 R1b-2/3）；台账 4 行漂移更正（SG-18/SG-25 → PRODUCTION WIRED、SG-22 ritual_producer → Wired、SG-26 缺口实况）+ 3 条结构校验单测；lib 277 passed、62 测试目标全绿、clippy/fmt 绿；已知 W1 更新为 cognitive 4757→4763、新增 STALE 1（play_tools 1414）留收口 | `6837d9b7` |
| 2026-10-20 | **EXC-0002 登记**：R7-2.1 + R1b-1b 引入的 `cognitive.rs` 尺寸增长（4757→4763，+6 行）按 ADR-0001 走 size 类例外（weight=1，created_wave=4，expires_wave=6，owner=architecture-line），不改基线（只减不增原则未破）；守卫输出 VIOLATIONS 0 + 抑制 NOTE；清偿任务 = 收口期把开关构造迁出 cognitive.rs | `docs/architecture/exceptions.toml` / `3dd9e4e0` |
| 2026-10-20 | **R1b-2 交接单**：`r1b-2-handoff-2026-10-20.md` 交付（范围拆 2a bio_tone 订阅 / 2b bio_lifecycle + probe / 2c 审批面 publish；含前置闸口、如实边界、验收清单 11 条、回滚与共享文件协调）；ADR-0009 §4 R1b-2 指向交接单；待生物线 WIP 收口后开工 | 交接单文件 |
| 2026-10-20 | **R1b-3 交接单**：`r1b-3-handoff-2026-10-20.md` 交付；架构线二次复核收窄为**零新增信号变体**（ritual 由 `RitualStateStore` 共享文件打通 → 只补端到端测试+陈旧注释清理；sleep pressure 模块内自消费、scar learn 无生产者/订阅者、session event 消费方需 `UncertaintySignal` → 如实 NotWired + 具体 reason）；ADR-0009 §4 R1b-3 同步收窄；硬约束（默认关零变化、不反转依赖、schema v2 归 R1b-4、每源独立 commit）不变 | 交接单文件 / `fc69f6cd` |
| 2026-10-21 | **R1b-4a 提前交付**：`nervous_system` schema v2 additive——`bus` 子对象报总线聚合计数（subscribers/published/delivered；`last_observation` 恒 null；未绑总线时 null）；`SelfStatusModule::new_with_bus` 由装配根传入唯一 `NervousBus`（默认关时桥不注册、计数为 0）；v1 字段逐字节不变；新增 `EXC-0003` 登记 tool_modules.rs 尺寸增长（1204→1216 为真实接线与构造搬迁）；实测 lib 277 passed / nervous_snapshot 集成 13 passed / clippy·fmt 绿 | （R1b-4a 交付批） |
| 2026-10-21 | **R6-A/B 交付**（架构线接管，ADR-0011 §9）：`gate_full.ps1` 加进程锁 `logs/gate-full.lock`（`-FailIfBusy`/`-LockTimeoutMinutes`，锁冲突 exit 2）+ `FAILURE_CLASS=oom/env/flaky/regression`（`-ClassifyLog` 判类）；新增 `docs/02-guides/verification-architecture.md`（三层语义表）；`rust.yml`/`release-prep.yml`/`companion-desktop-ci.yml` 守门改读契约 `[locked]` 真源（守卫 1/3/4 与 R11/14 键，手抄 LOCKED 清单 0 残留）；`arch_guard.py` 增 CI 口径守卫 W11（扫 workflow 现行旧口径字面量，首跑捕获 2 处并修复）；实测 guard VIOLATIONS 0（exceptions=2 / STALE 1 保持） | `5858fc1f` |
| 2026-10-21 | **R8-A 交付**（架构线接管，ADR-0010 §9）：新增 `crates/foundation/governance/src/chain.rs`——`GovernancePhase` 七段（含 Reflection）/`ChainOutcome`/`ChainRecord`/`ChainSignatureMode` + 四注入端口与其默认实现（占位多签显式 fail-closed、内存审计链、进程内反思调度、系统时钟）；`run` 只收 `GovernanceRequest`（已含 security_context，避免双真源）；如实未交付：Deliberation 未调用、ApprovalClosure 不消费审批对、SelfDisable 未接 SelfDisableAudit、真多签 v2.1；实测 governance lib 204 passed（chain:: 10 passed）/ clippy / fmt 绿 | `fd74a669` |
| 2026-10-21 | **R6-C/D 载体批**（架构线落载体，ADR-0011 §9）：`docs/status/verification-ledger.yaml`（四层验收结构化真源：5 条单层条目，四层齐全 0 / signoff 0 / 阶段 6 NOT RUN）；`docs/status/formal-coverage.md`（Kani 终局矩阵 11 件逐件登记、matrix 外=未覆盖、TLA 本机不入 CI、Miri 面与未覆盖清单）；`.github/workflows/nightly-full.yml`（R6-D 成本分层载体：cron UTC 18:00 + dispatch，未实跑=NOT RUN）；顺带订正 `kani.yml`「22 件」旧口径为 11 件（K18 终局矩阵实况）；PR 层降级与 ledger 脚本化/签核仍归效率线与 owner；补只读校验器 `scripts/verify_ledger.py`（schema/证据/summary 漂移，实测 exit 0） | （R6-C/D 载体批） |
| 2026-10-21 | **R8-B 设计候选登记（未实施）**：新增 `r8-b-entry-convergence-design.md`——实测 `GovernancePipeline` 已实现 `GovernanceHook`，chain 的 Gate 面无需改 governance crate；收敛点在 CLI 装配根 `build_production_governance_parts_from_env` → `PermissionPresetGovernanceHook` → `Runtime::with_governance`；三候选方案（A 入口链不推荐 / B chain 作为 runtime 唯一治理面推荐 / C 持久审计先行）+ 批次 R8-B1..B3 + 5 条未决问题；如实标注 R8-B/C/D **NOT IMPLEMENTED**，本批 0 生产代码 | 设计候选文件 |
| 2026-10-21 | **R8-B1 交付**（架构线接管，ADR-0010 §9）：`governance/src/chain.rs` 新增 `ChainGovernanceHook`（chain → `GovernanceHook` 适配器，方案 B 第一步）：一次评估=一次 chain run；`ChainOutcome` 映射 `GovernanceVerdict`（Denied 带 `<phase>: <reason>`，NotConfigured fail-closed）；`approval_resolved` 经新增 `GovernanceChain::gate()` 转发；纯新增、**零生产接线**（Runtime 仍持 pipeline 包装）；实测 governance lib 209 passed（新增 5 条适配器测试）/ clippy / fmt 绿 | （R8-B1 交付批） |
| 2026-10-21 | **R8-C 交付**（架构线接管，ADR-0010 §9）：`governance/src/chain.rs` 新增 `ReflectionScheduler::take_due`（默认空=零行为变化）、`ReflectionDue`、`DelayedReflectionScheduler`（clock + delay_ms 注入的进程内延迟队列，到期项只交还一次）；`GovernanceChain::run` 裁决前 `sweep_due_reflections`，以 `governance.reflection_due` 落审计，审计失败 `Denied{phase: Reflection}` fail-closed；如实未交付：autopilot、常驻反思执行器、反思结果回写，延迟仅进程内（重启即丢），无生产装配注入；实测 governance lib 214 passed（新增 5 条）/ clippy / fmt 绿 | （R8-C 交付批） |
| 2026-10-21 | **R8-D 部分交付**（架构线接管，ADR-0010 §9）：`rust.yml` 新增 `Governance chain drill (R8-D)` 步骤（`cargo test -p apeireth-governance --test governance_chain_drill --locked`，本地实测 3 passed）；如实未交付：`docs/status/wiring.yaml` 治理面登记（生成器属效率线）、CI 新增步骤尚未在远端运行 | （R8-D 部分交付批） |
| 2026-10-21 | **R1b-2a/2b 验收复核**（架构线）：逐项复核生物线交付 `404919f6`/`326e1542`（调质订阅）与 `a71639f9`/`0f4ef4dc`（生命周期订阅 + probe，观测面 9/14）；确认默认关零行为变化、观测面单一发布点、台账 3 条校验单测仍在；如实未做：R1b-2c 审批面 publish 未开工、`AfterTurn` 仍走钩子（归 R1b-3）、全量门禁未在复核时复跑；验收记录见 `r1b-2-handoff-2026-10-20.md` §5/§8 与 ADR-0009 §8 | 复核批 |
| 2026-10-21 | **R1b-3 交付**（架构线，零新增信号变体）：ritual 已由 `RitualStateStore` 共享文件打通（工具写 `<care_dir>/ritual_state.json` → CLI care heartbeat 同拍读盘），只补端到端测试 `a_ritual_tool_write_is_visible_to_the_next_heartbeat_beat` + 清理陈旧注释；`sleep_pressure` / `scar_ledger_learn` / `session_event_source` 取证后如实 `NotWired` + 具体 reason；实测 runtime-assembly lib 291 passed / nervous_snapshot 13 passed / cli lib 57 passed、clippy·fmt 绿；guard VIOLATIONS 0（exceptions=2 / STALE 1 保持） | （R1b-3 交付批） |
| 2026-10-21 | **R1b-2c 交付并复核**（生物线）：`NervousSignalKind::ApprovalSocial` 生产者+订阅者同批 additive（代码 `f2dad300` / 记录 `815d85e7`），默认关零行为变化；全量门禁随 Wave 4 收口批绿 | `f2dad300`/`815d85e7` |
| 2026-10-21 | **Wave 4 收口**：R1b/R8/R6 交付矩阵见 §3；全量 `pwsh scripts/gate_full.ps1 -FailIfBusy` = main 264 套件 / 6263 passed + image 77 套件 / 2307 passed = **341 套件 / 8570 passed / 0 failed / 51 ignored**（HEAD `f2dad300`，26.8 min，exit 0）；基线缩表 2 条（`play_tools.rs` 1412、`cli/lib.rs` 3878）；`test-baseline.md` 现行基线刷新；`current_wave=5`；carries 如实（R8-B2/B3、R8-D wiring、R6-C/D 签核、夜间 full 未运行、R5 未开工）；R3c 2026-11-03 后 review | 收口批 commit |
| 2026-10-21 | **效率线重启批验收**（`d2621788..3d6d3d44` 10 提交）：E1 test-baseline 维护/全量执行权接管（数字未变 341/8570/0/51）；E2 治理面 4 条 claim（claims 20→24，均 IMPLEMENTED）+ wiring 重生成；E4 `verify_ledger.py` 接入 docs-truth（负向测试留记录）；E3 夜间 CI 按 owner 裁决**不推送、全本地 → NOT MET 保留**；D29/DTS/口径修复随批；本行验收 = docs_truth / arch_doc / wiring / guard / ledger 全绿，crates 零改动（仅 nervous_snapshot 注释 +5 属生物线） |

## 10. 波次复盘 · 阶段 6 反思归档（2026-10-31）

> **本节是 ADR-0011 D4 要求的「阶段 6 第三段：反思归档」** —— 载体 = 本手册，
> **逐条引用台账 id**（`docs/status/verification-ledger.yaml`，四层验收结构化真源）。
> 前两段（**清单** = 台账有条目 · **签核** = `signoff` 层）已由 2026-10-30 裁定闭合，本段补齐 ⇒
> **阶段 6 三段全绿** ⇒ ADR-0011 R6-C「**至少 1 条 claim 走通四层**」达成。

### 10.1 本波引用的台账条目（**四层齐全的那一条**）

| 层 | 台账 id | claim | 证据（摘要） |
|---|---|---|---|
| formal | `V-formal-003` | `governance.approval_path` | Kani 治理单调性族（4 件，含 `kani_governance_fail_closed_no_bypass`） |
| scenario | `V-scenario-004` | `governance.approval_path` | HTTP 审批闭环（含**防重复执行**） |
| bench | `V-bench-002` | `governance.approval_path` | `governance_decision_bench`（授权 16→1024 ⇒ 判定延迟 **1.75×**；三条判定路径都被走到） |
| **signoff** | `V-signoff-001` | `governance.approval_path` | **如实回填** owner **2026-09-28 的既有真机签署**（`owner: Mavis`）—— **是「转录」不是「新签」** |
| bench（另一条） | `V-bench-001` | `verification.kani_approval_sm` | `research_approval_bench`（研究用审批状态机） |

> ⚠️ **signoff 的诚实口径**：owner 当天签的 10 步里就含「**审批闭环**」，而本 claim 正是**生产审批路径**
> ⇒ ⇒ **绑定忠实** ⇒ ⇒ owner **无需任何新动作**（**不代签**：`signoff` 只记 owner 标识与日期，不入密钥）。

### 10.2 问题（**本波最该记的一条：我问错了**）

**我从「最后 5% 是 owner 才能给的东西」这个判断出发**，向 owner 提了四项问询
（A 新 DeepSeek key · B MiniMax key · C embeddings 端点 · D 真机操作 + 签核）。
**2026-10-30 裁定逐条用仓内证据核对后判定：四项里没有一项非要 owner 不可** ——
而**真正的原因是我只读了台账 §2 的挂账表，没读 §1.4 里那几行核销批注**：

| 我以为 | 实际（仓内早已有） |
|---|---|
| #9 要重跑（DeepSeek key 被撤销） | **2026-10-10 就已核销** —— §1.4 行 46 记着 provider 2/2 · organ 2/2 · council 1/1 全实跑 ⇒ §2 那行是**残留**，不是待办 |
| #8 要 embeddings 端点 | **本仓侧已闭环**（真 HTTP E2E 5/5）⇒ 缺的是**厂商侧**（OpenAI 兼容格式），属**厂商风险** |
| #4 要 owner 点桌面 | **自动化层已有完整闭环**（202 → 执行 0 → resolve → 200 且执行**恰好 1** → 重复 resolve 400 且执行仍 1）⇒ **缺口陈述不成立** |
| #2 要 owner 确认 | **owner 2026-09-28 已真机签署** ⇒ 可**如实转录** |

⇒ ⭐ **教训一（可复用）**：**挂账表里有行 ≠ 那行是待办** ⇒ **读台账必须先读 §1.4 的核销批注**。
⇒ ⭐ **教训二**：**向 owner 派活之前，先把「仓内已有证据」翻一遍并列出**；
⇒ **问不清楚的不是问题，是自己的检索缺口**。

### 10.3 决策（已落地，可复核）

1. ✅ **挂账表 §2 六行按裁定核销/已决，且**编号保留**（`#1` `#2` `#4` `#6` `#8` `#9`）
   ⇒ 理由：其它文档（`engineering-review-handoff-2026-10-06.md` 等）按编号回引，
   **编号只增不改** ⇒ 历史引用不能断；台账新增 **§2 标题**（此前正文引用的「§2 挂账 #2」**指向空处**）。
2. ✅ **删掉我自拟的 #11**（自动化 HTTP 闭环回归）—— 裁定核实**仓内已有**该闭环 ⇒ **属重复登记** ⇒ 删。
3. ✅ **新增 #10**（#2 的残余 2 小项：图形卸载真删验证 / 斜杠菜单点击流）—— 裁定只核销 #2 的**核心 10 步**，
   这两项**未被裁定覆盖** ⇒ 留账（不虚报已消）。
4. ✅ **厂商侧风险与本仓侧风险拆开记**（#1 / #8）：本仓侧（传输/解析/错误分类）由单测与真 HTTP stub 证完；
   厂商侧（真实响应形状与配额）是**厂商风险**，不与本仓代码风险混算。
5. 🟡 **留给下一波的两条（不阻塞）**：① #1/#9 若要留「厂商侧仍可用」的**当期**证据，
   走 **owner 侧只读复核命令**（**密钥不出 owner 的机器**，只把输出贴回）——
   2026-10-31 已用 MiniMax key 实测：**厂商真机 1 passed**（§1.5），但**换厂商复跑 #9 的两支不行**：
   `openai_compatible_live` **硬编码** `DEEPSEEK_BASE_URL`（不走 `from_env`）、
   `organ_live_llm` **硬编码请求模型** `deepseek-v4-flash` ⇒ **测试的厂商硬编码**才是真阻塞点
   （改动在 `crates/engine/provider/` + `crates/engine/organ/` = 总指挥区 ⇒ 只报不改）；
   ② `docs/status/verification-ledger.yaml` 的 **`stage6_drill` 字段**（`NOT RUN → 达成`）
   在 **#94 的 10 条版本**里 ⇒ **不属我这条线的文件** ⇒ 请随 #94 一并更新。

### 10.4 下一步

| # | 事项 | 归属 |
|---|---|---|
| 1 | 把两支 live 测试的**端点与模型名改为可配**（默认仍 DeepSeek，行为不变）⇒ 换厂商即可复跑 | 总指挥区（provider / organ） |
| 2 | `verification-ledger.yaml` 的 `stage6_drill` 随 #94 的 10 条版本更新 | 总指挥区（该文件是他的） |
| 3 | 若要**OpenAI 兼容** `/embeddings` 的**当期**厂商证据：给一个该格式的端点（本地部署即可） | owner（可选，不阻塞） |
| 4 | #10 两小项（图形卸载真删验证 / 斜杠菜单点击流）需 Windows 真机 | owner（可选，不阻塞） |

### 10.5 这一段的边界（如实）

- **没有**代签任何 `signoff`；`V-signoff-001` 是**转录** owner 2026-09-28 的**既有**签署；
- **没有**把「厂商侧未实测」写成「已实测」⇒ #1 / #8 的厂商侧仍标为**厂商风险**；
- **没有**动 `crates/engine/provider/` 与 `crates/engine/organ/`（总指挥区）⇒ 上述第 1 条**只报不改**；
- 本节的**结论不替代** `docs/status/verification-signoff-ruling-2026-10-30.md`（裁定原文）
  ⇒ **裁定是上位**；本节只做**归档与复盘**。
