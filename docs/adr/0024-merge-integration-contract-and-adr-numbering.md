# 0024. 跨线合并后的契约登记归并与 ADR 编号撞车处置

[Document-Meta]
Document: docs/adr/0024-merge-integration-contract-and-adr-numbering.md
Version: 1.0
Last-Modified: 2026-11-07
Status: Accepted（owner 2026-11-07 裁决「按你的建议来」）
Author: 主代理（总工程师线）

## 1. 背景

2026-10-24（分叉点 `8ac0f9de`）后两条线各自演进：总指挥线（本线，107 提交）与并行线
（625 提交）**各自维护 ADR 编号**、**各自维护 `contract.toml` 的独立 workspace 登记**。
2026-11-05～11-07 完成跨线合并（合并提交 `35cf3710`，收口批至 `507653b4`）⇒ 暴露三处
**结构性登记冲突**，都需要一次裁决。

## 2. 裁决一：ADR 同号撞车 —— **不重编号**

**事实**：`docs/adr/` 下 0016 / 0017 / 0018 / 0019 / 0020 / 0021 **各有两份**
（主题不同、互不替代）：

| 编号 | 本线 | 并行线 |
|---|---|---|
| 0016 | `0016-personal-line-carries-all-care-mechanisms.md` | `0016-frontend-visual-rebuild.md` |
| 0017 | `0017-engine-product-repo-split.md` | `0017-kernel-convergence-observation-port.md` |
| 0018 | `0018-frontend-visual-rebuild.md` | `0018-registry-kernel-tool-events.md` |
| 0019 | `0019-apeireth-facade-crate.md` | `0019-registry-runtime-dispatch.md` |
| 0020 | `0020-facade-slim-default.md` | `0020-capability-module-ownership.md` |
| 0021 | — | `0021-consolidation-ui-core-slice.md` / `0021-owned-agents-and-dynamic-modules.md` |

**裁决**：**不重编号**，改为「**并存索引 + 撞车说明**」（已落在 `docs/adr/README.md`）。

**理由**：
1. 重编号要**改文件名 + 全仓引用**（含 `crates-ui/**/Cargo.toml` 里按文件名的注释引用）
   ⇒ 跨两个 crate 的元数据改动，**风险高、收益低**。
2. ADR 编号唯一性是**约定**而非**机制**（仓内无「编号唯一」守卫）⇒ 撞车**不产生静默失效**，
   与「能力身份撞车会致授权其一即授权其二」那类**机制级撞车**性质完全不同。
3. 编号**重排会破坏历史可追溯性**（旧提交里的 ADR 引用会指向不存在的文件名）。

**代价（如实登记）**：「四位递增编号」约定被打破 ⇒ ⇒ **正文引用一律用文件名链接，不只用编号**
（`docs/adr/README.md` 已写明该要求）。

## 3. 裁决二：契约 `[[workspace.independent_workspaces]]` 同 path 重复登记 —— **归并成一条**

**事实**：`docs/architecture/contract.toml` 里 `path = "crates-ui"` 有**两条**登记：

| id | crates | ci | adr | note 的事实状态 |
|---|---|---|---|---|
| `ui_engine` | **7** | `ui-engine-ci.yml` | `docs/adr/0015-pure-rust-ui-engine.md` | 准确 |
| `ui_core_slice` | **1** | `ui-core-ci.yml` | `docs/adr/0021-consolidation-ui-core-slice.md` | **已过期**：note 写「holds 1 crate (apeireth-ui-core)」与「NOT one of the **30** root members」 |

⇒ `apeireth-ui-core` **本就是那 7 个 crate 之一** ⇒ `ui_core_slice` 是 **`ui_engine` 的子集重复登记**
⇒ 两条对同一路径给出**互相矛盾**的 crate 数（7 vs 1）与成员口径（31 vs 30）。

**裁决**：**保留 `ui_engine`（7 crate）**；**删除 `ui_core_slice`**；`ui_engine` 的 note 补记
「含 `apeireth-ui-core` 切片（来源并行线 ADR-0021）」与「根 members = 31」；**两个 CI 门禁都保留**
（`ui-engine-ci.yml` 与 `ui-core-ci.yml` 并存 ⇒ 见 §5 遗留）。

**理由**：
1. 契约是**结构化登记**，一个路径一条登记才可机检 ⇒ 两条矛盾登记会让「独立 workspace 有几个
   crate」这个问题**没有唯一答案**。
