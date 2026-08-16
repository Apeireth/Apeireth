# 记忆/上下文深度调研（2026-08-16）

> 主人指示: 记忆/上下文是 AI 界钻研很久的领域, 调研 VCP 实现 + 其他优秀记忆项目,
> 为记忆系统升级提供参考。**原则: 吸收机制设计, 不照抄代码。**

## 一、VCP 记忆体系（本地源码逐模块调研）

### 1. 日记本中心（RAGDiaryPlugin, 23 万字节主插件）
- 所有记忆以**日记/文档**结构化存储, 时间天然有序 — 记忆 = 时间线
- 我们的对照: episodes 平铺 + 时间戳 ✅ 同构; 缺"叙事整理"（做梦摘要近似, 但不完整）

### 2. AIMemoHandler（对话时 LLM 构建"动态记忆场"）
- **推理式召回**: 五步框架 (解构意图 → 多维探针网络 → 时间感知扫描 → 跨时空关联综合 → 剪枝聚焦)
- 输出 = 结构化证据综合 (保留关键细节/引用, 不是模糊概括)
- 我们的对照: recall_memory = 关键词匹配 (v1) + deep_recall = LLM 重排 (暗示词触发, v2)
  → **差距: 我们的推理召回是"有条件触发", VCP 是"每次对话都推理"** (成本/限流权衡, 见 §三)

### 3. LightMemo（BM25 + 分级记忆）
- jieba 分词 + BM25 检索 (k1=1.5, b=0.75) + 分级存储
- 我们已吸收为 lightmemo L1-L4 (分层/衰减), 但 **serve 未接完整分级** (注入排名是 v1)

### 4. ContextFoldingV2（语义折叠 — 关键差异点）
- 对正文中**远距离、低相关性**的 AI 输出做摘要折叠 (相关性阈值 0.5, 语义判定)
- 异步并发摘要 (maxConcurrent 5) + 折叠标记 `[VCP上下文语义折叠-本层摘要:...]`
- 我们的对照: 滚动摘要 = **全量折叠** (窗口外全摘要) → **差距: VCP 是选择性折叠** (只折叠低相关)
  我们的窗口裁剪更简单但可能丢掉仍相关的内容; VCP 方式更省 token 更精准

### 5. SemanticGroupManager（语义分组）
- 记忆按主题聚类 (向量缓存 + 分组), 查询命中分组 → 上下文注入
- 我们的对照: **无** (无主题聚类) → 可吸收 (记忆主题分组注入)

### 6. MetaThinkingManager（元思考递归链）
- meta_thinking_chains.json 配置链式思考模板, 递归推理链
- 我们的对照: 无 → 可选吸收 (深度反思的思考框架)

### 7. FoldingStore（折叠存储）
- SQLite, maxEntries 200 + 淘汰策略 (evict 20) — 上下文折叠的持久化
- 我们的对照: 滚动摘要是内存态 (每次请求重新摘要) → VCP 持久化折叠结果 ✅ 值得吸收

### 8. associativeDiscovery（跨日记联想）
- TagMemo 算法 + 向量库, 跨日记本语义联想 (发现隐藏关联)
- 我们的对照: 无 → 可选 (与语义分组同源)

## 二、AI 界优秀记忆项目（subagent 调研, 2026-08-16）

### 1. Generative Agents（Stanford 2023）
- **importance 捕获打分**: 写入时 LLM 按提示词打分 (1=平淡~10=深刻), 非规则
- **检索打分**: score = α_recency·recency + α_importance·importance + α_relevance·relevance;
  recency 按**最后访问时间**指数衰减 (0.995/小时), 每次检索刷新 — 高频记忆永不冷
- **反思触发**: 非定时, 最近 100 条事件 importance 之和 > 150 才触发; 反思携带引用 memory id (证据溯源), 递归成树

### 2. MemGPT / Letta
- **分层**: main context (窗口) vs external (archival 向量库 + recall 对话史) 硬分界; 核心块常驻
- **self-editing**: LLM 持有改记忆的工具 (append/replace/insert/search), 何时写由 LLM 自主决定
- **memory pressure**: 窗口将满 → 注入警告 → LLM 主动 evict 旧消息/滚动摘要 (OS 换页)
- **sleep-time compute**: 空闲时异步记忆补给 (重读会话→抽新记忆→去重合并→重写), 可用更强模型

