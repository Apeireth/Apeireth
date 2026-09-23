# W3 九缺口 legacy 考古 (2026-10-10)

```yaml
[Document-Meta]
Document:        docs/04-internal/w3-legacy-gap-archaeology-2026-10-10.md
Version:         Rev-1.0
Last-Modified:   2026-10-10
Status:          🟢 活跃 (W3 排期拍板依据)
```

> **给谁看**: 对 W3 九缺口排期拍板的主人 + 接续实施的工程师。
> **方法**: 逐个读 v1 **真代码**（`legacy/donor/...`），不用文件名下结论——上一轮审计（engineering-review-handoff-2026-10-06.md §4.3）自称只做了名称级扫描并要求复核。每条主张带 `path:line`。
> **九项清单来源**: 同交接包 §5 W3。**裁决约束**: 一次只做一个，禁止一次性全做。
> **四级口径**（`docs/01-architecture/system-capabilities.md` 头部定义）标注在各 verdict 后。

---

## 0. 一分钟结论表

| # | 缺口 | v1 真的做成了吗 | v2 现状 | 工作量 | 建议序 |
|---|---|---|---|---|---|
| 5 | `thought_cluster` 思维簇 | ✅ 真做成了，**且 v2 已移植**（= `cluster_store.rs` 改名，悬案闭合） | **库级缺口已闭合**（IMPLEMENTED）；欠生产接线 | S（核查接线即可） | **1** |
| 1 | `community` 社群识别与分诊 | ✅ 真做成了（图社群检测算法 + 确定性摘要 + 测试） | 未移植 | M | 2 |
| 6 | `onering` 账本 | ✅ 真做成了（318 行 + 8 项测试） | 未移植（v2 仅一条 VCP 注释提及） | M | 3 |
| 2 | `experiment_field` 隔离实验场 | 🟡 部分：**机制真实施，执行后端 0 装** | 未移植 | M | 4 |
| 7 | 真文件/网络沙箱 | ❌ **v1 也只有骨架**（seccomp/JobObject/netns/WFP 全是 TODO/Noop） | v2 进程树遏制**已反超 v1** | L（新造，走 W1 设计） | 5 |
| 9 | 三洋葱 L3-L5 | 🟡 部分：**层模型 + 判定逻辑真实施；无物理执行面** | v2 有数据模型 + hex 占位签名 | L | 6 |
| 3 | `HybridCognitiveRouter` | ❌ **不存在**（v1、v2 全部零命中） | 无 | XL（纯新造） | 8 |
| 4 | `ToolSynthesizer` | ❌ **不存在**（同上） | 无 | XL（纯新造） | 9 |
| 8 | SDK 真 HTTP·WS | （v1 有 `apeireth-api` 传输可参照） | **W5 已开挖** | L | 已开工 |

**核心校准**：九项里只有 **2 项是真"移植"活**（community、onering）+ 1 项核查活（thought_cluster）；**2 项 v1 也没有**（HybridCognitiveRouter/ToolSynthesizer 是愿景新造）；沙箱 v1 同样是骨架——把它当"移植"排期会严重低估。

---

## 1. `community` 社群识别与分诊 — ✅ 真做成了（库级 IMPLEMENTED）

- **位置**: `legacy/donor/apeireth-companion/src/community.rs`（≈300 行，含测试）。
- **证据**:
  - `community.rs:64` `pub fn detect_communities(facts: &[GraphFact]) -> Vec<Community>` —— 对知识图谱事实做**社群检测**（`:92-128`：连通分量/邻接聚合出 `Community`），不是占位；
  - `community.rs:130` `deterministic_summary(c, top_n)` —— 确定性摘要（排序稳定、可重放）；
  - `community.rs:44-57` `CommunityBriefs` / `summarize` trait 双实现（brief 生成）;
  - 测试在 `community.rs:222-286`（归一化断言 + 确定性输出比对）。
- **v2 归属**: `crates/engine/memory`（图域：`bitemporal_graph.rs` / `graph_algo.rs` 同区）。
- **移植风险**: `GraphFact` 类型需对 v2 的 `BitemporalFact` 适配；分诊（"社群 → 行动建议"）在 v1 是 brief 级，接入召回管线属接线活。
- **工作量**: **M**（移植 + 适配 + 测试 ≈ 2-4 天）。

## 2. `experiment_field` 隔离实验场 — 🟡 部分（机制真实施 / 执行后端 0 装）

