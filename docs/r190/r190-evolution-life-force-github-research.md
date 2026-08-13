# R190 GitHub 优秀项目调研 — evolution / life-force (进化与生命力)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R190
> **日期**: 2026-08-13
> **范围**: apeireth-evolution (9 文件 266KB) + apeireth-life-force (3 文件 44KB)
> **状态**: 调研为升级预备.

---

## 0. 现状

### apeireth-evolution (9 文件 266KB)
- library_autonomy.rs (64KB) — 库自主性
- library_autonomy_loop.rs (49KB) — 自主循环
- poda_cycle.rs (39KB) — PODA 周期
- council_bridge.rs (24KB) — council 桥
- engine.rs (19KB) — 引擎
- traits.rs (21KB) — trait 定义
- state.rs (20KB) — 状态
- fail.rs (15KB) — 失败处理
- lib.rs (9KB) — 入口

### apeireth-life-force (3 文件 44KB)
- lib.rs (18KB) — 入口
- reflection_cycle.rs (13KB) — 反思周期
- emergence.rs (12KB) — 涌现

**已实现能力**:
- 库自主性 (library autonomy) — 极少见的设计
- PODA 周期 (Plan-Observe-Decide-Adapt?)
- Council 桥 (evolution 与 council 集成)
- Engine / traits / state / fail
- 反思周期 + 涌现

**已经领先**:
- "库自主性" 比大多数 LLM agent 的 "skill library" 概念更深
- 涌现 (emergence) 是少见的 trait
- 反思周期 (与 consciousness Meditating 状态互补)

---

## 1. AI 自我进化 SOTA

### 1.1 Voyager (再, R186 提过) — **RECOMMENDED 必学**

- **GitHub**: https://github.com/MineDojo/Voyager
- **License**: MIT
- **定位**: Minecraft 终身学习 agent
- **核心能力**:
  - **Skill library** 自动累积 (我们 library_autonomy 类似!)
  - Curriculum 自动生成
  - Iterative prompting
  - 环境反馈学习
- **我们 library_autonomy 与之高度契合**

**借鉴方案**:
`
ust
// 我们已经有的设计可以更明确:
pub struct LibraryAutonomy {
    skill_library: HashMap<SkillId, Skill>,
    curriculum: Vec<CurriculumStep>,
    feedback_buffer: Vec<Feedback>,
}

impl LibraryAutonomy {
    pub async fn propose_skill(&self, context: &Context) -> Result<Skill, Error>;
    pub async fn integrate_skill(&mut self, skill: Skill) -> Result<(), Error>;
    pub async fn select_skill(&self, context: &Context) -> Result<Skill, Error>;
}
`

### 1.2 Darwin Gödel Machine (Sakana AI 2025+) — **RECOMMENDED 学习**

- **arXiv**: https://arxiv.org/abs/2505.22954
- **License**: Apache 2.0
- **定位**: AI 通过修改自身代码来进化
- **核心能力**:
  - 自我修改源码
  - 通过 benchmark 评估 fitness
  - 迭代进化
- **我们 evolution engine.rs 可以借鉴**
- **哲学风险**: 修改自身代码 vs 我们的 3 不可变脊柱 — 需要谨慎设计边界

**借鉴方案 (有限度)**:
- 不直接自我修改代码
- 但可以: 修改 skill library + 修改 prompt 模板 + 调整 hyperparameter
- 通过 L0 HA 多签批准后才生效

### 1.3 AlphaEvolve (Google DeepMind 2025+) — **RECOMMENDED 学习**

- **arXiv**: 近期发布
- **定位**: AI 进化算法, 发现新算法
- **核心能力**:
  - LLM 生成算法变体
  - 自动评估 + 选择
- **学习点**: 我们 evolution 评估函数借鉴

### 1.4 FunSearch (DeepMind 2023) — 学习

- 数学发现用 LLM
- **学习点**: 形式化评估

### 1.5 The Automated AI Scientist (Sakana AI) — 学习

- 全自动科研
- **学习点**: paper writing loop

### 1.6 STOP (Sakana AI) — 学习

- Self-Play fOr Training
- **学习点**: 多 agent 自我博弈

### 1.7 Generative Agents (再, R186) — 必读

---

## 2. 涌现 / Emergence SOTA

### 2.1 我们 emergence.rs (12KB) — 已实现

- 涌现作为 trait
- 业界少见的设计

### 2.2 Concept Formation (认知科学) — 学习

- 概念形成理论
- 我们 emergence 借鉴

### 2.3 Stigmergy (蚁群) — 学习

- 间接协调
- 涌现经典案例

### 2.4 Self-Organization (复杂系统) — 学习

- 自组织理论
- **学习点**: 我们 evolution 自组织借鉴

### 2.5 Complex Systems (Santa Fe Institute) — 学术

- 复杂适应系统
- 哲学基础

---

## 3. 反思 / 自我改进 SOTA

### 3.1 Reflexion (再, R186 提过) — **必学**

