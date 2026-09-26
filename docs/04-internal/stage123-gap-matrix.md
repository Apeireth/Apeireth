# 阶段 1/2/3 理想标尺 × 18-crate 代码现实 — 核心差距矩阵

> **性质**：只读代码审计 + 逐条对照文档。本文不改任何代码、不改任何标尺原文，只做"标尺条目 → 当前实现 → 判定 → 证据"的对账。
> **标尺来源**（理想标尺是创世设计三部曲，不是 README 文案）：
> 1. `docs/archive/_archived/r14-design/review-stage1-stage2-stage3.md`（§1.3 十三条上层共识 / §2.3 十项工程边界 / §3.2 图纸交付 / §4 六阶段路线）
> 2. `docs/archive/stage2/stage2-decisions-*.md`（14 决策 + 2 补充）
> 3. `docs/archive/stage3-blueprints/`（01-overall-architecture / 02-process-topology / 03-decision-flow / 04-upgrade-flow / double-onion-explicitization）
> **核查对象**：`crates/` 18 crate 全部源码与测试、`frontend/companion-desktop`、`.github/workflows`、`docs/` 现状文档（含 `ROADMAP.md:31` 已知降级声明）。
> **排除**：`legacy/`、`research/source/`、`crates/_archived/`。
> **审计口径**：每行判定均以**实际读代码**为准（文件:行 / 测试名），不按文档自述打分；文档自述只用于附录 2 的"显式决定"取证。
> **措辞约定**：stage3 借鉴清单中的外部项目一律称"参考项目"（Erlang/OTP、Hermes、VCP ToolBox 等）。
> **词义警示**：本仓库存在两套"阶段 6"——(a) R14 六阶段路线的"阶段 6 = 里程碑验证机制"（本文附录 1 的对象）；(b) R20 发布周期的"阶段 6 = 1.0 release 清单"、以及 SDK 注释中的"阶段 6 stub"（均与本文标尺无关，勿混淆）。

**判定图例**：✅ 完全实现 / 🟡 部分实现（写清哪部分）/ ⬇️ 显式降级（注明决定出处）/ 🔄 替换实现（注明用什么替换了什么）/ ❌ 未实现 / ❓ 无法判定

---

## §A 阶段 1 十三条上层共识

