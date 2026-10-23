# 01 · 架构与能力审计结论册

> **本册定位**：把「能力缺口 / SPEC 漂移 / 成熟度 / 功能旋钮 / 施工工序」类审计的结论合成一册，
> 保留每份来源的结论要点、关键表、残余清单与 owner 决策位，并逐条复核「今天还成立吗」。
>
> **审计对象与日期**（各份来源日期按原样标注，不改写）：
>
> | # | 来源（原路径） | 日期 | 篇幅 |
> |---|---|---|---|
> | 1 | `docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md` | 2026-10-06（含 10-06 夜四审/五审/六审、10-10 收口） | 488 行 |
> | 2 | `docs/04-internal/w3-legacy-gap-archaeology-2026-10-10.md` | 2026-10-10（含同日复盘修正） | 201 行 |
> | 3 | `docs/04-internal/r9-r10-spec-drift-audit-2026-08-28.md` | 2026-08-28 | 228 行 |
> | 4 | `docs/04-internal/round-12-final-gap-audit-2026-08-28.md` | 2026-08-28 | 320 行 |
> | 5 | `docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md` | 2026-08-28 | 323 行 |
> | 6 | `docs/04-internal/capability-knob-audit-2026-09-12.md` | 2026-09-12 | 60 行 |
> | 7 | `docs/04-internal/sub-agent-audit-round-4-2026-08-28.md` | 2026-08-28（含 2026-09-05 对账批标注） | 203 行 |
>
> **审计方法**：读源码实证 + 逐份规格与台账逐行对账，按四级口径
> （`IMPLEMENTED` / `PRODUCTION WIRED` / `DEFAULT ENABLED` / `HARDWARE VALIDATED`，
> 定义见 [system-capabilities.md](../../01-architecture/system-capabilities.md) 头部）定级，
> 不以历史文档自述为证据。
>
> **复核基线**：2026-10-23，仓内事实核验（工作树 HEAD `b4aabfc2`）。复核只查「结论今天是否成立」，
> 不重跑全量门禁；测试数字一律引用 [test-baseline.md](../../03-reference/test-baseline.md)，
> 能力声明一律引用 [claims.yaml](../../status/claims.yaml)。
>
> **结论摘要**
>
> 1. **v2 不是 v1 的降级，是形态重构**：v2 用远小于 v1 的工作区承载了主链路（协议 / 网关 / 运行时 /
>    治理 / 工具 / 记忆核心 / 器官库级），并新增了 v1 没有的层化治理 hook、会话级权限预设、
>    错误码契约、无重启配置、桌面伴侣全链（2026-10-06 审计 §5.1）。
> 2. **「v1 有 v2 无」这个前提本身被推翻过两次**：2026-10-10 的 legacy 源码级核验把
>    `HybridCognitiveRouter`、`ToolSynthesizer` 改判为**纯愿景项**（v1 亦无），
>    把 `thought_cluster` 改判为**v2 早已实现**（`cluster_store.rs` 改名件）。
> 3. **主账的 🔴 清单大面积过时**：2026-08-28 主账（`apeireth-1-0-vs-2-0-functional-gap`）的 33 项
>    🔴 里，多数在 2026-10-06 复核时已升级为**库级实现**，且后续批次陆续接线；
>    该主账已带 2026-10-06 历史批注并降级为「历史记录 + v1 功能全集清单」用途。
> 4. **规格漂移面已经收敛**：2026-08-28 的 R9/R10 spec drift 审计判定「0 drift（除 HEAD 快照漂移）」，
>    当时唯一的两处真错账是 HEAD 漂移；该结论对**当时文件**成立，对今天不成立（快照已多次前进）。
> 5. **能力旋钮的文档—代码错配已被整改**：2026-09-12 记录的「只读工具永远 Deny」「桌面无旋钮」
>    等错配，在同批整改中落地（只读工具默认放行 + 隐私逃生门 + 审批策略预设档 + 认知深度预设档）。
> 6. **「0 装」纪律双向性是本组审计的核心结论**：审计必须对源码实证，不能只信历史文档——
>    既不能把「已实现未接线」说成 0 装，也不能把 0 装说成已接线（2026-10-06 §7.3 教训）。

---

## 1. v1 vs v2 后端能力差距 + 未实现愿景审计（2026-10-06）

**来源**：`docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md`
**方法**：四方并行审计 —— v1 源码清单（`legacy/`，105 crate）/ v2 现役源码（18 crate）/
既有差距文档全文提取（8 份）/ 愿景与 ROADMAP 状态对账；口径一律取四级分层。

### 结论（原文要点保留）

1. **规模与形态**：v1 = 105 crate 的「伙伴器官」聚合（donor 77 / archived 15 / frozen 13）；
   v2 = 18 crate 的微内核 + 显式装配（PluginRegistry + CapabilityRegistry + Runtime）；
   治理形态由 v1 的 5 重守门 + PermissionPack 变为 GovernanceHook 三态 + 3 hook 生产常挂 +
   会话级权限预设三态。
