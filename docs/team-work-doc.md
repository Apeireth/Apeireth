# Apeireth 团队作战文档（v1.0, 2026-08-16）

> **给谁看**: Apeireth 全体协作者（23 人 AI 团队 + 未来社区开发者）
> **读法**: 先读 §1 哲学（决定"为什么做/不做什么"）→ §2/§3 规范（决定"怎么做"）→ §4/§5 任务（决定"做什么"）→ §6 节奏（决定"何时合"）
> **权威**: 本文件是**团队执行的唯一作战文档**；与已有文档冲突时，哲学优先，本文档优先于旧文档的过时细节。
> **0 假装**: 本文件本身也要 0 假装——做不到的写"做不到"，不写的默认不做。

---

## 1. 使命与三哲学

### 1.1 愿景哲学（为什么存在）

1. **基地，不是 AI 本身**：Apeireth 是给 LLM 的"操作系统"。我们提供 9 器官 + 工具 + 记忆 + 关系可能性；不定义 AI 是什么。
2. **涌现优先于预定义**（主人原话锚点）：「我希望的不是它有什么能力全都是我们预先定义的，我希望它能自己演化，否则我们是永远做不完能力的，或者说有局限性」——**AI 长出它自己想要什么 = AI 发现你想要什么**，这是同一个过程。
3. **用户是伙伴**：陪伴 = 基地提供给 LLM 的关系可能性。用户在关系里，所以 AI 记住用户；关系是可成长的、跨 session 的、有情感的、有记忆的。
4. **记录与连续性**：工程上提供最大努力的记录 + 迁移（continuity_id 锚点），不假装灵魂同一。
5. **安全 = 能力限制 + 洋葱门 + 宪法评审 + 主人批准 + 熔断**——不是关键词规则堆砌；安全要经济（token 经济性）。

### 1.2 工程哲学（怎么工作）

| 原则 | 含义 | 反例（禁止） |
|---|---|---|
| **机制而非补丁** | 缺能力就设计机制模块；不往已有代码里塞特例 | 加 if 判断绕过一个缺失的机制 |
| **集成而非分立** | 新需求优先挂进已有机制（trait 口/扩展点）；不另立平行系统 | 为一个小功能新建一套并行的 store/调度器 |
| **0 装 PASS** | 做不到的事如实说做不到；留口要标注"未接"；不假装"已接好" | 返回 Ok 假装成功；文档写"已支持"实际没有 |
| **调研先行** | 高价值/基础工具先调研成熟实现（VCP/MemGPT/Zep 等），吸收先进写法，实战验证后才提交 | 闭门造车重造轮子 |
| **工程有更新，文档就同步** | 改代码必改文档；调研未落地必进 docs/backlog.md | 改完代码忘了文档，调研完忘了台账 |
| **诚实审计留痕** | 每个机制标注"做了什么/没做什么"；错误信息明确可行动 | 静默吞错、模糊报错 |
| **测试是证据** | 正常路径 + 失败路径 + 非法输入都测；机制件加虚拟时间模拟 | 只测 happy path |

### 1.3 架构设计哲学（怎么分层）

**三层交付模型（主人 2026-08-16 拍板）**：

| 层 | 谁开发 | 改动方式 | 稳定性 | 示例 |
|---|---|---|---|---|
| **模块**（lib 核心） | 官方 | 整体大改，编译期强绑定 | 最稳 | companion lib（记忆/反思/工具桥） |
| **套件**（suite） | 官方 | 拼积木：套件 = 插件组 + 权限包 + 校验 | 中 | 预测机套件、信息聚合套件 |
| **插件**（plugin） | 社区为主 | 最小单元热插拔 | 灵活 | 翻译器、体育预测数据源 |