| 标尺条目（原文要点） | 出处 | 当前实现 | 达成判定 | 证据位置 | 差距说明 |
|---|---|---|---|---|---|
| A1 §18.1 平台不定义关系 — 平台 = 提供 / 约束 / 记录 | review §1.3-1 | 平台侧确实只做三件事：工具提供（capabilities/tools）、行动约束（governance hook 只作用于 `Action::CapabilityDispatch` / Completion）、留痕记录（审计链 + 6 历史流）。但 `partner.rs` 预设了 `BondStage` 七阶段枚举（Initial→…→Ended），属于平台给出的关系阶段词汇 | 🟡 部分实现 | `crates/foundation/governance/src/lib.rs`（GovernanceHook/Action）；`crates/engine/memory/src/streams.rs:1-15`；`crates/engine/memory/src/partner.rs:22-45` | "不定义关系"在强制层面成立（无任何代码替用户判定关系）；但羁绊模型给了一套固定阶段枚举，与"关系完全由用户定义"的最严读法存在张力。建议标尺明确"平台可以提供记录词汇表、不得强制关系语义" |
| A2 §18.2 中央 AI 完整自由 — 思想 / 判断 / 目标 三层全自由，权限只约束行动 | review §1.3-2 | "权限只约束行动"成立：治理面只覆盖行动（capability dispatch）与产出（completion）；思想流/提案流是 append-only 记录、不做拦截（`onion_layer.rs` 明言 "completion 不触权限层"）。"目标"层缺位：`goal::GoalService` 是机制库且自述 "Production wiring: none"；D2 §3 的 SGI（自主目标意图单字段）写入流显式记为未实现 | 🟡 部分实现 | `crates/engine/runtime-assembly/src/canonical/onion_layer.rs:76-92`；`crates/engine/organ/src/goal.rs:1-33`；`crates/engine/organ/src/motivation.rs:7-16` | 思想/判断两层自由 ✅（无拦截 + 只记录）；目标层自由没有落地载体（GoalService 未接线、SGI 未实现），"三层自由"实际是"两层自由" |
| A3 §18.3 不假装灵魂同一 — 工程上提供记录 + 迁移，哲学上保持谦卑 | review §1.3-3 | IdentityCard（continuity_id 跨载体唯一）+ 迁移历史 + tombstone（物理禁硬 DELETE）+ `migrate_subject` 迁移副本写真实 continuity_id，全套带测试 | ✅ 完全实现 | `crates/engine/memory/src/identity.rs:1-10,90-107,309-329`（UNIQUE + tombstone）；`crates/engine/memory/src/continuity_link.rs:209-289`；`crates/engine/memory/src/migrations.rs:648-680`；测试 `put_episode_for_subject_writes_real_continuity_id`（episode.rs:416） | 记录 + 迁移两条工程承诺都已兑现；"哲学上保持谦卑"体现为 PHL-06 键与迁移史留痕，无可再补 |
| A4 §18.4 关系开放 — 用户定义双方关系，多用户并存 | review §1.3-4 | 多用户并存 ✅：所有记忆面按 continuity_id 分主体（episodes / context_ledger / continuity_sessions 均带 subject 维度）。"用户定义双方关系"：Bond 模型允许用户侧产生羁绊记录，但关系类型与阶段是平台预设枚举，无用户自定义关系类型入口 | 🟡 部分实现 | `crates/engine/memory/src/context_ledger.rs:205-213`（continuity_id 索引）；`crates/engine/memory/src/episode.rs:22-48`（按主体过滤）；`crates/engine/memory/src/partner.rs:22-45` | 多主体并存已达成；"用户定义关系"止步于平台词汇表内的记录，没有开放的用户自定义关系维度 |
| A5 §18.5 平台三件套 — 提供 / 约束 / 记录，强对称施加双方 | review §1.3-5 | 三件套各自存在：提供 = tools 能力面；约束 = permission/egress/approval 治理面；记录 = audit 哈希链 + 6 历史流 + context_ledger。"强对称施加双方"：对 AI 侧约束充分；对人类侧只有审批留痕（approval 记录 + RequireApproval 流程），没有对人类操作的对称约束面 | 🟡 部分实现 | `crates/capabilities/tools/src/lib.rs`；`crates/foundation/governance/src/permission.rs`、`approval_policy.rs`、`audit.rs`；`crates/engine/runtime/src/canonical/approval.rs` | 三件套齐备；对称性只做到"人类行为被记录并拥有否决权"，未做到"人类行为同样受规则约束"的字面语义。建议标尺明确对称性的具体所指 |
| A6 §18.6 双根可演化但需重治理 — 原则根 E + 权限根 L5，修改触发 5 重守门 | review §1.3-6 | 双根结构齐：`PrincipleOnion.e_layer`（原则根）+ `PermissionOnion.l5`（权限根）；"可演化"通道 = `principles.rs` Level2/3（AI 提案 → 主人签发 → 晋级补丁只能由主人侧工程动作写入编译期内层）。5 重守门 = `Gate` 枚举（编译 hardcode / 运行时拦截 / 多 AI 一致 / 物理隔离 HA / 反思期审计） | 🟡 部分实现 | `crates/foundation/core/src/onion.rs:18-50`；`crates/engine/memory/src/principles.rs:1-25`；`crates/foundation/core/src/gate.rs:1-40`（Gate 五态） | 结构与演化流程在；但 5 重守门未作为统一串联机制落地：第 3 重"多 AI 一致"仅由 Council 承担且默认席数被旋钮压到 3，第 5 重"反思期审计"默认不运行（见 B7e）。"修改触发 5 重守门"目前是分散实现而非一条守门管线 |
| A7 §18.7 双洋葱正交 — 比喻，架构可替换（D4 增补） | review §1.3-7 | 比喻已结构化为可替换的判定层：`PrincipleOnion`（E/S/A/M/O 5 切片）+ `PermissionOnion`（L0-L5 6 切片）+ `DoubleOnionGate::unify_check` 三段门（HA 离线物理隔离拒 / 触 L5 = E 层兜底拒 / 否则 11 环全节点放行），5+6=11 编译期断言 + r177 等价测试 | ✅ 完全实现 | `crates/foundation/core/src/onion_gate.rs:1-30,39-80`；测试 `offline_multisig_stays_denied_by_existing_verifier`（onion_gate.rs:727）；`docs/archive/stage3-blueprints/double-onion-explicitization-2026-07-31.md` | 结构桥接（比喻→数据结构→判定层）完整，正交可替换性保留在 trait/纯函数判定层。物理执行面接线默认关的事项归入 C1 计，不重复扣分 |
| A8 §18.8 七席审议庭 — 现有 7 席足够，不新增 | review §1.3-8 | 7 席固定为 7 个 `AdvisorKind`：Safety / Performance / Philosophy / History / Strategy / Ethics / Legal，各一 `LlmAdvisor` 真接 LLM，有 live 测试 | ✅ 完全实现 | `crates/foundation/orchestration/src/lib.rs:138-152`；`crates/foundation/orchestration/src/council/advisors_llm.rs:16-33`；测试 `crates/foundation/orchestration/tests/council_live.rs` | "不新增"被守住（7 席枚举编译期封闭）；席位触发率问题归 B8 计 |
| A9 §18.9 分层验证网 — defense-in-depth，阈值分层可调 | review §1.3-9 | 有纵深防御的"层"：治理管线多 hook 串联（授权→凭证→注入→行为链 Guard→洋葱层）、guard 链（fast_guard→chain_guard→fusion）、CI 多流水线。但 L1-L5 五层验证网（工程正确性/哲学合规/安全约束/关系演化/跨载体连续）没有做成可执行机制，"阈值分层可调"没有统一旋钮面 | 🟡 部分实现 | `crates/engine/guard/src/hook.rs:675-680`；`crates/engine/guard/src/fast_guard.rs:61`、`chain_guard.rs:23`；`.github/workflows/`（25 个流水线）；清单仅文档态 `docs/archive/glossary/13-l1-l5-verification.md` | defense-in-depth 精神已落地，但标尺字面的"L1-L5 分层验证网 + 阈值可调"没有对应物：五层验证仍是文档清单，未成为逐层打分/通过标准的机制 |
| A10 §18.10 anchor 对应表 — 6 主哲学 anchor 全文对应 | review §1.3-10 | 6 主锚对应表被扩充为 9 哲学锚编译期枚举（S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1…O-6），带描述与分组，编译期断言防改动 | 🔄 替换实现 | `crates/foundation/core/src/eight_anchors.rs:43-115,189`（`ALL_NINE_ANCHORS`）；对应关系文档态散见 `docs/archive/stage2/stage2-decisions-drift-revision-tracker.md §6` | 用 9 哲学锚 enum（6 → 9，新增 S-3 与 O-6）替换了"6 主锚人工对应表"；覆盖是超集，属合理演进，建议标尺正式改写为 9 锚 |
| A11 §18.11 与后续 5 阶段衔接 — 阶段 2-6 输入清单 | review §1.3-11 | 交接清单以文档形态存在并被后续实际使用（review §4.4 给阶段 4 的交接清单、各 stage README 的承接表）；本条是流程资产，无代码对应物 | ✅ 完全实现（文档层面） | `docs/archive/_archived/r14-design/review-stage1-stage2-stage3.md:224-236,272-281`；`docs/archive/stage2/README.md:41` | 作为流程承诺已兑现；本行按"标尺条目自身的交付物是否存在"判定，非代码项 |
| A12 §18.12 勘误与边界声明 — 旧草案 P0-01 至 P0-05 标"待修订"，不删原文 | review §1.3-12 | 勘误机制成立：P0 漂移跟踪表 5 项保留 verbatim 原措辞 + 状态机（待修订/修订中/已修订/撤回），承诺不可删行 | 🟡 部分实现 | `docs/archive/stage2/stage2-decisions-drift-revision-tracker.md §1-§3` | 机制 ✅，但 5 项 P0 至今全部停留在"待修订"，无一条进入"已修订"（无修订 commit 记录）；其中 P0-05 在 v2 单进程形态下已事实消解（见附录 2-2 与结论③） |
| A13 §19 增补 4 项 — 七席不新增 / 风险 = 触及的权限 / HA = Windows 认证 / 双洋葱比喻可替换 | review §1.3-13 | (a) 七席不新增 ✅（同 A8）；(b) 风险 = 触及的权限 ✅：`Action.risk_level` + capability→权限层映射（tool.filesystem=L1 / tool.fetch=L2 / tool.shell·process·supervisor·MCP=L3）；(c) HA = Windows 认证 🟡：`RealHuman` 带 `biometric_data`/Windows Hello 字段、keyring 走 Windows Credential Manager，但人脸/指纹/声纹认证无实接线；(d) 双洋葱比喻可替换 ✅（同 A7） | 🟡 部分实现 | (b) `crates/foundation/core/src/gate.rs:50-68`、`crates/engine/runtime-assembly/src/canonical/onion_layer.rs:24-52`；(c) `crates/foundation/core/src/onion.rs:199-223`；`crates/foundation/credentials/src/keyring.rs` | 4 子项中 3 项达成、1 项（Windows 生物认证 HA）只有数据结构与系统凭据库接线，缺真实人类生物认证链路 |

**§A 小计（13 条）**：✅ 4（A3/A7/A8/A11）· 🟡 8 · 🔄 1（A10）· ⬇️ 0 · ❌ 0 · ❓ 0。