2. **按域对账（定性：✅已实现 / 🟡部分 / ⛔明确延期 / 🔴0 装 / 🔒LOCKED / 🖥️待硬件）**：
   - 记忆：SQLite pool + MemoryStore v2 ✅；VectorIndex（cosine 语义对齐、**BM25 缺**）🟡；
     graph ✅ / **causal engine 缺** 🟡；consolidation_writeback / daily_summary / diary /
     cross_diary / memory_injection **0 真实施** 🔴；ONNX 嵌入 stub 🔴。
   - 工具/进程/沙箱：Shell/Filesystem/Fetch/Search/Repo ✅；Browser 🟡（借 MCP 调研）；
     Invest/Learning/SystemMonitor ⛔(P3)；WorktreeSandbox ⛔(P2)；
     **PlatformSandbox（真隔离）0 真实施** 🔴；ToolSynthesizer ⛔(P3)。
   - 感知：Text ✅；**Vision / Voice / 其余 modality 0 或待硬件**。
   - 认知/器官：9 organ **库级真实现**、串联层已补（IMPLEMENTED / 生产串联 🟡）；
     12 slot → 6 WIRED（judge/council 默认关）+ 6 DEFERRED 收敛；
     HybridCognitiveRouter / meta_thinking / reflexion / thought_cluster / intent_brier **0**；
     reflection 4 阶段并入 self_assessment ⛔。
   - 物种/关系/教养（v1 愿景核心）：education / partner / community / principles / tone /
     morphology / milestone / oracle_adapters / experiment_field **0** 🔴。
   - 治理/安全：Governance 5-gate → 3 hook ✅；Onion ABAC（L0 不可变）🔒LOCKED；
     **三洋葱 L3-L5 = 0 装** 🔴；PII/注入/凭据泄漏 hook 2 hook ✅；AuditHashChain ✅；
     SelfDisableGuard ✅；SovereignControl 🟡；信任分级/审批频率/免审批黑名单 ⛔(RC-13)。
   - 协议/网关/交付：Gateway 路由 + egress filter ✅；SSE 真流式（2026-09-10，210 帧实测）✅；
     MCP handler 🟡；EventBus core ✅ / backbone 🟡；**SDK = stub（`unimplemented!()`）** 🔴。
   - 形式化/质量：organ_kani ✅(6 crate) / bridge_kani 0 🟡；CI 由 5 重升 7 重全绿 ✅；
     R11 baseline 3 值 LOCKED（0.8682 / 0.8532 / 0.9063）🔒。
3. **未实现愿景分组**：0 真实施（记忆塑形 / 元认知 / 物种与关系 / 协调与上下文 / 系统）；
   明确延期（Invest/Learning/SystemMonitor P3、WorktreeSandbox P2、ToolSynthesizer P3、
   Scheduler/Telemetry P2、DesktopActionTool、reflection→R22、SovereignControl P2、MCP 桥 P2、
   EventBusBackbone P2、KV 逐出后置）；待硬件（Vision 三件、Voice Whisper、Windows Hello 生物识别、
   RC-7 感知真 modality、P7 连续感知）；永久降级/挂账（13 键 verdict cache `RUNTIME_ENFORCED=false`、
   v1 legacy 永久维护、26 项 Lost Capabilities 按 P3-P7 恢复）；明确「不做」（器官前端视图、
   三层模型、`cognitive.critic` 并入 judge、`cognitive.reflection` 并入 self_assessment、
   planner/orchestrator/perception 不做 per-turn module）。
4. **文档矛盾与裁决（7 条）**：oracle 采纳 🟡；proactive/E7 采纳 ✅（FG 内部自相矛盾）；
   Kani 采纳 🟡；confidence 采纳 🟡；VectorIndex/Graph 采纳「均 🟡 partial」；
   SSE 采纳「真流式已打通」；`hello.rs` 语义 = **生物识别（NGC 凭据探测）**，非启动/装配。
   附带治理建议：主账 FG 仍标「🟢 活跃」且无过时声明 → 建议加历史批注并指向本文。
5. **v1 全量域清单（文档未列部分）**：终端沙箱 6 后端、Docker 沙箱真接（冻结 crate）、
   microVM FFI、网络隔离、受限 token + AppContainer、Web 前端、TUI、Python 桥、
   图编排/工作流引擎、5 阶段 pipeline、L0-L4 五层总线、MEWG 五重治理、HASH-SQL 仲裁、
   13 键 FourGates 实现、三洋葱 trait 抽象、7 种记忆 provider、跨 session token 折叠、
   7 强制 Advisor、工具三 crate、9 工具子 crate、进程 supervisor、CentralAI + 11 Skill、
   Agent 管理 + subagent、Team Lead Orchestrator、节律/cron、遥测栈、形式化验证、交付通道、
   SDK、配置/状态/i18n/扩展/限流基建 —— **v2 全部未实现（除注明外）**。
   补充判断：v1 的**治理与工具链其实最实**，**自我改进类最「愿景化」**；v1 的器官层大量是
   「确定性规则完整 + LLM trait 口留而未接 + stub/0 装 PASS」，故「v1 有而 v2 没有」的账须按
   IMPLEMENTED 层级理解，不等于「v1 已达成、v2 退步」。
6. **源码级复核（§7，含逐条四审/五审/六审）**：
   - **§7.1 库级已实现未接生产**（原 14 项 + 二次/三次复核）：partner、principles、
     diary/daily_summary/cross_diary、consolidation/dreaming、memory_injection、meta_thinking、
     intent_brier/confidence/calibration、reflexion、topic_predictor/proactive_recall、morphology、
     education、worktree_sandbox、BM25 混合检索、吸收批 4 件 —— **接线状态实测**：
     绝大多数在 runtime-assembly / cli 中引用数为 0 ⇒ IMPLEMENTED ✅ / PRODUCTION WIRED ❌；
     `context_rot` = 3 处引用（WIRED + DEFAULT ENABLED）；`proactive_recall` = 13 处
     （WIRED 但 DEFAULT 关闭，且**全仓无 `APEIRETH_*` 旋钮可开**）。
     **W2 线终局（2026-10-10）**：接线 6 项（dreaming/diary、partner、morphology、education、
     吸收批 4 件）均 WIRED + DEFAULT OFF + 带逃生门；`principles` **有意不接**（P2 生产接线禁令
     自证）；`worktree_sandbox` 接好线等主角；`topic_predictor`、`intent_brier/confidence/calibration`
     维持未接（归守夜人批）。
   - **§7.2 仍为真缺口（legacy 源码级核验后的修正）**：community（真缺口成立，v1 真有社群检测 +
     确定性摘要 + 测试）、experiment_field（v1 机制真、真 VM 执行后端 0 装）、真文件/网络沙箱
     （v1 只有骨架 + 冻结区整 crate 真接 Docker daemon 的混合结论）、SDK 真 HTTP·WS、三洋葱 L3-L5
     （v1 整 crate 真 + 形式化证明，v2 只有数据模型 + hex 占位）。
     **反例两条**：`HybridCognitiveRouter`、`ToolSynthesizer` —— **v1 亦无，全树零命中 = 纯愿景项**；
     `thought_cluster` —— v1 真有，v2 `cluster_store.rs` 是完整改名实现，**从缺口清单移出**。
   - **§7.3 对结论的修正**：§3.1「0 真实施」应分两级读（真缺口 vs 库级已实现未接线）；
     主账不仅缺历史批注，其 🔴 清单还**低估 v2**；**0 装纪律双向性**（审计必须源码实证）。
     **六审收官（2026-10-10）**：W1/W2/W3/W5 四线闭环（16 批，台账 #45–#64），
     **剩余真缺口 = 0**；剩余工作均为需外部条件或专门批次的中件。
