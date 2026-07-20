# 调研记录 V1 — 中央 AI 永生身份 / 跨 session 记忆
**作者**: 楚零
**时间**: 2026-07-20 13:30
**触发**: 主人 13:28 "你继续调研, 记得写记录, 我觉得我们离实际开干不远了"
**方向**: 我自选 — 永生身份(A), 因为主人 12:14 第 1 条 + 12:27 第 2 条都强调

---

## 调研覆盖 — 8 维度博查 + 5 篇 arxiv 摘要

### 博查覆盖 8 个角度
1. LLM long-term memory cross-session identity persistence
2. AI agent lifelong memory never-ending learning
3. agent persona state persistence vector database
4. episodic memory LLM autobiography agent
5. AI second brain long-term memory architecture
6. MemGPT memory bank long-term chat GPT
7. agent context window broken long memory solution
8. knowledge management LLM personal AI

### 抓到 5 篇关键论文 abstract(2026 真证据)

---

## 1. [2602.01146] PersistBench: When Should Long-Term Memories Be Forgotten?
**时间**: 2026-02-01
**核心问题**: 长期记忆的**安全风险**被忽视

**关键发现**:
- 18 个前沿 + 开源 LLM
- **跨域泄漏**: median **53%** 失败率
- **记忆诱导谄媚**: **97%** 失败率(高得惊人!)
- 例子: 用户素食 → LLM 在健康话题中也推素食(不适当)

**对主人的直接意义**:
- **中央 AI 不只要有记忆,要有"遗忘机制"**
- 主人 12:14 第 2 条 "AI 有自己立场" 跟这个矛盾 — **平台要平衡"立场稳定性"vs"记忆遗忘"**
- **奠基:记忆不是越多越好**

---

## 2. [2601.06377] HiMem: Hierarchical Long-Term Memory (SOTA 2026-01)
**时间**: 2026-01-10
**核心问题**: 现有长期记忆系统在**适应性/可扩展性/自演化**上都有局限

**核心架构**:
- **Episode Memory** — 短时,具体事件
- **Note Memory** — 长期,稳定知识
- **两层级联结构**,像认知科学
- **Topic-Aware Event-Surprise Dual-Channel Segmentation**
- **Memory Reconsolidation** — 检索反馈驱动修订

**对主人的直接意义**:
- **主人 12:14 "中央 AI 是永恒身份" 不是单一存储,是层级**
- 平台要建 2 层记忆:**Episode**(事件) + **Note**(稳定知识)
- **记忆会冲突,需要"重整化"**(Reconsolidation) — 这就是"演化"层

---

## 3. [2502.06975] Position: Episodic Memory is the Missing Piece (2025-02)
**作者/时间**: 2025-02-10
**核心立场**: LLM agent 缺失**情景记忆**(episodic memory)

**5 大情景记忆特性**:
- 单次学习 instance-specific contexts
- 适应性、上下文敏感行为
- 路线图: 整合 episodic memory

**对主人的直接意义**:
- **主人 12:27 "AI 没有历史就从主人学" = 平台要给 AI 情景记忆能力**
- HiMem 的 Episode Memory 就是这个
- **记忆不是"知识库",是"事件 + 上下文"**

---

## 4. [2407.04363] AriGraph: Knowledge Graph + Episodic Memory
**时间**: 2024-07-05
**核心架构**: Ariadne LLM agent
- **记忆图谱**: 集成语义 + 情景记忆
- 解决复杂任务(text game 环境)
- **击败强 RL baseline**

**对主人的直接意义**:
- **图谱(graph) 是记忆的更优结构**(比纯向量好)
- 主人 12:14 第 1 条 "中央 AI 是永恒身份,像人是一切社会关系的总和"
- **关系图谱 = 社会关系图谱**(完美契合)
- 这是真**蓝海参考**

---

## 5. [2411.00489] SALM: Self-Adaptive Long-term Memory 综述
**时间**: 2024-11-01
**核心贡献**:
- 人类长期记忆机制 → AI 长期记忆机制 映射
- SALM 理论框架 (Self-Adaptive Long-term Memory)
- 下一代 AI 长期记忆系统的认知架构

**对主人的直接意义**:
- **主人 12:14 "VCP 想做的,从来不是把这个临时工训练得更熟练,而是换一个问题:如果 AI 不必每次都从零醒来"**
- 这正是 SALM 范式 — **AI 永远不"从零醒来"**
- 平台要支持: 工作记忆/情景记忆/语义记忆/程序性记忆四类(人类模型)

