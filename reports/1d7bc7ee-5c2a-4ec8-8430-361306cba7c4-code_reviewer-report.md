# 自审报告: context-fold 增强包 (语义折叠 + N11 FoldBlock 分级显隐)

- 任务 ID: 1d7bc7ee-5c2a-4ec8-8430-361306cba7c4
- 角色: 代码审查 (自审)
- 提交: `1f5d2fd` (代码) + 本文档提交
- 验收命令: `cargo test -p apeireth-context-fold -j 4` → **45 passed; 0 failed**（原 26 + 新增 19）

## 1. 改动文件

| 文件 | 改动 |
|---|---|
| `crates/apeireth-context-fold/src/semantic.rs` | 新增: 语义折叠 (§5.1, VCP ContextFoldingV2 精神) |
| `crates/apeireth-context-fold/src/fold_block.rs` | 新增: FoldBlock 分级显隐 (N11, VCP foldProtocol 精神) |
| `crates/apeireth-context-fold/src/marker.rs` | `MarkerKind::Semantic` 占位类型 (+2 match 分支) |
| `crates/apeireth-context-fold/src/lib.rs` | 注册 2 新模块 + re-export + crate 文档更新 |
| `docs/maintenance-guide.md` | 模块地图补 semantic/fold_block 条目 |
| `docs/backlog.md` | N11 划 ✅ |

**未触碰**: `context.rs` 核心块逻辑、注入管线主干（边界合规，仅纯函数层协作）。

## 2. 设计要点与正确性

### ① 语义折叠 (semantic.rs)
- `RelevanceScorer` trait：score ∈ [0,1]，评分接口纯函数 → **确定性可测**。
- 嵌入可 mock：`Embedder` trait + `EmbeddingScorer`（余弦相似度，维度不匹配/零向量 → 0.0），测试用 `MockEmbedder` 固定向量验证。
- 内置 `BigramOverlapScorer`（字符二元组重叠 |A∩B|/√(|A|·|B|)）：0 依赖、确定性、可测。
- `fold_segments`：score ≥ threshold 保留；score < threshold 折叠为占位行 `[折叠#i score=x.xx] {摘要} <<SEMANTIC:N bytes>>`。
- **无损展开**：占位行含下标保证唯一，`unfold_semantic` 逐一还原原文（测试断言精确相等）。
- 摘要诚实：默认前 N 字符截取（UTF-8 按 chars 安全截断 + 省略号），summarizer 回调可注入，**无内置 LLM**（与 crate 既有 Summary 策略诚实风格一致）。

### ② FoldBlock 分级显隐 (fold_block.rs)
- 行标记 `[===vcp_fold:阈值===]` / `[===vcp_fold:阈值::desc:描述===]`，行级手工解析（无 regex 新依赖），语义对齐 VCP `FOLD_REGEX`。
- 首标记前内容归入 threshold=0.0 前置块（恒展开档），与 VCP 一致。
- `render_fold_blocks(blocks, similarity)`：**threshold ≤ similarity 才展开（≥ 含等号，边界测试覆盖）**；隐藏块整体收纳，尾部留 `[已折叠] 还收纳了 N 组内容 (相似度未达阈值)` 提示（N=0 无提示行）。
- `FoldBlock` derive serde（Serialize/Deserialize，serde 为既有依赖），可作数据模型跨 IPC 传递。

### ③ 与 ContextAssembler 预算截断协作不冲突
- 职责正交：语义折叠决定"留谁"（内容侧），`fold()` 决定"留多少"（预算侧）。
- 测试 `composes_with_budget_truncation` 验证：`fold_segments` 产物再过 `fold(Truncate/HeadTail)` 不破坏、不 panic。
- 空段/全空白段在折叠前丢弃，不占预算。

## 3. 测试覆盖（新增 19，全部在验收命令内）

| 场景 | 测试 |
|---|---|
| 折叠 | folds_low_relevance_segments / render_expands_by_threshold / low_similarity_hides_all_but_zero |
| 不折叠 | no_fold_when_all_relevant / no_markers_single_block / threshold 0.0 前置块恒展开 |
| 阈值边界（含等号） | threshold_boundary_equal_kept / threshold_boundary_equal_expands |
| 空段 | empty_segments_dropped / empty_blocks_render_empty / empty_content_yields_empty_blocks |
| 无损展开 | unfold_restores_original_losslessly / fold_block_serde_roundtrip |
| 降级/异常 | non_finite_threshold_fail_open_keeps_all / non_finite_similarity_treated_as_zero / invalid_threshold_line_is_content / cosine 维度不匹配·零向量 |
| UTF-8/确定性 | utf8_summary_truncation_safe / bigram_scorer_deterministic_and_ordered |

## 4. 0 装 PASS 标注（做了什么 / 没做什么）

**做了**：语义折叠评分+占位+无损展开；FoldBlock 解析+分级渲染+收纳提示；serde 数据模型；与预算截断协作测试。
**没做（诚实标注）**：
- 无真实嵌入模型接入（`Embedder` trait 口已备，mock 验证；真接入属向量检索层职责）；
- 摘要无内置 LLM（默认截取 + 回调口）；
- 阈值非法行按内容处理（VCP 正则行为等价）；空文档 → 空块列表（VCP 会塞兜底文案，Rust 侧交由调用方，不假装）；
- 未接线注入管线主干（边界禁止；本包只提供纯函数层，接线属后续集成任务）；
- `R144_DELIVERABLES = 3` 常量保留（R144 历史交付数，新模块不篡改 organ 不变量，r177 organ 5 测试仍全绿）。

## 5. 评分（自审）

| 维度 | 分 | 理由 |
|---|---|---|
| 正确性 | 9/10 | 边界语义（≥含等号/NaN降级/空输入）均有测试钉死；扣 1 分：占位行唯一性依赖下标前缀，极端并发拼接场景未测（当前无此用法） |
| 兼容性 | 10/10 | 只增不改既有 API；`MarkerKind` 新增变体仅扩 match；既有 26 测试全绿无回归 |
| 可维护性 | 9/10 | 纯函数+trait 口，0 新依赖，模块文档含 0 装标注与 VCP 参考路径 |
| 测试覆盖 | 9/10 | 19 新测试覆盖任务要求 4 类场景 + 降级路径；Kani 证明未扩（organ 既有 2 条仍有效，新模块纯函数性质可后续补） |
| 风险 | 低 | crate 隔离，无人接线即无行为变化（0 装 PASS） |

**综合：9.2/10** — 可合并。

## 6. 给守门员的合并提示

- 改动仅限 `crates/apeireth-context-fold/`（4 文件）+ 2 文档；无 Cargo.toml 依赖变化。
- 与他人并行任务无文件交集（guard/pii.rs 的修改属 backend 成员，未触碰）。
- 全工作区构建未跑（边界纪律只测本 crate）；本 crate 无下游依赖者 API 破坏（纯新增）。