7. **附录 A（v2 逐 crate 源码清单，2026-10-06 复核）**：18 crate 职责与公开入口、逐域能力状态
   （真实现 / 部分 / stub）、测试形态（92 个 `tests/*.rs` 文件、`#[ignore]` 47 处全集中于
   需真 key / 真硬件的 E2E）、源码内部不一致（`plugin/src/organ.rs` 的 9-organ 状态表是陈旧遗留，
   engine 侧为权威）。
8. **§8.5 本节自我限制**：只覆盖 `crates/` 现役 crate；「真实现」= 库级代码路径存在且自洽，
   真机点击流验证以 [live-verification-ledger.md](../live-verification-ledger.md) 为准；
   行号取自 2026-10-06 工作树，检索以符号名为准。

### 现状复核（2026-10-23）

- **已过时（crate 规模口径）**：文中「v2 = 18 crate」「v1 = 105 crate」是 2026-10-06 口径。
  现在的工作区规模口径出自 [AGENTS.md](../../../AGENTS.md) 与
  [contract.toml](../../architecture/contract.toml)（workspace 30 members = 29 crates + bench harness；
  2026-10-20 R4b 归档 4 孤岛后由 31→30；历史 31 / 20 members 口径见 AGENTS.md §3）。
- **已过时（§7.1 未接线清单）**：W2 接线工作线 2026-10-10 已全批处置完毕，原表多数项已升级；
  §7.1 表格请按「历史口径（2026-10-06）」读。
- **已过时（§7.2 真缺口）**：`community`、`onering`、`experiment_field`、三洋葱 L3-L5、
  SDK HTTP 面、真文件/网络隔离在 2026-10-10 前后陆续落地；剩余项的当前台账 =
  [system-gap-ledger-2026-10-18.md](../backend-enhancement/system-gap-ledger-2026-10-18.md)。
- **仍成立（方法论与双向性教训）**：「引用数为 0」必须脚本全量扫；`Option<...>` 配置项要连
  `Default` 一起看；「crate 内部互调也是生产路径」；0 装纪律双向性四条教训，均未失效。
- **仍成立（四级口径定义）**：见 [system-capabilities.md](../../01-architecture/system-capabilities.md) 头部。
- **已闭环（治理建议）**：主账 FG 已加 2026-10-06 历史批注并降级为历史记录，原审计 §4 的建议已执行。
- **部分过时（slot 账本）**：12 slot 口径已被 13 槽（含器官槽）与偏好学习接线更新，
  现行逐槽状态见活账本 [cognitive-module-wiring.md](../cognitive-module-wiring.md)（**保留件，不合并**）。
- **未核实**：`§8.3` 的 `#[ignore]` 计数（47 处）与「92 个测试文件」是 2026-10-06 计数，本批未复算。

---

## 2. W3 九缺口 legacy 考古（2026-10-10）

**来源**：`docs/04-internal/w3-legacy-gap-archaeology-2026-10-10.md`
**方法**：逐个读 v1 真代码（`legacy/donor/...`），不用文件名下结论；每条主张带 `path:line`；
判定三关法（代码内 / 归档区 / 测试存在性都要扫）。

### 结论（原文要点保留）

**一分钟结论表（九项）**

| # | 缺口 | v1 真的做成了吗 | v2 现状（2026-10-10） | 工作量 | 建议序 |
|---|---|---|---|---|---|
| 5 | `thought_cluster` 思维簇 | ✅ 真，且 **v2 已实现**（`cluster_store.rs` 改名件） | 库级缺口闭合，欠生产接线 | S | 1 |
| 1 | `community` 社群识别与分诊 | ✅ 真（检测算法 + 确定性摘要 + 测试） | ✅ 已实现（`memory/community.rs` 三件 + 9 测，台账 #57）+ ✅ 生产消费已接线（all_facts 契约扩展 + 检索前置，台账 #64） | M | 2 |
| 6 | `onering` 账本 | ✅ 真（318 行 + 8 测试） | ✅ 本体早已在 v2（`context_ledger.rs` = companion ledger 打捞，API 同构）+ 补生产消费（台账 #58） | M | 3 |
| 2 | `experiment_field` 隔离实验场 | 🟡 机制真、真 VM 执行后端 0 装 | ✅ 机制已实现（状态机 + VMRunner 口 + Noop 诚实 Err + 回滚学习 sink，台账 #59） | M | 4 |
| 7 | 真文件/网络沙箱 | ❌ v1 也只有骨架（seccomp/JobObject/netns/WFP 全是 TODO/Noop） | v2 进程树遏制已反超 v1 | L | 5 |
| 9 | 三洋葱 L3-L5 | 🟡 层模型 + 判定真、无物理执行面 | v2 有数据模型 + hex 占位签名 | L | 6 |
| 3 | `HybridCognitiveRouter` | ❌ 不存在 | 无 | XL | 8 |
| 4 | `ToolSynthesizer` | ❌ 不存在 | 无 | XL | 9 |
| 8 | SDK 真 HTTP·WS | （v1 有 `apeireth-api` 传输可参照） | W5 已开挖 | L | 已开工 |

