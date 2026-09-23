# v1 vs v2 后端能力差距 + 未实现愿景审计 (Capability Gap Audit)

> **日期**: 2026-10-06。**方法**: 四方并行审计——v1 源码清单 (`legacy/`,
> 105 crate) / v2 现役源码 (18 crate) / 既有差距文档全文提取 (8 份) / 愿景与
> ROADMAP 状态对账。**口径**: 一律采用 `system-capabilities.md:8-10` 的四级状态
> 分层 —— **IMPLEMENTED (代码存在) ≠ PRODUCTION WIRED (接主路径) ≠
> DEFAULT ENABLED (默认开) ≠ HARDWARE VALIDATED (真机验证)**。文档自标的 ✅
> 多数只达第一级, 本文逐项标注真实层级。

## 1. 规模与形态对比

| 维度 | v1.0 (`legacy/`, v1.0-master) | v2.0 (当前 main) |
|---|---|---|
| crate 数 | **105** (donor 77 / archived 15 / frozen 13) | **18** (foundation 6 / engine 8 / capabilities 1 / adapters 3) |
| 形态 | "伙伴器官"聚合 (donor/apeireth-companion ≈ 90 模块) | 微内核 + 显式装配 (PluginRegistry + CapabilityRegistry + Runtime) |
| 治理 | 5 重守门 + PermissionPack (临时授权) | GovernanceHook 三态 + 3 hook 生产常挂 + 会话级权限预设三态 (+ 协作者行为链 Guard) |
| 已有实测 | 仅 Windows 装机 e2e | Windows 装机 e2e 17/17 + 真模型 live + 全套边界测试 |

## 2. 能力对账 (按域)

> 定性列: ✅已移植 / 🟡部分 / ⛔明确延期 / 🔴0 装 / 🔒LOCKED / 🖥️待硬件。
> 证据以文档行号或源码路径给出, 详见各子审计报告。

### 2.1 Storage / 记忆

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| SQLite pool + write channel / migrations | REAL | 已就位 | ✅ |
| MemoryStore v2 (ACT-R/temporal/tombstone) | REAL | WIRED | ✅ |
| VectorIndex (cosine + BM25 hybrid) | REAL | cosine 已 1:1; **BM25 缺** | 🟡 |
| Graph primitives / causal engine | PARTIAL | graph ✅; **causal engine 缺** | 🟡 |
| consolidation_writeback / daily_summary / diary / cross_diary / memory_injection | REAL | **0 真实施** | 🔴 |
| memory_extractor / memory_graph | REAL | ADAPT | 🟡 |
| ONNX 嵌入 | — | stub 待决策 | 🔴 |

### 2.2 工具 / 进程 / 沙箱

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| Shell / Filesystem / Fetch / Search / Repo | PARTIAL–REAL | ADAPT/DIRECT_PORT | ✅ |
| Browser | PARTIAL | 借 Playwright MCP 调研 | 🟡 |
| Invest / Learning / SystemMonitor | PARTIAL | DEFER (P3) | ⛔ |
| WorktreeSandbox | REAL | DEFER (P2) | ⛔ |
| **PlatformSandbox (JobObject 真隔离)** | PARTIAL | **0 真实施** (Windows 文件/网络隔离 Unsupported) | 🔴 |
| ToolSynthesizer | PARTIAL | DEFER (P3) | ⛔ |

### 2.3 感知 / 多模态

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| Text modality | REAL | 真实现 | ✅ |
| Vision (ScreenCapture/pHash/OmniParser) | REAL (Windows) | **0**, 待 D 块硬件 | 🖥️🔴 |
| Voice (VAD/duplex/Whisper) | REAL | ADAPT P2; whisper 0 | 🖥️🟡 |
| Tactile / 其余 modality | — | 声明但 0 装 (6 modality 仅 text 真) | 🔴 |

### 2.4 认知 / 器官 / 编排

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| 9 organ (W1/W2/W3/E4/F4/F1/F6/E7/Memory) | REAL (v1 形态) | **9/9 库级真实现** (`apeireth-organ`); 串联层 OrganOrchestrator 已补 (ROADMAP P0+ ✅) | ✅ IMPLEMENTED / 生产串联 🟡 |
| 12 slot cognitive module | 9 器官 | 6 WIRED (judge/council 默认关) + 6 DEFERRED 收敛 | ✅ |
| judge / council 评审 | v1 critic | WIRED, 默认关, 四级降级已加固 (2026-10-06) | ✅ |
| HybridCognitiveRouter | PARTIAL | **0 真实施** | 🔴 |
| meta_thinking (643 行) | REAL | **0** | 🔴 |
| reflexion / thought_cluster / intent_brier | REAL | donor 已入库, **active 0 移植** | 🔴 |
| reflection 4 阶段周期 | REAL | DEFERRED → 并入 self_assessment (R22) | ⛔ |
| confidence (BetaBinomial) | REAL | world_model 本地简化 ✅; trait 0 移植 | 🟡 |

### 2.5 物种 / 关系 / 教养 (v1 愿景核心, v2 最大缺口)

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| education (技能教学, 402 行) | REAL | grep 0 命中 | 🔴 |
| partner (Partner+Bond+PrivacyBoundary) | REAL | 0 | 🔴 |
| community (detect+triage, 360 行) | REAL | 0 | 🔴 |
| principles (DynamicPrinciple+Store+master token) | REAL | 0 (F6 value_cases 已 WIRED) | 🔴 |
| tone (3 层调制 + ToneRefiner) | REAL | 0 | 🔴 |
| morphology (RetrievalMode) | REAL | 0 | 🔴 |
| timeline / continuity / spill / context / context_rot | REAL | 部分 (continuity 部分) / 余 0 | 🟡🔴 |
| milestone / onering / oracle_adapters / experiment_field | REAL | 0 (organ world_model trait ✅) | 🔴 |

