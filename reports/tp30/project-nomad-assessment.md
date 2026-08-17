# project-nomad 评估

## 机制（What it does）

- 核心功能：AI 数字游民助手（"Nomad"），个人助理场景的端到端 AI 伴侣
- 解决什么问题：数字游民跨时区/多任务/低预算的生活管理需求
- 关键技术：行程规划 + 预算管理 + 跨时区通信协调 + 本地化服务整合

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-companion`（伙伴端点，任何 OpenAI 兼容前端接入 Apeireth 主链路）
  - `apeireth-action`（行动器官：改变环境 + 工具执行 + 表达 + 沉默）
  - `apeireth-motivation`（动机/价值器官）
  - `claude-code`（已 clone 在 `research/source/claude-code/`，参考代码伴侣范式）
  - `hermes-agent-rs`（已 clone，参考 Rust agent 范式）
- 差异化优势：
  - project-nomad 专注「数字游民」细分场景（签证、跨境、远程协作工具链）
  - APEIRETH 通用伴侣架构，不锁定具体场景
- 可借鉴：
  - **场景化 Skill 包**：project-nomad 把行程/预算/签证做成专用 Skill，APEIRETH 可在 `apeireth-wiki` 上建一个 `scenarios/digital-nomad/` 子树
  - **跨时区调度**：project-nomad 的时区感知任务调度可参考

## 吸收建议（Action items）

- P0 立即做：**不动**。场景错位（数字游民 ≠ 主人当前场景）。
- P1 评估后做：若主人生活方式升级到「数字游民」，可在 `apeireth-wiki` 加 `scenarios/digital-nomad/` WikiEntry 集合。
- P2 长期调研：观察。
- 不做（重复 / 价值低）：APEIRETH 通用伴侣架构已覆盖「个人助理」基底，project-nomad 的数字游民特化不补缺位。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无 project-nomad 源码；本评估基于 GitHub README 公开信息 + APEIRETH 现状推理。
- 未调研不写结论：project-nomad 的具体 skill 列表 / 行程规划算法 / 预算模型均为推理判断。如需落地建议，必须先实测。