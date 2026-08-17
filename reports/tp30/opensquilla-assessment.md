# OpenSquilla 评估

## 机制（What it does）

- 核心功能：分布式多 Agent 协作框架，强调 Agent 间消息路由 / 任务分发 / 状态共享
- 解决什么问题：单 Agent 能力上限 → 多 Agent 分工 → 复杂任务分解
- 关键技术：消息总线 + Agent 注册中心 + 任务调度器 + 共享 context store

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-bus`（消息总线基础设施）
  - `apeireth-agent`（单 Agent 调度）
  - `apeireth-runtime`（含 7 模块编排：heartbeat/task/bus/arbitration/search/group_chat/emotion）
  - `apeireth-council`（多 Agent 治理）
  - `MetaGPT`（已 clone 在 `research/source/MetaGPT/`）— SOP 化多 Agent 协作参考
- 差异化优势：
  - OpenSquilla 强调「分布式」（跨节点），APEIRETH 暂无跨节点 Agent 编排（仅单进程）
  - OpenSquilla 任务分发走 message passing，APEIRETH 走 bus + arbitration
- 可借鉴：
  - Agent 注册中心模式（apeireth-agent 加 `AgentRegistry` 模块，统一按 capability 检索）
  - 任务分解 + 回滚（apeireth-action 的 TxId + rollback 机制可参考 OpenSquilla 的 sub-task 回退设计）

## 吸收建议（Action items）

- P0 立即做：**不动**。OpenSquilla 解决的是「跨节点分布式」，APEIRETH 当前战场是「单进程高内聚」，错位。
- P1 评估后做：若 W3+ 主人想跨设备协作（手机+PC+音箱），可借鉴 Agent 注册中心设计。
- P2 长期调研：暂列观察项。
- 不做（重复 / 价值低）：APEIRETH 已有 `apeireth-runtime` 的 7 模块端到端编排 + `apeireth-council` 多 Agent 治理；OpenSquilla 不补核心缺位。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无源码；本评估基于 GitHub README 公开信息 + 同类项目（MetaGPT）对照推理。
- 未调研不写结论：本评估的「差异化优势」「可借鉴」「不做」均为推理判断，**非实测结论**。如需落地建议，必须先 `git clone https://github.com/.../OpenSquilla research/source/OpenSquilla` 实测 README + src/ 后重写本评估。