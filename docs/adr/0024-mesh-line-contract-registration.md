# 0024. 登记 mesh 跨节点栈的层归属、机制内核依赖与规模上限

[Document-Meta]
Document: docs/adr/0024-mesh-line-contract-registration.md
Class: DECISION
Version: 0.1
Last-Modified: 2026-11-11
Status: Accepted
Author: Xephylos

## 0. 批准记录

- **批准**：本次会话总指挥（依据 owner 的常设授权「下一批都要挨个解决，按你的建议来」「让子代理把 PR 合并好」），日期 **2026-11-11**。
- **批准范围**：本文第 2 节所列的契约改动；不含任何例外条目、不含守卫阈值语义变更、不含度量口径变更。
- **授权边界**：本决策只允许「上限 == 实测」的显式对齐与新增 crate 的层归属登记；任何进一步的规模扩张仍需新 ADR。

## 1. 背景与问题

PR #117（`feature/mesh-rc2-integration-rebased`）把 mesh 跨节点栈并入主 workspace。实测（2026-11-11，
`python3 scripts/arch_guard.py --strict`；同一命令在 upstream/main 侧 EXIT=0）：

- workspace crate 30 → 34（新增 `apeireth-mesh` / `apeireth-mesh-protocol` / `apeireth-mesh-quic` / `apeireth-mesh-rendezvous`）；
- 生产依赖边 111 → 125（新增 14 条）；
- `arch-strict` 报 7 条违规：4 条「mesh crate 未登记进 `[layers]`」+ 1 条「`apeireth-runtime -> apeireth-mesh` 未登记 `[kernel].allowed_production_deps`」+ 2 条「规模上限 30 / 111 已被实测超过」。

> **口径说明（2026-11-11 合并后树重测，勿混用两套数字）**：上列 **30 → 34 / 111 → 125** 是**本线并入前的主干口径**
> （upstream/main@`7d0a8111`）⇒ 是历史记录，**不追溯改写**。本线随后按 §2.3 的规则把 upstream/main@`7abfa282`
> （含门面 crate `apeireth`，ADR-0019）合并进本分支，并**按合并后树重新测量** ⇒ ⇒ 合并后树实测为
> **35 crate / 132 生产边**（明细见 §2.5）⇒ ⇒ **契约现值以 §2.5 为准**。

契约自己写明：W10 结构/方向类违规**永远不走例外通道**（`[exceptions].structural_budget_weight = 0`），只能 ADR 改契约；
`[budgets]` 是「上限 == 实测」的只减不增棘轮，新增边需 ADR 并先问总指挥 ⇒ ⇒ 现状要么登记，要么这条线永远红。

## 2. 决策

1. `[layers]`：`apeireth-mesh-protocol` → **foundation**；`apeireth-mesh` / `apeireth-mesh-quic` / `apeireth-mesh-rendezvous` → **engine**。
2. `[kernel].allowed_production_deps` 增加 `apeireth-mesh`。
3. `[budgets].max_workspace_crates` 30 → **34**、`max_production_edges` 111 → **125**（对齐本树实测，保持「上限 == 实测」口径）。
   ⇒ ⇒ **合并后树（upstream/main@`7abfa282` 已合入）实测 35 / 132 ⇒ 契约现值按 §2.5 更新。**
4. `[workspace].members` 30 → **34**、`product_crates` 29 → **33**（现值早已过期，本次一并对齐）。
   ⇒ ⇒ 合并后树实测 `members = 35`、`product_crates = 34`（35 − bench harness）。
5. `[deps].allowed_layer_edges` **不新增**（现有表已覆盖全部 14 条 mesh 边）。

不做的事：不改守卫阈值语义、不加例外条目、不删 mesh crate、不把 mesh 拆出主 workspace、不新开 `services` 层。

### 2.1 层归属的判定依据（按依赖方向，不是按目录名）

- `apeireth-mesh-protocol` 只定义协议类型、无本地状态，与 `apeireth-protocol` 同层 ⇒ foundation。
- `apeireth-mesh` → core / protocol / mesh-protocol / plugin / governance ⇒ engine→foundation，方向合法。
- `apeireth-mesh-quic` → core / mesh-protocol / mesh / plugin / protocol / governance / runtime ⇒ engine→engine，方向合法。
- `crates/services/mesh-rendezvous` → mesh-protocol / core / protocol / plugin / governance / mesh / runtime：它是 server 侧**运行时二进制**（`src/main.rs`），不是独立服务层 ⇒ 归 engine，其依赖方向全部落在既有白名单内。

