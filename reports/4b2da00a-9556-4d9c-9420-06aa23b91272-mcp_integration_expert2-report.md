# 自审报告 — MCP 集成专家2

- **任务ID**: 4b2da00a-9556-4d9c-9420-06aa23b91272
- **任务**: dynamicToolRegistry 预算化（tool-registry 注入注意力预算 + 分类四级降级链, P1）
- **角色**: mcp_integration_expert2 | **产出类型**: code
- **提交**: `8b6a825d feat(tool-registry): VCP 吸收 — 注入注意力预算 + 分类四级降级链`
- **结论**: ✅ 验收全项达成（139 lib 测试全绿 / 0 装 PASS 如实标注 / 文档三处同步）

## 一、任务理解与现状确认（重置哲学：动手前先确认）

开工前先核实 crate 现状，发现既有资产（避免重复造轮子）：

| 既有资产 | 来源 | 本任务态度 |
|---|---|---|
| `token_budget.rs`（LIGHT=15 / BRIEF=6 / MAX=16000 常量 + 截断函数） | R17 战役 2-1 | **复用**，0 重写 |
| `classifier.rs`（9 类别 + Classifier trait + Heuristic/Embedding/Llm 三实现） | R25 战区 5 | **复用**，关键词级直接用 HeuristicClassifier |
| 事件驱动同步（notify 热加载, registry.rs watch_plugin_dir） | R17 | **保留**，0 触碰（任务明确"不重写"） |

缺口（本任务补齐）：① 无"light list + 仅相关展开 + 截断提示"的注入渲染器；② 无"自定义→小模型→RAG→关键词"四级降级编排（原 3 实现各自独立，无责任链、无决定级记录）。

对标 VCP 新版源码 `research/source/vcptoolbox/modules/dynamicToolRegistry.js`：
- `buildInjection` (line 560-649)：light list 段 + expandedKeys 仅相关展开 + `maxBriefListItems`/`maxExpandedPlugins` 上限
- `_truncateInjection` (line 1390-1395)：超 `maxInjectionChars` 截断 + suffix 提示"可点名索取更多"
- `_classifyRecord` (line 986-1000)：descriptionOverride → small model → embeddings → keyword 四级 fallback

## 二、实现内容（只改 apeireth-tool-registry）

### ① 注入注意力预算 — `src/injection.rs`（新增）

| 组件 | 说明 |
|---|---|
| `InjectionBudget` | `max_chars`（默认 16000 = VCP maxInjectionChars）/ `max_expanded`（默认 5）/ `max_light_items`（0=不限） |
| `InjectionEntry` | name + brief（一行简介）+ details（完整详情）；`from_description(&ToolDescription)` 为**描述注入挂接点** |
| `render_injection(entries, relevant 闭包, budget)` | 三段式渲染 |
| `InjectionOutput` | text（保证 ≤ max_chars）+ truncated + expanded（实际展开名）+ hidden_light |

渲染行为（VCP buildInjection 1:1 精神）：
1. 空注册表 → "no tools available"（0 假装）
2. 轻量清单：全部工具 `- name: brief`，单行过 `LIGHT_LIST_TOKEN_BUDGET`(15 token) 截断
3. 仅相关展开：`relevant(name)` 为真的工具展开 details（≤ max_expanded）
4. 超预算裁剪顺序：砍展开段尾 → 砍轻清单尾行（计 hidden_light）→ 硬切 + `TRUNCATION_HINT`（"truncated by maxInjectionChars; request a specific tool for more detail"）

**挂接边界**：crate 内不依赖 prompt 装配；调用方（tool-runtime/pipeline，属 backend_engineer2 辖区）拿 ToolDescription → InjectionEntry → render_injection → 文本拼 system prompt。本次不改 tool-runtime，只留干净 API。

### ② 分类四级降级链 — `src/chain.rs`（新增）

| 级 | 状态 | 载体 |
|---|---|---|
| 1 自定义 | ✅ 实装 | `CustomMapClassifier`（name → Category 映射表, 0 远程, 命中=满置信） |
| 2 小模型 | 🔌 trait 注入口 | `Option<Arc<dyn Classifier>>`（可接 LlmClassifier 等；未接真模型） |
| 3 RAG | 🔌 trait 注入口 | `Option<Arc<dyn Classifier>>`（可接 EmbeddingClassifier 等；未接真模型） |
| 4 关键词 | ✅ 实装 | `HeuristicClassifier`（关键词字典 1:1 VCP, 0 远程兜底） |