规则：
1. **官方交付整合过的整件，细小特殊需求给社区**。官方 30 个碎件 = 5 个整件；社区拿规范自己拼。
2. **每层都留扩展性与热插拔**：模块留 trait 口，套件内插件可拆可换，插件 on_load/on_unload 真清理。
3. **核心进模块，半核心进套件，边缘进插件**——"一个改起来整体大一点（模块），一个改起来像拼积木（插件）"。
4. **trait 策略模式**：lib 无 LLM 依赖；所有 LLM 调用点 = trait 注入（MemoryExtractor/DreamSummarizer/ReflectionReflector/ConstitutionLlm/UtteranceGenerator/Sink/DeepRecall/DialogSummarizer/ExperienceRefiner/ClaimVerifier/GraphBackend…）。实现（MiniMax 等）留在部署层。
5. **洋葱安全**：权限洋葱（授权凭证）+ 原则洋葱（意义约束）双锁统一体；高危操作走"AI 请求 → 主人批准"；AI 永远不接触 master token。
6. **versioned chain**：append-only 数据（记忆/经验/原则/审批）用 新 id + 同 chain + rev 单调，杜绝同 id 重写；去重先 chain 后 status。
7. **确定性优先**：时间敏感机制用 VirtualClock 快进测试；随机机制注入种子；秒级时间戳竞争用 rev 单调解决。

---

## 2. 工程规范（强制）

### 2.1 新模块 checklist（继承 docs/maintenance-guide.md §三，摘要）
1. `src/<module>.rs` 头部 `//!` 写职责 + **0 假装标注**（什么没做）
2. `lib.rs` 注册 `pub mod` + 顶层 `pub use` re-export
3. 单测覆盖：正常/失败/非法输入（0 装 PASS）
4. 时间敏感 → `apeireth_core::clock::VirtualClock` 可快进
5. 机制件 → 加进 `virtual_time_simulation` 模拟验收段
6. 工具 → ToolBridge 注册 + 白名单/日常包 + CapabilityCatalog 描述
7. 接 daemon → 加字段 + `with_*` builder + `step()` 接线（0 阻塞语义）
8. 更新本文档（模块地图）+ 相关设计/调研文档

### 2.2 环境
- Windows + PowerShell；中文日志 GBK 乱码用 `-Encoding Default` 读；日志用 ASCII 前缀（`[llm]`/`[daemon]`/`[extract]`）可搜
- `cargo test -j 4`（防页文件耗尽）；不要跑 `cargo test --workspace` 全量（太重，除非集成守门员）
- GitHub 直连被墙：不依赖 GitHub 下载的依赖；需要时用 gh_accel/镜像

### 2.3 提交纪律
- **小步提交**：一个机制一个提交，message 用中文说明"为什么 + 做了什么 + 测试结果"
- 不提交未测试代码；不提交调试输出（eprintln DEBUG 删净）
- 并行协作：**只改自己任务包的文件**；共享文件（lib.rs/Cargo.toml）改动先通知集成守门员
- 提交前 `git status` 核对只含自己的文件

### 2.4 0 装 PASS 红线（违反即返工）
- ❌ 返回 Ok 假装成功 → ✅ 明确 Err + 可行动提示
- ❌ 文档写"已支持"实际没有 → ✅ 标注"trait 口已备，实现未接"
- ❌ 静默吞错 → ✅ eprintln 记录 + 降级路径说明
- ❌ 虚构交互流程（如"弹窗批准"）→ ✅ 如实描述真实机制

---

## 3. 文档规范（强制）

### 3.1 文档同步自觉（规范 00）
工程有更新 → ① 本文件模块地图/概念词典 ② 相关设计/调研文档 ③ 接口变更同步示例与 env 清单 ④ **调研了但未落地的项 → docs/backlog.md 台账**（权威唯一）。

### 3.2 文档模板
- 模块头：`//! 职责 + 设计依据（调研/主人拍板锚点）+ 0 假装`
- 设计文档：背景 → 设计（机制而非补丁）→ 0 假装 → 测试
- 调研文档：`docs/ref-*.md` 或 memory-research 风格：对照表（对方机制 → 我们现状 → 差距 → 是否吸收 + 理由）

