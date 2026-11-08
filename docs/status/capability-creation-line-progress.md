# 能力创造线 · 进度页（做到哪了）

[Document-Meta]
Document: docs/status/capability-creation-line-progress.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-08
Author: lead（插件/组件/能力开发线 总指挥）

> **本页是「做到哪了」的唯一权威页**（`AGENTS.md` §0.2：状态只引文件，不引对话记忆）。
> **目标形态** ⇒ [设计稿](../01-architecture/capability-creation-line-spec.md)；
> **授权** ⇒ [ADR-0025](../adr/0025-capability-creation-line-charter.md)（owner 2026-11-08：只拓新面 · 允许破棘轮 · 本轮只出设计与排期）。
> **工作线登记** ⇒ [workline-registry.md](workline-registry.md)。

---

## 0. 一句话

> **W0（登记批）已落地**：ADR + 设计稿 + 本进度页 + 守卫脚本 + 登记表五行。
> **W1（W1 = 8 族）尚未开工** ⇒ 全部 23 族仍为 ❌ 未开工；
> 另有 **6 项存量接线已移交**（owner 裁「不动存量」）⇒ 不在本线。

---

## 1. 波次看板

| 波 | 族数 | 状态 | 落地证据 | 还差什么 |
|---|---:|---|---|---|
| **W0** | — | ✅ **已落地**（本批） | 本页 + 设计稿 + [ADR-0025](../adr/0025-capability-creation-line-charter.md) + `scripts/check_creation_line.py` + 登记表新增行 | ❌ 待 owner 复核设计稿（**唯一待办**） |
| **W1** | 8 | ❌ **未开工** | — | ❌ 8 族全部未落地（C-B1 · C-B2 · C-D1 · C-D2 · C-H1 · C-H2 · C-H3 · C-H4） |
| **W2** | 1 | ❌ **未开工** | — | ❌ C-C1 未落地 |
| **W3** | 3 | ❌ **未开工** | — | ❌ C-E1 · C-E2 · C-F3 未落地 |
| **W4** | 3 | ❌ **未开工** | — | ❌ C-A1 · C-A2 · C-B4 未落地（**含引擎选型待定**） |
| **W5** | 3 | ❌ **未开工** | — | ❌ C-B3（选型待定）· C-G3 · C-G4 未落地 |
| **W6** | 5 | ⛔ **未排期**（需 owner 逐项重拍） | — | ❌ C-D3 · C-F1 · C-F2 · C-G1 · C-G2 **待 owner 拍板**（D2 已裁「T0 不做」⇒ 本线不擅自推翻） |

**W0 的验收证据（实测）**：

```bash
python3 scripts/check_creation_line.py --check     # ⇒ 23 族逐条判定，全绿
python3 scripts/check_doc_class.py --check         # ⇒ 0 违规（SPEC/STATUS 强制项齐）
python3 scripts/docs_links.py --check              # ⇒ 断链 0
python3 scripts/docs_index.py --check              # ⇒ 问题数 0
python3 scripts/arch_guard.py --strict             # ⇒ VIOLATIONS: 0（额度未动）
python3 scripts/docs_truth.py --verify --check     # ⇒ claims=N fail=0 stale=0
```

---

## 2. 族级看板（**23 族 · 状态由 `scripts/check_creation_line.py` 机器判定**）

> 「探针命中」= 该族存在性判据的实测命中数；「未开工」态**必须仍等于基线**（基线 = 2026-11-08 实测值）。
> ⚠️ 标「✅ 已落地」而探针未长 ⇒ **守卫红**（谎报）；标「❌ 未开工」而探针已长 ⇒ **守卫红**（已被他线实现，须认领或改判）。

