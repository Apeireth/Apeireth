[Document-Meta]
Document: docs/04-internal/01-handoffs/backend-l4-convergence-2026-11-07.md
Class: STATUS
Status: 🟢 已核验（本地门禁全绿；远端 CI 待复核）
Last-Modified: 2026-11-07
Author: lead

# 后端 L4 零债收口（2026-11-07）

> **四级口径**：本页是**进度/对账页**，不是能力声明页。
> 每条能力的等级与证据以 `docs/status/claims.yaml`（唯一真源）为准；
> 本页只回答「这一轮把哪些缺口补上了、怎么复验、还差什么」。
> 本轮**不**宣称任何 **HARDWARE VALIDATED**（无真机、无真模型 E2E）。

**分支** `integration/consolidate-2026-10-31` ⇒ 推送目标 `origin/debt-round8`（PR #94 → `main`）。
**收口时 tip** `ced07982`（本地与远端一致）。

---

## 一、结论：L4 三条判据本地全部达成

| 判据 | 状态 | 对账 |
|---|---|---|
| **① L2 声明自洽** | ✅ | 52 条 = **51 PRODUCTION WIRED / 1 NOT IMPLEMENTED**（唯一那条是**已归档且如实标注**的 `gap.prompt_stabilizer_unwired`）。`check_claims_verify --check` ⇒ **51 条 verify_cmd 真跑全过 / exit 0**（1 条归档豁免、0 条延后）；`docs_truth --verify --block-stale` ⇒ **stale 0 / fail 0 / suspected_missing 0** |
| **② L3 骨架完整** | ✅ | **7/7 骨架全部接线**：`rt.system_one_internal` · `gov.reflection_queue` · `rt.module_execution_contract` · `rt.module_messages` · `rt.owned_agents` · `mem.registered_namespace` · `rt.mesh_local_worker`（最后一条带 ⚠️ public-status 部分标注）。记忆 crate：可达 **90 → 109**，不可达 28 条**全部登记「已解释」**（各引既有台账）⇒ **未解释缺口 32 → 0** |
| **③ L4 零债** | ✅ | 孤儿 **89 → 87**，且**每条都带「保留理由 + 触发条件」**（未解释 = 0）；arch-strict **VIOLATIONS 0**；`scripts/harness.py verify` ⇒ **VERDICT PASS（17 PASS / 0 FAIL / 11 按 scope 跳过）** |

> 接线清单（含**生产调用点**与**反证**）已汇总在
> [`docs/03-reference/capabilities-matrix.md` 附录 A](../../03-reference/capabilities-matrix.md)，
> 本文不重复贴代码位置，避免两处漂移。

---

## 二、本轮新增/变更的生产接线（六面 + 一个审计面）

1. **反思队列**（`gov.reflection_queue`）：`governance_chain::production()` 按
   `APEIRETH_ENABLE_REFLECTION_QUEUE`（只认 `1`）注入 `DelayedReflectionScheduler(0ms)`；
   默认关 = 与接线前同行为。delay=0 语义：`run` 先 sweep 到期、后 schedule ⇒ 本轮刚排的条目**下一轮**才到期 ⇒ 不依赖墙钟。
2. **模块执行契约选举**（`rt.module_execution_contract`）：`module_contract_wiring` 静态装配表 +
   构造期 `elect_provider_module` / builder 期 `attach`（由 `cognitive_registry::register_with_policy` 调用）。
   表外模块逐字节零变化；选举**只收窄面不授权**。
3. **模块消息管道**（`rt.module_messages`）：桌面 Host 策略装 `ModuleMessagePolicy::default()`
   ⇒ 管道从「不存在（发送恒 `Disabled`）」变为「存在且**零授权**」。
4. **注册式记忆工具面 + 模块自有子 Agent**（`mem.registered_namespace` / `rt.owned_agents`）：
   `memory_assembly::wire(...)` 让 7 个 `memory_*` 工具进生产工具面，Host 映射**迟绑定同一实例**；
   `memory_organize` 起模块自有子 Agent（用它自己的 Host grant）。
5. **社会修复 → 提示词修复指令**（`bio.social_repair`）：共享槽（总线拒绝写 / `TurnStart` 每回合读）。
6. **本地 Mesh worker**（`rt.mesh_local_worker`）：新 Host 配置面 `mesh.json`（StoredDoc 拒开）+ **双条件硬失败**
   （配置合法 **且** System One 监督者已绑才注册）。