- **位置**: `legacy/donor/apeireth-companion/src/experiment_field.rs`（260 行）+ `sandbox_pass.rs`（358 行，编译期契约守门）。
- **证据**:
  - 真实部分：确定性状态机 `ExperimentStatus`（Proposed → Building → …，`:18-24`）；完整回路设计"提案 → 实验 → 通过 → 主人批准 → 部署 → 监控 → 回滚 → 学习"（`:5-9`）；回滚学习写 `ExperienceStore`（`:16`）；
  - 0 装部分（原文自证）："`VMRunner` trait 口已备; 默认 `NoopVMRunner` 诚实 Err (VM 未接, 不假装能跑实验)"（`:13-15`）；`sandbox_pass.rs:7-10` 显示其执行面 `sandbox_net.rs`/`vm_sandbox.rs` 也全是 `trait + Noop`（netns/cgroup/WFP、libkrun/Hyperlight/Firecracker 均"待接/待选型"）。
- **v2 归属**: `crates/engine/runtime-assembly`（升级回路已有 `upgrade_cycle.rs`）+ `crates/capabilities/tools`（VM 执行面）。
- **移植风险**: 机制件可直接移植；**真执行面是新造活**（v1 也没做成，见 §7）。
- **工作量**: **M**（机制移植）+ 执行面并入 §7/L。

## 3. `HybridCognitiveRouter` — ❌ 不存在（v1 也没有）

- **证据**: `legacy/` + `crates/` 全树零命中（`HybridCognitiveRouter|ToolSynthesizer` 精确 grep = 0）。
- **结论**: **不是移植活，是愿景新造**。若排期按"从 v1 移植"立项会扑空。
- **工作量**: **XL**（混合认知路由属架构级新子系统，需先出设计）。

## 4. `ToolSynthesizer` — ❌ 不存在（同上）

- **证据**: 同 §3，全树零命中。
- **工作量**: **XL**（工具合成 = 运行时造工具，先要设计 + 治理裁决）。

## 5. `thought_cluster` 思维簇 — ✅ 真做成了 且 **v2 已移植**（悬案闭合）

- **v1 位置**: `legacy/donor/apeireth-companion/src/thought_cluster.rs`（486 行）：`ThoughtClusterManager` + 只读 `ThoughtClusterReader`，消费方 `dream.rs:58` / `reflection.rs:37` / `meta_thinking.rs:33`。
- **v2 现状（此前"没查清"的疑点，本次闭合）**: v2 的 `crates/engine/memory/src/cluster_store.rs`（502 行）**就是它的改名移植**：
  - 头注逐字同源：v1 `thought_cluster.rs:1` "思维簇管理 + 元自学习读取口 (backlog N4)" ↔ v2 `cluster_store.rs:1` "思维簇管理与元自学习读取口 (N4 / 认知长程聚类)"——**同一个 N4 编号**；
  - 机制 1:1：簇目录（`CLUSTER_SUFFIX`）、条目 `{YYYY-MM-DD}-{seq:03}.md`、链式注册表 `meta_thinking_chains.json`、只读回读口（v1 `ThoughtClusterReader` ↔ v2 `ClusterReader`）。
- **结论**: **库级缺口已闭合（IMPLEMENTED）**；剩余是**生产接线核查**（消费方 `meta_thinking`/`dreaming`/`reflexion` 在 v2 生产装配路径的引用计数为 0，属 W2 记忆闭环波次——恰是另一条线正在推的批次，可顺势带上）。
- **工作量**: **S**（核查 + 随 W2 接线）。

## 6. `onering` 统一上下文账本 — ✅ 真做成了（库级 IMPLEMENTED）

- **位置**: `legacy/donor/apeireth-companion/src/onering.rs`（318 行，**8 个测试**：`:256-338`）。
- **证据**:
  - `onering.rs:59` `pub struct OneRingLedger` + `:69` impl——跨前端（SSE/Web/Lark/Telegram/CLI）统一时间线账本；
  - 存储：自有表 `onering_messages`（continuity 锚点 + role + sender + frontend + ts，**seq 单调自增**替代秒级时间戳竞争，`:10-18` 对照表）；
  - `prune` 默认 200 可配；与 episodes 记忆管线**分流**（账本不污染提取/做梦/反思）。
  - 0 假装边界自证（`:18-19`）：fuzzy diff / 时间线插入策略**明确不吸收**。
- **v2 现状**: 零移植（`crates/foundation/protocol:498` 仅一条注释提及 VCP `__oneRingMeta`，非实现）。
- **v2 归属**: `crates/engine/memory`（建表模式对齐 `continuity_link.rs`）或 `runtime` 会话域；建议随会话转录缺口一起设计归属。
- **工作量**: **M**（移植 318 行机制 + 表迁移 + 测试）。

## 7. 真文件/网络沙箱 — ❌ **v1 也只有骨架**（重要校准）

