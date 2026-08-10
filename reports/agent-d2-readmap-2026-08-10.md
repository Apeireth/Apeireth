# Agent D-2 — D2-1 Readmap (R25 2026-08-10)

> **任务**: Apeireth-rust 战区 5 tool protocol — `apeireth-tool-registry` 加小模型分类器
> **出处**: v2.0 strategy Step 5 + v2.1 路线图 §0.3 + v2-strategy/07-VCP-GAP §P0-4
> **节奏**: 7h (D2-1 1h 读全 → D2-2 2h trait+Heuristic → D2-3 1.5h Embedding+Llm → D2-4 1h 集成 → D2-5 1h tests+examples → D2-6 0.5h 报告)

## 1. 现有 tool-registry 全貌 (2026-08-10 02:55 baseline)

### 1.1 src/ 模块 (5 个,总 1.9KB→实 100+ KB)

| 文件 | 行数 | 角色 | 关键 API |
|---|---|---|---|
| `lib.rs` | 203 | 入口 + 编译期 hardcode | `pub const TOOL_KIND_COUNT = 6` + 6 mock re-export |
| `registry.rs` | 600+ | ToolRegistry CRUD + 6 mock + notify 热加载 | `register/get/list/list_by_kind/len/watch_plugin_dir` |
| `trait_def.rs` | 156 | `Tool` async trait (4 方法) + `ToolDescription` | `name/kind/axes/call` |
| `types.rs` | 470 | 6 类 enum + 5 轴 struct | `ToolKind::all()` + `ToolAxes::default_for_kind()` |
| `token_budget.rs` | 270+ | VCP §6.2.2 #15 借鉴 | 3 const + `estimate_tool_tokens/truncate_to_token_budget` |

### 1.2 tests/ (R18 #2.4 已加 10 个, 内容 156 行)

`tests/registry.rs` 156 行, 7 个 mock impl + 10 测试:
- registry_register_and_get / registry_get_nonexistent_returns_none / registry_unregister_removes_tool
- registry_list_sorted / registry_len_and_is_empty / registry_clear_empties_all / registry_overwrite_same_name
- registry_call_through_get (async) / registry_failing_tool_returns_err (async) / registry_list_by_kind

### 1.3 examples/ (1 个)

`examples/registry_demo.rs` 5.6KB — 演示注册 3 mock + 6 类分组 + call 真跑 + token 预算 + notify 热加载。

### 1.4 lib.rs 的编译期 hardcode (TOOL_KIND_COUNT = 6 + 5 轴 243 组合)

`lib.rs:90-124 const _: () = { ... }` 块:
- 6 类 enum = 6
- AXIS_COUNT = 5, AXIS_COMBINATION_COUNT = 243
- LIGHT/BRIEF/MIN/MAX 4 const 跟 VCP 真值
- MOCK_TOOL_COUNT = 6

**D-2 行为**: **0 触碰** lib.rs 编译期 hardcode, 只在 `pub mod` 块加 1 行 `pub mod classifier;` + 1 段 re-export。

## 2. VCP dynamicToolRegistry 思路 (字段级引用)

### 2.1 7 类 CATEGORY_RULES (`dynamicToolRegistry.js:40-80`)

| # | VCP 类别 | 关键词 (中英) |
|---|---|---|
| 1 | `search` | search/web/lookup/query/retrieval/google/tavily/serp/url/paper/citation/搜索/检索/网页/查询/论文/资料 |
| 2 | `file_code` | file/code/read/write/edit/patch/repo/git/directory/文件/代码/仓库/读取/写入/编辑 |
| 3 | `image_media` | image/photo/picture/media/video/audio/ocr/screenshot/图片/图像/视频/音频/截图 |
| 4 | `memory_knowledge` | memory/knowledge/rag/diary/note/vector/context/知识/记忆/日记/笔记/向量 |
| 5 | `agent_task` | agent/task/schedule/plan/workflow/assistant/任务/计划/调度/代理 |
| 6 | `communication` | mail/email/message/notification/push/forum/wechat/telegram/邮件/消息/通知/推送 |
| 7 | `data` | json/csv/excel/sql/database/table/parse/数据/表格/数据库/解析 |

