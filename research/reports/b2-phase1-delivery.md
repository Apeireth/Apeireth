# B2 · Phase 1 交付报告：派生记忆图与遗忘传播审计

> 依据 `_research_mem/ra/00-selection-meeting.md` §5 B2 · 2026-09-04
> 基线：`research/baselines/baseline-2026-09-phase0.md`

## 1. 问题定义与假设

- **问题**：`forget_episode` 是单点软删；note/diary/wiki/chronicle/cache 等派生面不经血缘传播继续泄漏已遗忘事实（RA-1 泄漏向量 1/2）。
- **假设（可证伪）**：显式血缘表 + taint/support(θ) 闭包可在**只审计不删除**的前提下量化泄漏面；血缘完整度与召回安全构成可测权衡。

## 2. 交付内容（全部 Research 前缀，默认关闭）

| 交付项 | 位置 | 状态 |
|---|---|---|
| `derived_from` 血缘表（A.4.1 schema） | `migrations.rs` V8 `research_derived_from` + `research_lineage_events`（A8 append-only） | ✅ |
| `research_record_derivation()`（A5 派生必记血缘，幂等） | `research_derived_memory.rs` | ✅ |
| `research_forget_closure()`（taint / support(θ)，只审计不删除） | 同上 | ✅ |
| `research_audit_forgotten_leaks()`（forget_propagation_audit） | 同上 | ✅ |
| `GovernedRecall`（独立 API，默认关闭，不替换生产检索） | 同上 | ✅ |
| 缓存代际联动（泄漏向量 1 粗粒度缓解：闭包非空 → 整代失效） | `research_invalidate_cache_on_forget()` | ✅ |
| notes 血缘回填桥接（"note 已有"→ 血缘表） | `research_import_note_lineage()` | ✅ |
| 四类泄漏探针（确定性测试） | 模块 tests：probe1 直接召回 / probe2 转述召回 / probe3 跨会话推理 / probe4 衍生知识重建 | ✅ |
| LLM-as-judge 双评者协议 | `ResearchJudge` trait + `dual_rater_protocol()`（不一致保守取泄漏）；确定性 stub 测试；真 LLM 留部署层（0 装） | ✅ |

## 3. 写入侧补齐口径（诚实标注）

- note：血缘已在产品表（`notes.source_episode_ids_json`），经 `research_import_note_lineage()` 只读回填 ✅
- diary / wiki / chronicle：三者为**内存计算引擎（无 store 句柄）**，血缘由调用方在持久化点经 `research_record_derivation()` 显式登记。本批次交付通用 API + 四类 kind 约定（'diary'/'wiki'/'chronicle'/'cache'），**不侵入引擎本体**（铁律 1/4）
- cache：`research_invalidate_cache_on_forget()` 显式联动，默认不挂

## 4. 闸门核对

- 旧 `forget_episode` 语义不变 ✅（本模块零触碰 `episode_governance`）
- 不自动删除任何数据 ✅（闭包/审计只写 `research_lineage_events`；`ClosureReport.deleted_anything` 恒 false）
- 默认行为零变化 ✅（V8 为纯新增表；GovernedRecall/审计均显式调用才生效）
- 术语门 ✅（文档无"已证明/SOTA"表述）

## 5. 已知局限（0 装）

1. 血缘覆盖度取决于登记完整度；未登记派生的召回面不在审计视野内（`unobservable_note` 显式标注）
2. 缓存联动为整代失效（粗粒度），按 query_hash 级驱逐留后续
3. support(θ) 的 lost 计数按"已入闭包节点"计（BFS 序），与按"根集"计的口径差异需在评测中对照
4. LLM-as-judge 真接线未做（无 LLM 依赖的确定性 stub 先行）

## 6. 评测与实验

- 单元探针：见模块 tests（probe1–4 + 语义测试）
- 下一步实验（Phase 1 评测批）：构建含噪声的派生语料，量测闭包召回率/泄漏率随血缘登记率的曲线，写 `logs/` JSONL（schema 见 `research/logs/README.md`）