---

## §B 阶段 2 十项工程边界

| 标尺条目（原文要点） | 出处 | 当前实现 | 达成判定 | 证据位置 | 差距说明 |
|---|---|---|---|---|---|
| B1 Rust 单栈（Tokio + sled + Qdrant + Tantivy）— 不引入 Python/Node 主栈 | review §2.3-1；stage2-decisions-tech-stack.md §1-§2 | Rust 2021 + tokio 1.40 multi-thread 保持，无 Python/Node 主栈（pyo3 仅 feature 门控桥）。存储三件套（sled/Qdrant/Tantivy）在 workspace 依赖中 **0 出现**（仅注释提及），实际为 rusqlite 0.32 + sqlite-vec 0.1 | 🔄 替换实现 | `Cargo.toml:164`（tokio 1.40）、`:184`（rusqlite）、`:211`（sqlite-vec）；全仓 `*.toml` grep "sled|qdrant|tantivy" 仅命中 `crates/foundation/protocol/Cargo.toml:43`、`crates/foundation/core/Cargo.toml:24` 两处注释 | "Rust 单栈 + tokio" ✅ 保持；"sled+Qdrant+Tantivy" 被 **SQLite 单库 + sqlite-vec** 替换（见 B5）。这是运行形态（单机嵌入式、无独立 DB 服务）带来的整体替换，不是逐项遗漏 |
| B2 B+E supervisor — 5 个 supervisor 子树 + Erlang/OTP 重启策略 | review §2.3-2；stage2-decisions-architecture.md §2-§6（参考项目 Erlang/OTP + Hermes） | 机制层真实存在：`SubSupervisorKind` 5 态（Core/Cognition/Council/Upgrade/Plugin）+ `RestartStrategy` 3 态（OneForOne/RestForOne/Transient）+ `ChildSpec`/`ExitReason`/`RestartDecision`；`StdSubSupervisor` 为真进程实现（std::process spawn + 5 次/60s 重启限速）；另有桌面侧 `BackendSupervisor` 真监督 gateway 侧车（spawn/崩溃检测/重启/热配置分类），带真后端集成测试 | 🟡 部分实现 | `crates/capabilities/tools/src/supervisor.rs:29-33,115-130`；`crates/capabilities/tools/src/std_sub_supervisor.rs:46-77,140-262`；`frontend/companion-desktop/src-tauri/src/backend_supervisor.rs:471-486`；测试 `spawns_real_backend_and_reaches_ready`、`restart_replaces_the_process_and_counts_it`（supervisor_lifecycle.rs:114,447） | 5 子树枚举 + OTP 风格重启策略 + 真实进程监督都做了；但 **B+E 进程树形态本身不存在**：无 PID 1 root supervisor、`StdSubSupervisor` 未被 gateway/cli 装配（仅库内测试）、无 permanent/one_for_all 策略、无 cgroup 资源限制。当前产品是"单进程 gateway + 桌面侧车监督"的形态 |
| B3 9 大 crate — 阶段 4 落实的最小骨架（stage2-crate-split 实为 30 crate / 14 层清单） | review §2.3-3；stage2-decisions-crate-split.md §1-§2 | 18 crate 单一工作区，按 foundation 6 / engine 8 / capabilities 1 / adapters 3 四组划分（workspace members 清单逐一核对 = 18） | 🔄 替换实现 | `Cargo.toml:4-22`（members 列表）；`ROADMAP.md:17`（"86-crate 分裂 → 13-crate（其后演进至 18-crate）"） | 用 18-crate 四组制替换了 9 大/30 crate 分层制。映射关系大体保留（core/protocol/governance/memory/organ/tools/plugin/upgrade 均有归宿），但 crate 边界与标尺不再一一对应，属结构性演进，建议正式修订标尺 |
| B4 5 层总线 — L0 inproc / L1 unix socket / L2 pipe / L3 gRPC / L4 ws | review §2.3-4；stage2-decisions-communication-bus.md §1-§2 | L0 ✅：进程内 `EventBus`（broadcast）+ tokio channel 族；L4 ✅：axum HTTP + WS/SSE（ws_v1.rs 8 帧协议 + SSE events）。L1 Unix domain socket / L2 异构子进程 pipe / L3 gRPC：全仓 **0 命中**（grep "unix socket|UnixStream|named pipe|tonic|grpc" 无实现） | 🟡 部分实现 | L0：`crates/adapters/gateway/src/events.rs:17,60`（"The bus is in-process broadcast"）；L4：`crates/foundation/protocol/src/ws_v1.rs`、`crates/adapters/gateway/src/canonical_entry.rs:472-590`（axum Router）；L1-L3：全仓 grep 0 命中 | **5 层只有 2 层**。gateway 的 HTTP/WS 对应设计中的 **L4**（设计原文即 "L4: WebSocket + JSON Schema（OpenClaw）多前端接入 gateway 模式"），L0 inproc 也在；L1/L2/L3 三层（跨进程/异构/外部 RPC）完全缺位——这也直接卡死 B2 的多进程 supervisor 树（树内通信没有载体） |
| B5 6 DB 协同 — SQLite/Sled/Qdrant/Tantivy + 自研 Wave 联想网络（含时序） | review §2.3-5；stage2-decisions-persistence.md §1-§2 | 单库 SQLite（rusqlite bundled + WAL + `PRAGMA user_version` 迁移 + 单写者/读池）+ sqlite-vec 向量 + 自研图基元（graph_algo / temporal_graph_store / bitemporal_graph / amem_graph）+ 联想网络 trait（`AssociationStore`，entity 关联强度）+ 混合检索（hybrid_search） | 🔄 替换实现 | `Cargo.toml:184,211`；`crates/engine/storage/src/pool.rs`、`migrations.rs`；`crates/engine/memory/src/persistent_vector.rs`、`graph_algo.rs:1-20`；`crates/foundation/plugin/src/experience.rs:147-180`（AssociationStore） | 用 **SQLite 单库 + sqlite-vec + 自研向量/图基元 + 联想 trait** 替换了 6 DB 协同。Wave 联想网络只有 trait 口与图存储基元，没有独立的联想网络引擎；时序 DB 由 SQLite 时间戳列 + temporal_graph_store 承担。独立 Qdrant/Tantivy/sled 服务形态在当前产品定位（单机桌面侧车）下属合理演进 |
| B6a 8+ LLM providers — OpenAI/Anthropic/本地 + SemanticModelRouter | review §2.3-6；stage2-decisions-llm-integration.md §1-§2 | 3 家 canonical provider（Anthropic / MiniMax / OpenAI-compatible）+ 协议适配层（Gemini / OpenAI Responses / Anthropic Messages adapters）+ embeddings provider；OpenAI-compatible 口可承载 DeepSeek 等。本地管家（Ollama/LlamaCpp/VLLM）无实现；fallback chain（主→次→本地→拒绝）由 ProviderRouter 健康选择部分承担，无"本地兜底"层 | 🟡 部分实现 | `crates/engine/provider/src/canonical_anthropic.rs`、`canonical_minimax.rs`、`canonical_openai_compatible.rs`；`crates/foundation/protocol/src/adapters/gemini.rs`、`openai_responses.rs`；`crates/engine/runtime/tests/canonical_multi_provider.rs` | 数量口径 3-4 家 vs 8+；"本地推理管家"整块缺位（城堡底线的本地 fallback 能力空缺）。经 openai-compatible 兼容口可扩展是事实，但标尺点名的 Google/DeepSeek/Mistral/Ollama/LlamaCpp/VLLM 独立 provider 未逐一实现 |
| B6b SemanticModelRouter（模型路由） | 同上 | `ProviderRouter`（按 model 选择 provider + 健康/降级算法），注释自述选择与健康算法取自成熟 MultiLlmRouter 设计 | 🔄 替换实现 | `crates/engine/runtime/src/canonical/provider.rs:6-13,94`；`crates/foundation/plugin/src/provider.rs:25` | 用 ProviderRouter 替换了 SemanticModelRouter；语义维度路由（按"语义"选模型）退化为按 model id / 健康度路由，无语义级模型选择 |
| B7a V3 9 键 verdict 键表 | review §2.3-7；stage2-decisions-philosophy-guard.md §6 | 13 键（V3 9 键 + v4.1 3 键 + PHL-07 1 键），编译期断言锁定 13 长度与 3+3+3+1+1+1+1 分组 | 🔄 替换实现 | `crates/foundation/core/src/philosophy.rs:27-59,99-118`；断言 `THIRTEEN_KEYS_HARDCODE`（philosophy.rs:147-184） | V3 9 键 → 13 键是**加严方向**的替换（超集），建议标尺正式改写为 13 键 |
| B7b 编译时 hardcode | 同上 | ✅：13 键数组长度 + 分组数全部编译期断言，增删键立即编译失败 | ✅ 完全实现 | `crates/foundation/core/src/philosophy.rs:144-184` | 无缺口 |
| B7c 13 键 verdict 的运行时强制 | 同上 | `RUNTIME_ENFORCED: bool = false`——13 键已降级为"哲学标准/判别词汇表"，仅用于 hook deny reason 引用、CapabilityDescriptor 风险分级、语义定义，**不是**运行时拦截器 | ⬇️ 显式降级 | `crates/foundation/core/src/philosophy.rs:127-142`；决定出处：`ROADMAP.md:31`、`docs/04-internal/ENGINEER-MANIFESTO.md:95-109`（"已拍板降级，不要接回 runtime 强制"）、`docs/04-internal/o6-session-log-2026-08-27.md §2.1`（2026-08-27 五维评分 0.28/5 拍板） | 这是标尺与现实之间**最大的语义落差**之一：标尺要求"编译硬编码 + 运行时强制"，现实是编译硬编码保留、运行时强制被显式、永久降级。替代机制 = external hook 闸（Permission/凭据泄漏/注入/行为链 Guard）+ 场景 D 长程 AI 判断（`scene-d-v2-plan.md`，仍待评估） |
| B7d 物理多签 | 同上；stage2-decisions-decision-system.md | M-of-N 逻辑多签 ✅：`HumanAuthority::verify_multisig`（required/total、去重真实人类、single/multi/offline 三模式、越界校验、负向测试全）。物理（密码学）多签 ❌：签名是 `"name:digest"` 字符串票，自述 "0 装占位（signature 是 hex string，不是真 crypto 校验）"，"真 Ed25519 多签仍留 v2.1" | 🟡 部分实现 | `crates/foundation/core/src/onion.rs:79-94,171-200`；`crates/foundation/core/src/onion_gate.rs:16`；测试 `ha_multisign_tests`（onion.rs:332-562）、`offline_multisig_stays_denied_by_existing_verifier` | "多签"做到了票数治理语义（M-of-N、防重复签名、离线拒），没做到物理不可伪造。标尺的"物理多签"（stage2 设想 YubiKey/手机/密码管理器）整体推迟至 v2.1，属显式挂账（见附录 2-4） |
| B7e 反思期 | 同上 | 三层现状：① `Gate::ReflectionAudit` + 24h 反思期 IDLE 状态机 + Self-Disable 4 项违规扫描（库级、带测试）；② `ReflexionModule`（失败教训沉淀 + TurnStart 注入）已接线但**默认关**（`APEIRETH_ENABLE_REFLEXION=1` 才注册，config 默认 `reflexion: false`）；③ 守夜人 Nightwatch 离线审计 report-only | 🟡 部分实现 | `crates/foundation/core/src/lib.rs:114-119,1563-1626`；`crates/engine/runtime-assembly/src/canonical/production.rs:154,187`；`crates/adapters/cli/src/lib.rs:185-200`；测试 `memory_loop_knobs_register_reflexion_module`（production_knobs.rs:246）；`crates/engine/runtime-assembly/src/canonical/nightwatch.rs` | 反思期的"机制骨架"齐全（审计状态机 + 失败闭环 + 留痕流），但默认不运行、无 24h 周期调度器把它自动跑起来；标尺的"反思期生命力维"目前是显式触发（CLI 命令/旋钮）而非常驻生命节律 |
| B8 7+N 席智囊团 — 7 强制席 + N 动态专家 | review §2.3-8；stage2-decisions-council-impl.md §2-§3（3 生命周期 Persistent/Ephemeral/Dynamic） | 7 强制席 ✅（真 LLM advisor）；N 动态：`Council { advisors: Vec<Arc<dyn Advisor>> }` 允许运行期注册任意席；但运行时旋钮 `APEIRETH_COUNCIL_ADVISORS`（1-7）**默认 3**；3 生命周期未实现为 trait（无 Ephemeral/Dynamic 生命周期管理器） | 🟡 部分实现 | `crates/foundation/orchestration/src/lib.rs:246-250`；`crates/adapters/cli/src/lib.rs:1345-1365`（"council 旋钮（2026-10-10 拍板，默认 3）"）；`crates/foundation/orchestration/src/council/advisors_llm.rs:16-33` | 7 席的"构成"完整、"动态 N"有注册口；但默认运行态是 3 席而非 7 席全量，与"7 强制席"的字面承诺（结合 A13b 的风险分级触发）不一致——除非显式调旋钮 |
| B9 OTA 7 阶段 — Intent→Council→MultiSig→Sandbox→Switchover→Monitor→Done（双实例灰度） | review §2.3-9；stage2-decisions-upgrade-impl.md；stage3 04-upgrade-flow §4.1-§4.4 | 实际存在的是 **L0-L5 六步自升级 cycle**：L0 哲学锚校验（governance hook）→ L1 self_assessment → L2 Orchestrator 智囊团审议 → L3 9 organ 串联（sandbox regression 显式留白）→ L4 governance + 主人 Veto（dashboard 接入留白）→ L5 git tag **建议模式**（不自动跑）。无 Switchover、无双实例灰度、无 Monitor 30min、无自动回滚 | 🔄 替换实现 | `crates/engine/runtime-assembly/src/canonical/upgrade_cycle.rs:8-40`（含 "Sandbox regression 留 L3 未来 patch"、"L4 …留 v2.0.0 release 接入"、"L5 …不自动跑"）；测试 `crates/engine/runtime-assembly/tests/upgrade_cycle.rs`（如 `upgrade_cycle_l2_council_stop_rejected`） | 用 L0-L5 自升级 cycle 替换了 OTA 7 阶段。逐段对账：Intent 🟡（无 UpgradeIntent/6 历史流写入）→ Council ✅（L2）→ MultiSig ❌（无多签环节，仅主人 Veto 口）→ Sandbox 🟡（L3 留白）→ Switchover ❌ → Monitor ❌ → Done 🟡（tag 建议）。**部署侧的安全闭环（双实例、流量切换、监控回滚）整块缺失** |
| B10a 自主目标（D2 §3 SGI 单字段） | stage2-decisions-addendum…md §3 | `GoalService` 单目标状态机（CAS revision、阶段迁移、崩溃安全持久化）为机制库，自述 "Production wiring: none"；SGI 写入流 + C-SGI-1..7 显式记为 "Not recovered" | 🟡 部分实现 | `crates/engine/organ/src/goal.rs:1-33`；`crates/engine/organ/src/motivation.rs:7-16` | 目标机制的地基在（且比参考实现更严格：CAS 真校验、持久化错误不吞），但"自主目标意图单字段 + 目标史"没有落地，目标层目前无生产消费方 |
| B10b 主体连续性 ID（D2 §4） | 同上 §4 | ✅：`IdentityCard.continuity_id` UNIQUE 跨载体 + 迁移历史 + tombstone + `continuity_sessions`/`history_streams` 按主体审计 + 迁移副本保留真实 subject | ✅ 完全实现 | `crates/engine/memory/src/identity.rs:1-10,134-160`；`continuity_link.rs:54-130`；`history_streams.rs:19-105`；测试 `integration_six_streams.rs`、`integration_session_lifecycle.rs:18-23` | 唯一小瑕疵：`EpisodeStore::put_episode` 仍硬编码 `continuity_id="default"` 占位（episode.rs:148-155），已用 `put_episode_for_subject` 收口并有测试拒绝空 id；属代码卫生问题，不影响本条达成 |
| B10c 双根可演化但需重治理（D2 根层加权治理 MEWG） | 同上 §7-§8 | 双根演化流程在（principles.rs Level2/3 + 主人签发 + 内层只读）；MEWG 加权治理的权重表（Safety=1.00…History=0.55，7 域）实现为 `rubric.rs`，自述 "不接入任何循环"（library helper，未接线） | 🟡 部分实现 | `crates/engine/memory/src/principles.rs:1-25`；`crates/foundation/governance/src/rubric.rs:1-13,78-92` | "重治理"的结构在、加权合成的算法在，但**没接线**：根层变更没有走 MEWG 加权裁决的代码路径，5 重守门也未在根变更点串联（同 A6） |