**核心校准**：九项里只有 2 项是真「实现」活（community、onering）+ 1 项核查活（thought_cluster）；
2 项 v1 也没有（纯愿景新造）；沙箱 v1 同样是骨架 —— 按「实现」排期会严重低估工作量。

**逐项证据要点（保留）**
- `community`：`legacy/donor/apeireth-companion/src/community.rs` ≈300 行：
  `detect_communities`（连通分量/邻接聚合）、`deterministic_summary`、
  `CommunityBriefs`/`summarize` 双实现、测试在 `:222-286`；实现风险 = `GraphFact` 类型适配。
  落地：`crates/engine/memory/src/community.rs`（三件全落 + 9 测；风险项实测结论：v2 的
  `GraphFact` 字段与 v1 消费形态同构，**无需适配层**；防重 diff：`graph_algo::connected_components`
  是 Memory(Id) 域原语，与本模块值域共现图不同域）。
- `experiment_field`：`experiment_field.rs` 260 行 + `sandbox_pass.rs` 358 行（编译期契约守门）；
  真实部分 = 确定性状态机 + 「提案→实验→通过→主人批准→部署→监控→回滚→学习」回路 + 回滚学习写
  `ExperienceStore`；0 装部分 = `VMRunner` 默认 `NoopVMRunner` 诚实 Err、`sandbox_net.rs`/`vm_sandbox.rs`
  全是 trait + Noop。落地：`runtime-assembly/src/canonical/experiment_field.rs`（五态状态机 +
  `FailureLearningSink` → WikiEntry 集成；执行后端仍 0 装）。
- `onering`：`onering.rs` 318 行 / 8 测试（`:256-338`）；`OneRingLedger` + 自有表 `onering_messages`
  （seq 单调自增替代秒级时间戳竞争）+ prune 默认 200 可配 + 与 episodes 管线分流；
  0 假装边界自证：fuzzy diff / 时间线插入策略**明确不吸收**。
- 真沙箱：`tool-shell/src/lib.rs:25-28` 自述 seccomp/JobObject/namespaces 皆 TODO；
  `tool-shell:123` TODO 三平台；`sandbox_pass.rs:7-10` 的 `NetworkIsolation`/`VMSandbox` 全 Noop。
- 三洋葱：v1 `apeireth-onion/src/lib.rs:21` 原则 5 层 (E/S/A/M/O) + 权限 6 层 (L0-L5) + 11 节点电子环；
  `:66-79` `PermissionLayer` 全枚举；`onion_demo.rs:65-72` `unify_check` 判定活；
  但**无物理执行面**。落地：`core/src/onion_gate.rs`（层身份枚举 + 5+6=11 编译期断言 +
  `arbitrate_principles` + `ElectronicRing` + `DoubleOnionGate::unify_check` 三段门 + r177 Kani
  等价证明平移为 15 测）+ `runtime-assembly/src/canonical/onion_layer.rs` 的 `OnionLayerHook`
  （能力→权限层映射：filesystem=L1 / fetch=L2 / shell·process·MCP=L3 / 未知=L2；
  管线位点 = 授权→凭证→注入→guard 之后；旋钮 `APEIRETH_ENABLE_ONION_LAYER=1` 默认关）。
  **真 Ed25519 多签留 v2.1**（0 装占位不动）。

**建议执行序（一次一个）**：§5 核查接线 → §1 community → §6 onering → §2 experiment_field →
§7 真沙箱（W1，前置 = 主人对轻量沙箱设计拍板）→ §9 洋葱判定模型 + 执行面 → §3/§4 愿景新造（先出设计）+
§8 SDK。

**本报告没有做的事**：未逐行验证 v1 各模块运行时行为（`legacy/` 不参与构建，没跑过它的测试）；
未评估实现的具体 API 形状；§3/§4 无 v1 实现可考证；未替主人拍板顺序。

**复盘更正（§9）**：§7「v1 也只有骨架」是**过度概括**，更正为 🟡 部分 —— `legacy/frozen/apeireth-sandbox/`
整 crate 真接 Docker daemon HTTP API v1.43+（`real.rs` 1228 行；exec/kill/status/network/filesystem/
resource_limit 六 API；三 RuntimeKind；`DaemonClient` 三层实现）+ wiremock 测试 496 行 + demo 227 行；
原判把「我没扫到」写成「不存在」，违反本报告自己主张的三关判定法（**归档区也要扫**）；
工作量从「L 纯新造」修正为 **M–L 可回收实现**，与轻量沙箱设计**互补而非替代**。
§1 community「分诊在 v1 是 brief 级」低估，更正为真分诊函数（`community.rs:155` `triage` + 专项测试），
工作量判断 M 不变但实现范围应含 triage。

### 现状复核（2026-10-23）

- **已过时（九项建议序）**：九项已按序落地（community #57/#64、onering #58、experiment_field #59、
  洋葱 #55/#56、SDK HTTP #61 等），当前未接线项以
  [system-gap-ledger-2026-10-18.md](../backend-enhancement/system-gap-ledger-2026-10-18.md) 为准。
- **仍成立（两项纯愿景判定）**：`HybridCognitiveRouter`、`ToolSynthesizer` 在 v1/v2 全树零命中，
  属「愿景新造」；按「从 v1 实现」立项会扑空 —— 此判定未被推翻。
- **仍成立（沙箱校准）**：轻量沙箱设计仍是 v1 骨架以上的新造活（Windows 优先路线），
  且与 Docker daemon 方案互补；**真 Ed25519 多签仍挂 v2.1**（`onion.rs` 多签仍是 hex 占位）。
- **仍成立（判定方法）**：三关判定法（归档区也要扫）—— 本册后续各次审计都按此复核。
- **未核实**：§9.1 的 Docker 整 crate 与 §1 的 9 测断言本批未逐行复验（`legacy/` 与冻结区不在
  active 工作区构建面）。

---

## 3. R9 + R10 spec drift 审计（2026-08-28）

**来源**：`docs/04-internal/r9-r10-spec-drift-audit-2026-08-28.md`
**方法**：读 6 个文件 + 实地核验当时的 R9 spec 行号；只读取证，不 commit。

