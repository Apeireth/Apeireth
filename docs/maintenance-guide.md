# Apeireth companion 维护指南（2026-08-16）

本文件是**维护用的活文档**：概念词典、模块地图、加新模块的规范、生命周期接线点。
改代码前先看这里，避免概念混用。

## 一、概念词典（澄清易混词）

### 能力栈（上位 → 载体 → 打包）
```
能力 (capability)  ← 上位概念: AI 能做的事 (可预定义, 也可 AI 自己长出来)
├─ 动作 (action)     = 主动动作形态 (问候/问进展/提议帮助/提醒, 动作空间)
├─ 工具 (tool)       = 基地工具形态 (recall_memory / save_memory / FileOperator...)
├─ 技能 (skill)      = 流程封装形态 (CapabilityKind::Skill, 如「换元陪练」)
载体: 插件 (plugin)  = 能力的可分发/可卸载载体 (注册工具+权限+生命周期)
打包: 套件 (suite)   = 插件组的官方打包 (一键装「完整能力」, 如教育/渗透/预测机)
      能力包 (pack)  = 80%→完全体的基础补全 (沙盒/审计/多通道/GUI)
```
**规则**: 对话/文档里「能力」泛指上位; 具体说形态时用 动作/工具/技能; 说生态时用 插件/套件。

### 易混词对
| 词 | 含义 | 易混对象 |
|---|---|---|
| **权限包** (PermissionPack) | 授权凭证 (覆盖哪些工具/多久/预算) | **能力包** (CapabilityPack) = 发布物, 不同物! |
| **continuity_id** | 哲学锚点: 跨载体/重启稳定身份 (唯一真相) | **session_id / subject** — 工程里三者曾混用, 现在统一以 continuity_id 为锚, subject 只是它的别名 |
| **做梦** (dream) | 记忆整合的夜间周期 (合并+摘要写回) | **consolidation** (记忆巩固, 工程别名) |
| **反思** (reflection) | 4 阶段反思周期 (写回反思记录) | — |
| **涌现** (emergence) | 主动/能力「长出来」的机制 (非写死) | — |

### 生命周期词
- 白昼: 主动涌现 (节律+驱动+门禁) → 渲染 → 送达
- 夜间: 做梦 (6h 无互动 → 合并+摘要写回)
- 周期: 反思 (24h → 4 阶段写回)
- 演化: AI 提案能力 (propose_capability) → 宪法评审 → 批准 → 激活 → catalog 动态段 → 可部署为插件

## 二、模块地图（crates/apeireth-companion/src/）