### 2.6 治理 / 安全 / 凭据

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| Governance 5-gate → 3 hook 生产常挂 | PARTIAL | ✅ | ✅ |
| Onion ABAC (L0 不可变) | PARTIAL | LOCKED (`onion.rs:249`) | ✅ |
| **三洋葱 L3-L5** | 愿景 | **0 装** (runtime 仅 L1-L2) | 🔴 |
| PII / injection / 凭据泄漏 hook | REAL | 2 hook 已装 | ✅ |
| AuditHashChain | REAL | ✅ | ✅ |
| SelfDisableGuard | PARTIAL | ✅ (3 不可变脊柱之一) | ✅ |
| SovereignControl | REAL | ADAPT P2 | 🟡 |
| 信任分级 / 审批频率 / 免审批黑名单 | v1 有 | 缺 (RC-13 排期) | ⛔ |
| 会话级权限预设三态 + 会话内记住审批 | — | ✅ (2026-09 新功能, v1 无) | ✅ v2 独有 |

### 2.7 协议 / 网关 / 交付

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| Protocol 归一化 DTO / provider 适配 | REAL | DIRECT_PORT | ✅ |
| Gateway 路由 / egress filter | MIXED | ✅ (+ 2026-10-06 新增 admin config / session settings / 错误码契约 10 码) | ✅ |
| Gateway SSE broadcaster | REAL | 真流式已打通 (2026-09-10, 210 帧实测) | ✅ |
| Gateway MCP handler / MCP 桥 | PARTIAL | ADAPT P2 / 遗留 transport 桥 | 🟡 |
| EventBus core / backbone | REAL | core ✅ / backbone ADAPT P2 | 🟡 |
| Scheduler / Telemetry | PARTIAL | DEFER P2 | ⛔ |
| SDK (HTTP/WS 客户端) | REAL | **stub** (`unimplemented!()`, 待 R21) | 🔴 |
| Voice 交付通道 (lark/telegram 等) | REAL | archived stub | ⛔ |

### 2.8 形式化验证 / 质量

| 能力 | v1 | v2 | 定性 |
|---|---|---|---|
| Kani proofs | bridge 146 + organ 116 行 | organ_kani ✅ (6 crate); bridge_kani 0 | 🟡 |
| 7 重 CI 守门 | 5 重 | ✅ 升 7 重, 全绿 | ✅ |
| R11 baseline 3 值 | — | LOCKED (0.8682/0.8532/0.9063) | 🔒 |

## 3. 未实现的愿景 (分组清单)

### 3.1 🔴 0 真实施 (v1 有真实现, v2 未移植)

**记忆塑形**: consolidation_writeback / daily_summary / diary / cross_diary /
memory_injection / BM25 hybrid / causal engine
**元认知**: meta_thinking (643 行) / reflexion / thought_cluster (522 行) /
intent_brier (817 行) / confidence BetaBinomial trait
**物种与关系**: education (402 行) / partner (141 行) / community (360 行) /
principles (478 行) / tone (374 行) / morphology (284 行) / timeline
**协调与上下文**: onering (346 行) / context+context_rot (1440 行) /
assemble (1101 行) / milestone / experiment_field (282 行) / oracle_adapters (1300+ 行)
**系统**: PlatformSandbox 真隔离 / HybridCognitiveRouter / SDK 真实 HTTP+WS /
bridge Kani proofs / 三洋葱 L3-L5

### 3.2 ⛔ 明确延期 (文档排期, 非遗忘)

Invest/Learning/SystemMonitor (P3) · WorktreeSandbox (P2) · ToolSynthesizer (P3) ·
Scheduler/Telemetry (P2) · DesktopActionTool · reflection (→R22) ·
SovereignControl (P2) · MCP 桥 (P2) · EventBusBackbone (P2) · KV 逐出三篇 (后置)

### 3.3 🖥️ 待硬件 (无硬件无法真接)

Vision ScreenCapture/pHash/OmniParser · Voice Whisper 真接 · Windows Hello 生物识别 ·
RC-7 感知真 modality · P7 连续感知 (voice/screen, v1 亦从未落地 main)

### 3.4 🔒 永久降级 / 挂账 (愿景让位于诚实)

13 键 verdict cache: `RUNTIME_ENFORCED=false` (从运行时强制降级为哲学标准/判别词汇表) ·
v1 legacy 永久维护 · 26 项 Lost Capabilities 按 P3-P7 恢复 · 旧"8 硬墙"不再逐条跟踪

### 3.5 明确"不做" (设计决策, 非遗漏)

器官/模块前端视图 (用户要求) · 三层模型 (改单 plugin 层) · cognitive.critic
(并入 judge) · cognitive.reflection (并入 self_assessment) · planner/orchestrator/
perception 不做 per-turn module

## 4. 文档矛盾与裁决 (子审计发现)