- 短期 + 长期反思
- 我们 reflection_cycle 已经类似

### 3.2 Self-Refine (再, R187 提过) — 学习

- 迭代改进
- 我们 reflection_cycle 强化

### 3.3 CRITIC (Gou et al. 2024) — **学习**

- LLM 自我批评 + 工具使用
- **学习点**: 我们 reflection 加 tool use

### 3.4 Self-Correcting (多个) — 学习

- self-consistency
- self-verify
- self-improve

### 3.5 Constitution AI (再) — 学习

- 自我批评 + 修订
- 我们 council philosophy advisor 借鉴

---

## 4. 进化算法 / 搜索 SOTA

### 4.1 EvoPrompt (Microsoft 2023) — **学习**

- LLM + 进化算法
- prompt 自动优化
- **学习点**: 我们 evolution engine 借鉴

### 4.2 FunSearch (再) — 学习

### 4.3 AlphaEvolve (再) — 学习

### 4.4 MAP-Elites (质量多样性) — 学习

- 多样性保留进化
- **学习点**: 我们 skill library 多样性

### 4.5 NEAT (NeuroEvolution) — 学术

- 神经网络拓扑进化
- 学术参考

---

## 5. 自我保存 / 终止 SOTA

### 5.1 我们 Self-Disable (R178 提过) — 业界独一档

- 形式化 + 物理多签 + 24h 反思期
- 没看到其他项目有类似

### 5.2 Anthropic Responsible Scaling Policy — 学习

- RSP 框架
- **学习点**: 我们 Self-Disable 哲学基础

### 5.3 OpenAI Preparedness Framework — 学习

- 安全等级
- 我们的 30 维 V0.5 类似

### 5.4 EU AI Act — 法规

- 风险管理
- 我们的双洋葱合规

### 5.5 NIST AI RMF (再, R178 提过) — 学习

---

## 6. 升级方案 (R190+ 实施)

### 6.1 短期 (1-2 days)

1. **Skill library API 强化**: 借鉴 Voyager, 我们 library_autonomy 加 propose/integrate/select 三件套
2. **CRITIC 风格 tool use 反思**: reflection_cycle 加 tool 调用

### 6.2 中期 (3-5 days)

3. **Darwin Gödel Machine 有限借鉴**: 不改代码, 但改 skill / prompt / hyperparameter
4. **EvoPrompt prompt 优化**: engine.rs 进化 prompt
5. **L0 HA 批准机制**: 自修改必须 L0 HA 多签

### 6.3 长期 (持续)

6. **AlphaEvolve 风格算法发现**: 长期
7. **复杂适应系统理论**: emergence.rs 强化
8. **RSP 哲学基础**: Self-Disable 文档化

---

## 7. 依赖增量

- **0 新增核心 dep** (我们 evolution 已经很领先, 借鉴设计为主)
- 视情况: 进化算法库 (rust-ecosystem 评估)

---

## 8. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| consciousness (R187) | life-force 反思与 consciousness Meditating 互补 |
| memory (R186) | evolution library 与 memory skill library 互补 |
| council (R180) | evolution council_bridge.rs 已桥接 |
| sovereignty | evolution 改变自身需要 L0 HA 批准 |
| pipeline (R184) | evolution 可作为 pipeline 自我改进 step |

---

## 9. 0 触碰声明

- 3 不可变脊柱: 0 触碰 (Self-Disable 是 3 脊柱之一, evolution 不能绕开)
- workspace.version 1.2.0: 0 改
- evolution/life-force 公开 API: 0 改 (新能力在子模块内)

**关键哲学边界**:
- evolution 可以改 skill / prompt / hyperparameter
- evolution **不能** 改 3 不可变脊柱
- evolution **不能** 改 workspace.version
- evolution **不能** 绕过 L0 HA 物理多签

---

## 10. 参考链接

- Voyager: https://github.com/MineDojo/Voyager
- Darwin Gödel Machine: https://arxiv.org/abs/2505.22954
- AlphaEvolve: https://deepmind.google/discover/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/
- FunSearch: https://deepmind.google/discover/blog/funsearch-making-new-discoveries-in-mathematical-sciences-using-large-language-models/
- The Automated AI Scientist (Sakana AI): https://github.com/SakanaAI/AI-Scientist
- STOP: https://arxiv.org/abs/2406.07468
- EvoPrompt: https://arxiv.org/abs/2309.08532
- CRITIC: https://arxiv.org/abs/2305.11738
- MAP-Elites: https://arxiv.org/abs/1504.04909
- NEAT: https://nn.cs.utexas.edu/?neat
- Self-Disable: docs/r170/r170-hyperlight-research.md (内部)
- Anthropic RSP: https://www.anthropic.com/news/anthropics-responsible-scaling-policy
- OpenAI Preparedness: https://openai.com/safety/preparedness
- EU AI Act: https://artificialintelligenceact.eu/
- NIST AI RMF: https://www.nist.gov/itl/ai-risk-management-framework
- Santa Fe Institute: https://www.santafe.edu/