| 模块 | 职责 | 关键接线 |
|---|---|---|
| emergence.rs | 涌现循环: 节律/驱动/门禁/反馈 | AwakeCompanion.loop_ |
| organs.rs | 全器官: 情绪/审议/演化/主权/洋葱 | daemon.awake |
| actions.rs | 动作空间 + CapabilityCatalog (静态+动态) | 渲染层 |
| daemon.rs | 总装: 心跳/做梦/反思/送达 | CompanionDaemon |
| dream.rs | 做梦调度 (SleepCycle+DreamSubsystem+摘要) | daemon.dream |
| reflection.rs | 反思周期 (ReflectionCycleScheduler) | daemon.reflection |
| judicator.rs | 宪法评审 (LlmJudicator 按 E 层判案) | ToolBridge.with_judicator |
| tool_bridge.rs | 工具桥: 洋葱门/评审/权限/路径/隔离/spill/post 钩子 | 干活链路 |
| packs.rs | 权限包 (授权凭证) | ToolBridge.packs |
| security.rs | SecurityGate / SovereigntyGate | ToolBridge.gate |
| exec_worker.rs | 执行体隔离 worker (per-call 子进程) | ToolBridge.with_isolation |
| spill.rs | 工具结果溢出 (超大输出落私有文件) | ToolBridge.with_spill |
| continuation.rs | 续行快照 (原子写+崩溃恢复) | multi_turn 循环 |
| session_log.rs | 事件溯源会话 (append-only 日志+surface+崩溃修复) | 多轮循环 |
| goal.rs | Goal 状态机 (AI 长目标, 严格 fold+持久化) | 独立 |
| capability.rs | 能力提案 (AI 自己长能力第一段) | propose_capability 工具 |
| plugin.rs | 插件机制 (生态最小单元) | ToolBridge.registry |
| suites.rs | 三件套目录+装配 (套件=插件组) | install_with_plugins |
| prompt_cache.rs | Prompt Cache 稳定化 (稳定前缀+动态单点) | 渲染组装 |
| tone.rs | Bond→语调提示 | 渲染层 |
| daily_summary.rs | 每日摘要数据源 | §6.4 UI 后端 |
| clock.rs (core) | 虚拟时钟 (时间机制快进测试) | 全部时间敏感模块 |
| constitution_gate.rs | 结构化宪法硬门 (编译期规则表, 零成本, LLM 评审前) | ToolBridge (全风险级别) |
| memory_injection.rs | 反幻觉记忆注入 (闭世界证据: 编号列表+禁止声称记得) | 渲染层 |
| confidence.rs | Beta-Binomial 置信度 (数学化自信度) | capability / 自测 |
| evolution_gate.rs | 验证闸门流水线 (fix loop/no-progress/预算 fail-open/回滚收据) | 能力演化回路 |
| oracle.rs | 预言机套件核心: WorldState/ScenarioEngine/Forecast+Brier+BetaBinomial 校准/DecisionEngine expectimax/ForecastRegistry | simulate/forecast 工具 |
| web_crawl.rs (tools) | Crawl v2: 并发 BFS+重试退避+限速 (调研驱动, 实战验证) | Crawl 工具 (9 号) |
| education.rs | 教育套件真内容: dx_check 规则层检查器 (忘换 dx/混用/缺微分/残留 x/根号模式表) + EducationDxPlugin (注册+授权, 卸载真清理) | dx_check 工具 |
| pentest.rs | 渗透套件真内容: recon_plan (计划编排+E-1 范围闸) + scan_report (nmap 行解析) + 双插件 (卸载真清理) | recon_plan/scan_report 工具 |
| gh_accel.rs (companion) + github_accel.rs (tools) | GitHub 加速插件: xiake.pro 节点池 → 本机并发实测 (2xx+PNG魔数内容验证) → 选最快 → 加速 URL/命令 (docs/ref-gh-accel.md) | gh_accel 工具 (插件注册) |
| audit.rs | 审计能力包: audit_log 工具 (留痕查询, masked 脱敏不还原, append-only) | audit_log 工具 (内置注册) |
| append_only.rs (memory) | HistoryStream 新增 list_recent (最近 N 条, 审计/摘要用) | 6 流共享 |
| experience.rs | 自成长 Level 0/1: 经验库 (场景/做法/结果/验证计数/EMA) + 达标促能力提案 (versioned chain, rev 单调) | save/list/verify_experience 工具 |
| principles.rs | 自成长 Level 2/3: 动态原则层 (AI 提案→主人 master token 批准→执行检查拦截) + 晋级候选导出 (内层=主人侧工程) | propose/approve_principle 工具 |
| approval_requests.rs | 授权请求机制: 工具被拒→待批请求 (apreq-*, 同参数去重) → 前端轮询展示+一键批准 (权限洋葱真实载体) | GET /v1/apeireth/approval-requests |
| memory_extractor.rs | 通用记忆提炼器: LLM 提炼 facts/preferences/commitments/emotional/graph (带 importance) + Mem0 式对账 (ADD/UPDATE/DELETE, tomb 逻辑删除) + 偏好库 (pref-*) + active_episodes 过滤 | 对话后节流 + 6h 批量 |
| memory_graph.rs | 时序知识图谱 (Zep 双时态边 factg-*, rev 链内单调) + A-MEM 带权链接/CRAWL (link-*, 规则重叠) + 注入【事实图】 | graph 三元组 + crawl 注入 |
| goal_tools.rs | 目标驱动 (模块 6): goal_create/status/complete/pause/block (严格状态机) | 5 目标工具 |