- **证据**（v1 自己的注释）:
  - `legacy/donor/apeireth-tool-shell/src/lib.rs:25-28`："Linux seccomp BPF filter: 需要 `seccompiler` crate + unsafe pre_exec / Windows JobObject: 需要 `windows-sys` … / Linux namespaces … 需要 `nix`"；
  - `tool-shell:123`："TODO Linux seccomp / Windows JobObject / macOS sandbox_init"；
  - `tool-shell:4`："保持 workspace ponytail ceiling。真 seccomp BPF / JobObject syscall filter [未做]";
  - `sandbox_pass.rs:7-10`：`NetworkIsolation` trait + `NoopNetworkIsolation`（netns/cgroup/WFP"接入后启用"）、`VMSandbox` + `NoopVMSandbox`（libkrun/Hyperlight/Firecracker"待选型"）——0 装守门文件的存在本身就证明执行面未落地。
- **v2 现状**: `crates/capabilities/tools/src/process/` 的**进程树遏制反超 v1**（Windows JobObject + CREATE_SUSPENDED 是真实施；Linux FilesystemIsolation/NetworkIsolation = `Unsupported` 诚实标注，见交接包 §2.4 D3）。
- **结论**: 这是**纯新造活**，不是移植。v2 的正确路径是已出的 `docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md`（W1，轻量档 AppContainer + 工作区限定 + 断网），**动工前置 = 主人对设计拍板**（该文档状态仍为待拍板）。
- **工作量**: **L**（按 W1 三阶段计划）。

## 8. SDK 真 HTTP·WS — 已开工（W5）

- v1 参照物：`legacy/donor/apeireth-api`（WS 8 帧协议与工具白名单的原始出处，SDK `client.rs` 是其 1:1 翻译）。
- **现状**: W5 首刀已另行开挖（STUB_MODE 可配置化 + v2 canonical 面真实 HTTP 传输 + wiremock 测试），本报告不再展开。

## 9. 三洋葱 L3-L5 — 🟡 部分（层模型 + 判定真实施 / 无物理执行面）

- **位置**: `legacy/donor/apeireth-onion/src/lib.rs` + `examples/onion_demo.rs`。
- **证据**:
  - 真实部分：`lib.rs:21` "原则 5 层 (E/S/A/M/O) + 权限 6 层 (L0-L5) + 11 节点电子环"；`lib.rs:66-79` `PermissionLayer` L0..L5 完整枚举（L3 关键操作 / L4 核心升级 / L5 核武器级）；`onion_demo.rs:65-72` `unify_check(&action)` 对 L3/L5 触碰给出判定——**判定模型是活的**；
  - 但**无物理执行面**：权限层是数据模型 + 检查函数，不接 OS 权限/进程边界（与 §7 的骨架同因）。
- **v2 现状**: `crates/foundation/core/src/onion.rs` 有数据模型；M-of-N 多签是 **hex 占位签名**（交接包 §5 W7 自证），真 Ed25519 标 v2.1。
- **结论**: 移植判定模型 = M；**L3-L5 的物理执行面 + 真签名 = 新造**（且真 crypto 明确排在 v2.1）。
- **工作量**: **L**（模型移植小，执行面大；建议只在 W1 沙箱落地后再动 L3/L5 执行面）。

---

## 建议执行序（一次一个）

| 序 | 项 | 理由 |
|---|---|---|
| 1 | §5 thought_cluster 接线核查 | **S 级收尾**：悬案闭合、只剩接线；可搭 W2 记忆闭环批次的顺风车（零额外协调成本） |
| 2 | §1 community 移植 | v1 真货、边界清晰、M 级；落 `engine/memory` 图域，检索质量可感知 |
| 3 | §6 onering 移植 | v1 真货 + 8 测试；建议与"会话转录重载 API"（挂账项）合并设计归属 |
| 4 | §2 experiment_field 机制移植 | 状态机/回路真货先落（执行面与 §7 合并），与 `runtime-assembly/upgrade_cycle.rs` 衔接 |
| 5 | §7 真沙箱（=W1） | **前置：主人对 shell-sandbox-lite 设计拍板**；这是九项中唯一有现成设计文档的 |
| 6 | §9 onion L3-L5 判定模型移植 | 执行面等 W1；真签名留 v2.1 |
| 7-9 | §3/§4 愿景新造 + §8 SDK | XL 项先出设计再谈排期；SDK 已在 W5 |

## 本报告没有做的事

1. 未逐行验证 v1 各模块的**运行时行为**（结论基于静态代码阅读 + 测试存在性；`legacy/` 不参与构建，没跑过它的测试）。
2. 未评估移植的具体 API 形状（那是各立项的实施 brief 的活）。
3. §3/§4 因 v1 无实现，无从考证"该长什么样"——立项前需要单独设计文档。
4. 未替主人拍板顺序（上表是**建议**；§5 W3 原裁决"一次一个"仍然有效）。
