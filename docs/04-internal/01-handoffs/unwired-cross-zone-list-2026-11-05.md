[Document-Meta]
Document: docs/04-internal/01-handoffs/unwired-cross-zone-list-2026-11-05.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-05
Author: deputy

# 跨区孤儿清单（66 条）⇒ **逐条建议，只出清单不动别人的代码**

> **性质**：**建议清单**，不是执行记录。**这 66 条全在别人的区**
> （`capabilities/tools` · `engine/storage` · `foundation/plugin` · `engine/runtime` ·
> `engine/guard` · `adapters/*` · `foundation/*`）⇒ ⇒ **我不出手**，只出证据。
> **口径**：`debt-round8` @ `19c96dfc93e6`，孤儿基线 **118 条**，
> 其中 **已处置 52 条**（带逐条理由）· **待处置 66 条**（本清单）。

## 一句话结论

**66 条按「处置动作」分四类，跨区责任方不同**：
- **17 条 = 微型便利构造**（体长 3～8 行）⇒ ⇒ **建议批量删**（**最省事，收益最大**）；
- **13 条 = `with_*` 注入点**（体长 3～10 行）⇒ ⇒ **建议逐条答「要不要这个旋钮」**；
- **36 条 = 领域操作入口** ⇒ ⇒ **逐条问业务**（这个能力要不要有）。

⇒ ⇒ **建议先做「17 条批量删」** ⇒ ⇒ **零跨区协作、零行为变更**（都是零调用的便利构造）。

---

## 一、17 条**微型便利构造**（建议**批量删**）—— 判据：**体长 ≤ 8 行 + 全仓零引用**

| 符号 | 区 | 体长 | 建议 |
|---|---|---|---|
| `claim_count` | `foundation/governance` | 3 | **删**（计数字段可由 `claims.len()` 得） |
| `has_signal` | `foundation/governance` | 3 | **删或接线**（输入安全信号查询） |
| `matches_word` | `foundation/governance` | 4 | **删或接线**（审批族关键词匹配） |
| `with_task_scope` | `foundation/governance` | 4 | **删或接线**（任务范围注入点） |
| `repetition_policy` | `engine/runtime` | 3 | **删或接线** |
| `family_enforced` | `adapters/gateway` | 3 | **删或接线** |
| `active_profile` | `adapters/cli` | 3 | **删或接线** |
| `adapt_gate` | `adapters/cli` | 3 | **删或接线** |
| `dispatch_gateway_serve` | `adapters/cli` | 3 | **删或接线** |
| `with_system_clock` | `adapters/gateway` | 3 | **删或接线** |
| `with_runtime_overrides` | `foundation/core` | 10 | **删或接线** |
| `serve_canonical` | `adapters/gateway` | 7 | **删或接线** |
| `with_safety_provider` | `engine/guard` | 7 | **删或接线** |
| `recent_actions` | `engine/guard` | 8 | **接线**（行为链最近动作 ⇒ 与 `actions` 配对） |
| `with_metrics_window` | `engine/runtime` | 4 | **删或接线** |
| `execute_streaming` | `engine/runtime` | 15 | ⚠️ **体长偏大 ⇒ 先答「流式执行要不要」** |
| `with_used` | ⚠️ **定义未定位** | — | ⇒ **先查**（`grep` 未命中 `pub fn with_used`）⇒ ⇒ 可能已不存在 |

## 二、13 条 `with_*` 注入点（建议**逐条答「这个旋钮要不要」**）

**判据**：这类是**配置面宽度**。生产装配点已经**做了选择** ⇒ ⇒ 零引用 = 「装配时没拧这个旋钮」。

| 符号 | 区 | 体长 | 建议 |
|---|---|---|---|
| `with_task_scope` | `foundation/governance` | 4 | 同上表 |
| `with_metrics_window` | `engine/runtime` | 4 | 同上表 |
| `with_system_clock` | `adapters/gateway` | 3 | 同上表 |
| `with_runtime_overrides` | `foundation/core` | 10 | 同上表 |
| `with_safety_provider` | `engine/guard` | 7 | 同上表 |

## 三、36 条**领域操作入口**（建议**逐条问业务**）

**判据**：这些**不是冗余**，是「没人用的领域动作」⇒ ⇒ 该问的是「**这个能力这个产品要不要有**」。