### 3. Mem0
- **LLM 对账**: add() = 抽候选 → embedding 检索存量 → **LLM 判定 ADD/UPDATE/DELETE/NONE**
  (冲突→删/改旧, 语义被包含→UPDATE 合并, 全新→ADD) — 全程 LLM 判断
- 每条带 importance + access_count + 时间戳; 三级 scope (user/session/agent)

### 4. Zep / Graphiti
- **双时态边**: facts 存 (subject, predicate, object, valid_at/invalid_at); 变化时旧边置 invalid, 历史留痕
- **幻觉检测**: 新事实写入后 LLM 反向校验与既有图知识矛盾 → 矛盾拒收
- **摘要历史图**: 每会话一个摘要节点, 链接成 episode 树, 可回溯

### 5. A-MEM（Zettelkasten 式）
- 每条记忆 {content, tags, links, 时间}; WRITE 时 LLM 生成对既有 note 的**带权链接**;
  REWRITE 合并重叠; **CRAWL** 写入时沿最相似 note 链接展开 (图检索)

### 6. Anthropic memory tool / context editing
- 条目类型化: {content, type: fact|preference|commitment|task, context, access_count}
- 操作语义 = create/update/delete 工具 (按 memory_id), 服务端去重合并
- context editing: agent 可对自身上下文片段 rewrite/delete/compress

## 三、可吸收设计清单（对照我们的现状）

| 设计 | 来源 | 我们的差距 | 吸收成本 |
|---|---|---|---|
| 推理式召回 (每次对话 LLM 综合) | VCP AIMemoHandler | deep_recall 仅暗示词触发 | 中 (限流环境下需节流) |
| 选择性语义折叠 | VCP ContextFoldingV2 | 滚动摘要全量折叠 | 中 (需语义相关度判定) |
| 折叠结果持久化 | VCP FoldingStore | 滚动摘要是内存态 | 低 (存 store) |
| 记忆主题分组 | VCP SemanticGroupManager | 无 | 中 |
| importance 捕获打分 + last-access 衰减 | Generative Agents | 提炼器无 salience/access 字段 | 低 (提炼器加字段) |
| 累计 importance 触发反思 (非纯周期) | Generative Agents | 反思纯周期 | 低 (配合模块 5) |
| 提炼器对账 (ADD/UPDATE/DELETE) | Mem0 | 单向捕获, append-only 重复/矛盾并存 | 中 (最大升级点) |
| 做梦补去重/重写 (sleep-time) | Letta | 做梦合并无去重 | 低 |
| memory pressure 分页警告 | Letta | 注入预算硬截断, agent 不参与取舍 | 低 |
| 双时态边 + 幻觉校验 | Zep | 无 | 高 (图谱, 后续) |
| 摘要链可回溯 | Zep | 滚动摘要无链接 | 低 |
| 带权链接 + CRAWL 图检索 | A-MEM | 无 | 高 (后续) |
| memory type 加 task + access_count | Anthropic | 提炼器分类无 task | 低 |

## 四、记忆 v2 升级包（建议实施, 不是补丁 — 一个"记忆生命周期"模块）

把低/中成本项归并为**一个记忆生命周期模块** (v2), 顺序实施:

1. **字段升级**: 提炼器/写入打 importance (LLM 打分) + access_count/last_access (注入时刷新)
   → 注入排序 = importance + last-access 衰减 + 主题 (对齐 Generative Agents 打分公式)
2. **提炼器对账化** (Mem0 式): 候选 vs 已有记忆 LLM 判定 ADD/UPDATE/DELETE
   → 治 append-only "同一事实存七遍/新旧矛盾并存" (当前真实问题)
3. **反思触发**: 累计 importance 阈值 + 周期双触发 (省 LLM 调用, 语义化)
4. **做梦去重/重写** (Letta sleep-time): 合并前先去重 (embedding 近重复), 摘要重写
5. **折叠持久化 + 摘要链**: 滚动摘要存 store + 链接上次摘要 (可回溯)

高成本项 (Zep 图谱 / A-MEM 图检索) 记入 backlog, 等基础扎实后再做。