| 族 | 能力 id | 波次 | 状态 | 治理档 | 落点（新面） | 还差什么 |
|---|---|---|---|---|---|---|
| C-A1 | `tool.ui_control_tree` | W4 | ❌ 未开工 | R0 | 新 crate `capabilities/automation` | ❌ 实现 + Windows 真机用例（Linux 须显式标未实现） |
| C-A2 | `tool.gui_action` | W4 | ❌ 未开工 | R3 | 同上 | ❌ 实现 + 预览票强制 + 证据帧 |
| C-B1 | `tool.process_tree` | W1 | ❌ 未开工 | R0/R3 | `tools/src/process/snapshot.rs` | ❌ 实现 + 快照零审批断言 + 平台机制名只落 backend |
| C-B2 | `tool.http_write` | W1 | ❌ 未开工 | R2 | `tools/src/http_write.rs` + 扩 `ControlledEgress` | ❌ 实现 + **扩受控出网写方法** + 新增「无第二 HTTP 客户端」不变量测试 |
| C-B3 | `tool.code_exec` | W5 | ❌ 未开工 | R3 | `tools/src/code_exec.rs` | ❌ **引擎选型待定**（子进程解释器 vs 进程内绑定）+ 四类逃逸拒绝用例 |
| C-B4 | `tool.browser` | W4 | ❌ 未开工 | R2/R3 | 新 crate `capabilities/web` | ❌ **引擎选型待定** + 白名单 + 证据帧 |
| C-C1 | `gw.duplex_ws` / `sdk.stream` | W2 | ❌ 未开工 | R0/R2 | `gateway/src/duplex_route.rs` + `sdk/src/stream.rs` | ❌ 实现 + `tokio-tungstenite` 引入 + 回环实测 |
| C-D1 | `mem.object_grant` | W1 | ❌ 未开工 | R0/R2 | `memory/src/object_grant.rs` | ❌ 实现 + **撤销即时生效**测试 |
| C-D2 | `rt.checkpoint_resume` | W1 | ❌ 未开工 | R0 | `runtime/src/canonical/checkpoint.rs` | ❌ 实现 + 「恢复后副作用不重复」测试 |
| C-D3 | `rt.cross_device_sync` | W6 | ❌ 未开工 | R4 | 新 crate | ❌ **待 owner 重拍**（阶梯 T1 + 跨主体披露） |
| C-E1 | `pkg.skill_package` | W3 | ❌ 未开工 | R3 | 新 crate `foundation/trust` | ❌ 实现 + 越权包拒装 + 卸载不留残影 |
| C-E2 | `pkg.signature` | W3 | ❌ 未开工 | R3 | 同上 | ❌ 实现 + `ed25519-dalek` 引入 + **失败无降级分支** |
| C-F1 | `gov.multisig` | W6 | ❌ 未开工 | R4 | `core/onion_gate.rs` 邻域 | ❌ **待 owner 重拍**（D2 已裁 T0 不做） |
| C-F2 | `gov.reflection_period` | W6 | ❌ 未开工 | R4 | `governance/src/chain.rs` 邻域 | ❌ **待 owner 重拍**（库已实装，装配即改产品范围） |
| C-F3 | `sec.supply_chain` | W3 | ❌ 未开工 | R2/R3 | 新 crate `foundation/trust` | ❌ 实现 + 篡改二进制不被派发 |
| C-G1 | `tool.ota_update` | W6 | ❌ 未开工 | R4 | 待定 | ❌ **待 owner 重拍**（R4 系统级控制面） |
| C-G2 | `tool.replay_eval` | W6 | ❌ 未开工 | R0 | 待定 | ❌ **待 owner 重拍** + 回放语义须先定义 |
| C-G3 | `obs.timeline` | W5 | ❌ 未开工 | R0 | `runtime/src/canonical/` | ❌ 实现 + 两次导出逐字节等 |
| C-G4 | `rt.crash_recovery` | W5 | ❌ 未开工 | R0 | `runtime/src/canonical/` | ❌ 实现 + 崩溃注入后账本一致 |
| C-H1 | `rt.routing_policy` | W1 | ❌ 未开工 | R0 | `core/src/routing_policy.rs` | ❌ 实现 + 决策可解释 + 缺价不编价 |
| C-H2 | `quota.depth` | W1 | ❌ 未开工 | R0 | `runtime/src/canonical/quota.rs` | ❌ 实现 + 自述面深度维不再为 null |
| C-H3 | `tool.mcp_protocol_compliance` | W1 | ❌ 未开工 | R0 | `tools/src/mcp_bridge/` | ❌ 实现 + **等他线在途收口后重跑探针** |
| C-H4 | `tool.web_search` | W1 | ❌ 未开工 | R2 | `tools/src/web_search.rs` | ❌ 实现 + 来源缺失即拒 + 双上限 |

