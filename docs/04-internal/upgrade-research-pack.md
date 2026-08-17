# 升级计划资料包（Upgrade Research Pack）

> **定位**: 升级计划（W1/W2 世界模型、N25 数据可信度、N26 多模态、N27 连续感知、N28 重放巩固、E3 校准/预测市场）的行业顶尖方案调研汇总——给团队实施时的参考资料。
> **来源**: ①4 个 subagent 全网文字调研（世界模型 S1/金融 S2/多模态感知 S3/重放巩固 S4）②主线程知识视频平台调研（B站/知乎/YouTube/百度开发者）③主线程 web_search 补充。
> **重建记录**: 2026-08-18 团队分支 rebase 导致本文件丢失，主线程按对话历史完整重建并补入 §7 M-FLOW 对照。

---

## 1. 世界模型前两层（W1/W2）

### 行业前沿共识（视频/中文源证实）
- **"语言世界模型：为什么下一代 Agent 要先学会预演后果"**（B站 BV1oJTV6HEae）——LLM 时间线推演 = 行业前沿共识，我们 W1 方向正确
- **"Agent 的世界模型到底是什么？从第一性原理，讲透它为什么总在同一个坑里栽倒"**（B站 BV1AmNH6EENf）——世界模型 = 反事实预演；"总在同一个坑里栽倒"= 没有因果记忆 + 没有推演
- **Genie 3**（DeepMind）：第三层墙的最新实况——BAAI 全球独家首测（hub.baai.ac.cn/view/47958）+ 36kr 独家访谈——确认是视频级交互世界生成（像素级），与推理链不同质，跟踪不趟的结论不变
- **AI 反事实实验室的原理方法和挑战**（B站 BV1Zz4KzPEEN）——反事实推演的工程挑战

### §6-2 文字调研（S1 369676c8）：W1/W2 详细结论

**工作区核实**：oracle.rs 已有 ForecastRegistry/CalibratedResolver/ScenarioEngine/DecisionEngine（注释自认"多轮 MCTS 是下一步"）；planning.rs 是 LATS 风格（E2 已完成）——W1 编排器缺口确认，W2 底图 memory_graph 确认。

**Top 5 真值得抄**：
1. **CHILL-Harness（arXiv 2607.25825）——编排层因果干预学习**：干预 = 展开分支/触发反思/分配预算，用置信度加权执行证据估计"干预优势"——**直接补 W1 推演编排器的缺口**（落 oracle 新 orchestrator 模块）。比"LLM 自由发挥推演"多一个学习层：她学会"哪种干预有效"
2. **Generative Agents + AgentSociety**：记忆流（recency/importance/relevance）-反思-计划循环 + 大规模并行模拟分层调度——oracle 编排器骨架 + memory_graph crawl 预算
3. **Future-as-Label + 自动出题结算**：proper scoring rule 做 RL 奖励（Qwen3-32B Brier -27%、校准误差减半）；自动题库 96% 可验证/95% 结算准确——oracle 校准从"记账"升级"闭环"（落 CalibratedResolver + oracle_adapters）
4. **EvoCause 图演化 + Kıcıman 成对因果 + Era-of-Agents 护栏**：**LLM 只提候选边/方向，确定性 Rust 校验（无环/标注集增益）后提交**——W2 因果边维护流水线（验收集 = 已结算预测）；"LLM 提议 + 机制校验"正是我们的 trait 策略哲学
5. **多样性集成 + 辩论式反思**：按相关性矩阵选集成成员；反思改跨模型辩论（同模型自省无增益, log loss -4%）——E3 集合预报 + 反思机制升级

**不值得/只跟踪**：视频世界模型全线（LIRWS/UniSim/Genie 1/WorldSimBench/Genie 3）——第三层墙确认跟踪不趟；因果 GNN、文本游戏 RL 微调、DoWhy/CausalNex 本体集成（只取 API 语义）

---

## 2. 金融数据与投资伙伴（N25/E3/N3）

### 行业顶尖实践（视频/中文源证实）
- **TradingAgents**（71.4K star, arXiv 2412.20138）——多智能体 LLM 交易框架，直接对应我们的"团队三件套 + 投资场景"：研究员/分析师/交易员/风控多角色分工 + 多头/空头辩论——与 council 审议精神同源但为交易特化
- **"金融 Agent 有了多 Agent 认知层，但喂给它的还是过时数据"**（tickdb.ai 数据层深度拆解）——**完美佐证 N25 是生死线**：TradingAgents 的弱点就在数据层
- **Qlib**（微软）：研究流水线（B站 BV1jzEz6REvi）——因子研究/回测流水线参考
- **AI 量化开源全景**（B站 BV1MLNu6QEji）——"真正能用的工具 vs 只适合演示的 Demo"
- **Polymarket 躺赚机器人**（0daily/HTX 指南）——预测市场自动化实操（E3 适配器参考）

