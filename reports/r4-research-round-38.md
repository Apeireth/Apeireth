# R4 · Round38 调研报告
主题 C1 因果推断 + Pearl do-calculus。why now: 29 轮无 do-calculus/Pearl/causal inference 关键词; R17/R28 Friston 仅 Bayesian 变分 ≠ 实际因果; V1076 真 LLM 路由无因果信号。R3 推荐 C1

## 12 Query (10/12 命中, 总 50; Q1+Q11 空)
1 Pearl do-calculus [空] · 2 CBN → CausalTrace/CausalPulse · 3 Counterfactual Planning + ToolAnchor · 4 SCM → Causal Agent Replay + AI-Native Causal Platform · 5 Causal discovery → GNN brain + fMRI · 6 Pearl ladder → Three Layer Causal Hierarchy · 7 Actual causation Hall-Winston → "causal commit log not tool traces" · 8 DoWhy → py-why/dowhy · 9 ananke → uber/causalml + ananke · 10 EconML · 11 反事实幻觉[空] · 12 因果归因 → Causal commit log

## 3 跨域亮点
- Q4: "Causal Agent Replay: Counterfactual Attribution for LLM-Agent Failures" 直对齐 V1082 audit, agent failures 因果反事实归因模式
- Q3+Q7: ToolAnchor counterfactual context 锚定 + "causal commit log not tool traces" → 改 V1076 路由 + V1082 审计
- Q6: Three Layer Causal Hierarchy (Pearl L1-L3) → agent 推理分层 substrate, 直灌 V1083 决策路由

## 2 Gap 借鉴点
- Gap A 反事实幻觉: Q11 唯一空查询, **未在 2026 公开文献形成方法**, Apeireth 可抢首发 (V1082 audit 加反事实自检)
- Gap B 因果归因透明: Q4+Q7 "causal commit log" → V1076 加因果归因字段 + V1082 改 tool trace → causal commit log

## 3 GitHub 真读项目
py-why/dowhy (MS Research 4 步 API model/identify/estimate/refute) · uber/causalml+ananke-causal (uplift+graph) · microsoft/EconML (CATE/ITE)

## R39 主题建议
C2 RL 基础 + MuZero/Silver (R1 末位, V1083 balanced policy 缺理论背书; muzero-general/cleanrl/SB3 待深读)。或重补 Q1+Q11。

## cron 同步跳 record
cron-research-runs.jsonl 末行 22:03 `done` (R4-RES-03 手动, 10/12 AnySearch; Q1+Q11 空)。文件: research-v7-round-38.json 44887B + round-38-runner.py 5419B。边界: 不动 R8-R37。

— 调研专家 · R4-RES-03