## 三、加新模块规范（维护 checklist）

0. **基础工具工程原则（强制）**：高可靠性基础工具（爬虫/网络/文件/执行等）**不得独写**——① GitHub 调研同类成熟实现 → ② 吸收先进写法（并发/重试/限速/上限）→ ③ 实战验证（真环境跑通, 如 crawl_probe）→ 才可提交；调研结论记 docs/ref-*.md。

00. **文档同步自觉（强制, 2026-08-16 主人确立）**：**工程有更新, 文档就同步**——新增/修改模块时同步更新: ① 本文件模块地图+概念词典 ② 相关设计/调研文档 (docs/*.md) ③ 接口变更同步示例与 env 清单。**调研了但未落地的项, 必须显式记入对应文档的 backlog/待办节** (项目庞大易忘, 白纸黑字防欠账堆积)。

1. `src/<module>.rs` — 头部写 `//!` 职责 + 0 假装标注 (诚实: 什么没做)
2. `lib.rs` 注册 `pub mod` + 顶层 `pub use` re-export
3. 单测覆盖: 正常路径 / 失败路径 / 非法输入 (0 装 PASS)
4. 时间敏感机制 → 用 `apeireth_core::clock::VirtualClock` 可快进测试 (0 真等待)
5. 若是机制件 → 加进 `virtual_time_simulation` 模拟验收段
6. 若是工具 → ToolBridge 注册 + 白名单/日常包 + CapabilityCatalog 描述
7. 若接 daemon → 加字段 + `with_*` builder + `step()` 接线 (0 阻塞语义)
8. 更新本文档 (模块地图 + 概念词典如有新概念) + 同步相关设计/调研文档
9. 全量 `cargo test --workspace -j 4` (降并行防页文件) + 工作区干净再提交

## 四、环境变量与示例清单

| 示例 | 用途 |
|---|---|
| companion_serve | **伙伴端点 (主入口)**: OpenAI 兼容 + 状态感知 + 记忆生命周期 + 目标/自成长/SSE 主动送达 |
| companion_daemon | 常驻主动问候 (env: TICK/MAX_TICKS/MEMORY_PATH/SUBJECT/MIN_LLM_INTERVAL/SINK/LARK_*/DREAM/REFLECT/SEED_DEMO) |
| production_daemon | 全机制集成验收 (宪法评审+隔离+spill+日志+goal+做梦+反思+每日摘要) |
| release_acceptance | AI 自己长能力端到端 (提案→评审→激活→干活) |
| multi_turn_agent | 多轮 function calling + 断点续传 (--crash-after / --resume) |
| full_acceptance / self_summary_engineering | 一期/二期验收 (真 MiniMax) |
| virtual_time_simulation | 时间机制模拟验收 (23 项, 虚拟时钟) |
| constitution_demo | 宪法评审真 LLM 判案演示 |
| education/pentest/gh_accel 各 demo | 三套件+加速插件演示 |
| exec_worker (bin) | 执行体隔离 worker (被 ToolBridge spawn) |

统一锚点: `APEIRETH_CONTINUITY_ID` (默认 companion-main) — 记忆/日志/目标/反思共用。

### companion_serve 环境变量
`APEIRETH_API_KEY` (必) · `APEIRETH_MASTER_TOKEN` (主人批准用) · `APEIRETH_PORT`/`PORT` (默认 8090)
· `APEIRETH_DEEP_RECALL=1` (推理召回) · `APEIRETH_MAX_TOKENS` (输出上限, 默认 8192)
· `APEIRETH_EXTRACT_INTERVAL_SECONDS` (提炼节流, 默认 600) · `APEIRETH_DREAM_QUIET_SECONDS` (做梦安静期, 默认 6h)
· `APEIRETH_REFLECT_PERIOD_HOURS` (反思周期, 默认 24h) · `APEIRETH_GRANT` (启动即授权 "工具:小时")
· `APEIRETH_LARK_APP_ID/SECRET/RECEIVE_ID` (离线送达, 可选) · `APEIRETH_SEED_MEMORY` (种子, 演示)