---

## 3. 额度台账（**每波收口回填**）

| 棘轮 | 现值 | 本 ADR 授权上限 | W0 落库后 | 还差什么 |
|---|---:|---:|---:|---|
| `max_workspace_crates` | 31 | 35 | **31（未动）** | ❌ 按实际用量在波次收口时抬（用不完就少抬） |
| `max_production_edges` | 118 | 128 | **118（未动）** | ❌ 同上；⚠️ 设计稿估算 **+15 超授权 +10** ⇒ W3 收口须复评 |
| `max_distinct_names`（env） | 160 | 176 | **160（未动）** | ❌ 同上；⚠️ 设计稿估算 **+18 超授权 +16** ⇒ W3 收口须复评 |
| `max_file_loc` | 6300 | 6300（不动） | 6300 | — （零余量 ⇒ 新面一律新开文件） |

---

## 4. 移交清单（**存量接线，不属本线**；owner 2026-11-08 裁「不动存量」）

| 项 | 实测依据 | 移交去向 | 本线状态 |
|---|---|---|---|
| 记忆 P0 列封缄接线 | `storage::crypto::classify_column` 已逐列定级；`memory→storage` 边已存在 | 存量安全线 | ⛔ 不做 |
| MCP 结果面凭据绊线（批次 3c-1b） | `merge-integration-closeout-2026-11-07.md` §5-9 | **在途线**（owner 在盯） | ⛔ 不碰 `production.rs` / `env-registry.toml` / `mcp_bridge` 在途段 |
| 网关路径任务源路由 | `CostRouter` 已在 CLI 侧生产在用 | 存量线 | ⛔ 不做 |
| 网关侧子代理编排端点 | `RuntimeSubLoopSpawner` 已生产在用（CLI 侧） | 存量线 | ⛔ 不做 |
| R4 反思期装配 | `DelayedReflectionScheduler` 库已实装、生产未装配 | 治理线 | ⛔ 不做（W6 重拍时再议） |
| ROADMAP §3 回填（G1/G2/G3/G5/G7 五行陈旧） | 实测见缺口报告 §2.A | 文档线 | ⛔ 不做（本线**不改他人行**） |

---

## 5. 本线纪律（**每批开工前先看**）

1. **不碰他线在途面**：`production.rs` / `env-registry.toml` / `mcp_bridge` 在途段 ⇒ 写域回避 + 排队。
2. **新面一律新开文件**（< 800 行）；既有大文件（`doc_ops.rs` 6255 等）**只读不写**。
3. **新增写能力默认关** + 关时**逐字节零变化**测试 + env 登记（含 purpose 与 evidence）。
4. **不自带第二套治理/HTTP 客户端/进程通道**：复用既有受控出网唯一通道、既有审批链、既有进程封装。
5. **平台机制名只落 backend 模块**（既有不变量 `public_contract_does_not_name_platform_sandbox_mechanisms`）。
6. **每族至少 1 个对抗测试**（越权 / 篡改 / 越界 / 超限 **必拒**）。
7. **0 装诚实**：未实现的平台/引擎**显式标未实现**，不得静默绿；本线最多承诺 `PRODUCTION WIRED`，
   **不承诺任何 `HARDWARE VALIDATED`**（本仓现值 0 条）。

---

## 6. 变更纪律

* **状态变了必须改本页**（并在 `docs/INDEX.csv` 重新生成）；
* 引用状态时**只引本页**，不引对话记忆（2026-10-28 教训：上下文压缩会丢掉「我做过什么」）；
* 每族落地 ⇒ 把**证据路径**写进第 2 节该行的「还差什么」列（清空即完成），
  并让 `scripts/check_creation_line.py` 的探针从基线长出来（**状态词与探针必须同时变**）。