| # | 矛盾 | 裁决 |
|---|---|---|
| 1 | oracle/oracle_adapters: FG 标 🔴 vs CC 标 🟡 | **采纳 CC**: world_model trait 已 1:1, oracle_adapters 全套 0 |
| 2 | proactive/E7: FG:156 标 🔴 但 FG:119 自标 ✅ | **采纳 ✅**: E7 emergence 已 1:1 移植; FG 内部矛盾 |
| 3 | Kani proofs: FG 🔴 vs CC 🟡 | **采纳 CC**: organ_kani 已装 6 crate, bridge_kani 0 |
| 4 | confidence: FG 🔴 vs MC 🟡 | **采纳 MC**: world_model 本地简化 ✅, v1 trait 0 |
| 5 | VectorIndex/Graph: FG 内部 🔴/🟡 自相矛盾 | **采纳 FG §5 修订**: 均 🟡 partial |
| 6 | SSE 流式: system-capabilities 说"非逐 token" vs ROADMAP 说"真流式已打通" | **采纳 ROADMAP** (2026-09-10 更新, 有 210 帧实测) |
| 7 | hello.rs 语义: "启动/装配" vs "Windows Hello 生物识别" | **CC 修正**: 是生物识别 (NGC 凭据探测), 非启动语义 |

**文档治理问题 (建议修)**: 6 份 R11 子调研已自标"2026-09-05 对账: 本文为历史记录",
唯 **主账 FG (`apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md`) 仍标 🟢 活跃且无过时声明**
——本文的矛盾 1/2/3/4/5 全部源于主账未同步修订。建议给 FG 加历史批注并指向本文。

## 5. 结论

1. **v2 不是 v1 的降级, 是形态重构**: 18 crate 承载了 v1 105 crate 的**主链路**
   (协议/网关/运行时/治理/工具/记忆核心/9 器官库级) 与 v1 没有的能力
   (层化治理 hook、会话级权限预设、错误码契约、无重启配置、桌面伴侣全链)。
2. **未达成的愿景集中在四块**: 物种与关系 (education/partner/community/principles)、
   记忆塑形 (diary/cross_diary/consolidation)、元认知 (meta_thinking/reflexion/
   thought_cluster/intent_brier)、协调上下文 (onering/context/assemble)。
   v1 的实现都在 `legacy/` 里躺着, 属"可移植的既成资产", 不是从零研发。
3. **诚实的完成度**: 文档自标 ✅ 的项目多数停在 IMPLEMENTED; 达到
   PRODUCTION WIRED/DEFAULT ENABLED/HARDWARE VALIDATED 的是少数 (主链路 + 治理 3 hook
   + 桌面全链 + 记忆 v2.2)。感知/语音/视觉三块全部待硬件。
4. **建议的恢复序** (与 ROADMAP P 序一致, 未变更): P3 记忆移植 (M1B) → P6 companion
   器官 (含物种/关系/元认知四块) → P7 连续感知 (硬件到位后) → P5 沙箱强化
   (可提前: `shell-sandbox-lite-design-2026-10-06.md`)。

## 6. 附录: v1 全量域清单 (差距文档未覆盖的部分)

> 既有差距文档 `apeireth-1-0-vs-2-0-functional-gap` 只覆盖 v1 的一部分子系统。
> v1 源码清单 (105 crate) 显示 v1 的实际广度远超该文档, 以下为**文档未列**的
> v1 子系统及其 v2 状态 (均未移植, 除非注明):