### §6-3 金融调研（S2 2e75de1b 最终交付 + 主线程 web_search 补充）

**🥇 N25 数据可信度分层设计（T0-T4 四件套元数据 + 动态信任分）**：
| 层 | 来源类型 | 例 | PIT | 质量闸门 | 消费方式 |
|---|---|---|---|---|---|
| T0 权威事实 | 交易所/监管原始 | 交易所行情/SEC EDGAR/央行决议 | 全 PIT | 强校验+多源互证 | 事实锚点, ground truth |
| T1 持牌/专业 | 付费/专业 API | 宏观数据商/财报数据库 | 尽量 PIT | 契约测试 | 因子与回测主数据 |
| T2 新闻/聚合 | 快讯/聚合 | 财联社/东财/FRED | 快照或截断 | 非空/新鲜度/截断告警 | LLM 分析师上下文 (禁止直接断言价格) |
| T3 舆情/一致预期 | 社交/共识 | 同花顺一致预期/StockTwits | 无→显式告警"非当日事实" | 时间戳+来源必填 | 仅作信号 |
| T4 模型输出 | LLM 结论 | 其他 agent 报告 | 无 | 必须带来源链引用 | 只进辩论, 不进事实层 |

- **静态信任** = tier 先验 + PIT 能力 + 质量闸门通过率
- **动态信任（核心创新）** = 数据源历史可证伪断言在 **Brier/校准曲线**上的表现（与 CalibratedResolver 同一计分, 滚动更新）——E3 集合预报权重 = f(源信任分)；Manifold 站点校准页为业界同构先例
- 落地顺序：①PIT 数据层（抄 Qlib Provider 接口语义）→ ②质量闸门脚本（抄 astock test_data_quality.py 17 端点）→ ③未来函数纪律（抄 `_is_historical + _snapshot_notice`）→ ④信任分入 E3 权重

**🥈 astock 未来函数防护（血泪教训）**：时点截断 vs 显式告警二选一；僵尸数据识别；截断告警四形状（Anthropic max_tokens / OpenAI length / Gemini MAX_TOKENS / Responses incomplete——漏一种就静默丢数据）；**防护覆盖面必须枚举全部 vendor 逐个实测**

**🥉 Qlib PIT + 成对成本回测**：PIT 杜绝 lookahead bias（否则 Brier 校准全假）；回测 with/without cost 成对报告；风险指标集 IR/max_drawdown/annualized_return

**④ TradingAgents 反幻觉 grounding + 决策记忆闭环**：LLM 数字断言带 snapshot_id + 身份确定性解析前置 + 数据接入契约；决策日志→收益结算→反思注入——与 oracle 预测记录同构（评级换概率, 结算换 Brier）

**⑤ 风控收口**：校准概率 → 分数 Kelly（1/2~1/4, 未校准概率不能进 Kelly）→ 单笔风险上限 + 最大回撤熔断 + 组合相关性检查 → paper→live 分阶段（Alpaca 同一 API, 每阶段独立验收）；绩效口径抄 astock 严谨性；三方风险辩论作第二道防线

**BloombergGPT 数据管线**：3630 亿金融 + 3450 亿通用语料；9 大类来源分类矩阵——N25 分层思想与业界最大金融模型同源

---

## 3. 多模态与连续感知（N26/N27）

### 行业顶尖实践（视频/中文源证实）
- **OmniParser**（微软屏幕解析）：B站大量本地部署教程——纯视觉 GUI agent 的屏幕→结构化数据解析，N26 的现成参照
- **wallie-V2**（开源 AI 看屏幕+听声音实时反应）——N27 的完整开源参照
- **"AI 屏幕感知技术如何重塑数字心智"**（neican.ai）——屏幕感知产品化思考

### §6-1 文字调研（S3 58b4a78b）：N26/N27 详细结论

**关键事实**：`tool-image-process` 的 `ocr.rs` 是**诚实桩**（返回空文本）；`hash.rs` 已有 `perceptual_hash`（真实 pHash）——**感知门的地基代码已存在，只差上位**。`event_log.rs` 是内存环形缓冲（cap 1024, 0 持久化）。