- 任一级 `Ok` 即定案，`ClassifyOutcome.stage` 记录决定级（Custom/SmallModel/Rag/Keyword）+ 置信度；`Err` 降级下一级；全失败 → `NoMatch`（0 假装）
- `has_custom()/has_small_model()/has_rag()` 如实报告接入状态
- `ClassifyChain` 自身 `impl Classifier` → 可直接传给既有 `ToolRegistry::register_with_classifier`（挂接点，0 改 registry）
- `CHAIN_LEVELS = 4` 编译期断言（lib.rs）

### ③ lib.rs 挂接
`pub mod injection; pub mod chain;` + pub use 导出 9 个符号 + `CHAIN_LEVELS` 编译期断言。

## 三、验收核对

| 验收项 | 结果 |
|---|---|
| `cargo test -p apeireth-tool-registry -j 4` 全绿 | ✅ **139 lib 测试全过**（+8 example 测试 +20 doc 相关套件，0 failed）；`cargo check --all-targets` 干净（examples 编译通过） |
| 预算内路径 | ✅ `within_budget_keeps_light_list_and_relevant_details`（3 工具, 1 相关, 全保留, truncated=false） |
| 超预算路径 | ✅ `over_budget_drops_expansions_first_and_truncates_with_hint`（24000 字符→先砍展开段）+ `tiny_budget_hard_truncates_with_hint`（120 字符硬切留提示）+ `over_budget_counts_hidden_light_lines`（轻清单尾行计 hidden_light） |
| 空注册表 | ✅ `empty_registry_renders_notice` |
| 分类降级各路径 | ✅ keyword-only / custom 定案 / custom miss 降到 keyword / small_model 定案 / small_model Err 降到 RAG / 三级全 Err 到 keyword / 四级全 miss → NoMatch（7 个测试） |
| 0 装 PASS | ✅ 未接的 LLM/RAG 级**如实标注**：`has_small_model()==false`/`has_rag()==false` 有专门测试；文档注明"trait 注入口, 未接真模型"；无任何 mock 冒充真分类 |
| 文档同步 | ✅ maintenance-guide §2 模块地图新行 + backlog N15 登记并标 ✅ + team-work-doc §8.4 该行标 ✅ |

测试修正记录（不假装）：初版 `tiny_budget_hard_truncates_with_hint` 用 300 字符预算失败——渐进裁剪（砍展开→砍轻清单）后头部框架 ~149 字符已能塞下，属正确行为；改为 120 字符（< 头部自身）才真正触发硬截断路径。

## 四、边界遵守

- ✅ 只改 `crates/apeireth-tool-registry`（injection.rs / chain.rs / lib.rs）+ 三处文档
- ✅ 0 触碰 tool-runtime（N10, backend_engineer2 辖区）/ tool-approval（security_reviewer 辖区）/ context-fold
- ✅ 事件驱动同步（notify 热加载）保留未动
- ✅ 0 新依赖；0 unsafe（workspace deny）

## 五、风险与后续建议（记 backlog / 留给 Leader）

1. **真接线待后续**：`render_injection` 需 tool-runtime/pipeline 侧拿 ToolDescription 调（挂接点已留, 属 backend_engineer2 的 N10 邻域）；`ClassifyChain` 可直接替换 register_with_classifier 的 classifier 参数。
2. **小模型/RAG 级真接**：LlmClassifier（R21+ 计划）/ EmbeddingClassifier（已有, fastembed 留 R21+）成熟后可直接注入链，无需改 chain。
3. **N14 脏树**：工作区存在他人未提交的 companion 改动致其编译失败（安全审查2 已登记），与本任务无关，未触碰。
4. 未发现新的可吸收项（本任务即 §8.4 既列项的落地）。

## 六、验证方式（复现）

```bash
cargo test -p apeireth-tool-registry -j 4      # 139 + 8 + 20 全绿
git show 8b6a825d --stat                        # 3 files, +793
```
