# B7+B8 · Phase 6 交付报告：漫游记忆 CRDT + 模块非干扰性（纯研究原型）

> 依据 `_research_mem/ra/00-master-plan.md` §5 Phase 6（"只做研究原型，不进默认路径"）· 2026-09-04
> 基线：`research/baselines/baseline-2026-09-phase0.md`

## 1. 问题定义与假设

- **漫游记忆 CRDT**：多载体离线编辑同一记忆集合后无中心协调地合并；删除必须与治理纪律一致（墓碑而非物理删除）。
- **模块非干扰性**：记忆模块组合时任一模块的操作不得改变其他模块的可观测结果；并发交错等价于某种顺序执行。

## 2. 交付内容（全部 Research 前缀，不进默认路径）

| 交付项 | 位置 | 状态 |
|---|---|---|
| `ResearchRoamingMemory`：LWW-Element-Set + LWW-Register 组合，逻辑时钟 (ts, replica_id)，幽灵删除墓碑、删除后新时钟重建、`merge` = 逐条目 join | `memory/src/research_roaming_memory.rs` | ✅ |
| CRDT 三律验证（交换/结合/幂等）+ merge 顺序确定性（网络重放安全）+ 墓碑防旧时钟复活 | tests ×5 | ✅ |
| `ResearchModule` 抽象 trait + `research_check_non_interference`：全交错枚举（位掩码栈，确定性无 flaky）验证强非干扰 + 交换性 | `memory/src/research_non_interference.rs` | ✅ |
| 样例模块（计数器/集合）+ 20 交错全枚举 + 多规模组合（3×3/4×2/2×4/5×1）零违例 | tests ×3 | ✅ |

**验证**：两模块 8/8 绿；memory 全量 **661/661 绿**；无新依赖、无生产改动。

## 3. 四道门核对

- 等价性门 ✅：生产路径零改动（原型独立模块）。
- 性能门 ✅：无生产路径计算。
- 证据门 ⏳：原型为可行性验证；真实载体（桌面/移动）同步实验与真实 SQLite 跨表隔离测试留后续。
- 术语门 ✅：无"已证明/SOTA"表述；CRDT 性质以"确定性测试验证"表述。

## 4. 已知局限（0 装）

1. 单条内容为 LWW（后写覆盖），不解决文本级并发编辑（需 RGA/序列 CRDT）。
2. 逻辑时钟跨设备依赖真实时钟近似，需 NTP/混合逻辑时钟校准。
3. 非干扰验证的是抽象状态机；真实 `SqliteMemoryStore` 跨表隔离与共享子资源（FTS/文件）需单独验证。
4. 墓碑累积无 GC 压实策略（与治理纪律一致，留后续）。

## 5. Phase 0–6 全景收官

| Phase | 交付 | 状态 |
|---|---|---|
| 0 | research/ 工作区 + 基线 3061 passed + 指标层 + JSONL schema | ✅ |
| 1 | 派生记忆血缘 + 遗忘闭包审计 + GovernedRecall | ✅ |
| 3 | StackPin + ShadowLogger + 离线 replay（竞争比 ≤ k 验证） | ✅ |
| 2 | BTFM 真双时态五元组 + 三类查询 + 信任半环 | ✅ |
| 4 | 校准门控自治（风险阶梯 + hysteresis + shadow） | ✅ |
| 5 | 审批状态机形式化（Dispatched 拆分 + 崩溃模型 + 故障注入） | ✅ |
| 6 | 漫游记忆 CRDT + 模块非干扰（纯原型） | ✅ |
