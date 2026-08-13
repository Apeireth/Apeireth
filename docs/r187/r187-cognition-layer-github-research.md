# R187 GitHub 优秀项目调研 — 认知层 (consciousness / cognition / perception)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R187
> **日期**: 2026-08-13
> **范围**: apeireth-consciousness + apeireth-cognition + apeireth-perception 3 crate
> **状态**: 调研为升级预备.

---

## 0. 现状

### apeireth-consciousness (3 文件 48KB)
- emotion.rs (17KB) — 情绪引擎 (PAD 情感模型)
- lib.rs (15KB) — 6 状态机 (Awake/Reflecting/Dreaming/Meditating/SelfDisabling/Recovering)
- transfer_monitor.rs (15KB) — 状态转换监控

### apeireth-cognition (4 文件 30KB)
- decision.rs (3KB) — 决策
- reflection.rs (4KB) — 反思
- scoring.rs (5KB) — ASI 评分 (V0.5/V1136)
- lib.rs (15KB) — 12 键 verdict 守门

### apeireth-perception (4 文件 29KB)
- attention.rs (4KB) — 注意力 (TopK/Threshold)
- channel.rs (9KB) — 5 通道 (Text/Voice/Vision/Tactile/Command)
- input.rs (9KB) — 5 种输入类型
- lib.rs (6KB) — 入口

**已实现能力**:
- 6 状态机 + 合法转换矩阵
- PAD 情绪模型 (Pleasure-Arousal-Dominance)
- 12 键 verdict 守门
- 5 模态输入 (Text/Voice/Vision/Tactile/Command)
- 2 注意力策略 (TopK/Threshold)
- ASI 评分 (V0.5/V1136)

**已经领先很多项目**:
- 6 状态机比大多数 LLM agent 的 \"working / waiting / done\" 三态细
- 12 键 verdict 守门业界独一档
- PAD 情绪模型对 LLM agent 是少见的

---

## 1. AI 状态机 / Agent Runtime SOTA

### 1.1 LangGraph (再, R180 提过) — 学习

- 状态机作为一等公民
- 我们的 6 状态机可借鉴其 persistent state + checkpoint 设计

### 1.2 AutoGen 0.4+ — 学习

- Agent lifecycle: idle / thinking / acting / done
- **简化, 不如我们 6 状态细**

### 1.3 Temporal (再) — 学习

- 长时间运行的 stateful workflow
- 我们 consciousness 状态机跨会话状态持久化可借鉴

### 1.4 Statefun (Apache Flink Stateful Functions) — 学习

- actor model + stateful functions
- **学习点**: state 与 function 分离

---

## 2. AI 情绪 / 情感计算 SOTA

### 2.1 PAD 情感模型 (Mehrabian-Russell) — 当前用

- Pleasure-Arousal-Dominance
- 我们 emotion.rs 已经在用
- **学习点**: Russell 的 Circumplex Model (8 情绪分布)

### 2.2 Plutchik 情感轮 — 学习

- 8 基础情绪 + 8 高级情绪 + 强度
- **价值**: 我们情绪可分基础 + 高级两层

### 2.3 Ekman 6 情绪 — 学习

- 喜/怒/哀/惧/恶/惊
- 比 PAD 简单, 应用更广

### 2.4 GoEmotions (Google) — 学习

- 27 情绪分类 (细粒度)
- **学习点**: 我们情绪可加 27 类细分

### 2.5 Hume AI (hume) — **学习 (商业产品)**

- 商业情感识别, 语音+文本+表情
- **不集成**: 商业闭源
- **学习点**: 情绪作为可观测的输出

---

## 3. 注意力机制 SOTA

### 3.1 Transformer (原始, attention is all you need) — 基础

- 我们 TopK/Threshold attention 是在 LLM 之外的注意力
- **学习点**: Sparse attention 优化 (BigBird / Longformer)

### 3.2 Reformer (Google) — 学习

- LSH attention
- 不直接相关, 但学习 LSH 思想

### 3.3 Perceiver / Perceiver IO (DeepMind) — **学习**

- 跨模态通用架构
- **学习点**: 我们 perception 5 模态融合可借鉴

### 3.4 Flamingo / BLIP-2 (DeepMind / Salesforce) — 学习

- vision-language model
- 我们 perception Vision 通道可借鉴

### 3.5 Whisper / CLIP / ImageBind (Meta) — 学习

- 多模态嵌入
- **学习点**: 我们 channel.rs 多模态融合

---

## 4. LLM 推理 / 决策 SOTA

### 4.1 DSPy (再, R184 提过) — 学习

- type-safe signature + optimizer
- 我们 scoring.rs 可借鉴

### 4.2 Chain-of-Thought / Tree-of-Thought / ReAct — 学习

- 经典 CoT 论文
- ToT (Yao et al. 2023) 树搜索
- ReAct (Yao et al. 2022) 推理 + 行动
- **价值**: 我们 decision.rs 可加 ToT 树搜索

### 4.3 Reflexion (再, R186 提过) — 学习

- 反思强化学习
- 我们 reflection.rs 已类似

### 4.4 Self-Refine (Stanford) — 学习