| v1 子系统 | 内容 | v2 状态 |
|---|---|---|
| 终端沙箱 6 后端 | Local / Docker / SSH / Daytona / Modal / Singularity (`apeireth-environment`) | ⛔ 未移植 (v2 仅 ProcessExecutor + JobObject) |
| Docker 沙箱真接 | `frozen/apeireth-sandbox` (bollard REST v1.43) | ⛔ 冻结参考 |
| microVM (libkrun FFI) | companion `vm_sandbox.rs` / `sandbox_ffi_libkrun.rs` (feature 默认关) | 🖥️ 未移植 |
| 网络隔离 | netns / cgroup / WFP (`sandbox_net.rs`) | ⛔ 0 装 stub |
| 受限 token + AppContainer | companion `restricted_token.rs` / `app_container.rs` | ⛔ (本审计 §3.1; 轻量档设计将以此实现) |
| Leptos Web 前端 | `apeireth-web` (SSR+WASM, council/memory/asi 页) | ⛔ 未移植 (v2 走 Tauri 桌面) |
| TUI 5 页 | `apeireth-tui` (ratatui + 9 器官视图) | ⛔ 未移植 |
| PyO3 Python 桥 | `apeireth-pybridge` (含 `reflection_self_loop.rs`) | ⛔ 未移植 (v2 纯 Rust deny(unsafe)) |
| 图编排 / 工作流引擎 | `apeireth-graph` (LangGraph 式) / `apeireth-workflow` (Temporal 式) | ⛔ 未移植 (v2 单 agent loop) |
| 主 chat 管线 (独立 crate) | `apeireth-pipeline` (token 预算三层 + 165 单测 + wiremock e2e) | 🟡 概念并入 runtime |
| 5 阶段 pipeline + 熔断 | `apeireth-pipeline-g5` (circuit_breaker + bounded_reliability) | ⛔ 未移植 |
| L0-L4 五层总线 | `apeireth-bus` | 🟡 v2 仅 EventBus core |
| MEWG 五重治理 + 物理多签 | `apeireth-sovereignty` (mewg/physical_multisig/multi_human/multi_ai) | ⛔ 未移植 |
| HASH-SQL 仲裁 | `apeireth-arbitration` (唯一事实时间线) | ⛔ 未移植 |
| 13 键 FourGates 实现 | `apeireth-constraint` (+ SelfModifyGuard) | 🔒 v2 永久降级 (RUNTIME_ENFORCED=false) |
| 三洋葱 trait 抽象 | `apeireth-onion` (原则 5 层 + 权限 6 层) | 🟡 v2 仅 3 项脊柱 + hooks |
| 7 种记忆 provider | `apeireth-memory-extensions` (in_memory/redis/sqlite/postgres/s3/disk_lru/hybrid) | ⛔ 未移植 (v2 仅 SQLite) |
| 跨 session token 折叠 | `apeireth-context-fold` (FoldStrategy/FoldMarker) | ⛔ 未移植 |
| 7 强制 Advisor 智囊团 (独立 crate) | `apeireth-council` (含 mock LLM 默认 + multi_model_backend) | 🟡 v2 有 Council (WIRED, 默认关) |
| 工具注册/运行时/审批 三 crate | `tool-registry` (5 轴正交 + 热加载) / `tool-runtime` / `tool-approval` (5 规则 + 5 分钟窗口) | 🟡 v2 简化为 plugin+capability 注册 + 审批生命周期 |
| 9 工具子 crate | browser(Playwright a11y) / codesearch(Aho-Corasick) / image-gen / image-process / tool-shell(seccomp+SSH+多签) / tool-fetch(search+deep+Bilibili) | ⛔ 未移植 (v2 仅 5 内置工具) |
| 进程 supervisor (PID1) | `apeireth-supervisor` (5 sub-supervisor + 3 restart 策略 + actor mailbox) | ⛔ 未移植 (ROADMAP P5 "不在 17-crate 工作区") |
| CentralAI + 11 Skill | `apeireth-central` (含 Skill 注册 + semver) | ⛔ 未移植 |
| Agent 管理 + subagent | `apeireth-agent` (alias/LRU/notify 热加载/subagent) | ⛔ 未移植 |
| Team Lead Orchestrator | `apeireth-team-lead` (approval_bridge + lease) | 🟡 v2 有 orchestration crate (Council) |
| 节律 / cron | `apeireth-cron` + companion `emergence` (RhythmEstimator) | 🟡 v2 E7 emergence 已 1:1; cron 未移植 |
| 遥测栈 3 crate | telemetry / observability / metrics / tracing (frozen) | ⛔ 未移植 (v2 仅 session event + trace) |
| 形式化验证 | `archived/apeireth-formal` (Kani/TLA+ harnesses) | 🟡 v2 organ_kani 已装 6 crate |
| 交付通道 | Lark / LiveKit / ACP / Web / TUI / companion_serve(OpenAI 兼容) | ⛔ 未移植 (v2 仅 gateway HTTP + 桌面) |
| SDK (HTTP/WS 客户端) | v1 REAL | 🔴 v2 stub (`unimplemented!()`, 待 R21) |
| 配置/状态/i18n/扩展/限流 等基建 | config / state(9 organ state) / i18n / extension / rate-limiter / http-client(LIFO 池) | ⛔ 未移植 |

**补充判断**:
- v1 的**治理与工具链其实最实** (13 键/MEWG/三洋葱/工具注册运行时审批/9 工具),
  **自我改进类最"愿景化"** (演化的 LLM 实现多为 trait 口);
- 但 v1 的**器官层大量是"确定性规则完整 + LLM trait 口留而未接 + stub/0 装 PASS"**
  (`organs.rs:15` 明写 ToneRefiner 实现未接; curiosity/emotion_memory/hypothesis/
  value_cases 均标"确定性无 LLM") —— 所以"v1 有而 v2 没有"的账, 需要按
  **IMPLEMENTED 层级**理解, 不等于"v1 已达成、v2 退步";
- v2 是**形态重构 + 工程化收敛** (18 crate / 7 重 CI / 四级状态诚实标注), 代价是
  v1 的宽度 (105 crate 的生态) 大量留在 `legacy/` 待按 P 序回收。

## 7. 更正与源码复核 (2026-10-06 晚, 重要)

> **主账 `apeireth-1-0-vs-2-0-functional-gap` (2026-08-28) 的 🔴 清单已大面积过时** ——
> 其后多波 (R12-SpeciesCore-1 / memory v2.2 / R30 claude-mem 三层 / 研究吸收批)
> 已把其中许多做成**库级实现**。本节以源码实证更正 §2/§3 的对应条目。

### 7.1 已存在但**未接生产路径** (IMPLEMENTED, NOT PRODUCTION WIRED)

源码证据 (模块文件 + lib.rs 声明):