### 结论（原文要点保留）

- **R9 spec**（`docs/02-guides/v2-gateway-frontend-integration-spec.md`，当时 569 行）：
  主体已修；R13 review §1.1–1.3 标的 3 处错账（L25 / L330 / L342 / L483）**都是历史快照**，
  当时文件已与真账一致。逐节核对 30+ 条，除 HEAD 漂移外 **0 drift**。
- **R10 spec**（`docs/01-architecture/cognitive-9-organ-integration-spec.md`，当时 1001 行）：
  数字与 ledger 一致（L51 / L256 均标「6 WIRED + 6 DEFERRED」），R13 review §9.1 标的错账同为
  历史快照；**R10 spec 与 ledger 完全一致，0 drift**。
- **drift 分类**（9 条）：真错账 = 2（HEAD 漂移：R9 写时 `7d990297` → 当时 `22c6e72b`，
  80 commits ahead；R9 §9.1 的 `b9026186` 同类）；spec 演化（历史快照 OK 保留）= 6；
  未亲验 = 1（§1.1 §44 tools commit `18d6bf36`，未 grep 验证，信任 spec）；0 drift = 1。
- **主代理亲做 6 项核验**：#1 R9 4 处错账已修 ✅、#2 R10 3 处错账已修 ✅、#3 R12 working tree 能否跑通
  （当时跑中）、#4 9 organ UI 暴露范围决策（主代理亲做，默认不向用户暴露）、
  #5 主人审批 modal 行为决策（主代理亲做）、#6 桌面 keyring 决策（主代理亲做）。
- **0 装诚实标**：子代理 2 flag 的「R9 spec 错账」是依据 R13 review 的**历史快照**而非当前文件，
  属 sub-agent 调研方法偏差，**不是 R9 spec 当前错账**；不假装 OK —— 2 处 HEAD 漂移仍是真错账。

### 现状复核（2026-10-23）

- **已过时（HEAD 漂移结论）**：两处 HEAD 快照（`7d990297` / `22c6e72b`）已再次失效
  （复核日 HEAD `b4aabfc2`）。此类结论天然随提交前进失效，属「历史口径（2026-08-28）」。
- **部分过时（规格本身）**：两份规格文件仍在原路径，但 2026-08-28 之后经历多轮接线与口径修订，
  其行号级结论（0 drift / 7 处已修）不可直接复用；今日判定须以现行规格 + 活账本
  [cognitive-module-wiring.md](../cognitive-module-wiring.md) 现场核。
- **仍成立（方法论）**：**「快照 ≠ 错账」**与「子代理 flag 必须亲验当前文件」两条，是本组审计的
  通用教训（与 01 §1 的「引用数为 0 必须脚本全量扫」同族）。
- **仍成立（owner 决策位记录）**：9 organ UI 暴露范围、主人审批 modal 行为、桌面 keyring 三项
  决策位在本审计中记为「主代理亲做」——**该三项的最终裁决未在本审计内留痕**（未核实）。

---

## 4. Round 12 终极审计：调研完成度 + 缺口清单（2026-08-28）

**来源**：`docs/04-internal/round-12-final-gap-audit-2026-08-28.md`
**方法**：综合 Round 9–11 全部调研真账 + 1.0 功能全集对比 + v2 真实施现状。

### 结论（原文要点保留）

- **触发**：用户原话「研究下所有调研和 1.0 功能的现状，告诉我 Apeireth 还缺什么，调研完了吗」。
- **调研完成度**：Round 9–11 = 19 真账 doc / ~5400 行；按模块维度覆盖率自评 **100%**；
  明确列出 3 项「未调研/不可分派」：真实施前必亲验（未 clone v2 master）、10 commits ahead origin、
  新发现（v1 `context.rs` + `context_rot.rs` 同一 rot_score 重复实现；`hello.rs` 概念 collision）。
  **「调研 100% 完成，但真实施 0%」**。
- **能力三分类**：🟢 OK ~30 项（Storage 核心 / 工具核心 / Governance 5-gate + onion + PII +
  AuditHashChain + SelfDisableGuard / EventBus core / SessionManager / 协议归一化 / Provider DTO /
  Gateway 路由 + egress / 4 器官 / Round 11 四真实施）；🟡 Partial ~15 项（记忆支撑模块、
  VectorIndex+Graph、Browser、Learning+SystemMonitor、WorktreeSandbox+ToolSynthesizer、MCP 三件套、
  SovereignControl、EventBusBackbone+Scheduler+Telemetry、WsFrame/voice VAD、SSE broadcaster+MCP handler、
  DreamEngine+EpistemicHealer+PromptAssembler、TopicPredictor+PreloadChannel、memory_extractor+memory_graph、
  presence、streaming_chat+voice_session）；🔴 缺 ~25 项（分 6 类：Storage 抽象层 3、Tools+Sandbox 4~5、
  物种化核心 ~10、塑形维度 3、反思+元认知 6、协调+上下文 9）。
- **critical path**：估 3–4 周 → 修订 11–13 周（R11-CoordinationContext 最重）；
  总 release 路径 6–9 月；v2 release 估 2027-Q3（原 2027-Q1-Q2）。
- **派单顺序（10 项）**：R12-CoordinationContext 1/2/3、R12-SpeciesCore 1/2、R12-LongTermMemory、
  R12-Storage、R13-SpeciesForm、R13-MetaCognition、R13-ToolsSecurity。
- **0 装诚实标（6 项）**：主账部分状态 ❌→⚠️ 修订；v1 重复实现警告；`hello.rs` 概念 collision
  （主代理真账标错需修订）；**调研 0 实测**；R11 sub-agent 多次超 ≤300 行约束；主代理估时偏乐观。

### 现状复核（2026-10-23）

- **已过时（🔴 缺 ~25 项清单）**：其中绝大多数已由 2026-10-06 / 2026-10-10 两条审计的复核与
  后续接线批次消化（详见 01 §1 §7.1/§7.2、01 §2 九项表）。原清单按「历史口径（2026-08-28）」读。
