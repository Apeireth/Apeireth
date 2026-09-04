# B5 · Phase 4 交付报告：校准门控自治（风险优先阶梯 + hysteresis + shadow）

> 依据 `_research_mem/ra/00-master-plan.md` §5 Phase 4 与 `ra4-autonomy-policy-candidates.md` · 2026-09-04
> 基线：`research/baselines/baseline-2026-09-phase0.md`

## 1. 问题定义与假设

- **问题**：固定阈值按风险前缀一刀切——低危但校准差被误放行、高危但校准好仍过度打断。
- **假设**：风险为主序变量，校准/证据只在**相邻层**升降一级（高危不可上调）；快降慢升防边界抖动；冷启动必须退化为固定阈值（等价性门）。

## 2. 交付内容（全部 Research 前缀，默认关闭）

| 交付项 | 位置 | 状态 |
|---|---|---|
| `ResearchAutonomyState` 五态（Autonomous/Consult/RequireApproval/Reject/EnsembleDeliberate）+ 生产三态映射（`to_production_decision`） | `governance/src/research_autonomy.rs` | ✅ |
| 诊断向量（强度档/Wilson 宽度/ECE/漂移/分歧）+ 阈值结构（θ_ce=0.15、θ_ev=0.2，待生产校准） | 同上 | ✅ |
| Proposal A 风险优先阶梯（纯函数；blacklist 恒 Reject、critical/nuclear 恒 RequireApproval 两条硬约束） | 同上 | ✅ |
| Proposal B hysteresis 状态层（快降慢升，K=3 连续窗口；漂移跳过 Consult 直接熔断） | 同上 | ✅ |
| `research_fixed_threshold` 基线 + `ResearchShadowAutonomy` shadow 对比记录 | 同上 | ✅ |
| 单测 10 项（硬约束/阶梯软化/冷启动退化等价/快降慢升/窗口中断/漂移熔断/shadow 分歧/生产映射） | 同上 | ✅ |

**验证**：模块 10/10 绿；governance crate 全量 **117/117 绿**。

## 3. 四道门核对

- 等价性门 ✅：冷启动（calibration=None）路径**逐项断言等于固定阈值**；生产 `GovernancePipeline` / `approval_policy` 零改动。
- 性能门 ✅：纯函数决策，无 IO/无 LLM；hysteresis 状态层 O(1)。
- 证据门 ⏳：误放行率 vs 过度打断率的 frontier 实验（RA-4 §5 矩阵）需 (forecast, outcome, risk) 三元组，留评测批。
- 术语门 ✅：阈值标注"待生产校准初值"；无"已证明/SOTA"表述。

## 4. 已知局限（0 装）

1. 阈值未校准（θ_ce/θ_ev/K 需 RA-4 §5.3 扫参 + bootstrap CI）。
2. Proposal C（LMSR 触发 EnsembleDeliberate）未实现（1–2 人周增量）。
3. shadow 记录只有决策分歧计数；误放行/过度打断需要 outcome 离线回放。
4. drift/disagreement 信号源的实接线（DriftDetector/ensemble 价格）留接线批。