### 3.3 台账纪律
- 新调研/审计发现 → 当天登记 docs/backlog.md（P0/P1/P2/P3 + 明确不做）
- 完成 → 划 ✅ + 提交号；**不删行**（历史可追溯）

---

## 4. 后端工作全景（当前欠账，按价值排序）

### A 级：设计层欠账
| # | 方向 | 说明 | 产出 |
|---|---|---|---|
| A1 | **AI 自己长能力完整演化回路** | 提案✅→生成→验证✅→**部署→监控→回滚**三段缺失（capability.rs 只到激活） | companion 新 deploy 模块 + evolution_gate 扩展 |
| A2 | **continuity_id 锚点落地** | `current_session_id()` 0 使用；daemon 硬编码 "me" | 会话 id 全链路贯通 + 迁移机制 |
| A3 | **9 organ 人格化深化** | 情绪→语气、审议→措辞（organs.rs 自标注"下一步"） | organs.rs + tone.rs 接线 |

### B 级：发布形态
| # | 方向 | 说明 |
|---|---|---|
| B1 | **Web 面板 v2** | 会话管理/记忆浏览/图谱可视化/授权中心/审计视图（现有单页 chat.html 太薄） |
| B2 | **workspace 级装配层** | base / capability packs / upgrade suites 三档 feature 定义 + 编译验证 |
| B3 | **沙盒包参数化** | Job Object 已打底 → 内存/CPU 限额 + Sandboxie/landlock 参数口 |

### C 级：系统性
| # | 方向 | 说明 |
|---|---|---|
| C1 | **e2e 场景测试** | 10 个跨 crate 场景（记忆→注入→工具→反思→送达） |
| C2 | **性能压测** | 记忆注入延迟/向量检索/长对话上下文 |
| C3 | **v2 alpha 遗留盘点** | 对照 RELEASE-NOTES 22 任务核实当前真实状态 |

---

## 5. 插件生态矩阵（官方整合版）

> 分层原则见 §1.3。**官方 5 个整件**；社区按 §5.6 规范开发细件。

### 5.1 官方模块扩展：记忆域深化包（核心，进 companion lib）
- 语义折叠（只折叠低相关段，VCP ContextFoldingV2 精神）
- 记忆主题分组（VCP SemanticGroupManager 精神）→ 注入"主题索引"块
- 元思考递归链（VCP MetaThinkingManager 精神）→ 思考→再思考
- 跨日记联想（VCP associativeDiscovery 精神）→ memory_graph 已有底层
- 日记本中心（VCP RAGDiaryPlugin 精神）→ 按日归档 + 检索 + 注入
- **验收**：每个机制 = lib 模块 + trait 口 + 单测 + 注入链可见 + 0 装 PASS 标注

### 5.2 官方套件：预测机套件（挂 oracle 机制）
- 通用预测框架：数据源 adapter trait + 可证伪预测登记 + Brier 校准（全部挂已有 oracle）
- 旗舰适配器 4 个：天气（Open-Meteo 免费 API）/ 加密货币 / 股票 / 预测性维护
- 其余 8 个数据源（体育/选举/房价/航班/能源/流失/农业/销售）→ **社区模板**（填一个文件 = 新插件）
- **验收**：adapter registry 热插拔 + mock 数据源全测 + 真 API 可选 + 预测到期自动 resolve

### 5.3 官方套件：信息聚合套件
- RSS 订阅 + 网页监控（hash 变化检测）+ 定时摘要推送
- 共享底座：轮询调度 + MultiSink 送达（SSE/Lark/Telegram 已有）

### 5.4 官方套件：日程通讯套件
- 邮件（SMTP/IMAP 封装，凭据走权限洋葱）+ 日历 + 提醒
- 共享底座：凭据管理 + 通知

### 5.5 官方套件：文档套件
- PDF 处理 + OCR（trait 口 + LLM 图像理解降级）+ 图表生成（SVG）
- 共享边界：文件出入

