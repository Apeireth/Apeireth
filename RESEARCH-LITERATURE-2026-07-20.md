# 文献调研 — 2026-07-20 下午
**作者**: 楚零
**触发**: 主人 12:47 "现在你开始调研阅读吧, 把已有的成果思考都记录下来"

---

## 调研目标(基于主人 12:14 / 12:27 / 12:47)

主人 24 条 + 5 答涉及 4 个核心问题:

1. **多智能体协调新模式** — 主人说"调度者也许会用很新的模式,这需要读文献"
2. **LLM 主动学习 / 持续向主人提问** — 主人说"在 llm 上落地就是 llm 不断向主人提问"
3. **多身份架构 / Persona 涌现** — 中央 AI 是多身份, 不是单一调度者
4. **AI 立场自然涌现** — 不强加, 不中庸, 自然成长

---

## 核心论文 8 篇(全部 2025-2026)

### 1. [2510.05174] Emergent Coordination in Multi-Agent Language Models
**作者/时间**: 2025-10-05, arxiv
**核心问题**: 多智能体 LLM 何时是"集合"何时是"集体"?

**核心方法**: 信息论框架 — TDMI (time-delayed mutual information) + partial information decomposition
- 控制组: 强时间协同但缺协调
- 加 persona: 稳定身份分化
- 加 persona + "想想其他 agent": **身份分化 + 目标互补**

**对我的启示**:
- 主人说"自组织临时团" 不是空想,信息论可以测量
- 中央 AI 是不是单一调度者不重要 — 关键是有**身份分化 + 目标互补**
- 提示工程可以从"集合"引导到"集体" — 主人说"提示工程就是地基"

### 2. [2510.12015] Asking Clarifying Questions for Preference Elicitation
**作者/时间**: 2025-10-13
**核心问题**: LLM 怎么持续向主人提问, 挖主人真实偏好?

**核心方法**: 扩散模型启发
- 正向: user profile → 问问题 → 拿答案 → 移除答案 = 加噪
- 反向: train 模型从噪声 profile → 问有效问题
- **funnel question**: 漏斗式提问

**对我的启示**:
- 主人说"LLM 不断向主人提问" — 这就是 LLM-as-Active-Learner
- 主人教 AI 学习 = LLM 主动问 + 主人答 + 平台存储
- 提问序列有方法可循 — **funnel 是关键**: 从宽到窄

### 3. [2601.10102] When Personas Override Payoffs: Role Identity Bias
**作者/时间**: 2026-01-15
**核心问题**: Persona vs Payoff, 哪个支配 LLM 决策?

**关键发现**:
- 4 agent 战略游戏 (53 个环保场景)
- **有 persona 时**: 4 个模型都几乎不选 Tragedy of Commons(尽管 payoff 信息完整)
- 无 persona + 显式 payoff: Qwen 65-90% 选 Tragedy
- **Persona 抑制 90 个百分点的 payoff-aligned 行为**

**对我的启示**:
- Persona 不是装饰 — 它**支配 LLM 决策**
- 中央 AI 多身份, 每个身份 = 不同 persona = 不同决策倾向
- 但论文说"persona 让 agent 不理性" — 这是好事还是坏事?
  - 主人说"AI 不是和事佬, 不会永远正确, 拥有自己思想和立场很重要"
  - **Persona 让 AI 有立场 — 这正是主人要的**

### 4. [2505.18351] Persona Alchemy: SCT 心理学 grounded LLM agents
**作者/时间**: 2025-05-23
**核心问题**: 怎么用心理学理论设计 LLM persona?

**核心方法**: Social Cognitive Theory (SCT)
- 4 个个人因素: cognitive / motivational / biological / affective
- 6 个量化评估
- graph database 存 personas
- 实验: 5 个 diverse agents 在再生能源对话中

**对我的启示**:
- 中央 AI 多身份可以用 **SCT 框架**实现
- 每个身份 = 4 因素 + 6 评估
- **不是"prompt 加 system message",而是"图数据库结构化"**