### 2.2 为什么 `kernel -> mesh` 是对的方向（以及代价）

`apeireth-runtime` 是机制内核。mesh 线要求内核在**节点侧装配** peer 身份与传输装配 ⇒ 该能力必须从内核侧可见，
否则要么把装配逻辑复制进每个调用点，要么让 mesh 反向依赖内核（而 `apeireth-mesh-quic` / `mesh-rendezvous` 已经依赖
`apeireth-runtime`，反向会形成环）⇒ ⇒ 方向只能是 `kernel -> mesh`。

**代价（明确接受）**：内核边界从此**知道** mesh ⇒ 今后 mesh 侧的结构变更会触及内核边界，守卫与 ADR 成本随之上升 ⇒ ⇒
将来若要把这条边撤掉，需要一份新的 ADR 来评估面，不能顺手改。

### 2.3 规模上限的序列口径（两条线并发时以各自实测为准）

契约里的上限是「上限 == 实测」。本决策只登记**本树实测值**（34 / 125）。另一条线（门面 crate `apeireth`，ADR-0019，
其分支实测 31 / 118）与本线并发推进 ⇒ ⇒ **合入顺序上，后合入者必须 rebase 到新的 main 并按合并后树重新测量**，
再更新 `[budgets]`（例如后合入者需把上限更新为两条线合并后的实测值）⇒ ⇒ 任何一条线都不得预支另一条线的名额。

**实际发生（2026-11-11 收口）**：本线（mesh，ADR-0024）是**后合入者** —— upstream/main@`7abfa282`
（含门面 crate `apeireth`，ADR-0019）先落地 ⇒ ⇒ 本线按本节规则把该 main 合入本分支并**按合并后树重新测量**，
再更新 `[budgets]`（见 §2.5）⇒ ⇒ **两条线都没有预支对方的名额**：门面线的 7 条边与 mesh 线的 14 条边
在合并树上相加 = **132**，crate 数 31 + 4 = **35**。

### 2.4 备选选项与建议（owner 拍板依据）

| 选项 | 做法 | 后果 | 建议 |
|---|---|---|---|
| A | 把 mesh 线登记进契约：层归属 + 内核依赖 + 上限对齐实测 | 7 条违规清零；上限一次性上调 4 crate / 14 边并写进本 ADR，不再是隐性膨胀 | **建议采用** |
| B | 加例外条目压住违规 | 契约明文禁止结构/方向类走例外通道（`structural_budget_weight = 0`）⇒ 走不通 | 建议拒 |
| C | 把 mesh 栈拆成独立 cargo workspace | `apeireth-runtime` 生产依赖它 ⇒ 制造跨 workspace 生产边，守卫与语义都更糟 | 建议拒 |
| D | 撤掉 mesh crate / 删 mesh 线代码让门禁过 | 删功能换绿灯，PR #117 的既定目标作废 | 建议拒 |

### 2.5 合并后树重测（2026-11-11；upstream/main@`7abfa282` 已合入本分支）

**实测口径**：`python scripts/arch_guard.py --strict`（EXIT=0，VIOLATIONS: 0）
+ `cargo metadata --locked --no-deps --offline`（workspace 内、kind ≠ dev 的生产边）。

| 口径 | upstream/main@`7abfa282` | 本线（mesh）新增 | **合并后树实测** | 契约现值 |
|---|---|---|---|---|
| workspace members | 31 | +4（`apeireth-mesh` / `apeireth-mesh-protocol` / `apeireth-mesh-quic` / `apeireth-mesh-rendezvous`） | **35** | `max_workspace_crates = 35`、`members = 35` |
| 生产依赖边 | 118 | +14 | **132** | `max_production_edges = 132` |
| 能力开关（守卫 src 口径） | —（预算 154） | — | **152**（原始 findall 164；登记面 164 条**全部已登记** ⇒ 登记面零缺口） | `max_distinct_names = 154`（**未放宽**） |

