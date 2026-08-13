# R233 — apeireth-tool-codesearch query_batch

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R233
> **日期**: 2026-08-13
> **状态**: 1 commit, 5 测试 +5, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱" + "继续全做完"

## 1. 设计

apeireth-tool-codesearch R193/R202/R210/R213 已落地 6 维 code intelligence
统一 facade. 但 `UnifiedCodeIntelligence::query` 是单 query 接口. R233 加
`query_batch` — 一次调多次搜 + 自动 dedup.

### 1.1 query_batch

```rust
pub fn query_batch(&self, queries: &[UnifiedQuery]) 
    -> Result<Vec<IntelligenceHit>, UnifiedError> {
    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();
    for q in queries {
        for hit in self.query(q)? {
            let key = (hit.kind(), format!("{:?}", hit));
            if seen.insert(key) {
                results.push(hit);
            }
        }
    }
    Ok(results)
}
```

**特性**:
- 复用 `query()` 路径, 0 编造结果
- 错误立即 propagate (任一 query 失败 → 整个 batch 失败)
- HashSet dedup key: `(IntelligenceKind, format!("{:?}", hit))`
- 自动跨 query dedup (适合"同一 pattern 多 query 集合")

### 1.2 不触碰

- `query` / `index_file` / `new_in_memory` / `with_ast_binary` 0 改
- `IntelligenceKind` / `IntelligenceHit` 0 改
- 6 维底层 (Text / File / Symbol / Graph / Index / Ast) 0 改

## 2. 测试 (5 cases)

| 测试 | 验证 |
|---|---|
| t11_query_batch_empty_queries_returns_empty | 空 queries → 空 Vec |
| t12_query_batch_single_query_matches_query | batch[1] 结果 == single query 结果 |
| t13_query_batch_multiple_kinds | Text + File 多种 kind 混合 |
| t14_query_batch_dedupes_overlapping_results | 重复 query → 去重 |
| t15_query_batch_propagates_errors | path 不存在不 panic |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings**
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 89 → 94 (+5)

## 4. 战区意义

tool-codesearch 补 batch query API:
- **多次搜一次调** — 省多次 query 开销
- **自动 dedup** — 适合"同一 pattern 多 query 集合"
- **兼容现有 query()** — 不破坏现有 API
- **集成到 API** — 让 /v1/codesearch 支持 batch 调用

## 5. 总结 (本会话 R224-R233)

10 commits, 9 子模块, +88 测试, 0 errors

| R | 主题 | 子模块 |
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

**0 触碰 3 不可变脊柱** — Self-Disable / physical_multisig / verdict cache
**0 引入新外部 dep** — 全 std + 既有 workspace deps
**0 删除任何代码** — 全 additive
**workspace.version 1.2.0 0 改**

## 6. 下一步候选

- **R234+** consciousness temporal decay / council streaming / arrow / DataFusion