| 主账标 🔴 的项 | v2 实际 | 证据 |
|---|---|---|
| partner | ✅ 库级实现 | `memory/src/partner.rs` ("伙伴与双向羁绊模型, R12-SpeciesCore-1 实施") |
| principles | ✅ 库级实现 | `memory/src/principles.rs` ("动态原则层与原则洋葱晋级候选") |
| diary / daily_summary / cross_diary | ✅ 库级实现 | `memory/src/{diary,daily_summary,cross_diary}.rs` |
| consolidation / dreaming | ✅ 库级实现 | `memory/src/{consolidation,dreaming,dream_consolidation}.rs` |
| memory_injection | ✅ 库级实现 | `memory/src/memory_injection.rs` |
| meta_thinking | ✅ 库级实现 | `memory/src/meta_thinking.rs` |
| intent_brier / confidence / calibration | ✅ 库级实现 | `memory/src/{intent_brier,confidence,calibration,online_calibration}.rs` |
| reflexion | ✅ 库级实现 | `memory/src/reflexion.rs` |
| topic_predictor / proactive_recall | ✅ 库级实现 | `memory/src/{topic_predictor,proactive_recall}.rs` |
| morphology | ✅ 库级实现 | `organ/src/morphology.rs` ("Query morphology softmax") |
| education | ✅ 工具级实现 | `tools/src/education.rs` ("Education Dx-Check 换元检查工具") |
| worktree_sandbox | ✅ 库级实现 | `orchestration/src/worktree_sandbox.rs` |
| BM25 混合检索 (主账标缺) | ✅ 已实现 | `memory/src/hybrid_search.rs` (Okapi BM25 + 向量 RRF, 0 外部 NLP 依赖) |
| 吸收批 (betti/残差金字塔/河流拓扑/Kuramoto…) | ✅ 已实现 | `memory/src/{betti_hole_detector,residual_pyramid,river_topology,kuramoto_resonance}.rs` |

**接线状态实测**: 上述模块在 `runtime-assembly/src/canonical/` 与 `adapters/cli/src/`
中的引用数**几乎全部为 0** —— 即**库级真实现, 未接生产装配**。
按四级口径记: **IMPLEMENTED ✅ / PRODUCTION WIRED ❌**。

> **[2026-10-06 晚二次复核修正]** 精确重扫 (脚本见 `engineering-review-handoff-2026-10-06.md` §3.2)
> 后有两个**非零**引用, 原"唯 `context_rot` 有 3 处"的说法**不完整**:
>
> | 模块 | 引用数 | 真实状态 (四级口径) |
> |---|---|---|
> | `context_rot` | 3 | **WIRED ✅ / DEFAULT ENABLED ✅** (确定性压缩真接进上下文管理) |
> | `proactive_recall` | **13** | **WIRED ✅ / DEFAULT ENABLED ❌** —— `ProductionModulesConfig.proactive_recall: Option<ProactiveRecallPolicy>` (`production.rs:113`), `Default` 给 `None` (`:151`), 仅 `:347-348` 在显式配置时 `with_proactive_recall`; **且全仓库无 `APEIRETH_*` 旋钮可开** |
> | §7.1 表格其余全部模块 | 0 | **IMPLEMENTED ✅ / PRODUCTION WIRED ❌** |
>
> 教训: "引用数为 0"这种结论必须**用脚本全量扫**, 不能凭印象点几个名字;
> 且 `Option<...>` 类型的配置项要**连 `Default` 值一起看**, 否则会把"接线了但默认关"误判成"没接线"。

### 7.2 仍为真缺口 (源码确认不存在)

| 项 | 状态 |
|---|---|
| community (社群识别与分诊) | 🔴 v2 crates 内 0 命中 |
| experiment_field (隔离实验场) | 🔴 0 命中 |
| HybridCognitiveRouter | 🔴 0 命中 |
| ToolSynthesizer | 🔴 0 命中 |
| thought_cluster (按此名) | 🔴 0 命中 (有 `cluster_store.rs`, 疑似改名/部分) |
| onering (OneRingLedger) | 🔴 仅元数据透传注释, 账本本体 0 |
| 真文件/网络隔离 | 🔴 实测 `EnforcementLevel::Unsupported` (`process/linux.rs:62-67`), 仅进程树遏制 |
| SDK 真实 HTTP/WS | 🔴 7 处 `unimplemented!()` + 自标"阶段 6 stub, R21 真接" |
| 三洋葱 L3-L5 | 🔴 未实现 (runtime 仅 L1-L2) |

### 7.3 对结论的修正

- §3.1 "🔴 0 真实施"应分两级读: **真缺口** (§7.2, 源码确证不存在) 与
  **库级已实现未接线** (§7.1, 占多数) —— 后者距生产化只差"装配 + 测试 + 门禁",
  不是从零研发;
- 这**加强** §4 的文档治理建议: 主账 FG 不仅缺历史批注, 其 🔴 清单还**低估 v2**
  (把"已实现未接线"也标成 0), 修订时须按四级口径重标;
- **教训 (0 装纪律的双向性)**: 0 装要求"不假装完成", 同样要求"不假装未完成" ——
  审计必须对源码实证, 不能只信历史文档。

---

## 8. 附录 A: v2 现役 18 crate 源码清单 (逐 crate + 逐能力, 2026-10-06 复核)