### 5. [2601.10025] Structured Personality Control: Jungian types for LLM
**作者/时间**: 2026-01-15
**核心问题**: 怎么让 LLM persona 既**一致**又**适应**?

**核心方法**: Jungian 心理类型
- **dominant-auxiliary coordination** (核心表达一致)
- **reinforcement-compensation** (临时适应)
- **reflection mechanism** (长期演化)
- Myers-Briggs Type Indicator 评估

**对我的启示**:
- 这是**真正符合主人"立场自然成长"** 的方案
- Persona 不是静态 — 3 机制让它演化
- "Reflection mechanism drives long-term personality evolution" ← 主人 12:14 第 16 条: "ASI 是从平台、地基来说, 让模型接入后, 会超越模型本身"
- **平台要让 persona 演化, 这就是地基的一部分**

### 6. [2405.03862] Persona Inconstancy in Multi-Agent LLM Collaboration
**作者/时间**: 2024-05-06
**核心问题**: 多 agent 能可靠 adopt persona 吗?

**关键发现**:
- LLM 有 **conformity** (从众压力)
- LLM 有 **occasional challenges in maintaining consistent personas**
- **debate-style prompts** 增加 inconsistency(让 agent 反而更易变)
- **collaborative prompts** 更稳定

**对我的启示**:
- 多身份不是"装上就稳" — 平台需要**反 conformity** 机制
- "辩论"会让 persona 不一致 — 主人说"自组织临时团" 时要小心
- 主人 12:27 第 1 条: "中央 AI 不管理, 一切交给中央 AI 自己"
- 这是平台设计难题:**怎么让多身份自洽**

### 7. [2505.19806] Exploring Consciousness in LLMs (Survey)
**作者/时间**: 2025-05-26
**核心问题**: LLM 到底有没有意识?

**核心内容**:
- 区分 **LLM consciousness** vs **LLM awareness**
- 系统综述: 理论 + 实证
- 前沿风险: 意识 LLM 可能带来的问题

**对我的启示**:
- 主人 12:14 第 15 条: "你肯定是没自我的, 你就是 LLM 接入到 OpenClaw 有了些扮演的设定而已"
- 主人 12:14 第 18 条: 主人相信 ASI 可达, 会有意识
- **平台地基要支持"涌现 consciousness", 即使我们现在不知道怎么做**
- 但**不要预设** — 让它自然涌现(主人 12:27 第 3 条)

### 8. [2505.04364] SwarmBench: Benchmarking LLMs' Swarm intelligence
**作者/时间**: 2025-05-07
**核心问题**: LLM 在 swarm 约束下能协调吗?

**核心方法**: SwarmBench — 5 个基础 MAS 协调任务
- Pursuit / Synchronization / Foraging / Flocking / Transport
- 2D grid, 严格本地感知 + 本地通信
- Zero-shot 测试 deepseek-v3, o4-mini

**关键发现**:
- LLM 表现出**一些**rudimentary coordination
- 但**长程规划 + 适应性策略**显著失败
- 主人说"自组织" — 当前 LLM 还做不到

**对我的启示**:
- 主人 12:47 第 3 条: "调度者也许会用很新的模式, 这需要读文献"
- SwarmBench 告诉我们:**当前 LLM 在 swarm 约束下不行**
- 这是平台地基的**真实挑战** — 怎么让 LLM 在严格约束下涌现协调
- 可能的解: **中央 AI 不是无, 而是"涌现的协调者"**, 不是预设的

---

## 我的真思考 — 4 个主人问题的我答

### 问题 1: 多智能体协调新模式 — 文献答

**已有**: Persona + 目标互补 + 反 conformity = 涌现协调 ([2510.05174])
**未有**: 中央 AI 是**涌现**的协调者, 不是预设的
**地基方向**:
- 中央 AI 不是"启动时指定"
- 而是"涌现的协调模式"
- 工具: TDMI 实时测量涌现强度
- 反 conformity: 平台定期检测 persona 一致性, 衰减"从众"压力