- **合并口径**：合并后树的生产边 = 门面线 **7** 条（ADR-0019）+ mesh 线 **14** 条（ADR-0024）**在合并树上相加**
  ⇒ ⇒ 上限与实测重新对齐，**任何一条线都没有预支另一条线的名额**（与 §2.3 一致）。
- **`[env]` 未放宽**：能力开关实测 152 ≤ 预算 154 ⇒ 预算**保持 154**（只减不增棘轮；登记面零缺口 ⇒ 无需扩增）。
- **层归属不变**：mesh 4 个 crate 的层归属（`mesh-protocol` → foundation；`mesh` / `mesh-quic` /
  `mesh-rendezvous` → engine）与 `[kernel].allowed_production_deps += apeireth-mesh` 在合并树上**原样成立**
  ⇒ ⇒ 由 `arch_guard --strict` 的 **VIOLATIONS: 0** 证明（合并后树实测 `crates=35 edges=132 env=152`）。
- **基线重生成**：合并后树变了 ⇒ `scripts/arch-baseline.txt`（`arch_guard.py --update-baseline`）与
  `scripts/crate-budget-baseline.json`（`check_crate_budget.py --gen`）**按脚本重生成**，未手改基线。
- **二次合并后树复核（2026-11-11，upstream/main@`72f826634`（PR #115 已合入）再次并入本分支）**：
  按合并后树重跑 `python scripts/arch_guard.py --strict` ⇒ EXIT=0、`VIOLATIONS: 0`、
  `crates=35 edges=132 env=152 max_src_loc=6255`
  ⇒ ⇒ **实测与 §2.5 表一致（35 crate / 132 生产边 / 152 能力开关）⇒ 契约现值无需改动**
  （`[budgets].max_workspace_crates = 35`、`max_production_edges = 132`、`[env].max_distinct_names = 154`
  全部保持原值 ⇒ **未放宽任何棘轮**）。
- **三次合并后树复核（2026-11-11，upstream/main@`f3cad212a`（PR #118 已合入）再次并入本分支）**：
  按合并后树重跑 `python scripts/arch_guard.py --strict` ⇒ EXIT=0、`VIOLATIONS: 0`、
  `crates=35 edges=132 env=152 max_src_loc=6255 exceptions=2`
  ⇒ ⇒ **数字未变**（35 crate / 132 生产边 / 152 能力开关 / 最大 src 6255，与 §2.5 表一致）
  ⇒ ⇒ **契约现值无需改动**（`[workspace].members = 35`、`product_crates = 34`、
  `[budgets].max_workspace_crates = 35`、`max_production_edges = 132`、`max_file_loc = 6300`、
  `[env].max_distinct_names = 154` 全部保持原值 ⇒ **未放宽任何棘轮**）；
  另跑 `python scripts/check_crate_budget.py --check` ⇒ EXIT=0（34 个 crate · 合计 581317 行，
  `scripts/crate-budget-baseline.json` **无需重生成**）。
  ⇒ 本次 main 侧变更只落在 `crates-ui/**`（独立 cargo workspace）、`docs/04-internal/**`、
  `docs/status/**` 与 `.github/workflows/{rust,kani}.yml` 的 paths 过滤
  ⇒ ⇒ **未触碰主 workspace 的 crate / 生产依赖边 / 能力开关** ⇒ 数字未变符合预期（§2.3 口径）。

## 3. O-6 三阶审查

- **总体最优**：mesh 跨节点能力已进入生产装配路径，继续让它「在契约之外」等于把架构债留给下一任代理；一次登记把 7 条违规清零，代价是 4 个 crate + 14 条边的**显式可见**（写进 ADR 与契约，不藏）。**拒了**「删掉 mesh crate 让门禁过」——那是删功能换绿灯；**拒了**「加例外条目压住」——契约明文禁止结构类走例外。
- **系统最优**：mesh-protocol 放 foundation（纯协议类型、无状态，与 `apeireth-protocol` 同层）、mesh / mesh-quic / rendezvous 放 engine（运行时能力），依赖方向全部落在既有 `foundation->foundation` / `engine->foundation` / `engine->engine` / `adapters->engine` 白名单内 ⇒ 不新增方向白名单。**拒了**「新开 services 层」——当前 `crates/services/` 只有 1 个 crate，单独立层是过度抽象。
- **架构最优**：层归属由「是否持有运行时状态与装配责任」判定，而非按目录名；`apeireth-mesh-rendezvous` 是 server 侧运行时而非独立服务层，归 engine 才能让 `engine->foundation` 覆盖它对 mesh-protocol 的依赖。**拒了**「把 mesh 拆成独立 cargo workspace（像 crates-ui 那样）」——`apeireth-runtime` 生产依赖它，拆出去会制造跨 workspace 生产边，守卫与语义都更糟。