7. **记忆能力审计面**（只读，不写库）：`capability_audit.rs` + `apeireth memory handover` 自述 ⇒
   记忆可达率上台阶、未解释缺口归零。

   ⚠️ **可达 +19 的口径要说清（别读成「十九条功能接线」）**：新增的只读审计面**导入了约二十个记忆模块**
   （`agent_trace` / `arbitration` / `swap_flow` / `forget_coordinator` / `vector_rebuild` / `cold_archive` /
   `calibration_critic` / `ensemble` / `reflexion_tuning` / `daily_summary` 等）⇒ 这些模块从「整块不可达」
   变成「**经生产入口（handover CLI）可触达**」。按本仓判据这算**可达**，但它们提供的是**自述/审计读数**，
   **不是**新的功能性生产调用点。⇒ D4 候选池台账因此把这 10 个模块标成 `left-pool`
   （`scripts/d4-ledger.json` 已 `--gen` 同步，`left_pool_count` 3 → 13），这是**口径后果**，不是功能新增。

同时清掉三处 **arch 尺寸违规**（都用**真实拆分**，不是删注释装样）：
`retrieval_pipeline.rs` 1324→**1273**（融合三规则搬进 `retrieval_pipeline/fusion.rs`，两语料 **Δ +0.0000** 证明行为等价）、
`production.rs` 1897→**1896**、`bio_production_wiring.rs` 1238→**812**（668 行既有测试原样搬出）。

---

## 三、复验命令（原样可跑；`RUSTC_WRAPPER=` 必须为空）

```sh
cd /home/bai/wt-apeireth-r3
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 RUSTC_WRAPPER= bash -c '
  cd /home/bai/wt-apeireth-r3 &&
  python3 scripts/harness.py verify &&                 # 权威门禁（本地与 CI 同一套）
  python3 scripts/check_claims_verify.py --check &&    # 51 条 verify_cmd 真跑（分钟级）
  python3 scripts/docs_truth.py --verify --block-stale &&
  python3 scripts/wiring_report.py --verify &&
  python3 scripts/arch_guard.py --strict &&
  python3 scripts/audit_unwired.py --check &&
  python3 scripts/check_memory_reachability.py --check &&
  cargo fmt --all -- --check
'
```

⭐ **`RUSTC_WRAPPER=` 不是可选项**：`/usr/local/cargo/config.toml` 设了 `rustc-wrapper = "sccache"`，
而本机 sccache 守护进程以 **root** 跑 ⇒ 产物 root 拥有 ⇒ 以 `bai` 运行 cargo 会读到
`E0786` / `Permission denied`（**看起来像代码问题，其实是身份污染**）。本轮把这条根因也修进了
`scripts/check_fusion_channels.py`（闸门内强制清空 wrapper），此前它一直表现为「融合通道闸门假红」。

---

## 四、棘轮与预算对账（**只减不增**，一处例外已留痕）

| 项 | 变化 | 说明 |
|---|---|---|
| `scripts/arch-baseline.txt` 三处尺寸 | **下调** | 1324→1273 / 1897→1896 / 1238→812（真实拆分清偿） |
| 孤儿 API | **89 → 87** | 生产抓取补 UA+限流（2 条）、`elect_provider_module` 被生产引用（1 条） |
| 记忆可达 / 未解释缺口 | **88→109 / 33→32→0** | 两处棘轮同批收紧 |
| crates / production edges | **30/30 · 111/111** | 未动（增长面已封死） |
| **env 名（`max_distinct_names`）** | ⚠️ **161 → 162** | **本轮唯一一次预算增加**，由主指挥拍板并写进 `docs/architecture/contract.toml`：`APEIRETH_ENABLE_REFLECTION_QUEUE` 是「默认关能力」的**唯一可见开关**，装配根是代码构造（`cli/src/lib.rs` 头部零余量）⇒ 无配置面可挂；已核对**无可让位的死开关**。⇒ **162/162 用尽**，下一轮任何新增 env 名**必须先删一个或改配置项** |

---

## 五、诚实边界（**达标也不抹掉**，逐条都写进了 claims notes）

1. **`rt.mesh_local_worker` 的 `public status` 发布面生产未启用**（没有可接收 PublicStatus 的 sink）⇒
   标题里已用 ⚠️ 标注；本轮只证明「worker 注册 + 单包回环收发」。
2. **`mem.registered_namespace` 的自动检索默认关**（`with_automatic_retrieval(false)`）：开它会让既有不变量
   `learning_does_not_pollute_the_primary_transcript` 从 2 条变 4 条（每个回合多一次工具往返），且生产另有
   `ProactiveRecallPolicy` 这条**显式**召回入口 ⇒ 「注册工具面 ≠ 每回合自动注入」。生产默认只注册**兼容 ABI**；
   契约/Protected 工具面是 Host 显式 opt-in（6 个工具 Protected + 需 System One 复核面）。