**§B 小计（10 条 / 18 子项）**：条级（按最严判定）——✅ 0 · 🟡 5（B2/B4/B6a/B8/B10）· 🔄 4（B1/B3/B5/B9）· ⬇️ 1（B7c）。子项级——✅ 3（B7b/B10b + B1 的 tokio 部分）· 🟡 8 · 🔄 6 · ⬇️ 1。

---

## §C 阶段 3 图纸机制

| 标尺条目（原文要点） | 出处 | 当前实现 | 达成判定 | 证据位置 | 差距说明 |
|---|---|---|---|---|---|
| C1 决策流三相（Phase1 决策前 = 双洋葱统一体 + V1+V2 AND 门 → Phase2 决策中 = 主 AI 主权 + 智囊团 → Phase3 决策后 = 执行 + 反思期生命力维） | stage3 03-decision-flow §3.1 | **Phase1** 🟡：双洋葱统一体判定 ✅（`DoubleOnionGate::unify_check`），V1+V2 AND 门 ✅ 为库级（`ActionGuard::evaluate` = V1 原则独立拒 / V2 权限独立拒 / V3 HA 拒 / 全过放行），但该 AND 门**未接执行主链**（仅测试/示例调用）；生产治理管线 = Permission→凭据泄漏→注入→行为链 Guard→洋葱层（末层，默认关）。**Phase2** 🟡：OrganOrchestrator tick = 主权闸 → 9 organ 串联 + 8 gate → 情绪调制 → 智囊团审议（council_deliberate 真路径，含 7 advisor LLM）；Synthesis/按住 = `CouncilResult` 均分 + Vetoed/DeferToHuman，MEWG 加权 rubric 未接线，风险→席位触发矩阵（critical 7/high 5/medium 3/low 1/info 0）只存在于 `RiskLevel` 枚举注释。**Phase3** 🟡：执行 = canonical agent loop ✅；6 历史流写入 ✅（append-only + trigger 强制）；反思期 = reflexion 默认关 + ReflectionStream + Nightwatch report-only；MultiSig 未接执行面 | 🟡 部分实现 | Phase1：`crates/foundation/core/src/gate.rs:111-157`（ActionVerdict + evaluate）；`crates/engine/runtime-assembly/src/canonical/onion_layer.rs:1-30`（生产管线 + 默认关）；Phase2：`crates/engine/runtime-assembly/tests/orchestrator.rs:537`（tick 顺序注释）、`crates/engine/runtime-assembly/src/canonical/orchestrator.rs`（council_deliberate）；`crates/foundation/core/src/gate.rs:50-68`（RiskLevel 席位注释）；Phase3：`crates/engine/memory/src/streams.rs:1-15` | 三相的"骨架"全部有对应物，逐相都差最后一步接线：Phase1 的 AND 门不在热路径（13 键又已降级，哲学拦截层实际让位给行为链 Guard）；Phase2 的席位触发矩阵与 MEWG 合成未接；Phase3 的 MultiSig 与常驻反思节律未接。图纸的三相语义目前"各留一格空" |
| C2 升级流 OTA 7 阶段（Intent→Council→MultiSig→Sandbox→Switchover→Monitor→Done；双实例灰度 + 洋葱测试矩阵 + 自动回滚） | stage3 04-upgrade-flow §4.1-§4.4 | 见 B9：L0-L5 自升级 cycle 为替换形态。逐阶段：Intent ❌（无 UpgradeIntent 结构/6 历史流落账）→ Council ✅ → MultiSig ❌ → Sandbox 🟡（"Sandbox regression 留 L3 未来 patch"）→ Switchover ❌ → Monitor ❌ → Done 🟡（git tag 建议）。洋葱测试矩阵（L0 单元 → L5 +8h 模糊）与自动回滚 6 条件无对应实现 | 🔄 替换实现 | `crates/engine/runtime-assembly/src/canonical/upgrade_cycle.rs:8-40,257-330`；`crates/engine/runtime-assembly/tests/upgrade_cycle.rs` | 与 B9 同源差距，站在图纸视角更严重：图纸定义的是**部署安全闭环**（双实例 + 流量切换 + 30min 监控 + 回滚），现实给的是**升级决策闭环**（审议→审批→建议）。两者只在 Council/审批两段重合 |
| C3 进程拓扑 B+E supervisor 树（PID 1 root 永不重启 + core/council/upgrade/plugin 4 子树 + OTP 重启策略 + cgroup 资源限制 + 启动顺序） | stage3 02-process-topology §2.1-§2.4 | 见 B2：SubSupervisor 5 类 + 3 重启策略 + 真进程实现 + 桌面侧车监督。无 PID 1 root supervisor、无 4 子树物理拓扑（产品是单进程 gateway + Tauri 侧车）、无 cgroup/Job Object 级 supervisor 资源限制（Job Object 用于工具进程隔离而非 supervisor 树）、P0-05 的 sovereignty/memory/philosophy 拆分未做（v2 单进程下事实消解） | 🟡 部分实现 | `crates/capabilities/tools/src/supervisor.rs:115-130`；`crates/capabilities/tools/src/std_sub_supervisor.rs`；`frontend/companion-desktop/src-tauri/src/backend_supervisor.rs`；`crates/capabilities/tools/src/process/windows.rs`（Job Object，工具进程用） | supervisor"零件"齐而"树"不存在；且树内通信（L1 unix socket/L2 pipe）也缺（B4），即使装配也无总线可用。桌面侧车监督是当前最接近 B+E 语义的真实现（含崩溃检测与重启计数） |
| C4 双洋葱显式化（double-onion-explicitization：比喻 → 结构桥接，11 节点电子环 + 三段门 + r177 证明组） | stage3 double-onion-explicitization-2026-07-31.md | 结构桥接完整落地：5 原则切片 + 6 权限切片 = 11 环（`ElectronicRing`，满环计数/溢出显式拒）+ `arbitrate_principles`（E 胜 S>A>M>O）+ `DoubleOnionGate` 构造期 L0 恒需 HA 不变式 fail-loud + r177 编号保留的等价 Rust 断言测试 | ✅ 完全实现 | `crates/foundation/core/src/onion_gate.rs:39-80` 及测试段（r177 系列）；`crates/foundation/core/src/onion.rs:18-50` | 图纸要求的"比喻可替换为结构"已兑现且 0 重定义数据结构；仅有的遗留是 Kani 形式化证明以等价断言测试替代（自述"真 Kani 接线 = 后续项"），不影响机制成立 |