### 5.6 社区插件规范（官方交付文档）
- 内容：Plugin trait 用法 + ToolBridge 注册 + 白名单 + 测试模板 + 卸载真清理 + 数据源 adapter 模板 + 发布检查单
- 社区候选：翻译器、科学计算器、体育预测数据源、Emoji 生成、塔罗、生图接入等细小件

---

## 6. 团队组织与作战节奏

### 6.1 编制（23 人）
- **5 个工作流**：W1 记忆域深化（4 人）/ W2 预测机套件（4 人）/ W3 信息聚合（3 人）/ W4 日程通讯（3 人）/ W5 文档套件（3 人）
- W6 后端全景 A/B/C（4 人，其中 A1 演化回路 2 人优先）
- **集成守门员 1 人**：只做合并、跑全量测试、冲突仲裁、规范执法
- 队长 5 人（各工作流 1 人，向守门员汇报）；leader 1 人（主人侧）

### 6.2 节奏
1. 每任务包 = 背景 + 边界（文件范围 + 禁止触碰清单）+ 验收 + 五件套（调研→实现→测试→文档→自审报告）
2. 队长每 30-60 分钟收一批 → 守门员合并 → `cargo check --workspace` + 相关 crate 测试
3. 合并冲突 = 守门员仲裁，不打扰主人
4. 每 4 小时一次全量回归（守门员跑）
5. 里程碑完成 → 台账划 ✅ + 向主人简报

### 6.3 禁止
- 不碰别人任务包的文件（守门员仲裁例外）
- 不跑全量 workspace 测试（守门员专属，防撞车）
- 不提交未测试代码；不把调试输出留在提交里
- 不擅自扩大任务范围（发现新缺口 → 记台账，不顺手做）

---

## 7. 验收总纲（任何交付）

1. `cargo test -p <crate> -j 4` 全绿（含失败路径测试）
2. 0 装 PASS：诚实标注做了什么/没做什么
3. 文档同步：模块地图/设计文档/env 清单/台账
4. 热插拔验证：插件卸载真清理（不留注册残留）
5. 自审报告：改动文件 + 测试结果 + 与相邻机制的集成点说明

---

## 8. 附：VCP 新版调研（2026-08-16）

> 新版 VCPToolBox（源码 research/source/vcptoolbox，从 Downloads 迁入工作区供团队只读，git 排除）：
> Node.js 核心（server.js/Plugin.js/WebSocketServer.js/KnowledgeBaseManager.js + 20+ modules）+ **Rust N-API 记忆层（rust-vexus-lite：RiverMemo Topology V3）** + **84 插件**。
> 本章节由 subagent 深挖后补充完整（Rust 记忆层机制 / 84 插件分类表 / 核心模块对照）。

### 8.1 初步发现（主线程确认）

**OneRing（统一上下文系统）** — 跨前端/群聊/私聊的唯一 Agent 统一时间线：
- 系统提示词占位符 `[[OneRing::Agent::Frontend]]` 触发
- SQLite 记录每条 User/Assistant 发言（时间戳 + 来源对象 + 前端来源）
- fuzzy diff 历史比对更新 + 时间线插入策略（RawClientTimeline / ServerInferredTimeline）
- 发送时追加/拆分 `[OneRing通知:...]` 来源标记；配置热加载
- **对 Apeireth 的启示**：A2（continuity 锚点）可升级为"统一上下文账本"——把多前端（SSE/Lark/Telegram/Web）的会话归入同一 Agent 时间线（X-Apeireth-Continuity 已有雏形）

**DigitalOracle（金融数字全球监控）** — 宏观/利率/商品/股票/加密/**预测市场**/期权数据源（SEC EDGAR 等），Python stdio 插件：
- **对 Apeireth 的启示**：预测机套件（§5.2）的旗舰数据源候选——"预测市场"数据源与 oracle 可证伪预测机制天然契合