- **已过时（规模口径）**：文中「17 crates」「86-crate v1」「~100 modules」等为当时口径；
  现值见 [AGENTS.md](../../../AGENTS.md) §3 与 [contract.toml](../../architecture/contract.toml)。
- **已过时（release 估时）**：2027-Q3 / 6–9 月 / 11–13 周的推演已不适用 —— 今天版本号已到
  `2.0.0-rc.4`（口径出处：根 `Cargo.toml` `[workspace.package] version`），架构波次推进到 Wave 5
  （`contract.toml` `current_wave = 5`）。
- **仍成立（三条纪律）**：①「调研 ≠ 真实施」；②**调研 0 实测必须如实标**（本组审计长期标 0 装）；
  ③v1 `context.rs` / `context_rot.rs` 的 rot_score 重复实现属 v1 侧资产问题，仍需在回收时融合。
- **仍成立（owner 决策位）**：education 物种化核心 spec、真 CAS 选型、派单 brief 模板等决策位
  在本审计中挂账，最终裁决未留痕于本文（未核实）。

---

## 5. Round 13 · 1.0 maturity 真账补查（2026-08-28）

**来源**：`docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md`
**方法**：主代理亲测 8 个核心 1.0 `.rs` 文件的成熟度（REAL / PARTIAL / 0 装 PASS / trait 口），
修订 Round 11/12 的估时与缺口口径。

### 结论（原文要点保留）

- **Q1「修订 release 路径是啥意思」**：= 重新估 v2 release 时间表 + critical path + 依赖计划；
  修订 4–6 月 → 6–9 月（2027-Q1-Q2 → 2027-Q3）；critical path 3–4 周 → 11–13 周；
  总进度 80% → 70–75%；必补清单 0 项 → ~23 项 P0。
- **Q2「1.0 有的东西 v2 没有吗？调研彻底吗」——用户 catch 对，主代理自省**：
  R11 6 个 sub-agent **0 实测 1.0 真代码**（仅凭路径列表 + 真账 + 推断 + 公开领域知识）；
  已逐项可实现者（实测验证）= daily_summary(99 行 REAL)、diary(442 行 REAL + 注入 trait 口)、
  cross_diary(301 行 REAL + 关联注入 trait 口)、memory_injection(66 行 REAL)、
  reflexion(497 行 REAL + 3 trait 口)、education(402 行 **PARTIAL**：字符串规则表，无 CAS 引擎，
  不宣称能解积分)、partner(141 行 REAL)；部分不完整项 = diary 注入实接线、cross_diary 关联注入、
  reflexion 的 LLM 版 CRITIC / 失败事件实接线 / 注入块消费侧、education 真 CAS。
- **1.0 maturity 补查汇总（~35 项）**：~28 项 REAL（逐项可实现）、~4 项 REAL with trait 口、
  ~5 项 PARTIAL（0 装诚实标）、~10 项 0 装 PASS 标注。逐项表含行数、成熟度、
  1.0 真实施内容与 2.0 现状（Storage 抽象层 2 项 / 长期记忆塑形 6 项 / 物种化核心 4 项 /
  塑形维度 3 项 / 反思元认知 6 项 / 协调上下文 15 项）。
- **修订估时**：总 critical path 11–13 周 → **12–14 周**（+ 主代理亲做 spec ~2 周）。
- **主代理亲做清单（10 项）**：rot_score 融合、consolidation/reflection writeback trait spec、
  `hello.rs` 主题确认、**education 真 CAS spec**、**confidence BetaBinomial trait spec**、
  **reflexion 3 trait 口实接线 spec**、6 真实施派单 brief 模板、clone v2 master 真对照、
  修订 release 路径、修订主代理估时。
- **0 装诚实标（4 项）**：R11 sub-agent 0 实测 1.0 真代码；主代理 §3.1 估时偏乐观；
  R11 真账 ~25 项 → ~23 项修订；R11 §1.8 部分状态 ❌ → ⚠️ partial 修订。

### 现状复核（2026-10-23）

- **已过时（成熟度矩阵整体）**：2026-08-28 的 REAL/PARTIAL 判定对象是 v1 `legacy/` 源码，
  而当时 v2 侧结论已被 2026-10-06 / 10-10 复核大量推翻（多数 v1 项 v2 已库级实现并陆续接线）。
  本表只能作「v1 资产清单」读，**不能**作今日能力判定。
- **已过时（估时与 release 路径）**：12–14 周 / 6–9 月 / 2027-Q3 的推演已被后续批次消化
  （版本 `2.0.0-rc.4`，架构波次 Wave 5）。
- **仍成立（方法论教训，最有价值的一条）**：**「R11 6 sub-agent 0 实测 1.0 真代码」是 O-5 失守**；
  任何「1.0 有 / 2.0 无」的断言都必须逐文件打开源码确认，不能凭名称级扫描 —— 这条直接催生了
  2026-10-10 的 legacy 考古（01 §2）与 2026-10-06 的四审（01 §1 §7.2）。
- **仍成立（owner 决策位）**：education 真 CAS 选型、confidence BetaBinomial trait、
  reflexion 3 trait 口接线 —— 三项在本审计挂「主代理亲做」，其最终落地口径见
  [live-verification-ledger.md](../live-verification-ledger.md) 与活台账（未在本审计内留痕；
  本批未逐项核实 → 标「未核实」）。

---

## 6. 功能旋钮四方对账审计（2026-09-12）

**来源**：`docs/04-internal/capability-knob-audit-2026-09-12.md`
**方法**：后端代码为权威（组合根 `crates/adapters/cli/src/lib.rs` + governance + tools plugin），
桌面注入层与设置页 UI 为第二方，docs 现行文档由三路子代理精读 + 主代理精读；只记事实与矛盾。

### 结论（原文要点保留）

**四方对账总表（后端实际语义 / 默认 / 桌面旋钮 / 文档口径 / 判定）**

