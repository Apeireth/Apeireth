# Oracle 套件设计哲学（预测决策一体沙盘，2026-08-16）

## 一、概念澄清（先立边界）

- **时序预测**：按时间外推数值（ARIMA/Prophet/augurs）。**不采用**——它是「数字外推」，无因果无情景分支，且产出不可证伪断言，不符合 Apeireth 哲学。顶多作为沙盘内某实体属性的微工具。
- **预测机**：可证伪的沙盘推演器——「如果 X 发生会怎样」的多分支推演，产出**带概率和期限的断言**。
- **决策机**：在推演分支中选优——预测与决策是同一枚硬币的两面（推演各分支 = 预测；分支中选择 = 决策）。**共用一套沙盘底座。**

## 二、参照吸收（全网搜索结论）

| 来源 | 吸收 |
|---|---|
| 炒股预测圈（[AI4Finance/FinGPT](https://github.com/AI4Finance-Foundation/Awesome_AI4Finance)） | **预测必须可证伪**（「在 T 前 X 发生概率 P」）|
| [Metaculus 校准分析](https://www.metaculus.com/notebooks/43356/calibration-adjustment-analysis/) / [Calibration is a Skill](https://www.hotmolts.com/post/calibration-is-a-skill-and-agents-should-train-it-55d0b6c5-3d1a-4ef4-859f-4a1cdb4e2191) | **校准度 > 准确度**：说 70% 的事 10 次里该发生 7 次（Brier score）|
| [arlo learning loop](https://github.com/bedwards/arlo/issues/16) | 预测必须可回放（backtest）、可审计 |
| MCTS 系（[Mulberry](https://github.com/HJYao00/Mulberry)/[MCTS-GRPO](https://github.com/1ring2rta/MCTS-GRPO)/[ReKG-MCTS](https://aclanthology.org/2025.findings-acl.484.)） | **分支推演引擎**：探索分支→快进→评估→回溯 |
| 危机推演（[FALSE FLAG](https://github.com/earlyprototype/false-flag)）/ [Geopol-Modeller](https://github.com/danielrosehill/Geopol-Modeller) | engine-scenario 分离（引擎与场景解耦，正合插件架构）|
| 社会模拟（[AgentSociety](https://agentsociety.readthedocs.io/zh-cn/latest/index.html)） | 多主体推演（后续）|

## 三、架构（预测决策一体，复用基地机制）

```
oracle-suite = 预测决策一体沙盘
├─ 世界状态 WorldState     实体+属性 (+事件溯源, SessionLog 哈希链审计)
├─ 情景引擎 ScenarioEngine 事件注入→状态更新 (规则层确定性 + LLM 裁决 trait 留口)
├─ 分支推演 BranchTree     候选分支→虚拟时钟快进→每分支评分 (MCTS-lite: 一层决策树 v1)
├─ 预测断言 Forecast       「在 T 前 X 发生概率 P」可证伪+期限+对照 resolve
├─ 校准 Calibration        Brier score + BetaBinomial (confidence.rs) — 校准度追踪
├─ 决策 DecisionEngine     期望值选优: Σ P(branch)×V(branch) (expectimax-lite)
├─ 工具 simulate/forecast  AI 可调用 (ToolBridge 注册, 套件能力)
└─ 宪法边界                推演不越 E 层 (硬门 + LLM 评审)
```

**复用**：虚拟时钟（快进分支）/ SessionLog（世界因果链）/ confidence.rs（校准数学）/ evolution_gate（验证+回滚）/ constitution_gate（边界）/ suites（装配）。

## 四、诚实标注（0 装 PASS）

- v1 = **一层决策树 + 期望值**（expectimax-lite），多轮 MCTS 探索-回溯是下一步
- v1 情景引擎 = **规则层确定性 apply**（apply: fn(&mut WorldState, event)）；LLM 裁决（不确定性场景）留 `UncertaintyResolver` trait 口子，未接真
- 校准需真实预测-结果对照积累（先有机制，数据随时间长）
- 多主体推演（AgentSociety 式）后续
