# B6 · Phase 5 交付报告：审批状态机形式化（模型级验证）

> 依据 `_research_mem/ra/00-master-plan.md` §5 Phase 5 与 `ra5-approval-state-machine-spec.md` / `ra5-formal-proof-plan.md` · 2026-09-04
> 基线：`research/baselines/baseline-2026-09-phase0.md`

## 1. 问题定义与假设

- **问题**：生产 `Claimed` 状态混淆"未派发"与"已派发未落账"（G1）；`Claimed→Interrupted` 缺持久化（G2）；工具无副作用类别声明（G4）。
- **假设**：`Dispatched` 拆分 + durable 前缀崩溃模型 + 副作用描述符可让 InvA（无双副作用）/InvB（批准意图不丢）/InvC（效果不确定强制 fail-closed）被机械验证。

## 2. 交付内容（全部 Research 前缀，默认关闭）

| 交付项 | 位置 | 状态 |
|---|---|---|
| `ResearchApprovalStatus` 七态（含 Dispatched 拆分）+ 终态锁 + Next 关系（非法转移拒绝） | `runtime/src/canonical/research_approval_sm.rs` | ✅ |
| `ResearchApprovalMachine`：P1–P6 持久化点语义、durable 前缀崩溃模型（`simulate_crash`）、恢复处置建议 | 同上 | ✅ |
| 三不变量判定（InvA/InvB/InvC + `all_invariants`） | 同上 | ✅ |
| 副作用描述符 schema（5 类别 + 幂等键 + 补偿 + 缺省 irreversible+reauthorization fail-closed） | 同上 | ✅ |
| `research_allowed_recovery` 类别→约束映射（§7 规则，可机器检查） | 同上 | ✅ |
| 模型级故障注入 harness（6 持久化点 × 崩溃交错 × 100 轮 × 4 种子，零违例） | 同上 | ✅ |
| Kani harness ×3（`#[cfg(kani)]` 门控，本机未装工具链，待 `cargo kani`） | 同上 | ✅ |

**验证**：模块 10/10 绿；runtime 全量 **90/90 绿**；故障注入 2213+ 步/种子 × 4 种子，不变量违例 = 0。

## 3. 规格裁定（0 装，与 RA-2/RA-4 同模式）

RA-5 规格原文有**两处自相矛盾**（InvA 的 `Interrupted ⇒ executed=TRUE` 与自身 `Claimed→Interrupt` 动作冲突；InvB 不允许 `Interrupted` 却有自己的 Interrupt 动作产生之）。本实现按语义修正并注释：
- InvA：`Dispatched/Consumed ⇒ executed`；`executed ⇒ ∉ {Pending, Claimed, Rejected, Expired}`；`Interrupted` 允许 executed=false（无副作用发生过）。
- InvB：增加 `approved` 前提字段（对应规格 `decision=Approve`）；`Interrupted` 视为显式 fail-closed 终态而非静默丢失（与规格 liveness `◇(Consumed ∨ Interrupted)` 一致）。

## 4. 四道门核对

- 等价性门 ✅：生产 `approval.rs` / `execute.rs` 零改动（G1–G7 差距未改码，仅模型验证）。
- 性能门 ✅：纯内存状态机，生产路径零新增计算。
- 证据门 ⏳：TLA+/TLC 真机验证与 Kani 运行待工具链；M4 真实进程 killpoint 注入待接。
- 术语门 ✅：明确"模型级验证 ≠ 生产验证"；fsync 语义只测不证（Pillai et al. OSDI'14）。

## 5. 已知局限（0 装）

1. 本模块是模型级验证，生产代码的 G1–G7 差距修复（改 `approval.rs`/`execute.rs`）需单独排期。
2. 崩溃模型为 durable 前缀布尔抽象，未建模真实 fsync 行为。
3. Kani 未装（harness 已写好）；TLA+/TLC 未跑。
4. 故障注入为模型级交错，非真实进程 killpoint（M4 待接）。