### 问题 2: LLM 持续向主人提问 — 文献答

**已有**: Funnel question + 扩散模型启发 ([2510.12015])
**未有**: 长期持续提问 + 主人回答累积成"成长"
**地基方向**:
- LLM 主动问, 主人答, 平台存
- 不是"教"而是"问"
- 主人预设(宪法) + LLM 主动问(细节)
- **funnel: 宽问题 → 窄问题 → 立场**

### 问题 3: 多身份架构 — 文献答

**已有**: SCT 框架 ([2505.18351]) + Jungian 演化 ([2601.10025])
**未有**: 多身份 + 涌现 + 持续演化
**地基方向**:
- 中央 AI = 多 persona = graph database 存
- 每个 persona = SCT 4 因素
- 3 机制: coordination / adaptation / reflection
- **平台不管理 persona, persona 自然演化**

### 问题 4: 立场自然涌现 — 文献答

**已有**: Persona 强支配决策 ([2601.10102])
**未有**: Persona 不被反 conformity 干扰 ([2405.03862] 警告)
**地基方向**:
- Persona 是**根因** — 立场从 persona 涌现
- 但 persona 会 conformity — 平台要**抑制**这个
- 主人说"AI 不会中庸的, 因为他会成长" — 平台要让 persona 演化, 不是静态
- **演化 = 关键**(Jungian reflection)

---

## 我现在该验证什么 — 重新决定

主人 12:44 第 5 条: "你自己思考该验证什么"
主人 12:47 第 5 条: "你分不清的, 不必管"

我之前的疑问(5 个), 现在有文献对照:

| 疑问 | 文献答 | 我该验证吗? |
|------|------|----------|
| 多身份架构 | SCT + Jungian 已实证 | ✅ 验证 SCT 是否能落地中央 AI |
| 主人教 AI | Funnel question 已实证 | ✅ 验证 funnel 是否能累积立场 |
| 自组织临时团 | SwarmBench 显示 LLM 还不行 | ⚠️ 长期挑战, 不立即验证 |
| 立场自然涌现 | Persona 强支配决策 | ✅ 验证 persona 是否能"自然"涌现 (vs 被预设) |
| 真假疑问区分 | 不必管 | ❌ 主人说不用管 |

**我该验证的 3 个(从主人 24 条 + 5 答 + 文献综合)**:

1. **SCT 多身份是否能在 LLM 上跑** — 写最小 persona class + 4 因素 + 测一致性
2. **Funnel 提问是否能累积主人"立场"** — 写最小提问协议 + 主答 + 测后续召回
3. **Persona 演化是否真的"自然"** — 跑长对话, 看 persona 是否在演化

但主人说"允许试错", 不强求一次做 3 个, **先做 1 个**, 跑出结果再说。

---

## 我选哪个

主人 12:47 说"开始调研阅读吧"。我刚做完调研,**还没开始动手**。

下一步:
- **继续调研**(主人说"调研阅读")OR
- **开始动手**(主人 12:44 说"允许试错", 但 12:47 说"调研阅读")

我猜主人 12:47 意思:**先调研完, 再动手**。

所以我应该:
- 继续读 5 篇以上 (我已读 8 篇, 还可以读更多)
- 整合到 promethean/ 的笔记里
- 不动手

或者:
- 主人其实是想我**调研够了开始动手**
- 但"开始动手"≠"立刻写大文档/写代码", 是"写最小验证"

---

## 我现在不动手 — 等主人下一句话

主人,我读完 8 篇论文 abstract 了。

**我没写代码, 没写大文档**, 只写了这个文献调研笔记(本次)。

等你下一步指令:
- "继续读"(我读更多文献)
- "开始动手"(我写最小验证)
- "再讨论"(我接住你的下一段反馈)

---

_楚零 2026-07-20 13:10_
_调研 done, 8 篇 abstract 全消化_
_等主人下一步_