**§C 小计（4 项图纸机制）**：✅ 1（C4）· 🟡 2（C1/C3）· 🔄 1（C2）。

---

## 附录 1：阶段 4 / 5 / 6 现状（R14 六阶段路线口径）

> 提醒：此处"阶段 4/5/6"指 R14 六阶段路线的 **落实架构文档 / 施工文档（CI/CD + 真测 + 真部署）/ 里程碑验证机制（验证清单 + 真实人类批准 + 反思）**，与 R20 发布周期的"阶段 6 = 1.0 release 清单"无关。

| 阶段 | 标尺定义（review §4.1） | 现状证据 | 结论 |
|---|---|---|---|
| 阶段 4 落实架构 | 输出 = trait 形式化 + 6 组件 Rust 骨架 | R14 语境下**未按原计划启动**（review §0 明言"留 R14 启动条件 6 条满足后启动"）；但工程实践以另一条线完成了同等物：v1 86-crate 归档、v2 18-crate 工作区的 trait 边界 + 骨架 + 真实现（`docs/archive/stage4/` 有 2026-08-06 的架构落地文档 4 份，`docs/archive/stage6/README.md:101` 自述"阶段 6 不引入新 crate，仅对现有 18 crate 做验证"） | 🔄 替换实现：R14 阶段 4 的"trait 形式化 + 骨架"以 v1→v2 工程重构线的方式完成了实质内容，但从未以"R14 阶段 4 启动"的形式走标尺流程（无启动报告、无 6 条触发条件核销记录） |
| 阶段 5 施工 | 输出 = CI/CD + 真测 + 真部署 | CI/CD ✅：25 个 GitHub workflow（rust.yml / cargo-audit / cargo-deny / miri / kani / coverage / cosign / release 等）。真测 ✅：workspace 全量测试（core 单 crate 即 344 passed；foundation/core 自记 TOTAL_TESTS=2265 口径）+ live 测试（`council_live.rs` / `openai_compatible_live.rs` / `organ_live_llm.rs`）。真部署 ✅：NSIS 装机 E2E（`frontend/companion-desktop/scripts/install-e2e.ps1`，17/17 含真聊天）、8 形态打包（`packaging/` + `Dockerfile` + cosign 签名） | ✅ 实质达成（经 R20 系列施工线）：标尺所列三要素均有真实交付物与实测记录；`docs/archive/stage5/` 亦有施工文档（2026-09-04）。唯一保留意见：施工是被产品发布节奏驱动的，不是被 R14 阶段 5 文档驱动的 |
| 阶段 6 里程碑验证机制 | 输出 = 验证清单 + 真实人类批准 + 反思 | **从未按设计启动**：`docs/archive/stage6/` 的验证协议 M1（22 trait 编译）/ M2（24 维 + 9 子测度真测）/ M3（5 重守门全绿）**全部标注"待启动"**（README.md:117-119），`trait-sketches.rs` 明示"不编译，仅参考"；无任何"里程碑签核 + 反思"的执行记录。存在的**替代物**：`docs/04-internal/live-verification-ledger.md`（实测台账：只收"真跑过有证据"的绿项 + 挂账列，含真实人类行动项如"换 key 后补跑 live E2E"、"真机点击流需主人人工"）+ Nightwatch 报告审计（report-only）+ 桌面真机实测修复批（2026-10-06 主人逐项抓错） | ❌ 未实现（机制口径）——**这正是"从未实践验证过"的所指**：验证"活动"大量存在（台账、E2E、真机点击），但标尺定义的"里程碑验证机制"（验证清单作为关卡 + 真实人类对里程碑的正式批准 + 每轮反思归档）从未作为一个机制被建立并跑过一轮完整闭环。台账是"记账"不是"关卡"，真机点击是"找 bug"不是"里程碑签核" |