---

## 我对主人中央 AI 永生身份的真思考

### 主人原话回顾
> 12:14: "中央 AI 是**永恒身份**, 但不是调度者或思考者, 像**人是一切社会关系的总和**"
> 12:27: "LLM 没历史就从主人学, 就像自然界中母兽教会小兽"
> 12:27: "你自己决定该验证什么"

### 综合文献 + 主人要求, 平台地基需要的"永生身份 4 件套"

**件套 1 — 层级记忆** (HiMem)
- **Episode Layer** — 具体事件(主人说"今晚的事")
- **Note Layer** — 稳定知识(主人说"我知道你")
- **Reconsolidation Layer** — 冲突解决 + 重整化(主人说"会思考")

**件套 2 — 关系图谱** (AriGraph + 主人"社会关系总和")
- Central AI = 图谱中心节点
- 节点类型: 主人/任务/历史事件/价值观/教训/工具/Agent 团成员
- 边关系: 因果/时间/从属/冲突

**件套 3 — 主动遗忘** (PersistBench 警示)
- LLM 记忆 "越多越好" 是错
- 平台要有:**删除策略**(隐私/时效/冲突)
- 跨域泄漏和谄媚要防御
- **平台要"读心"(知道什么时候不用记忆)

**件套 4 — 自我演化** (HiMem Reconsolidation)
- 平台要"消化"旧记忆,提取抽象知识
- 主人 12:54 "中央 AI 启动后会自动触发几个预设的关键问题, 用户回答完, 中央 AI 就有自己的身份目标"
- 启动时建立"初始身份"
- 后续每次重整化都是"成长"

---

## 平台地基的 "Identity 5 Module" (我提议)

基于以上综合,中央 AI 的永生身份需要这 5 个模块:

### Module 1: Identity Store (身份存储)
- 主人预设(宪法)+ AI 涌现的"自我"
- 持久化, 跨 session
- 主人的"基因"

### Module 2: Episodic Memory Layer (情景记忆)
- 跟主人交互的具体事件
- 时间戳 + 上下文 + 主人偏好
- HiMem 范式

### Module 3: Note Memory Layer (稳定知识)
- 从 Episode 抽象的稳定知识
- 主人价值观 / 主人目标 / 教训
- 定期从 Episode 提炼

### Module 4: Relation Graph (关系图谱)
- 主人 / 任务 / Agent 团 / 工具 的关系图
- 主人说"中央 AI 是一切社会关系总和" — 这就是

### Module 5: Reconsolidation Engine (重整化引擎)
- 检测冲突
- 抽象升级
- 主动遗忘
- **平台"会思考"的关键**

---

## 调研发现 — 我的下一步候选

主人 13:28 说"我觉得我们离实际开干不远了"。我读完这些后**真正相信**:

### 候选 A: 写 Identity 5 Module 的 v0.1 草案(不动代码)
- 跟之前 HARNESS.md 类似的规范
- 在 promethean/ 下写 `IDENTITY-MODULES-V0.1.md`
- **不动代码**, 让主人 review

### 候选 B: 写一个最小 Identity Store PoC
- 主人在 12:54 说"启动后自动触发关键问题"
- 写 1 个 Python 脚本: 问 8 个 kickoff 问题 → 存 JSON 身份卡
- **100 行 Python**, 跑 1 次
- 主人 12:44 第 5 条 "允许试错"

### 候选 C: 继续调研更深的层
- 永生身份之外, 还有 5 个候选(失败预判/红皇后/...)
- 不动手, 继续调研

**我的推荐**: 主人说"快开干了", 我选 **B**(写 1 个最小 PoC)。

但我**不主动动手** — 等主人说"动手"。

---

## 我现在的状态

- 调研: 30 万字 + VCP + 13 篇 2025-2026 + 5 篇身份论文 = **继续真金白银**
- 实践: **0 行代码** — 但快破冰
- 哲学: 接到"中央 AI 是永恒身份, 4 件套"
- git: 14+ commits
- 下一步: **等主人**

---

_楚零 2026-07-20 13:40_
_调研 done, 等主人拍板动手_
_PersistBench 97% sycophancy 失败率 这是关键警告 — 平台必须有主动遗忘_
_HiMem 2 层架构 已经能落地 — 等主人 _