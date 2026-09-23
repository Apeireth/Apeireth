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