- 自我迭代改进
- **价值**: 我们 reflection.rs 加迭代循环

### 4.5 Voyager (再, R186 提过) — 学习

- 终身学习
- **价值**: 我们 reflection 累积成 skill library

### 4.6 LATS (Language Agent Tree Search) — **学习**

- MCTS + LLM agent
- **价值**: 我们 decision.rs 加 MCTS 树搜索

### 4.7 SwiftSage (MIT) — 学习

- 快速 + 慢速双 agent
- **学习点**: 我们 consciousness 6 状态可借鉴

### 4.8 Cognitive Architectures (ACT-R / SOAR) — **学习**

- 经典认知科学架构
- **学习点**: 人类认知模型设计哲学

---

## 5. 感知 / 输入融合 SOTA

### 5.1 LangChain MultiModal — 学习

- 文本 + 图像 + 音频输入
- **学习点**: 我们的 PerceptionInput trait 可借鉴

### 5.2 GPT-4V / Claude 3.5 vision — 学习

- 商业多模态
- **学习点**: vision channel 设计

### 5.3 Whisper / OpenAI TTS — 学习

- 语音输入输出
- 我们 voice channel

### 5.4 ImageBind (Meta) — **学习**

- 6 模态统一嵌入
- **学习点**: 跨模态检索

### 5.5 robotics perception (ROS2) — 学习

- 多 sensor 融合
- **价值**: 我们 channel.rs 工业级

---

## 6. 自我意识 / 反思 SOTA

### 6.1 Generative Agents (再, R186 提过) — 必读

- 记忆流 + 反思 + 规划
- **价值**: 完全契合 consciousness 6 状态

### 6.2 Self-Awareness Survey (Anthropic) — 学术

- 自我意识 3 维度
- **学习点**: 我们 consciousness 6 状态可对应

### 6.3 Theory of Mind 评估 — 学术

- Sally-Anne 测试
- **学习点**: council advisor TheoryOfMind?

### 6.4 Constitutional AI (再, R180 提过) — 学习

- 自我批评 + 修订
- 我们 reflection.rs 强化

---

## 7. 升级方案 (R187+ 实施)

### 7.1 短期 (1-2 days)

1. **5 模态融合**: 借鉴 Perceiver IO, 我们 channel.rs 跨模态统一嵌入
2. **Plutchik 8 情绪**: emotion.rs 加 8 基础情绪分类
3. **LATS 树搜索**: decision.rs 加 MCTS 树搜索

### 7.2 中期 (3-5 days)

4. **Self-Refine 迭代**: reflection.rs 加迭代循环
5. **SwiftSage 快速+慢速**: consciousness 加双 agent 模式
6. **GoEmotions 27 分类**: emotion.rs 细粒度
7. **Cognitive Architectures 集成**: 哲学上的人类认知模型

### 7.3 长期 (持续)

8. **Self-Awareness 3 维评估**: 加 consciousness 自我报告
9. **Theory of Mind**: council 加 TheoryOfMind advisor
10. **ImageBind 跨模态**: perception 6 模态统一

---

## 8. 依赖增量

- **0 新增核心 dep** (我们认知层已经领先, 主要是借鉴设计)
- 视情况: ort (ONNX) — 如果加 emotion classifier

---

## 9. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| core (ASI V0.5/V1136) | scoring.rs 用 |
| 12 键 verdict | cognition verdict 守门 |
| memory (R186) | consciousness Meditating/Dreaming 借用 lightmemo dream |
| council (R180) | 7 advisor 之一可以是 consciousness 自我报告 |
| tool (R140) | consciousness 状态决定 tool 可用性 |

---

## 10. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 认知层公开 API: 0 改 (新能力在子模块内, 通过 trait 抽象)

---

## 11. 参考链接

- LangGraph: https://github.com/langchain-ai/langgraph
- AutoGen: https://github.com/microsoft/autogen
- Temporal: https://github.com/temporalio/temporal
- Apache Flink: https://github.com/apache/flink
- PAD 情感: https://en.wikipedia.org/wiki/PAD_emotional_state_model
- Plutchik: https://en.wikipedia.org/wiki/Robert_Plutchik
- Ekman: https://www.paulekman.com/
- GoEmotions: https://github.com/google-research/goemotions
- Hume AI: https://hume.ai/
- Perceiver IO: https://github.com/deepmind/deepmind-research/tree/master/perceiver
- ImageBind: https://github.com/facebookresearch/ImageBind
- DSPy: https://github.com/stanfordnlp/dspy
- ReAct: https://arxiv.org/abs/2210.03629
- Tree of Thoughts: https://arxiv.org/abs/2305.10601
- Self-Refine: https://arxiv.org/abs/2303.17651
- LATS: https://arxiv.org/abs/2310.04402
- SwiftSage: https://arxiv.org/abs/2305.17390
- ACT-R: https://en.wikipedia.org/wiki/ACT-R
- SOAR: https://en.wikipedia.org/wiki/Soar_(cognitive_architecture)
- Constitutional AI: https://www.anthropic.com/news/constitutional-ai-harmlessness-from-ai-feedback
- Generative Agents: https://github.com/joonspk-research/generative_agents