## 4. 迁移计划

1. 改 `docs/architecture/contract.toml` 的 `[layers]` / `[kernel]` / `[budgets]` / `[workspace]` — 验证：`python3 scripts/arch_guard.py --strict` ⇒ VIOLATIONS: 0
2. `python3 scripts/arch_doc.py --gen`（归 LF）— 验证：`python3 scripts/arch_doc.py --check`
3. `python3 scripts/gen_gate_manifest.py --gen`（归 LF）— 验证：`python3 scripts/gen_gate_manifest.py --check`
4. `python3 scripts/check_crate_budget.py --check` ⇒ EXIT=0

## 5. 回滚

```powershell
git revert <本 ADR 提交>            # 契约与生成物一并回退
python3 scripts/arch_doc.py --gen    # 重新生成 ARCHITECTURE.generated.md
python3 scripts/gen_gate_manifest.py --gen
```

回滚后 `[layers]` 不含 mesh crate ⇒ arch-strict 重新报「mesh crate 未登记 / 规模超限」（预期行为，合并裁决会红）。

## 6. 验收

- [x] `python3 scripts/arch_guard.py --strict` 通过，VIOLATIONS: 0（本次 +4 crate / +14 边已在本文明示）
- [x] **合并后树重测**（2026-11-11，upstream/main@`7abfa282` 已合入）：`crates=35 edges=132 env=152`，
      契约现值 35 / 132 已按 §2.5 更新（`[env]` 预算 154 未放宽）
- [x] **二次合并后树重测**（2026-11-11，upstream/main@`72f826634`，PR #115 已合入）：
      `crates=35 edges=132 env=152` ⇒ **数字与 §2.5 一致 ⇒ 契约现值无需改动**
      （`max_workspace_crates = 35` / `max_production_edges = 132` / `[env] max_distinct_names = 154` 全部保持）
- [x] **三次合并后树重测**（2026-11-11，upstream/main@`f3cad212a`，PR #118 已合入）：
      `crates=35 edges=132 env=152 max_src_loc=6255` ⇒ **数字未变 ⇒ 契约现值无需改动**
      （`[workspace].members = 35` / `product_crates = 34` / `max_workspace_crates = 35` /
      `max_production_edges = 132` / `max_file_loc = 6300` / `[env] max_distinct_names = 154` 全部保持）
- [x] `python3 scripts/check_crate_budget.py --check` / `gen_gate_manifest.py --check` / `arch_doc.py --check` 通过
- [x] 契约条款更新（Status: Accepted）
- [ ] 文档随行：`docs/status/mesh-rc2-status.md`「还差什么」中的 arch-strict 条目改为已决
- [x] LOCKED 5 项 0 触碰 / 0 新外部 dep

## 7. 后果（Consequences）

- 正面：arch-strict 归零；「合并裁决（唯一）」job 可绿；mesh 线的层归属有了唯一真源。
- 负面 / 接受的代价：workspace 规模上限与生产边上限一次性上调 4 / 14（写进 ADR，不再是隐性膨胀）；内核边界从此知道 mesh（见 §2.2）；规模棘轮的下一步仍要求真拆。
- **合并后树口径（2026-11-11）**：上游 main 先落地（门面 crate `apeireth`，ADR-0019）⇒ ⇒ 本线是后合入者
  ⇒ ⇒ 契约现值按合并后树实测 **35 crate / 132 生产边**（§2.5）⇒ ⇒ 两条线的名额**未互相预支**。
- 后续 stage：mesh 栈按 `docs/design/mesh-rc2-status.md` 的「还差什么」继续收口（生产装配、真实 NAT 实测）。