**真值得抄 TOP 5**：
1. **OmniParser 式"截图→结构化元素→文本"**（解 N26 财报截图阻塞）：OcrResult 升级为真 tesseract-rs OCR + bbox；LayoutElement { bbox, kind: Text|Table|Chart, text } + `<|box_start|>` 序列化；ProcessOp::Table（表格→Markdown）
2. **pHash 变化检测感知门**（N27 成本地基, 代码已有）：perceptual_hash 上位为 PerceptionGate——帧→diff→仅变化帧触发 OCR/VLM → 事件 `sensor.screen.changed` 上 bus。**一行思维转变, 连续感知成本从 O(每帧全理解) 降到 O(仅变化帧)**
3. **Screenpipe 式三层流水线**（N27 骨架, Screenpipe 本身是 Rust）：捕获层（屏幕/音频帧）→ 本地理解层（OCR/Whisper 文本块）→ SQLite 索引层（可查询"最近见过/说过什么"）
4. **ProAgent 式按需感知上下文（PerceptionGate）**：传感器事件先在门控层过滤/聚合/摘要，只把情境观察发布给 agent——杜绝裸流进上下文
5. **视觉/解析混合路由 + Letta 式记忆压缩**：router.rs 升级为成本路由（文本→OCR/布局, 图表→VLM 定性, 数值→回源结构化数据, 语义→整图 token）；event_log 持久化时设计"原始事件 + 周期摘要记忆块"两级记忆（PAL-UI 式按需回看）

**成本铁律**（行业实测）：
- 截图直喂 VLM ≈ 2100 tokens/屏（优化后 540）——"能少看就少看、能解析就别整图"
- **K线图数值: VLM 明确不可靠**（多尺度基准证实）→ 数值必须回源结构化数据，VLM 只做形态/趋势定性，报表截图走表格解析——**N26 的铁律**
- pHash 门控 + 图像 token 预算降级——对齐"浅尝辄止"哲学

**不值得**：UI-TARS 72B 常驻（成本/功能错配）/ Claude Computer Use 完整动作环（SaaS-Bench <4% 通过率）/ 24-7 麦克风监听（隐私红线）

---

## 4. 重放巩固（N28）

### 行业顶尖实践（视频/中文源证实）
- **"Agent 持续学习到底是什么？为什么它用一百次，还像第一次？"**（B站 BV13kKV6CEC7）——直击 N28 问题定义
- **"AI 操作浏览器又贵又不稳定？我做了一个 CLI 记忆层"**（知乎）——技能/经验复用实战

### §6-4 文字调研（S4 b8b4032a）：N28 详细结论

**核心结论：存储不解决记忆问题，回放（复习）才解决。"Recall is the real bottleneck。"**

**⭐1. FSRS 作为"到期引擎"（抄调度，不抄卡语义）**：
- fsrs-rs 是**纯 Rust crate（`cargo add fsrs`, MIT）**——直接可用
- **四点语义改写**：①R（可回忆概率）→ 重定义为"**这条经验/技能仍有效的概率**"（validity 而非 recall）②评分来源必须是**可执行验证**（evolution_gate 机械闸/critic 复核），不能模型自评（"tutor grades itself" 偏差；Voyager 消融 −73% 背书）③时钟换**活动时长**（非墙钟——闲置一个月经验不该集体到期；叠加绝对到期下限覆盖环境漂移）④内容变更 = 新卡（chain+rev 版本化天然匹配）
- FSRS-6 **Cost ADR**：按复习成本调节目标保留率——"浅尝辄止"写进算法

**⭐2. SRSA 复习回路骨架**：Card Generation → Scheduled Review → Self Evaluation → Memory Update + 记忆自修复——N28 模块形状；验证来源用我们自己的 evolution_gate/critic 补其自评短板

**⭐3. Sleeping LLM 逐条审计 + 渐进巩固门控 + drowsiness 触发**：per-item stage 晋级/退级 + 到期数量超阈值才触发复习批 + 验证后回滚；权重编辑（MEMIT/LoRA）明确不抄

**⭐4. Letta review-gated dreaming**：第二个后台 agent 审阅拟议记忆更新再应用 + 触发 = 周期 ∪ 步数 ∪ 到期数量——给 DreamScheduler 升级

**⭐5. 选择性遗忘清单（FSFM 四分法）+ 混合时钟**：被动衰减（FSRS 到期）/ 主动删除（capability 退役扩展: 逾期未复核→降权）/ 安全触发（**到期复习 = 定期审计 = 对冲记忆投毒**——MINJA 注入 98.2% 论文佐证）/ 自适应强化（FSRS 参数拟合）