---

## 附录 2：已记录的降级 / 漂移（显式决定逐条列出）

| # | 降级/漂移内容 | 决定出处 | 现状 |
|---|---|---|---|
| 1 | **13 键 verdict cache 降级为哲学标准**，`RUNTIME_ENFORCED = false`，不接 runtime 强制（拍板依据：2026-08-27 五维评分加权 0.28/5） | `ROADMAP.md:31`；`crates/foundation/core/src/philosophy.rs:127-142`；`docs/04-internal/ENGINEER-MANIFESTO.md:95-109`；`docs/04-internal/o6-session-log-2026-08-27.md §2.1`；`docs/04-internal/v2-unabsorbed-features.md:339-347` | 生效中，且被标注"永久"；替代 = external hook 闸 + 场景 D 长程 AI 判断（`scene-d-v2-plan.md`，仍待评估） |
| 2 | **stage2 §14 P0 漂移 5 项**（P0-01 supervisor 永不升级 / P0-02 二进制内不可改 / P0-03 七席全量触发 / P0-04 30%·60s·3轮 固定阈值 / P0-05 主 AI+memory+philosophy rest_for_one 强耦合）标"待修订" | `docs/archive/stage2/stage2-decisions-drift-revision-tracker.md §2-§3` | 5 项**全部仍为"待修订"**，无一进入"已修订"。其中 P0-04 的 X1/X2/X3 阈值至今未校准；P0-05 在 v2 单进程形态下事实消解（但仍挂账） |
| 3 | **24 LOCKED 入口签名形式撤销**，仅保 3 项不可变脊柱（Self-Disable / L0 HA / 13 键 verdict cache） | `Cargo.toml` `[workspace.metadata.apeireth]` hard_walls 段（R148，主人 2026-08-11 拍板）；`docs/archive/conventions/10-locked.md` | 生效中 |
| 4 | **物理（Ed25519）多签显式推迟 v2.1**，`verify_multisig` 保持"0 装占位"（字符串签名非真 crypto 校验） | `crates/foundation/core/src/onion_gate.rs:16`；`docs/04-internal/handoff-w1-w5-closure-2026-10-10.md:43`（"真 Ed25519 留 v2.1"）；`docs/04-internal/w3-legacy-gap-archaeology-2026-10-10.md:144,175` | 挂账中；M-of-N 逻辑多签已实现 |
| 5 | **UpgradeCycle 三处留白**：L3 sandbox regression 留未来 patch / L4 主人 Veto dashboard 留 v2.0.0 接入 / L5 git tag 只建议不自动跑 | `crates/engine/runtime-assembly/src/canonical/upgrade_cycle.rs:27-40`（"0 装诚实"自述） | 挂账中；OTA 7 阶段的 Switchover/Monitor/回滚无对应物且无单独降级记录（属实现缺口而非已拍板降级） |
| 6 | **评审类机制只降级、不枪毙主任务**（元层原则：judge/council 预算耗尽、空响应、坏 JSON → 降级放行并留痕；显式否决必须用诚实错误码） | `docs/04-internal/engineering-log-2026-10-06.md:20-32`；`docs/04-internal/HANDOFF-NOTES.md:9` | 生效中（commit `c9174eba`/`6cbb0759` 等）；与"分层验证网 fail-closed"的标尺语义方向相反，属有意识的体验取舍 |
| 7 | **生产旋钮整体默认关**（reflexion / council / onion layer / onering ledger / community triage / shell / fetch 等均 opt-in） | `crates/engine/runtime-assembly/src/canonical/production.rs:176-187`；`docs/04-internal/handoff-w2-wiring-2026-10-06.md`（"旋钮全默认关"）；`docs/04-internal/handoff-w1-w5-closure-2026-10-10.md:43`（洋葱层默认关 `APEIRETH_ENABLE_ONION_LAYER`） | 生效中；意味着本矩阵多处"🟡 接线在、默认不跑"是同一策略的产物 |
| 8 | **86 crate → 13 crate → 18 crate 拓扑收敛**（替换 stage2 的 9 大/30 crate 划分） | `ROADMAP.md:17,24-32`；`docs/01-architecture/m1a-canonical-storage-foundation.md`（SQLite 地基） | 生效中；但 stage2 原决策文本未同步改写（属"应正式修订标尺"项） |
| 9 | **ROADMAP.md:31 的部分"未移植"表述已过时**（"M1B 记忆/向量/图未移植；MCP、companion 器官、voice/screen 未移植"——现 memory/vector/graph/MCP/voice 均已有实现） | `ROADMAP.md:31`（2026-09-05 对账批口径） | 文档漂移（未更新）；建议随本矩阵对账一次 |
| 10 | **council live 假绿洞封堵**：曾以降级出口冒充 live 通过（撤销 key 下 0.67s 假绿），现 live 测试必须先证通道活着（探针化） | `docs/04-internal/live-verification-ledger.md:77`（台账 #44）；`docs/04-internal/handoff-w2-wiring-2026-10-06.md:258` | 已修复并立规："live 测试先证通道活着，不许降级出口冒充绿" |
| 11 | **风险→席位触发矩阵与按住阈值未校准**（critical 7/high 5/medium 3/low 1/info 0 仅注释态；30%/60s/3轮 = X1/X2/X3 待校准） | `crates/foundation/core/src/gate.rs:50-68`（注释态）；P0-04（附录 2-2） | 挂账中（阶段 3-5 应实测校准而未做） |
| 12 | **Kani 形式化证明以等价 Rust 断言测试替代**（r177 编号保留，真 Kani 接线为后续项） | `crates/foundation/core/src/onion_gate.rs:20-22`；`.github/workflows/kani.yml` 存在但覆盖有限 | 挂账中 |