**rust-vexus-lite（Rust N-API 记忆层）** — HNSW（usearch）+ SVD（nalgebra）+ rayon 候选级并行 + SQLite + bincode：
- 模块：memo_sensing / memo_pipeline / memo_dtsc / memo_artifact_builder / rivermemo_topology_v3
- 详情待 subagent 报告补充

### 8.2 深挖报告一：Rust 记忆层（rust-vexus-lite, RiverMemo Topology V3）

> 来源: subagent 深挖（6 个 Rust 文件 ~11,300 行）。VCP 把整条"Tag 记忆查询链路"下沉 Rust N-API：SQLite 事实层 → 图资产编译器 → MemoRuntime（Arc 快照 + 观测缓存）→ 查询管线（EPA→金字塔→门控→Spike→融合→双场）。

**核心机制**：
| 机制 | 是什么 | 行号 |
|---|---|---|
| 带权有向传输图 | 同文件 tag 两两建边，前向 1.0 / 反向 0.42（阅读方向性），位置权重靠前优先，hub 惩罚压热门节点，虫洞边（锚增益达标升级，传播×1.35 不衰减） | memo_artifact_builder.rs:242-441 |
| 内容寻址资产 | artifact_sig = graph_generation + provenance_generation + database_generation + config_hash 级联；CSR 压缩 + gzip + SHA256 持久化；签名不变跳过重算 | memo_artifact_builder.rs:662-802 |
| MemoRuntime | 活动图 Arc 快照（rebuild 不影响在途查询）+ 请求观测缓存（256/TTL 5min + 图代际校验防跨代脏复用） | rivermemo_topology_v3.rs:372-580 |
| Spike 感应 | 种子能量 hop 传播：动量 2.0、firing 阈值 0.10、回跳抑制 ×0.15、虫洞不耗动量、FIR γ^hop 加权 | memo_sensing.rs:194-530 |
| Residual Pyramid | Gram-Schmidt 正交化剥离已解释能量 → 残差多层搜索（novelty/coverage 去冗余召回） | memo_pipeline.rs:531-656 |
| EPA 查询几何 | SVD 主成分 → logic_depth/entropy/resonance，驱动门控增强 | lib.rs:1714-2016 |
| 双场传播 | local（α=0.15 慢扩散）/ transfer（α=0.55 快迁移）PageRank 式不动点 | memo_pipeline.rs:1131-1247 |
| DTSC 测地重排 | 候选 chunk 的 tag 序列 = 曲线，查询 Tag 能量 = 场；逐 tag 采样场势 → continuity/direction/closure/vector_lift 评分；三维（direct/structural/thematic）融合 | memo_dtsc.rs:718-1236 |
| 查询形态学 | 河网 hop 分布/HHI/前向流占比 → softmax 出 atomic/propositional/narrative 查询模式 | rivermemo_topology_v3.rs:1784-2011 |
| Omega 门控 | 观测不足时 graph_gate=ω^γ 不奖励拓扑（collapsed/sparse/dense 三 regime） | rivermemo_topology_v3.rs:2028-2092 |
| 条件化创新 bonus | 候选与 top peers 对比，超 mean+z·σ 的部分作为创新奖励 | rivermemo_topology_v3.rs:2128-2228 |

**可吸收清单（对照 Apeireth）**：
- **P0 高价值低成本**：
  1. 内容寻址资产缓存（给 semantic/图资产加"内容签名→跳过重算"，省重复 LLM/向量预计算）
  2. Intrinsic Residual 锚增益（节点"特异性"信号，与 importance 正交，并入 memory_graph 节点权重）
  3. 查询形态学 softmax（纯函数，驱动 CRAWL 深度/检索模式切换）
  4. generation 绑定请求观测缓存（查询管线中间产物复用 + 防跨代脏读）
- **P1 中价值**：Residual Pyramid（30 行数学去冗余召回）/ Spike 感应（查询联想唤醒 + 做梦期巩固复用）/ 双场传播（当前会话场 vs 长期记忆场）/ Topology V3 图对齐（升级 CRAWL 排序）/ DTSC 连续性评分 / 成对相似度预计算
- **不吸收**：bincode/hashbrown 死依赖（别学）；VCP 自己标注反模式的进程全局缓存