**D-2 行为**: 1:1 抄 (per 主人偏好 #6 "借鉴 1:1 翻译" + 0 假装 #7 "不假装, 直接抄 VCP 关键词")

### 2.2 3 层 fallback (VCP §1003-1000)

```javascript
async _classifyRecord(record, reason) {
    if (this.classifier) return ...                          // 1. 自定义 classifier
    const smallModelResult = await this._classifyWithSmallModel(record, reason);  // 2. 小模型
    if (smallModelResult) return smallModelResult;
    const embeddingResult = await this._classifyWithEmbeddings(record);          // 3. embedding
    if (embeddingResult) return embeddingResult;
    return this._fallbackClassify(record);                                       // 4. 关键词兜底
}
```

**D-2 行为** (跟 VCP 1:1):
1. **LlmClassifier** (接 OpenAI-compat 小模型 endpoint, 走 HTTP, mock 接口签名)
2. **EmbeddingClassifier** (接 `Arc<dyn EmbedFn>`, 9 类中心向量 cosine)
3. **HeuristicClassifier** (关键词字典, 0 远程依赖, 兜底)

### 2.3 关键词字典 1:1 + 1 个 fallback 类别

VCP `_fallbackClassify` (line 1214-1238): 0 匹配 → push `'general'`, confidence 0.45。
**D-2 行为**: 0 匹配 → `Category::General` (10 类别中第 10 个? 不,我们要 9 类别,General 不是其中之一)。

**冲突点**: VCP 7 类 + general fallback = 8 类别。任务要求 9 类别 = VCP 7 + safety + long_running。**General 怎么办?**

**决策** (per 决策权):
- **A**: General 当第 10 个类别 — 突破"9 类别"硬要求
- **B**: General 是特殊兜底 (Confidence < 0.5 时), 9 类别不包含 → 严守任务要求
- **C**: 删掉 General, 9 类别兜底回 Search (因为搜索类关键词最广)

**倾向 B**。理由: 0 假装 (VCP 兜底是 'general', 我们对齐行为但归类不同); 严守任务"9 类别"硬要求; `Category::General` 在 `confidence() < 0.5` 时返回, 不在 9 enum 之内, 走 `Result<Category, ClassifyError>` 兜底。

## 3. 9 类别设计 (D-2 自定)

### 3.1 9 enum + 1 句理由

| # | Category | VCP 对标 | 理由 (1 句) |
|---|---|---|---|
| 1 | `Search` | search | VCP 1:1 抄 (关键词 'search/web/lookup' 覆盖查询类) |
| 2 | `FileCode` | file_code | VCP 1:1 抄 (文件/代码/git 操作合并, VCP 真代码就这样) |
| 3 | `ImageMedia` | image_media | VCP 1:1 抄 (图片/视频/音频/OCR 多模态) |
| 4 | `MemoryKnowledge` | memory_knowledge | VCP 1:1 抄 (记忆/RAG/笔记/向量检索) |
| 5 | `AgentTask` | agent_task | VCP 1:1 抄 (代理/任务/计划/调度) |
| 6 | `Communication` | communication | VCP 1:1 抄 (邮件/消息/推送/IM) |
| 7 | `Data` | data | VCP 1:1 抄 (JSON/CSV/SQL/数据库) |
| 8 | `Safety` | (Apeireth 独有) | 红队 / 自禁用 / 4 重守门护送类, 5 重守门 v5 修正 + Self-Disable §3 用 |
| 9 | `LongRunning` | (Apeireth 独有) | > 5min 预期时长 (训练/索引/批处理), pipeline 调度需特判 |

**General (兜底)**: 不在 9 enum 之内, 9 类别都 confidence < threshold → 返 `Err(ClassifyError::NoMatch)` (跟 VCP 行为对齐, 但用 `Result` 类型更 Rust 风格)。

### 3.2 关键词字典设计

**D-2 决策**:
- VCP 7 类关键词: 1:1 抄 (中英双语, 包括 OCR 这种特殊 token)
- Safety 关键词: redteam/红队/self_disable/自禁用/护栏/guardrail/safety/sanitize/validate/permission/权限
- LongRunning 关键词: train/训练/index/索引/batch/批处理/migrate/迁移/compile/编译/crawl/爬取/embedding
- 通用模式: 1 个 tool name 可匹配多类 (VCP `selected.slice(0, 3)` 上限 3 类), Apeireth `Category` 单选 (Rust enum), 取首个匹配

**冲突点**: VCP 多类 vs Apeireth 单类。
**决策**: Apeireth 单类 (`Category` enum 不可组合), 匹配多个 → 按"优先级排序"取首个。
**优先级** (按安全敏感度): Safety > LongRunning > Memory > FileCode > Data > Search > AgentTask > Communication > ImageMedia
**理由**: Safety 第一优先 (任何含 "permission/guardrail" 的 tool 立刻标 Safety, 防 prompt injection 伪装)

## 4. 3 实现排序 + 接口设计

### 4.1 trait 设计 (per 任务规范)

```rust
// crates/apeireth-tool-registry/src/classifier.rs

pub trait Classifier: Send + Sync {
    fn classify(&self, tool: &Tool) -> Result<Category, ClassifyError>;
    fn confidence(&self, tool: &Tool) -> Result<f32, ClassifyError>;
}
```

**问题**: `&Tool` 怎么拿? `Tool` 是 trait, 没法直接 `&Tool`. VCP 用 `(record, ...)`, 我们的 Tool trait 没有 pluginName/description/usage 等 metadata 字段 (只 4 方法: name/kind/axes/call)。

**决策**: 3 选项:
- **A**: 加 metadata 字段到 `Tool` trait (破硬约束 #6 "0 触碰 24 LOCKED" — 不, Tool trait 不在 24 LOCKED 但会触发 R18 #2.4 已加测试)
- **B**: 加 `pub fn metadata(&self) -> &ToolMeta` 默认实现方法 (extension pattern, 用 blanket impl)
- **C**: 借用 Tool trait 的现有 4 方法 + 注册时另存 metadata 侧表

**倾向 B**。理由:
- 不破 Tool trait 现有签名 (向后兼容)
- 借用 Rust extension pattern, 实现者可选 override, 默认返空
- 加 `pub trait ToolMetaProvider: Tool` (新 trait, 强类型扩展)

**简化**: 任务规范要求 9 类别实现, 不是完整插件管理系统。直接给 `Classifier` 传 `&Tool` + 借助 trait 默认方法 + 不强求 metadata。

**最终 trait 签名**:
```rust
pub trait Classifier: Send + Sync {
    /// 单选分类 (Apeireth 简化, VCP 是多类 top-3)
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError>;
    /// 置信度 0.0 .. 1.0
    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError>;
}
```

**问题**: `tool: &dyn Tool` 需要 dyn 兼容, `Tool` 当前是 `async_trait`, dyn 兼容需要额外 #[async_trait] 配置。

**替代方案**: 加 `Tool::display_name(&self) -> &str` 默认方法, 用 name() 别名即可 (现有 name() 已经够)。

**决策**: trait 签名用 `&dyn Tool` (async_trait 0.1 默认 dyn 兼容), 所有 3 实现都用 `tool.name()` + `tool.kind()` + (可选) 自行维护 tool metadata 侧表。

**最终 trait (修正)**:
```rust
pub trait Classifier: Send + Sync {
    /// 对单个 tool 分类
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError>;
    /// 置信度 0.0 .. 1.0
    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError>;
}
```

### 4.2 3 实现排序

| 顺序 | 实现 | 文件 | 依赖 | 0 远程 | 优先级 |
|---|---|---|---|---|---|
| 1 | `HeuristicClassifier` | classifier/heuristic.rs | 0 | ✅ | 兜底 (D2-2) |
| 2 | `EmbeddingClassifier` | classifier/embedding.rs | `Arc<dyn EmbedFn>` (本地 trait) | ✅ (mock 模式) | 进阶 (D2-3) |
| 3 | `LlmClassifier` | classifier/llm.rs | HTTP client (async-trait + 0 真实调) | ❌ (mock 接口) | 远程 (D2-3) |

**理由**:
- Heuristic 立刻可测 (0 LLM 依赖), D2-2 主战场
- Embedding 接 mock (MockHashEmbedFn 写死 FNV-1a 32 维), 0 远程依赖
- LlmClassifier 留 trait + 1 个 `LlmClassifier::new_mock()` 构造, **不假装**真接 LLM (per 主人偏好 #3 #7)

### 4.3 EmbedFn trait 设计 (本地, 跟 memory 对齐)

```rust
/// 本地 EmbedFn trait (跟未来 apeireth-memory::semantic::EmbedFn 形状对齐)
pub trait EmbedFn: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}
```

**跟 VCP 1:1 对照**:
- VCP `getEmbedding(text) -> Promise<Vec<f32>>` (line 1108) — 异步, 但我们的 trait 同步简化 (跟 `apeireth-memory::semantic::EmbedFn` bench 写法一致)
- 真实接入: 留 R21+ 给 `apeireth-llm-gateway` 注入 (D-2 范围外)

**Mock 实现**: `MockHashEmbedFn` — FNV-1a 32 维, 确定性 hash, 0 远程, 0 随机 (跟 bench DeterministicEmbedder 1:1 抄)

## 5. 集成 plan (D2-4)

### 5.1 registry.rs 扩展

新增方法:
```rust
impl ToolRegistry {
    /// 注册时附加分类 (Apeireth 简化, VCP 是 observer pattern)
    pub fn register_with_classifier(
        &self,
        name: String,
        tool: Arc<dyn Tool>,
        classifier: &dyn Classifier,
    ) -> Result<Category, ClassifyError>;

    /// 按类别查
    pub fn tools_by_category(&self, category: Category) -> Vec<String>;

    /// 列出所有类别 + 工具数
    pub fn category_summary(&self) -> BTreeMap<Category, Vec<String>>;
}
```

**新字段**:
```rust
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    /// 类别索引 (name → Category, 一次性写入, 后续不修改)
    categories: RwLock<HashMap<String, Category>>,
    // ... 既有字段不动
}
```

**0 触碰现有方法**: register / get / list / unregister 等 7 个 CRUD 方法签名 0 改 (向后兼容 R18 #2.4 测试)。

### 5.2 lib.rs re-export

加 1 段:
```rust
pub mod classifier;
pub use classifier::{
    Category, ClassifyError, Classifier,
    HeuristicClassifier, EmbeddingClassifier, LlmClassifier,
    EmbedFn, MockHashEmbedFn,
};
```

**编译期 hardcode**: 新增 `CATEGORY_COUNT: usize = 9;` 跟 VCP `CATEGORY_COUNT = 7` 对齐 (但我们要 9 类别), const assert 在 `lib.rs`.

## 6. 验收 hard 路径

| 硬指标 | 路径 | 状态 |
|---|---|---|
| `cargo check -p apeireth-tool-registry --lib --tests --examples` exit 0 | 单一 crate 检查 | D2-5 验 |
| `cargo test -p apeireth-tool-registry` 0 failed | unit + integration | D2-5 验 |
| `cargo run -p apeireth-tool-registry --example classify_smoke` 跑通 | 3 classifier 都给结果 | D2-5 验 |
| HeuristicClassifier 准确率 ≥ 80% (9 demo) | unit test 跑 9 demo 算命中率 | D2-5 验 |
| 0 改 workspace.version | git diff Cargo.toml | D2-6 验 |
| 0 触碰 24 LOCKED | git diff apeireth-cognition/core/sovereignty/formal | D2-6 验 |
| 不与 A/B/C 冲突 | git diff 不交叉 | D2-6 验 |

## 7. 风险 + 决策权

### 7.1 风险

- **R1**: `async_trait` dyn 兼容 (0.1 默认不支持, 需要 `#[async_trait(?Send)]` 或 `Box<dyn Tool>`) — 缓解: `&dyn Tool` 试, 失败换 generic
- **R2**: `apeireth-memory::semantic::EmbedFn` 引用但未实现, D-2 抄过来对吗? — 缓解: 查证, 如果不存在就本地定义 (已是结论: 本地定义)
- **R3**: 9 类别 vs 7+VCP general 兜底 冲突 — 缓解: General 不在 9 enum, 走 `Result::Err(NoMatch)` 兜底
- **R4**: 9 类别优先级排序主观 — 缓解: 写"按安全敏感度排序"理由, 让主人 R26 调

### 7.2 决策权 (per 任务)

- 9 类别具体哪 9 个 ✅ 已定 (本 readmap §3.1)
- 3 实现排序 ✅ Heuristic → Embedding → Llm
- 关键词字典来源 ✅ VCP 1:1 抄 + Safety/LongRunning 自创

### 7.3 不做的事

- ❌ 0 引入 `fastembed` (重编译, 加重 deps, 留 R21+)
- ❌ 0 触碰 24 LOCKED (apeireth-cognition/core/sovereignty/formal)
- ❌ 0 改 Tool trait 4 方法签名
- ❌ 0 假装 LLM 真接 (留 mock + trait)
- ❌ 0 主动 commit (留主人拍板)

## 8. 阶段产物清单

- [D2-2] `crates/apeireth-tool-registry/src/classifier.rs` (trait + Category enum + ClassifyError)
- [D2-2] `crates/apeireth-tool-registry/src/classifier/heuristic.rs` (HeuristicClassifier + 9 类关键词字典)
- [D2-3] `crates/apeireth-tool-registry/src/classifier/embedding.rs` (EmbeddingClassifier + EmbedFn trait + MockHashEmbedFn)
- [D2-3] `crates/apeireth-tool-registry/src/classifier/llm.rs` (LlmClassifier mock + 留 OpenAI-compat endpoint 接口)
- [D2-4] `crates/apeireth-tool-registry/src/registry.rs` 改: 加 2 字段 + 3 方法 (register_with_classifier/tools_by_category/category_summary)
- [D2-4] `crates/apeireth-tool-registry/src/lib.rs` 改: pub mod classifier; re-export 9 type
- [D2-5] `crates/apeireth-tool-registry/src/classifier.rs` 单元测试 (≥ 10 个)
- [D2-5] `crates/apeireth-tool-registry/tests/classifier_integration.rs` 集成测试 (≥ 5 个)
- [D2-5] `crates/apeireth-tool-registry/examples/classify_smoke.rs` (3 classifier + latency 报告)
- [D2-6] `reports/agent-d2-final-2026-08-10.md` + `reports/agent-d2-decision-log-2026-08-10.md`

## 9. 开工时间盒

| 阶段 | 时间 | 累计 | 内容 |
|---|---|---|---|
| D2-1 | 0-1h | 1h | 读全 (本 readmap) |
| D2-2 | 1-3h | 3h | trait + Category + Heuristic |
| D2-3 | 3-4.5h | 4.5h | Embedding + Llm mock |
| D2-4 | 4.5-5.5h | 5.5h | 集成到 registry.rs |
| D2-5 | 5.5-6.5h | 6.5h | tests + examples |
| D2-6 | 6.5-7h | 7h | final report + decision log |