**复习预算**：优先级 = importance × (1−R(t)) × 失效后果；每周期 T token 预算按 top-N；便宜验证优先；**先例已在库**：oracle_adapters 的"登记→到期 resolve"就是 due 语义

---

## 5. 主线程调研结论（视频/知识源，先行版）

1. **W1 方向被行业共识证实**：语言世界模型（预演后果）是下一代 Agent 的公认方向
2. **N25 是差异化机会**：连 TradingAgents 都栽在数据层——我们把它做进机制，发布即领先
3. **N26/N27 有成熟开源参照**：OmniParser + wallie-V2——不必从零设计
4. **N28 问题定义清晰**："用一百次还像第一次"——缺的是"到期复习"调度
5. **E3 预测市场有实操路径**：Polymarket 机器人指南——适配器模式照抄

---

## 6. subagent 文字调研状态

- [x] S3 多模态感知（58b4a78b）——已并入 §3（§6-1）
- [x] S1 世界模型（369676c8）——已并入 §1（§6-2）
- [x] S2 金融（2e75de1b 最终交付 + 主线程补充）——已并入 §2（§6-3）
- [x] S4 重放巩固（b8b4032a）——已并入 §4（§6-4）
- [x] 主线程视频调研——已并入 §1-§5
- [x] M-FLOW 对照（主线程, 扫描兵两轮限流失败后接手）——已并入 §7

---

## 7. M-FLOW 对照（2026-08-18 主线程调研, 主人点名）

> **M-FLOW**（FlowElement-xinliuyuansu/m_flow, 19 岁常青藤团队"心流元素", 生物启发认知记忆引擎, Graph RAG 新范式）。
> 来源: 百度开发者中心架构解析（developer.baidu.com article 8216760）+ 澎湃/163 报道 + GitHub。

### 核心机制（四层）
1. **数据编织层**：粒度分解器把文本拆成事实单元（Fact Unit）+ 上下文块（Context Block）
2. **图构建层**：动态图引擎（事实/上下文/事件节点 + 关系边含 CAUSE + 权重）+ **时序处理器**（事件时间维度, 自动识别"之前/之后"）
3. **路由决策层**：**动态图路由**——按查询类型选推理路径（非固定 BFS/DFS, 路由决策网络, 多跳推理成功率 +37%）+ 成本评估器
4. **响应生成层**：证据聚合 + **逻辑校验**（检测推理链矛盾）+ 多格式输出

### 关键创新
- **跨粒度双塔索引**：事实塔（倒排+向量混合）vs 上下文塔（BERT 语义+时序）——查询自动选塔
- **Bundle Search**：检索→推理→生成封装为可组合原子操作——"检索即推理"范式
- **检索阶段纯符号推理, 仅生成阶段用 LLM**——大幅降计算成本（与我们 trait 哲学一致）
- 支持 5+ 跳推理（行业平均 2-3 跳）; 推理路径可解释

### 与我们对照 + 吸收点
| M-FLOW | 我们 | 吸收 |
|---|---|---|
| 动态图路由（按查询类型选路径） | crawl 是固定展开模式 | **W4 图路由版**: N7 查询形态学 → 选展开策略（因果问题沿 CAUSE 边/对比问题沿对比边/宽泛走社区） |
| 跨粒度双塔（事实塔+上下文塔） | factg-* 与 sum-* 分离但无显式双塔 | **M2 完整形态**: 事实塔（factg-* 实体级）+ 上下文塔（sum-*/社区摘要文档级）显式分离, 查询选塔 |
| Bundle Search 原子操作 | 检索/推演/验证分散 | **W1 编排器参考**: 检索→推演→验证→生成封装组合件 |
| 时序处理器（之前/之后自动识别） | M5 时间有效性计划中 | M5 自然延伸: 时序关系自动标注 |
| 逻辑校验（矛盾检测） | CRITIC 已有 | 已覆盖 ✓ |
| 5+ 跳推理 | crawl 预算内展开 | 图路由后可达（与 M2 同批） |

### 对 W4/W2/M2 的最终启示
- **W4 记忆主动推销的图路由版**：不等查询，先按"预期话题"选路径（M-FLOW 的路由决策层 + 我们的 N7 形态学 + 主动预载）
- **W2 因果推演的边**：M-FLOW 的 CAUSE 边是 LLM/规则构建——我们用 W3（记忆时间线统计挖掘）优先 + EvoCause 校验补充
- **M2 社区双级检索**：M-FLOW 双塔 = 完整形态（事实塔 + 上下文塔），落地时照此结构