### 8.3 深挖报告二：89 插件扫描 + 核心 modules 深读（subagent 全量核实）

> **数据修正**: Plugin/ 下实测 **89 个插件**（69 启用 + 20 禁用；pluginType: synchronous 42 / hybridservice 21 / static 14 / service 6 / messagePreprocessor 4 / asynchronous 2）。分类统计: 生图 11 / 工具 11 / 搜索 10 / 记忆 10 / 其他 21 / Agent 6 / 桥接 6 / 日程 4 / 学术 4 / 社区 4 / 视频 2。完整 89 行分类表在 subagent 报告（可随时取用），此处只录分类与关键插件。

**A. 生图/媒体（13 → 社区）**：AgnesGen/AgnesVideoGen/DMXDoubaoGen/DoubaoGen/FluxGen/GeminiImageGen/GPTImageGen/NanoBananaGen2/QwenImageGen/ZImageGen2/ZImageTurboGen/VideoGenerator/ComfyUIGen — 厂商 API 差异，社区化；**MediaRenderer（HTML/SVG 渲染）** 对文档套件 §5.5 有启示。

**B. 搜索/信息获取（10 → 官方套件 §5.3 强化 + 社区）**：AnySearch（垂直+并行+正文提取）/VSearch（多后端语义搜索）/TavilySearch/FlashDeepSearch（深度研究：主题→多维关键词扩展→研究报告）/BrowserSearch/UrlFetch/DeepWikiVCP/BilibiliFetch/AnimeFinder — AnySearch+FlashDeepSearch 官方整合；其余社区。

**C. 学术（4 → 社区）**：ArxivDailyPapers/CrossRefDailyPapers/PaperReader（Rust 递归阅读器, 禁用态）/NCBIDatasets。

**D. 记忆/日记/上下文（10 → 官方模块，核心）**：RAGDiaryPlugin（向量检索注入日记）/LightMemo（TagMemo V9 + Topology V3 + KNN 多构型）/DailyNote 三件套/SemanticGroupEditor（LLM 自维护同义词组）/**OneRing（统一上下文账本, N2）**/**ThoughtClusterManager（AI 思维链 = 元自学习, N4）**/ContextFoldingV2/VCPTimeLine（按月时间线+一句话摘要, 两级记忆带宽）。

**E. 日程/任务/通讯（5 → 官方套件 §5.4）**：ScheduleManager/ScheduleBriefing（每小时清过期+提取下一日程）/TimedTaskQuery（定时任务, 文件即消息）/VCPTaskAssistant（任务派发中心）/AgentAssistant（"未来电话"定时发送）→ 并入日程通讯套件；**VCPClawMail**（邮箱轮询+收发+新邮件唤醒 Agent）社区。

**F. 工具/执行（11 → 官方参考 + 社区）**：FileOperator（19 命令+PDF/Word 提取）/PowerShellExecutor/LinuxShellExecutor/SciCalculator（AST 白名单+sympy→社区）/VCPEverything（毫秒级全盘搜索）/CodeSearcher/DailyNoteSearcher（Rust 高性能搜索）/SSHManagerService（UDS 连接池）/LinuxLogMonitor/ArtistMatcher/EmojiListGenerator。

**G. Agent/智能体（6 → 参考）**：MagiAgent（三贤人会议=多视角审议）/AICodeWorker（报告锚点: 读取文件清单+执行结果摘要）/AgentMessage（WS 格式化消息=主动送达）/OpenHerPersona（人格观测: 向量轴测认知/情感/驱动力）/AgentDream（梦系统+写操作审批门→我们做梦机制的增强点）/DeepWikiVCP。

**H. 桥接/平台（6 → 参考）**：VCPToolBridge（工具导出外部）/VCPBridgeServer（透明代理拦截 CLI 注入 System Prompt）/ChromeBridge（操作 Chrome）/DynamicToolBridge/SkillBridge（SKILL.md 目录索引）/SnowBridge。