> **口径**: 全部结论来自 `crates/` **源码第一手** (`lib.rs` 声明 + 模块文件 + 行号),
> **未读 docs/** —— 目的是给"历史文档说 X, 源码其实 Y"提供独立证据面。
> 三档状态: **真实现** / **部分** (框架真、真 backend 未接: HTTP/crypto/持久化/硬件) /
> **stub·0装** (源码内显式标 0 装 或返 `NotImplemented`)。

### 8.1 逐 crate 职责与公开入口

| crate | 一句话职责 | 关键公开入口 |
|---|---|---|
| `core` (F) | 主路径核心类型 + 内核原语 + 双洋葱 + 哲学锚 + 状态机 + SelfDisable/OTA | `Episode/Note/Session/IdentityCard`; `kernel/gate/lifecycle/onion/philosophy/statechart/eight_anchors`; `OtaChannel`, `trait Evolution`, `trait SelfDisable`, `SelfDisableAudit` |
| `plugin` (F) | canonical plugin/capability 模型 + 注册表 + MCP + organ/perception 抽象 | `trait Plugin`, `PluginManager/Manifest`, `CapabilityRegistry`, `Tool/ProviderCapability`, `trait OrganTrait`/`OrganKind`, `trait PerceptionBackend`, `trait PreferenceStore/SelfAssessmentStore` |
| `protocol` (F) | LLM 协议归一化 (OpenAI Chat/Responses + Anthropic + Gemini) + WS 8 帧 + 用量 | `trait ProtocolAdapter`, `adapters::{openai_chat,openai_responses,anthropic_messages,gemini}`, `bridge/bridge_ext`, `normalized`, `canonical`, `ws_v1/ws_session`, `acp`, `usage::CostTracker`, `p2p_mesh` |
| `governance` (F) | 运行时行动前的**唯一**治理决策点 (单 hook + 五门 pipeline) | `trait GovernanceHook`, `Action/Decision`, `GovernanceVerdict`, `GovernancePipeline`, `Permission/PermissionSet`, `RateLimitGovernanceHook/TrustTier`, `AuditHashChain`, `input_security`, `colang`, `research_autonomy` |
| `storage` (E) | 存储基础: SQLite pool/config/migrations + cache + 限流 + quota | `SqliteConnectionPool/SqliteConfig`, `Migration/run_migrations/current_version`, `cache::{lru,shard,ttl,stats,evictor}`, `rate_limit::{retry,strategies}`, `quota`, `machine_id` |
| `credentials` (F) | 统一凭据存取 + keyring + 加密文件后端 + 脱敏 | `trait CredentialsStore`, `FileCredentialsStore`, `InMemoryKeyring`, `EncryptedFileBackend`, `NoopAudit/CountingAudit`, `SecretBuf/SecretString`, `KeyringCredentialResolver`, `trait CredentialResolver` |
| `guard` (E) | 两阶段行为链安全分类器 (runtime 治理) | `ChainGuard`, `FastGuard/FastGuardResult`, `DecisionFusion`, `BehaviorChain`, `ScenarioOracle`, `CommandEffectAnalyzer`, `SessionBehaviorHistory`, `EnforcementDirective`, `GuardDryRunRequest/Response` |
| `memory` (E) | canonical 记忆: SQLite 持久化 + ACT-R 检索 + 图 + BM25/向量混合索引 | `SqliteMemoryStore`, `MemoryMutationFacade`, `MemoryCoordinator`, `HybridSearchEngine`, `PersistentVectorIndex`, `BitemporalGraph`, `UniversalForgetFacade`, `layered_memo::*` (L1-L4), `dailynote::*`, `Episode/Note/SessionStore` |
| `organ` (E) | 9 organ 真移植 (v1 companion 1:1 → `OrganTrait`) | `Curiosity/Hypothesis/ValueCases/Emotion/WorldModel/CausalWorldModel/EdgeMiner/Emergence/MemoryMerger` 9 impl + `NoopOrgan`; `morphology/motivation/tone/context_assembly/prompt_assembly/goal/experience_growth` |
| `perception` (E) | 感知层 5 modality (v2.0 仅 Text 真) | `PerceptionEvent/Input/Modality`, `normalize/observe/capture/screen/owner/vision/voice`, `NoopScreenSource`, `XcapVisionBackend`, `WhisperHttpBackend`, `EnergyVadStream` |
| `runtime` (E) | runtime 机制内核 (session/路由/agent loop/审批), `#![deny(unsafe_code)]` | `Runtime/Builder/Config`, `SessionManager/SessionStore`, `ProviderRouter/RoutedCompletion`, `ModuleRegistry/Module/AgentModule/ModuleInvoker`, `PendingApproval/ApprovalDecision/TurnOutcome`, `RuntimeEvent`, `ExecutionTrace`, `ContextProjector` |
| `runtime-assembly` (E) | **生产装配** (concrete Memory/Organ/Tool/SQLite wiring) | `ProductionBackends/Modules/CognitiveModules`, `Council/Judge/Organ/MemoryRecall/MemoryWriteback/PreferenceLearning/SelfAssessment` Module, `Fetch/Filesystem/Repo/Search/Shell/Mcp` Module, `InvokerLlmFactory`, `SqliteSessionStore`, `PermissionPresetGovernanceHook`, `DEFERRED_COGNITIVE_SLOTS` |
| `provider` (E) | canonical 3 Provider (Anthropic/MiniMax/OpenAI-compatible) + LLM factory | `canonical_anthropic/canonical_minimax/canonical_openai_compatible`, `minimax_llm_factory/openai_compatible_llm_factory`, `openai_chat`, `reasoning_adapter`, `provider_model`, `credentials` |
| `orchestration` (F) | 7 Advisor + Council 评审 + Orchestrator + context/durable 工具 | `trait Advisor`, `Council/CouncilVerdict/CouncilInvoker`, `trait Orchestrator`, `SubagentRole/Spec`, `trait LlmFactory/LlmInstance`, `NoopLlmFactory`, `context_fold/context_rot/continuation/durable/cron/speech_arbiter/worktree_sandbox` |
| `tools-canonical` (C) | 内建工具 (filesystem/search/repo 默认开; shell/fetch opt-in) + 三 OS 隔离 | `BuiltinToolsPlugin`, `ProcessExecutor`, `ControlledEgress/EgressPolicy`, `ToolGuardrail`, `McpClient`, `SpillStore`, `TransactionalPatchApplier`, `RepoMapGenerator`, `StealthCrawlerEngine`, `StdSubSupervisor/NoopSubSupervisor` |
| `gateway` (A) | HTTP 网关适配器 (传输 ↔ runtime) | `GatewayState/Services`, `canonical_entry` (native/openai + SSE), `panels`, `EventBus`, `admin` (热配置), `BargeInController`, `DuplexSessionController`, `TransparentFileFetcher`, `EmberHudDriver`, `ErrorCode/ErrorFrame`, `trait CredentialWriter/PanelData` |
| `cli` (A) | CLI (session/chat/gateway 命令 + **生产装配入口**) | `enum CanonicalCliTurn`, `build_production_governance`, `build_canonical_runtime_from_env`, `execute_canonical_cli_turn`, `dispatch_gateway_serve_on`, `PortableBundleSynthesizer`, bin `apeireth` |
| `sdk` (A) | 客户端 SDK (**阶段 6 stub**: 类型/鉴权/白名单就位, 真 HTTP/WS 待 R21) | `ApeirethClient/AuthPipeline/ClientConfig`, `STUB_MODE`, `TOOL_WHITELIST`, `SdkError/SdkErrorCode`, `negotiate/SdkVersion`, `Envelope/WireKind` |

### 8.2 逐域能力状态 (真实现 / 部分 / stub)

**记忆域** — 全部真实现: SQLite 主库 + migrations (`memory/src/lib.rs:448,233`),
门面/协调器 (`facade.rs`/`coordinator.rs`), **BM25 + 向量 RRF 混合检索** (`hybrid_search.rs:137,292`),
持久化向量索引 (`persistent_vector.rs:37`), 向量距离原语, 双时相图 (`bitemporal_graph.rs`),
分层记忆 L1-L4 (`layered_memo/*`), 日记/日报 (`dailynote/*` + `diary.rs` + `daily_summary.rs`),
全局遗忘 (`universal_forget.rs`), persona/preference/experience/self-assessment SQLite store。
**部分/0 装 4 处**: `preference_store.rs` (`NoopPreferenceStore`, "rc 阶段换 SQLite"),
`gen_cache.rs:112-118` (`SigSource` trait 口 0 装), `admission_gate.rs:165-171` (冲突度打分 stub),
`derived_repair.rs:12,54` (真持久化留部署层)。

**认知/器官域** — 9 organ **9/9 库级真实现** (`organ/src/lib.rs:31`, 逐条 `:12-26`);
`OrganOrchestrator` 串联层真存在 (`runtime-assembly/src/canonical/organ_module.rs:107`);
E7 Emergence 8 重门控真 (`organ/src/lib.rs:20-25`); Judge 真 (`JudgeModule/JudgeVerdict`)。
**部分/未实现**: W1/W2 真接 LLM 但 dev 用 `NoopLlmFactory` → 诚实 `NotImplemented`;
E7 `PolicyStage` 5 状态机 forward-declared; MemoryMerger persist 用 `Vec` 非 SQLite (`memory.rs:38-39`);
`DEFERRED_COGNITIVE_SLOTS` (`cognitive.rs:1514-1527`) — `cognitive.critic/reflection/planner` 三槽 deferred。

**编排域** — Council 7 Advisor 框架真 + `LlmAdvisor` 真接 LLM (`council/advisors_llm.rs:34`);
`NoopAdvisor` (`lib.rs:700-730`) 无 key 兜底 0 装; `trait Orchestrator` (`lib.rs:851`) 0 装
(真调 subagent 需 runtime 介入); context_fold/rot 确定性真 (`DeterministicCompactor`),
LLM Summary 是 honest stub (`fold.rs:7,22`); durable 真重放已实现但**存储路径未接线** (`replay.rs:774`);
continuation/cron/speech_arbiter/worktree_sandbox/lineage 真实现。

**治理/安全域** — `GovernancePipeline` + `GovernanceHook` 单契约真 (`governance/lib.rs:471,319`);
权限/限流/未信任标记/工具描述审计/审计哈希链/输入安全 PII 真;
`colang`/`approval_policy`/`eval`/`evidence`/`rubric`/`risk` 是**真实现但 default-off helper**
(`governance/lib.rs:11-17` 明示"不实现 `GovernanceHook` 不装进 pipeline");
`research_autonomy` 校准门控自治 (RA-4) 真、默认关; `guard` 两阶段分类器真。
**0 装**: `credentials/gate.rs` 高危凭据审批门 (trait 口); `core/onion.rs:80-83,156` 多签 M-of-N
是 hex 占位签名, 真 crypto (Ed25519) 排 v2.1。

**工具/进程/沙箱域** — filesystem(只读)/search/repo(只读 git)/shell(opt-in)/fetch(opt-in) 真;
`ProcessExecutor` 三 OS (Job Object / CREATE_SUSPENDED) 真 (`process/mod.rs:664`, `windows.rs:67`);
受控网络出口 (DNS 钉扎) 真; `ToolGuardrail` (路径/命令/凭据绊线) 真;
`StdSubSupervisor` 真 spawn/kill/重启 (`std_sub_supervisor.rs:46`, 但 `:81` **未接 Job Object**)。
**0 装/未实现**: 写操作 (write/delete/rename/copy) 源码显式 deferred 到 M2B (`filesystem.rs:9-10`);
`trait SubSupervisor` + `NoopSubSupervisor` 0 装 (`supervisor.rs:131,189`);
`trait McpTransport` 无生产 impl (`mcp.rs:120`); `StealthCrawlerEngine` 仅 UA 选择 + 文本包裹,
无真抓取 (`stealth_crawler.rs:65`); **真文件/网络沙箱 = 未实现** (只有进程树遏制)。

**协议/网关/交付域** — 4 协议归一化 + bridge + WS 8 帧 + ACP 类型 + `CostTracker` 真
(纯翻译层, 无 HTTP); 3 Provider + 2 LLM factory 真; HTTP 网关 (native + OpenAI-compatible + SSE) 真
(`canonical_entry.rs:489-523`); 面板内省 + 501 诚实降级真 (`panels.rs:641-696`)。
**部分/0 装**: `protocol/gateway.rs:250,286` openclaw-gateway 是"简化 stub, echo 响应";
8 帧全双工 + 语音打断**类型/控制器/分句就位但无真实 WS 传输接线**;
`panels.rs:690,1168-1234` `memory.update` → `not_implemented`,
`voice.duplex`/`subagents.orchestration` → `not_assembled`;
`presence.rs:68-72,111-122` 固定 `heuristic_v0` + 置信 0.5, `Ritual` 永不发射;
`plugin/mcp/sse.rs:6` SseTransport deferred; `plugin/mcp/reconnect.rs:15` donor 重连原本就是 stub。

**持久化/可观测域** — SQLite pool/migrations/cache/限流/quota/machine_id 真;
SSE 事件总线 + trace/audit 落档真; 统一错误帧 + 错误码目录真; 认知/成本遥测真。
**部分**: 密钥审计 sink 默认 `NoopAudit` (`credentials/keyring.rs:195`), 真 audit 未挂;
audit salt 是"上层加盐占位" (`keyring.rs:275`)。

**SDK (唯一全量 stub crate)** — `STUB_MODE = true` 编译期硬编码 (`sdk/src/client.rs:112`);
`invoke_tool/invoke_stream` 返 `NotImplemented("R21 真接 apeireth-api")` (`:706-708,732-734`);
`QuotaStub::check` 永远 501 (`:441-446`); lark 8 API / livekit 6 API (JWT 占位
`stub.jwt.{identity}`, `livekit/auth.rs:186-204`) / sandbox 6 API / voice 6 API 全 stub;
C ABI `apeireth_sdk_init/last_error` 是 skeleton (`last_error` 返 -1, `abi.rs:16-24`)。

### 8.3 测试形态 (源码计数)

`crates/*/*/tests/*.rs` 共 **92 个**文件, 另有大量 `src` 内联 `#[test]`/`#[tokio::test]`
(credentials ≈50、protocol ≈51、sdk ≈410、memory 数十)。
`#[ignore]` 标记 **47 处** (`rg -n '#\[ignore' crates/` 全 crate 计数: `tests/` 目录 19 处 +
`src/` 内联 28 处), 分布**全部集中在需真 key / 真硬件的 E2E**:
`engine/organ` **35** 处 (`world_model` 5 / `causal_world_model` 5 / `value_cases` 5 /
`curiosity` 6 / `emotion_memory` 4 / `hypothesis` 4 / `memory` 3 / `organ_live_llm` 3)、
`engine/provider` **7** 处 (`openai_compatible_live` 3 / `minimax_llm_factory` 3 / 另 1)、
`foundation/orchestration` **3** 处 (`council_live.rs` 需 `OPENAI_API_KEY`)、
`engine/memory` **1** 处 (benchmark)、`engine/perception` **1** 处 (xcap, 需真显示器)。
→ 即 `cargo test` 默认**不跑**任何真模型/真硬件用例, 这正是台账
`live-verification-ledger.md` 存在的理由 (绿表 = 曾经手动跑过的 `--ignored`)。

### 8.4 源码内部不一致 (须修文档, 非须修代码)

| 位置 | 写的 | 实际 | 裁决 |
|---|---|---|---|
| `plugin/src/organ.rs:19-29` 9-organ 状态表 | W1/W2/W3/F4/F1/F6/E7/Memory **"0 装 (rc 阶段或 v2.1)"**, 仅 E4 ✅ | `engine/organ/src/lib.rs:31` **9 organ 全实装** | **engine 为权威**; plugin 表最后一次触碰 2026-08-30 (`9ce172a9`), engine 表 2026-09-04 (`3d663dbb`), 中间 R1-R5 子代理批把 9 organ 全落了 → **plugin 表是陈旧遗留**, 应改 |
| `plugin/src/organ.rs:1` 头注释 | 已更新为 "9-organ trait 抽象边界" | 与 `:19-29` 表矛盾 | 同文件内新旧未同步 |
| 主账 `apeireth-1-0-vs-2-0-functional-gap` (2026-08-28) | 🔴 大清单 + "🟢 活跃" | 见 §7 | 按 §4 建议: 加历史批注 + 四级口径重标 |

### 8.5 本节的自我限制 (0 装)

- 本节只覆盖 `crates/` **18 个 v2 crate**; `legacy/` 105 crate 的对照见 §1/§6;
- "真实现"= **库级代码路径存在且自洽**; 是否经真机点击流验证, 一律以
  `live-verification-ledger.md` 为准, 本节**不**代替台账;
- 行号取自 2026-10-06 的工作树; 代码变动后行号会漂移, 检索时以符号名 (grep) 为准。
