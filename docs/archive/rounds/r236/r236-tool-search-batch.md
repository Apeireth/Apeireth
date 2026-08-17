# R236 — apeireth-tool-search SearchEngine::search_batch

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R236
> **日期**: 2026-08-14
> **状态**: 1 commit, 6 测试 +6, 0 errors / 0 warnings

---

## 0. 主人指示

"继续" (2026-08-14)

## 1. 设计

apeireth-tool-search SearchEngine 已有 search / search_with_filter (单 query).
R236 加 `search_batch` — 多 query 合并 + 按 doc id dedup (高分优先).

### 1.1 search_batch

```rust
pub fn search_batch(&self, queries: &[&str], limit: usize) 
    -> SearchResult<Vec<RankedDoc>> {
    let mut best: HashMap<u64, RankedDoc> = HashMap::new();
    for q in queries {
        for hit in self.search(q, limit)? {
            match best.get(&hit.doc.id) {
                None => { best.insert(hit.doc.id, hit); }
                Some(existing) if existing.score < hit.score => {
                    best.insert(hit.doc.id, hit);
                }
                Some(_) => {} // 已有更高分, 跳过
            }
        }
    }
    let mut results: Vec<RankedDoc> = best.into_values().collect();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    results.truncate(limit);
    Ok(results)
}
```

**特性**:
- 复用 `search()` 路径, 0 编造结果
- HashMap<u64, RankedDoc> 按 doc id dedup
- 高分优先 (Same doc 多 query 命中, 保留高分版本)
- 错误立即 propagate
- 结果按 score 降序, truncate 到 limit

### 1.2 不触碰

- `search` / `search_with_filter` / `aggregate` / `index` / `remove` 0 改
- `Document` / `RankedDoc` / `FieldFilter` / `AggregateResult` 0 改

## 2. 测试 (6 cases)

| 测试 | 验证 |
|---|---|
| t13_search_batch_empty_returns_empty | 空 queries → 空 Vec |
| t14_search_batch_single_query_matches_search | batch[1] == single |
| t15_search_batch_dedupes_same_doc_across_queries | doc 1 被两次命中 → 去重 |
| t16_search_batch_keeps_highest_score | "hello world" 命中 doc 1 > 仅 "hello" |
| t17_search_batch_respects_limit | limit=5 截断 |
| t18_search_batch_sorted_by_score_desc | 结果按分降序 |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings**
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 12 → 18 (+6)

## 4. 战区意义

apeireth-tool-search 补 batch query API:
- **多次搜一次调** — 省多次 search 开销
- **自动 dedup + 高分优先** — 适合"同一文档多 query 集合"
- **与 R233 tool-codesearch query_batch 对称** — 两个 search 子系统都有 batch 能力

## 5. 累计 (R224-R236, 13 commits / 13 子模块)

| R | 主题 | 战区 |
|---|---|---|
| R224 | mcp JSON-RPC 2.0 §6 Batch | protocol + lib |
| R225 | 修 pre-existing 测试错 | workflow + codesearch |
| R226 | bus BackpressurePolicy +Coalesce +Adaptive | lib |
| R227 | bus topic pattern matching | pattern |
| R228 | L0Bus subscribe_pattern 集成 | l0 |
| R229 | bus event_log / replay | event_log + l0 |
| R230 | tool-fetch RateLimiter | rate_limit |
| R231 | tool-fetch engine rate limit 集成 | engine |
| R232 | council collect_opinions | deliberation |
| R233 | tool-codesearch query_batch | unified |
| R234 | consciousness EmotionEngine auto_decay | emotion |
| R235 | runtime auto_decay 集成 | lib |
| R236 | tool-search search_batch | lib |

**累计**: +100 测试, 0 errors, 0 触碰 3 不可变脊柱, 0 引入新外部 dep

## 6. 下一步候选

- **R237** consciousness 集成到 bus (auto_decay 触发 bus event)
- **R238** tool-codesearch ast-grep in-process
- **R239+** protocol Arrow / DataFusion (大项目, 最后)