### `capabilities/tools`（19 条）—— ⚠️ **工具清单 = 产品契约**
| 符号 | 建议 |
|---|---|
| `doc_ops_capability_shared` / `sheet_ops_capability_shared` | ⭐ **11-04 级联产物** ⇒ ⇒ 理由已写进基线 ⇒ ⇒ **待你裁决「不带 live 策略槽的装配变体要不要」** |
| `explicit_allow_list` | 与 `CapabilitySwitch` 族一起裁 |
| `not_wired_reason` | ⭐ **自曝「未挂载原因」** ⇒ ⇒ 它**只对未接线的功能有用** ⇒ ⇒ 随那个功能的接线/删除一起处置 |
| 其余 15 条 | 逐条问业务（工具面 = 产品契约 ⇒ **决策人是他**） |

### `engine/storage`（8 条）· `foundation/plugin`（6 条）· `foundation/orchestration`（5 条）
### `engine/runtime-assembly`（5 条）· `adapters/sdk`（5 条）· `engine/runtime`（3 条）
### `engine/guard`（3 条）· `adapters/gateway`（3 条）· `adapters/cli`（3 条）
### `foundation/governance`（4 条）· `foundation/core`（1 条）

⇒ ⇒ **逐条问业务**，我在正名候选表里已按 crate 分好（`unwired-canonical-name-candidates-2026-11-04.md` §4.2）。

## 四、📌 给 owner / 总指挥的**选项**

### 选项 A：**先做「17 条批量删」**（**我建议**）
⇒ ⇒ 跨区责任方只需**点头一次**（批量、零行为变更、零测试影响）⇒ ⇒ 基线 **118 → 101**
⇒ ⇒ 代价：动到 6 个 crate ⇒ ⇒ 需**跨区协作**（但改动极小、可 review）

### 选项 B：**只补 66 条理由，不动代码**
⇒ ⇒ 零跨区 ⇒ ⇒ 基线不变，但 **118 条全部有理由** ⇒ ⇒ **债照亮了**
⇒ ⇒ 代价：债没还

### 选项 C：**按 crate 打包，各区各领**
⇒ ⇒ 打包：`tools`（19）· `storage`（8）· `plugin`（6）· `orchestration`（5）·
`runtime-assembly`（5）· `sdk`（5）· `governance`（4）· `runtime`（3）· `guard`（3）·
`gateway`（3）· `cli`（3）· `core`（1）
⇒ ⇒ 每包附**逐条建议** ⇒ ⇒ 各区自己动手 ⇒ ⇒ 协调成本分散
⇒ ⇒ 代价：慢；但**归属清晰**（这是它的强项）

### 选项 D：**等「记忆引擎概念收敛」那轮一起做**
⇒ ⇒ 理由：跨区的这些零引用，有一部分会**在收敛时被自然消掉**（改完 API 形状，很多旧入口就没了）
⇒ ⇒ 风险：等着等着就没人做了

### 我的建议
**A + C 合并**：先把 **17 条批量删**打包发给各区（一次点头）⇒ ⇒ 同时把 **66 条逐条建议**按 crate 分好
⇒ ⇒ 等「概念收敛」那轮再处理剩下的领域入口 ⇒ ⇒ 避免**做两遍**。

## 五、我这次**没有做**什么

| 没做 | 原因 |
|---|---|
| **没动跨区的任何一行代码** | 66 条**全在别人的区** ⇒ ⇒ 我出清单，**他们动手** |
| **没做「概念收敛」的改 API 动作** | 那是**大工程**（133 模块 ⇒ ⇒ 动 API 形状）⇒ ⇒ 需单独排期与 owner 拍板 |
| **没删 `with_used`** | **定义未定位** ⇒ ⇒ 连它是什么都不知道 ⇒ ⇒ **不能盲删** |

## 六、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
grep -c '^orphan ' scripts/unwired-baseline.txt              # 118
grep -cE '^orphan .*# G[0-9]+  .+' scripts/unwired-baseline.txt  # 52（已处置）
python3 scripts/audit_unwired.py --check                     # 一致
```

## 相关文件

- 正名候选表（94 条逐条建议）：[`unwired-canonical-name-candidates-2026-11-04.md`](./unwired-canonical-name-candidates-2026-11-04.md)
- 处置建议书：[`unwired-disposition-proposal-2026-11-04.md`](./unwired-disposition-proposal-2026-11-04.md)
- `DailyNote` 工具规格：[`dailynote-mcp-tool-spec-2026-11-04.md`](./dailynote-mcp-tool-spec-2026-11-04.md)