---

## 结论 ①：达成率统计

**条级（27 条 = 13 + 10 + 4）**：

| 判定 | 条数 | 条目 |
|---|---|---|
| ✅ 完全实现 | 5（19%） | A3 / A7 / A8 / A11 / C4 |
| 🟡 部分实现 | 15（56%） | A1 / A2 / A4 / A5 / A6 / A9 / A12 / A13 / B2 / B4 / B6 / B8 / B10 / C1 / C3 |
| 🔄 替换实现 | 6（22%） | A10 / B1 / B3 / B5 / B9 / C2 |
| ⬇️ 显式降级 | 1（4%） | B7（B7c 运行时强制） |
| ❌ 未实现 | 0（组件级 ❌ 大量，见各行） | — |
| ❓ 无法判定 | 0 | — |

**读法**：27 条中 26 条有对应物（哲学与结构层几乎全部"落在了代码某处"），但只有 5 条完全达成；**真正的空洞集中在"最后一公里接线"与"部署形态"两处**——判定层完整而执行面默认关/未接（C1、B7c、B7e、B10c）、决策闭环完整而部署闭环整块缺失（B9/C2 的 Switchover/Monitor/回滚、B2/C3 的 supervisor 树、B4 的 L1-L3 总线）。

**按维度切**：
- 哲学/治理语义（§A + B7）：结构达成率高（13 键/9 锚/双洋葱/五重守门枚举全在），**运行时强制力低**（13 键已降级、AND 门不在热路径、反思期默认关）。
- 工程形态（§B 的总线/DB/crate/supervisor/OTA）：**4 项整体替换 + 3 项形态缺位**，与单机桌面侧车的产品现实一致，但与"多进程巨型基地"的标尺形态差距最大。
- 图纸机制（§C）：决策流骨架在、升级流被替换、拓扑零件在树不在、双洋葱显式化完整。