3. **`bio.social_repair` 的修复指令块是 untrusted V1 数据**、**不是 system 权威**；事件源目前只有
   「人工拒绝审批」一条（`plan_repair` 的其它 `UncertaintySignal` 变体在生产上仍不会被驱动）；计划由 organ
   固定规则表确定性算出（不推断动机）；默认关。
4. **`gov.reflection_queue` 只证明「可接进生产治理链并真产出审计」**，不证明存在 runner 消费队列
   ⇒ auto-pilot 仍 NOT IMPLEMENTED。
5. **`rel.release_asset_verify`**：「签名状态如实报」≠「签名有效」；代码签名与可复现构建证明仍属排期待办；
   本条证明校验器语义 + 流水线接线，不证明某次真实发布产物已被验证。
6. **三个判据盲区**（已登记在 `scripts/unwired-baseline.txt` 头部）：
   ① 孤儿扫描面 `crates/*/*/src` 只匹配**深度 2** ⇒ 11 个 `crates/engine/image/*` 与 `crates-ui/` **不在视野**（漏报方向）；
   ② 无花括号的 `#[cfg(test)] use ...;` 会把**下一个 item** 当测试码吞掉；③ PyO3 `#[pymodule]` 入口结构性看不见。
   ⇒ **「未解释 = 0」的结论必须带这三条声明**。

---

## 六、仍未完成 / 待复核（❌ 明确列出）

- ❌ **远端 CI 复核未做**：推完即收口，`harness / merge-gate` run 当时是 `queued`。本地 `harness.py verify`
  与 CI 同一套门禁且 PASS，但**远端 run 结果未复核** ⇒ PR #94 合并前必须看一眼。
- ❌ **`rt.mesh_local_worker` 的 public-status 面**：要启用需再加一个 `status_target` 配置字段 + 一个真 sink 模块
  （属新面，未擅自加）。
- 待 **`bio.social_repair`** 的其它事件源（用户说错 / 矛盾检出）接入 —— 现在是单事件源。
- 待 **`mem.registered_namespace`** 的自动检索是否要开（product 决策；开之前必须先解决转录污染口径）。
- 待 **记忆 crate 剩余 28 条「已解释不可达」** 逐条转正或确认长期保留（现在都有台账依据 + 触发条件）。
- ❌ **增长面封死**：crates 30/30、edges 111/111、env 162/162 ⇒ 下一轮任何新增必须先删一个。

---

## 七、与并行线（deputy）的协同记录

- 本轮多次 `git fetch` + `git merge --no-ff --no-edit origin/debt-round8`，把 deputy 的记忆实验线
  （中文疑问词规范化 / 停用字表 41→20 字 / 真实查询基线调研 / 辅助通道填充带准入）全部并入，
  **收口时本地与远端 0 ahead / 0 behind**。
- 两条线在同一文件上撞过：deputy 的「填充带准入」把 `retrieval_pipeline.rs` 顶到 1324 行（arch 棘轮红）
  ⇒ 我按既有做法做**真实拆分**（不是我行我素的改判据），并用 `check_golden_baseline` 的 **Δ +0.0000** 证明行为等价。
- 未删任何跨线在途代码；对「接线半程」的他人工作，我选择**回滚到绿**（`memory_assembly` 首版 2 个测试红）
  再按需重做，而不是留着红树。

---

## 八、关联材料

- 声明真源：[`docs/status/claims.yaml`](../../status/claims.yaml) · 仪表盘 [`STATUS.md`](../../status/STATUS.md) · 接线账 [`wiring.yaml`](../../status/wiring.yaml)
- 能力速查（含本轮附录 A）：[`docs/03-reference/capabilities-matrix.md`](../../03-reference/capabilities-matrix.md)
- 演练取证：[`docs/04-internal/governance-chain-drill-2026-10-20.md`](../governance-chain-drill-2026-10-20.md) §7
- 发布资产校验：[`docs/packaging/release-asset-verification.md`](../../packaging/release-asset-verification.md) §6
- 记忆台账：[`memory-module-atlas-2026-11-05.md`](memory-module-atlas-2026-11-05.md) · [`memory-experiment-ledger-2026-11-07.md`](memory-experiment-ledger-2026-11-07.md)
- 指挥看板（仓外）：`/home/bai/Projects/apeireth-coordination/lead-commander-board.md` §二百一十 ~ §二百一十三