| 能力·旋钮 | 后端实际语义 | 后端默认 | 桌面旋钮 | 判定 |
|---|---|---|---|---|
| `tool.repo` | 恒 grant | ✅ 可用 | 无需 | ✅ 一致 |
| `tool.filesystem` / `tool.search` | 默认注册但治理层**默认 Deny**，需 `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` 才 grant | ❌ 拒绝 | ❌ **无旋钮** | ❌ **代码违文档意图；桌面永远用不了** |
| `tool.shell` | 旋钮开 → 注册 + grant + 每次调用 `require_approval_for` | 关 | ✅ enable_shell | ✅ 一致 |
| `tool.fetch` | 旋钮开 → 注册 + grant + 每次审批；受控 egress 默认 PublicInternetOnly | 关 | ✅ enable_fetch | ✅ 一致 |
| `cognitive.organs` | `APEIRETH_ENABLE_ORGANS=1` → OrganModule（fail-open） | 关 | ✅ enable_organs | ✅ 一致 |
| `cognitive.judge` / `cognitive.council` | 各自 `APEIRETH_COGNITIVE_*=1`（council ≤7 advisor，10s/60s） | 关 | ✅ 各一 | ✅ 一致 |
| `cognitive.preference_learning` | `APEIRETH_ENABLE_PREFERENCE_LEARNING=1`（2026-09-08 已接线） | 关 | ✅ preferenceLearning | ✅ 一致 |
| memory_recall / memory_writeback / self_assessment / Experience 提炼 | 恒开（backends 恒 Some，无旋钮） | ✅ 开 | 无需 | ✅ 一致 |
| 3 治理 hook（权限 / 凭据披露 / 提示注入） | 生产恒挂 | ✅ | 无需 | ✅ 一致 |
| 敏感路径保护（.env/.ssh/.aws/.gnupg/.secret 屏蔽） | 恒开 | ✅ | 无需 | ✅ 一致 |
| 会话级权限预设 read_only/standard/full | 已实现（PermissionPresetGovernanceHook 接生产管线） | standard | ✅ 会话内选择器 | ✅ 一致 |
| 设置页「全局默认预设」 | **未接会话创建**（仅 localStorage） | — | ✅ 下拉（无实效） | ⚠️ UI 有旋钮后端不生效 |
| trust tier 信任分级 | **缺失** | — | ❌ | ⚠️ 文档设计了未实现 |
| 审批频率限制 / 免审批黑名单 | **缺失** | — | ❌ | ⚠️ 同上 |
| 审批策略档（每次 / 会话内记住 / 完全放行） | **无**（shell/fetch 写死每次审批） | 每次审批 | ❌ | ⚠️ 参考文档建议未实现 |
| 会话创建时快照冻结旋钮 | **无**（旋钮全局即时生效） | — | ❌ | ⚠️ 同上 |

**组合根事实（5 条）**：工具层（filesystem/search/repo 恒注册；shell/fetch 需显式 options）；
治理层（恒 grant `tool.repo`；`APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` 才 grant filesystem/search；
shell/fetch = grant + 每次审批；未知 capability default-deny）；认知模块（四旋钮默认关，
记忆/自评/经验提炼恒开且默认无额外模型成本）；Council 用独立 LLM factory
（openai-compatible → 另一家 → Noop 回退）；审批生命周期 TTL 5 分钟、at-most-once、
Deny ≠ RequireApproval（三态）。

**关键文档矛盾（7 条）**：①只读工具口径漂移（文档说「默认注册 3 个只读工具」，实际永远 Deny）；
②某 provider 的 key 变量名两套；③crate 数 13/16/17 并存（同一文件内两值）；
④canonical 网关路由口径相反（8/30「仅 5 路由」vs 9/4「全部已实现」）；
⑤`cognitive.self_assessment` 默认态（「OFF by default」vs「WIRED Judge-backed」）；
⑥L0 是否已硬实现；⑦其余 20+ 处（v1 遗留 / 时间演进）。

**整改批状态（2026-09-12，同批已落地 5 项）**：①只读工具默认可用（默认 grant 与 repo 同待遇，
`APEIRETH_DISABLE_LOCAL_READ_TOOLS=1` 为隐私逃生门，旧 ENABLE 兼容且 DISABLE 赢 fail-closed，
4 个新测试覆盖语义矩阵）；②全局权限预设接入会话创建（新建会话/分支时读 localStorage 默认预设，
send() 后静默 PATCH，404 静默重试）；③审批策略预设档（`SessionSettings.approval_remember` +
`GovernanceHook::approval_resolved` 回调 + 进程内记忆；standard+remember 把同（会话,工具）的
RequireApproval 改 Allow，**Deny 永不绕过**；桌面会话头 4 档选择器）；④认知深度预设档
（轻量/平衡/深度/自定义 映射 judge/council，文案注明延迟与 token 成本）；⑤文档对账修复
（key 名统一、crate 数 13→17、canonical 路由口径、self_assessment 默认开、L0 诚实口径）。

### 现状复核（2026-10-23）

- **已过时（只读工具默认 Deny）**：`production.rs` 的默认配置现为
  `filesystem: true, search: true, repo: true, shell: None, fetch: None`
  （`crates/engine/runtime-assembly/src/canonical/production.rs:456-460`）—— 只读工具已默认放行，
  shell/fetch 仍默认关（opt-in）。原文「桌面永远用不了」**已过时**。
- **已过时（crate 数 13/16/17 并存）**：现值见 [AGENTS.md](../../../AGENTS.md) §3（30 members）与
  [contract.toml](../../architecture/contract.toml)；文档侧当时的「13→17」是 2026-09-12 的中间口径。
- **已过时（trust tier 缺失）**：信任分级已存在（`crates/foundation/governance/src/rate_limit.rs`
  的 `TrustTier` + `RateLimitGovernanceHook`，经 `governance/src/lib.rs:107` 导出）。
- **仍成立（默认关矩阵）**：judge / council / preference_learning / organs 与 shell / fetch 默认关，
  是**旋钮命名即承诺**的现行口径；默认开关实况以 `canonical/production.rs` 与
  [capabilities-matrix.md](../../03-reference/capabilities-matrix.md) §0.2 为准。
