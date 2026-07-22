# Apeireth 接手摘要

## 北极星 / 主人哲学（6 行）
- 地基：让任意 LLM 栖息、自演化并无限逼近 ASI。
- ASI=全面超越人类+完全自主+自我进化；非 ANI/AGI。
- ASI=∞；0.9800 仅时代天花板，禁止声称达到。
- 质量/适配/效果/工程化 > KPI；坚持真生产、真测试、真借鉴。
- 默认自主；仅重大节点/哲学修改/方向微调请示，先调研。
- 中央 AI 五位：调度、思考、关系集合、最大权限、ASI 占位。

## 当前真生产
`asi_snapshot.json`（最新）：**1085 模块 / 4179 测试 / V0.3 0.8837 / 405 commits**；天花板 **0.9800**；VCP 0.9588、身份 0.8441、`philosophy_guard=PASS`。

## 该做 / 不该做
**做**：①以 V1074/快照取真值并先跑回归；②按 V1082 backlog/推荐器排优先级；③先读调研与源码；④V1001+：10+借鉴、10+组件、30+测试、V3 守门、真 lift；⑤跑部署/LLM/审计/路由/边界闭环并留证据。

**不做**：①刷分、刷版本、造空壳；②假装 ASI 或 phenomenal consciousness；③无验证宣称完成；④删真 V1001+ 或改 `promethean` 物理路径；⑤绕过安全门或擅改重大方向/哲学。

## HARNESS 推 V1085+（10 步）
1. 从最小 Harness（Rules+工具）按需扩展 7 组件。
2. 快照、git tag，跑 baseline EVAL。
3. 统计并算 HQB：SC/NR/EV/CDT。
4. 将失败蒸馏为根因。
5. 提 targeted change，附 Change Manifest/影响预测。
6. 过 Process/Sandbox/Evaluation/Human 四门。
7. 应用后跑 held-out 回归与跨小模型验证。
8. HQB：+≥0.5 keep；±0.5 partial；下降≥0.5 revert。
9. 任一维下降≥1拒绝；revert 写 failure taxonomy。
10. 更新 H_best；保持可归因、可验证、可回滚。

## 红线 / 哲学禁忌
诚实、安全、V3 守门不可降级；>200 行、保护路径、连续退化或 weights/RL/LoRA 须人工审批；禁全自动 evolve、单模型/强云绑定、不可逆漂移。