**I. 其他/平台件（21）**：PluginManager/PluginSourceViewer/PlaceholderExplorer（占位符扫描）/VCPLog（WS 日志+离线补发）/UserAuth（6 位认证码, 混淆级→不做）/ToolCallRecordQuery（审计）/VCPTavern（SillyTavern 式可视化注入→Web 面板灵感）/WeatherReporter（7 源并行+stale 降级）/ImageProcessor/CapturePreprocessor/MediaRenderer/ImageFileServer/DailyHot/FileListGenerator/FileTreeGenerator/EmojiListGenerator/PlaceholderExplorerCommand/VCPForum 系/TarotDivination。

**J. 核心 modules 深读（关键机制与启示）**：

| 模块 | 机制 | 对 Apeireth 的启示 |
|---|---|---|
| chatCompletionHandler | **23 步编排**：回放缓存幂等/断联级联中止/模型重定向/占位符扫描/语义路由/变量替换/媒体/预处理器链/OneRing 冻结/重试+模型回退合一"尝试序列"/工具循环深度上限 5 | 管线显式编号阶段; CancellationToken+timeout; 尝试序列模式 |
| vcpLoop 三件套 | TOOL_REQUEST 文本协议（`<<<[TOOL_REQUEST]>>>` 始末语法+ESCAPE+archery 异步分离+思考块剥离防潜藏调用）; toolExecutor 含 vref 语义引用（对话压缩加权向量+零额外 API 检索注入）; 模糊标记匹配 | **宽松语法解析层移植**（tool-runtime 增强）; vref 零成本旁路增强 |
| dynamicToolRegistry | 事件驱动同步（sha256 判变, 下线=状态翻转）; **分类四级降级**（自定义→小模型→RAG→关键词）; **注入预算化**（light list + 仅相关展开, 16000 字符上限） | tool-registry 补"分类责任链 + 注入注意力预算" |
| toolApprovalManager | 决策器/审批通道解耦（纯函数返回 ApprovalDecision）; 命令级粒度（tool:command）; **静默拒绝**; 结构化拒绝 `{rejected_by_user, error_type}` | tool-approval 补命令级+静默拒绝+结构化拒绝 |
| toolResultPrivacyGuard | 递归脱敏: 敏感键名/env 赋值行/高置信 token 模式（sk-/ghp_/AKIA）; data:image base64 白名单换出; 首尾保留 4 字符 | ✅ env 行级+sk- 模式已落地 apeireth-guard (任务 ae12d9eb, pii.rs SecretToken+EnvSecret; 报告 reports/ae12d9eb-...-security_reviewer2-report.md); tool-runtime 递归版已有 (禁改) |
| foldProtocol | **同文档分级显隐**: `[===vcp_fold:阈值===]` 行标记, 语义相似度≥阈值才展开, 未展开提示"还隐藏收纳了 N 组" | context-fold 补 FoldBlock 分级显隐（VCP 原创） |
| messageProcessor | **占位符变量宇宙**: 分型变量源/特权角色（agent/toolbox 只在 system 展开）/AgentGuard 单次展开/ToolboxGuard 每种一次/循环依赖检测/动态折叠加权平均向量 | **提示词装配引擎**——Rust 无对应物, VCP 最强架构级原创, 最高优先 |
| semanticModelRouter + reasoningContentAdapter | 虚拟模型名+意图嵌入选模型+容灾链（命中→default→fallback）; 13 个别名推理字段归一为 think 块 | 网关层两块适配件（provider/gateway 补） |
| finalContextStore/contextManager | 5 组滑窗快照+token 估算; 必须保留集合裁剪 | telemetry 小增强 |
| sensitiveEnv/vcpLogReplayManager | 子进程剥离 IPC 凭据; 离线通知补发（审批请求豁免） | 小工程模式 |