- **仍成立（审批三态与 Deny 不可绕过）**：`Deny ≠ RequireApproval`、Deny 永不绕过、
  approval TTL 与 at-most-once 语义未见变更。
- **未核实**：整改批 5 项的**测试与 commit 追溯**本批未逐条核对（仅按仓内现状推断）。

---

## 7. Sub-Agent 审计报告 Round 4（2026-08-28）

**来源**：`docs/04-internal/sub-agent-audit-round-4-2026-08-28.md`
**顶部批注（2026-09-05 对账批）**：本文为历史记录，写作时数字属实于当时
（当时实测基线：17 crates / 3120 passed / 0 failed / 13 ignored / workspace.version 2.0.0-rc.1）。

**方法**：sub-agent 审 6 个 commit 是否偏离哲学 → 主代理逐条亲验并修订 → 处理真账（amend 改 author）。

### 结论（原文要点保留）

- **sub-agent 总判**：CONDITIONAL PASS；逐 commit 表 6 条（`1d885299` / `5e18e65b` / `e3300347` /
  `ffa5ab4c` / `4b698a26` / `380e7f13`）：5 PASS + 1 CONDITIONAL（`5e18e65b` msg 改了但 tree 没改）。
- **findings 2 条**：#1 amend 没 `git add` → write-tree 用 `HEAD^{tree}` → msg 改了 tree 旧（已真修）；
  #2 `5e18e65b` msg 缺 O-6 三阶审查（建议不补，改为下次的 hook 强制）。
- **LOCKED 5 项 0 触碰**：实测 `git diff main~6..main --stat -- crates/` = 0 行；
  9 哲学锚 enum 实测 9 variants。
- **主代理亲验修订 3 处（sub-agent 误判）**：
  ① **Author 失守（CRITICAL，sub-agent 未 flag）**：4 个 commit 的 author 实际是 git config 默认值
  （非 sub-agent 派活分工），根因 = `git commit -F <file>` 漏设 `GIT_AUTHOR_NAME` env var；
  处置 = git plumbing 重做 4 commits + `update-ref` + `force push --force-with-lease`，
  自验 tree MATCH / msg 语义 MATCH / author 全 Mavis / `cargo check` 0 副作用；
  ② `5e18e65b` msg **实际有**完整三阶审查 + 拒方案（sub-agent grep 没找到），不补；
  ③ push 状态**已通**（`git rev-list --left-right --count origin/main...HEAD` = 0 0），sub-agent 报告过时。
- **哲学锚兑现**：S-2（3 处误判全 catch）、O-1（0 触碰 P0 governance 3 hook）、O-3（1 步 1 文档）、
  O-5（失守即 flag 即改）、O-6（选 amend 而非 followup 凑合）。
- **留 backlog 5 项**：§8.2 commit 模板 + §8.5 amend 强制（加 pre-commit hook 或 doc lint）；
  SDK 内 `const _SIX_PHILOSOPHY_ANCHORS` 命名与当前 9 锚不一致；
  `orchestration/src/lib.rs` 「8 哲学锚」注释漂移；`organ-orchestrator-completion-plan.md` 等三处
  「R11 baseline path」路径错；MANIFESTO §4 vs §10「X 重守门」cross-ref 混用。
- **工序教训新增第 12 条**：`git commit -F file` 漏设 `GIT_AUTHOR_NAME` env var 的症状与修法。
- **与 handoff log 关系**：handoff log 记 Round 1–3，本文件记 Round 4，两者互补。

### 现状复核（2026-10-23）

- **已过时（基线数字）**：17 crates / 3120 passed / 13 ignored / version 2.0.0-rc.1 —— 今日版本
  `2.0.0-rc.4`，规模口径见 [AGENTS.md](../../../AGENTS.md) §3，测试数字见
  [test-baseline.md](../../03-reference/test-baseline.md)。
- **已过时（哲学锚数）**：当时 9 锚 → 现在 [philosophy.md](../../01-architecture/philosophy.md)
  标题为「The Nine Anchors」，且 O-1 描述已写「键集（现行 14 键）」（键集口径在 2026-10-11 后为 14 键，
  历史「13 键」为更早口径，见 ADR-0006）。
- **仍成立（工序教训第 12 条）**：`GIT_AUTHOR_NAME` env var 漏设的失守与修法仍有效；
  同族纪律见 [CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §二/§三。
- **仍成立（0 触碰自查）**：LOCKED 5 项 0 触碰 + sub-agent 报告必亲验，是本组审计的通用做法。
- **部分过时（留 backlog 5 项）**：第 1 项（commit 模板 + amend 强制）已由现行
  `scripts/commit.ps1`（显式 author + staged secret/neutral 扫描 + 按次启用 hook）承接；
  第 3/4/5 项涉及的具体文件与行号已随多轮重构漂移 —— 本批**未逐项核实**其现状。
- **未核实**：Round 4 的 6 个 commit 现已不在近期历史中，原 commit 短哈希是否仍可达本批未查。

---

## 附：本册与其它册的交叉点

- **活账本（不合并）**：[cognitive-module-wiring.md](../cognitive-module-wiring.md) —— 逐槽接线状态真源。
- **未接线台账（不合并）**：[system-gap-ledger-2026-10-18.md](../backend-enhancement/system-gap-ledger-2026-10-18.md)。
- **能力声明真源（不抄写）**：[claims.yaml](../../status/claims.yaml) —— 逐 claim 四级口径与 `verified_at`。
- **架构契约真源（不抄写）**：[contract.toml](../../architecture/contract.toml) —— 依赖边 / 开关 / 波次。
- **安全与治理侧结论**：见 [02-security-and-governance-audit.md](02-security-and-governance-audit.md)。
- **口径与文本可信度侧结论**：见 [03-caliber-and-text-hygiene-audit.md](03-caliber-and-text-hygiene-audit.md)。