2. 保留 `ui_engine`（**7**）是因为它反映**合并后的真值**；`ui_core_slice` 反映的是**对方线
   单 crate 抽取阶段**的中间态 ⇒ 中间态已被 7-crate 引擎取代。
3. 归属不丢：切片来源写进 note ⇒ **可追溯到并行线 ADR-0021**。

**为什么走 ADR**：契约「新增依赖边/开关/结构必须 ADR + 本文件变更」⇒ 删除一条结构化登记
属结构变更 ⇒ 本文即依据。

## 4. 裁决三：`crates-ui` 的 `Signal::update` 签名变更 —— **认可 breaking，登记为 rc.6 项**

**事实**：合并采用并行线的 `signal.rs` ⇒
`update(&self, f: impl FnOnce(&mut T))` ⇒ `update(&self, f: impl FnOnce(T) -> T)`（闭包形状从
「原地改」变成「返新值」）⇒ **breaking**。

**事实**：`crates-ui` **全 workspace 0 个调用点**（含 6 个兄弟 crate）⇒ 432 测试全绿、编译无破。

**事实**：`apeireth-ui-core` 的 `2.0.0-rc.5` **已在 crates.io 发布且冻结**（契约
`publish_freeze = "crates-ui @ 2.0.0-rc.5"`）⇒ 合并后工作区版本号**仍是 rc.5**
⇒ **同版本号、不同 API** ⇒ 再 publish rc.5 会被 crates.io 拒（版本已存在）。

**裁决**：**认可 breaking**，并**明确登记**：「**rc.5 已发布冻结不变；`update` 的闭包形状变更属于
下一个版本（rc.6）的 breaking 项**」。**不加兼容包装**。

**理由**：
1. 兼容包装（保留旧 `update(&mut T)` + 新增 `map_update`）会让公开面出现**两个近义方法**
   ⇒ 长期债，且「哪个是对的那个」要靠文档说 ⇒ **不划算**。
2. 仓内 0 调用点 ⇒ breaking **当前零影响**；真要兼容，也该在 rc.6 的发布说明里讲清，
   而不是给 rc.5 的 API 留两个壳。
3. crates.io 已发布 rc.5 ⇒ **不可能**再发同一个版本号 ⇒ 「工作区版本号仍写 rc.5」是
   **发布流程的事实**，不是 API 承诺 ⇒ 必须在发布清单上登记，避免误发。

**落地**：登记进 `docs/03-reference/test-baseline.md` 的门禁口径外说明与
`crates-ui` 的发布口径；发布前按 `docs/04-internal/release-checklist.md` 处理版本残留。

## 5. 遗留（不在本文处置范围，如实登记）

1. **两个 UI 门禁并存**：`ui-engine-ci.yml`（本线：check/clippy/fmt/test）与 `ui-core-ci.yml`
   （并行线：check/clippy/fmt/test + doc-test 单独一趟）⇒ **门禁重复**，且 `-j` 口径不同
   （后者 `-j 2`）⇒ 归并属 CI 口径变更，需另批。
2. **契约 note 里的「30 root members」**随本裁决改为 31（ADR-0019 门面 crate 口径）。
3. **本 ADR 只处置「登记冲突」**，不处置任何能力等级；能力等级由
   `docs/status/claims.yaml`（单一真源）承担。

## 6. 契约影响

- `contract.toml` §[[workspace.independent_workspaces]]：**删 1 条**（`ui_core_slice`）⇒
  独立 workspace 登记 **2 → 1**（`crates-ui`）；**crate 数 7 不变**（真值）；
  **根 members 口径 30 → 31**（note 订正，与 ADR-0019 一致）。
- **0 触碰**：`[workspace].members`（31）、`[budgets]`（`max_workspace_crates = 31` /
  `max_production_edges = 118`）、`[env].max_distinct_names`（160）、`[locked]` 5 项、
  生产依赖边集合 —— **本裁决不改任何依赖边、不改任何预算**。

## 7. 验证

- `python3 -c "import tomllib; tomllib.load(open('docs/architecture/contract.toml','rb'))"`
  ⇒ **解析 OK**（无重复键）。
- `python3 scripts/arch_guard.py --strict` ⇒ **VIOLATIONS 0**。
- `python3 scripts/docs_truth.py --verify --check` ⇒ **claims=51 fail=0 stale=0**。
- `python3 scripts/docs_links.py --check` ⇒ **断链 0**。
- `cd crates-ui && cargo test --workspace --locked` ⇒ **432 passed / 0 failed / 4 ignored**
  （本裁决不碰 crates-ui 代码 ⇒ 数字不变，作为回归对照）。