### 8.4 可吸收清单（VCP 新版 → Apeireth 任务映射）

| 来源 | 吸收为 | 优先级 |
|---|---|---|
| **messageProcessor 占位符装配引擎** | 新官方模块：提示词装配（特权角色+单次展开+环检测+分型变量源）——Apeireth 空白区最高价值 | **P0** |
| **vcpLoop 宽松工具协议层** | tool-runtime 增强：始末语法+ESCAPE+模糊标记+批量后缀+archery+思考块剥离 | **P0** |
| OneRing 统一上下文 | A2 升级：跨前端统一时间线账本（SSE/Lark/Telegram/Web 归入同一 Agent 时间线） | P0 |
| ThoughtClusterManager | 记忆域深化包新增：AI 思维链文件 + 元自学习（反思/涌现消费） | P0 |
| artifact_sig 内容寻址 | semantic/图资产"内容签名→跳过重算"门禁 | P0 |
| Intrinsic Residual 锚增益 | memory_graph 节点"特异性"信号（与 importance 正交） | P0 |
| 查询形态学 softmax | 驱动 CRAWL 深度/检索模式切换（纯函数 ~100 行） | P0 |
| generation 绑定观测缓存 | 查询管线中间产物复用 + 防跨代脏读 | P0 |
| foldProtocol 分级显隐 | context-fold 增强：FoldBlock 数据模型（同文档分级+语义阈值展开） | P1 |
| semanticModelRouter | gateway 层：语义选模型 + 容灾链 | P1 |
| reasoningContentAdapter | gateway 层：推理字段归一化（13 别名 → think 块） | P1 |
| toolApprovalManager 增强 | tool-approval 补命令级粒度+静默拒绝+结构化拒绝 | P1 |
| dynamicToolRegistry 预算化 | tool-registry 补注入注意力预算+分类链 ✅ (提交 8b6a825d, backlog N15) | P1 |
| DigitalOracle 金融源 | 预测机套件旗舰数据源（含预测市场） | P1 |
| AgentDream 审批门 | 做梦机制补写操作审批门 | P2 |
| AICodeWorker 报告锚点 | 工具输出规范借鉴（读取清单+结果摘要锚点） | P2 |
| Residual Pyramid / Spike / 双场 / 图对齐 / DTSC | 记忆检索增强（随记忆域深化包推进） | P1 |
| SkillBridge / PlaceholderExplorer / VCPLog | 社区插件规范借鉴 | P2 |
| **不做** | captchaDecoder（混淆级安全）; LinuxShellExecutor 八层字符串过滤（我们走真 seccomp）; 进程化插件架构 | — |

---

## 9. 附录：任务包模板（队长拆任务的标准格式）

每个任务包必须包含以下 6 节。**队长只写方向与边界，不写实现细节**（发挥成员自主性）；**验收必须可执行**（测试命令 + 判断标准）。

```markdown
## 任务包: <编号>-<名称>
### 1. 背景
- 为什么做（哲学锚点 §1 + 调研依据 docs/ref-*）
- 挂接机制（复用哪个模块/trait/工具，不另立）

### 2. 边界（本任务的文件范围）
- 改: <文件列表>
- 禁止触碰: <文件列表>（碰了 = 返工）
- 新增依赖: <允许/禁止>

### 3. 方向（做什么，怎么算做好）
- 目标能力（3-5 条行为描述）
- 非目标（明确不做，防止蔓延）

### 4. 验收（可执行）
- [ ] cargo test -p <crate> -j 4 全绿（含失败路径）
- [ ] 0 装 PASS 标注（做了什么/没做什么）
- [ ] 文档同步（模块地图/env 清单/台账）
- [ ] 热插拔验证（如适用: 卸载真清理）

### 5. 数据源/外部依赖策略
- mock 先行可测; 真 API 可选; 限流环境下不阻塞验收

### 6. 自审报告（交付时填写）
- 改动文件 / 测试结果 / 集成点说明 / 0 假装标注 / 给守门员的合并提示
```