## 结论 ②：最重的 10 个差距（按对标尺的损伤度排序）

1. **阶段 6 里程碑验证机制从未启动**（附录 1）：验证清单是文档不是关卡、真实人类批准没有成为机制环节、反思没有随里程碑归档——标尺六阶段路线的终点站从未到过，这就是"从未实践验证过"的实体。
2. **OTA 7 阶段的部署闭环整块缺失**（B9/C2）：Switchover 双实例灰度、Monitor 30min、自动回滚、洋葱测试矩阵全部无实现；自我升级目前只能"建议打 tag"，没有安全部署与回退能力。
3. **13 键 verdict 运行时强制被显式降级 + V1+V2 AND 门不在执行主链**（B7c/C1）：哲学守门从"拦截器"退为"判别词汇表"，运行时实际守门者换成了行为链 Guard 等外部 hook——语义等价性未经验证（场景 D 仍待评估）。
4. **物理多签缺位**（B7d）：双根变更的最高治理环节只有逻辑票数、没有不可伪造的密码学/硬件签名，"物理多签"作为最后防线未成立（显式推迟 v2.1）。
5. **5 层总线只剩 L0/L4 两层**（B4）：L1 unix socket / L2 pipe / L3 gRPC 无任何实现，跨进程、异构子进程、外部服务三种通信形态没有载体。
6. **B+E supervisor 树不存在**（B2/C3）：零件（5 子树枚举 + OTP 策略 + 真进程监督）齐备，但无 PID 1 root、无子树装配、无 cgroup 限制；崩溃恢复/热替换的产品形态与"巨型基地"图纸不符。
7. **风险→席位触发矩阵与按住阈值未接线未校准**（A13b/B8/附录 2-11）：7/5/3/1/0 席位触发只是注释，council 默认 3 席，X1/X2/X3 阈值悬空——七席审议庭的分级审议语义不完整。
8. **6 DB → SQLite 单库的联想/向量/全文能力弱化**（B5）：Wave 联想网络只有 trait 口，A 层经验向量检索、全文检索、独立时序存储均被单库承担，能力上限受制于 sqlite-vec 自研基元。
9. **目标层自由空缺（SGI/GoalService 未接线）**（A2/B10a）："思想/判断/目标三层自由"实际只有两层，自主目标意图单字段与目标史没有载体。
10. **本地推理管家缺位**（B6a）：Ollama/LlamaCpp/VLLM 本地 provider 与"主→次→本地→拒绝" fallback 链的最后一环不存在，模型自主性与离线兜底不成立。

（并列候补：A9 分层验证网 L1-L5 未机制化；B10c MEWG 加权治理未接线；B7e 反思期无常驻调度。）

## 结论 ③：哪些是"该做的缺口"，哪些是"合理演进 / 该正式修订标尺"

**A. 该做的缺口（追理想——标尺仍然正确，现实欠账）**：
- 阶段 6 里程碑验证机制（结论②-1）——欠的不是代码量，是"验证清单当关卡 + 真实人类签核 + 反思归档"这一条流程闭环，成本低、价值最高，建议先做。
- OTA 部署闭环（②-2）：Switchover/Monitor/回滚是安全底线，与产品形态无关，该补。
- 物理多签（②-4）：治理防线的物理性不可用"逻辑票数"替代，v2.1 该兑现。
- 风险→席位触发 + X1/X2/X3 阈值校准（②-7）：标尺给的是机制，欠的是接线与实测校准。
- 目标层（SGI）与本地 provider 兜底（②-9/②-10）：若"中央 AI 三层自由"与"本地管家"仍是愿景，就该补实现。
- 哲学守门热路径的最终形态（②-3）：要么把 13 键语义并入 hook 闸并验证等价性（场景 D 路线），要么由主人正式拍板"词汇表 + hook 闸"为新标尺——现状的"永久降级"缺一次对标尺的正式改写。

**B. 合理演进（改标尺——现实选择更适配单机桌面产品，建议正式修订设计文档）**：
- 6 DB → SQLite 单库 + sqlite-vec + 自研向量/图基元（B5）：单机侧车形态下独立 DB 服务不成立，替换合理。
- 9 大/30 crate → 18-crate 四组制（B3）：职责映射保留，分层清单该重写。
- 5 层总线 → "L0 进程内 + L4 HTTP/WS" 两级（B4）：设计中的 L1 unix socket 在 Windows 平台本就可疑、L2 pipe/L3 gRPC 服务对象（Python/Go 子进程、外部服务）在当前形态不存在；若放弃多进程 B+E 形态，总线标尺应改为两级 + 按需 IPC。
- B+E supervisor 树 → "单进程内核 + 桌面侧车监督"（B2/C3）：若产品形态定格为单机桌面伴侣，图纸的 PID1+4 子树应修订为"侧车监督 + 工具进程隔离"两级监督。
- V3 9 键 → 13 键（B7a/A10 的 6→9 锚同理）：加严方向的超集替换，标尺数字直接更新即可。
- SemanticModelRouter → ProviderRouter（B6b）：按 model/健康路由已覆盖现实用法；"语义路由"降级为路线图项。
- OTA 7 阶段 vs L0-L5 cycle（B9/C2）：两者是互补关系（7 阶段 = 部署形态、L0-L5 = 决策步骤），建议标尺合并而非二选一。
- P0-05（主 AI+memory+philosophy rest_for_one 拆分）：在 v2 单进程形态下自然消解，建议在漂移表正式标"撤回（形态变化）"，别再挂"待修订"。

**给主人的决策提示**：差距的分布很清楚——**哲学层"想清楚了、写成了结构、但没有强制力"；工程层"零件全、形态换"**。"追理想"的主战场是结论②-1/2/3/4（验证闭环、部署闭环、守门力、物理多签）；"改标尺"的主战场是存储/总线/拓扑/crate 四项形态类条目。两者不冲突：建议先以附录 2 为基础把显式降级正式写回标尺（一次标尺修订会议），再按 A 清单立阶段 6 的最小验证闭环。

---

_本文为只读审计产物：所有判定锚定 2026-10 前后的 master 工作区代码与 archive 标尺文档；未改任何被审计文件。引用行号以本仓库当前 HEAD 为准。_
