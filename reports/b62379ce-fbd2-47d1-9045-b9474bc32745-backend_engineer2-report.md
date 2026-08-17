# W4 记忆主动推销 验收报告

**任务 ID**: `b62379ce-fbd2-47d1-9045-b9474bc32745`
**实施人**: backend_engineer2
**日期**: 2026-08-19
**返工轮次**: 2 (Round 1 score=1/10 → Round 2 重写)

---

## 1. 实现总结

W4 "记忆主动推销（意图分诊升级为主动预载）" 真实落地，非 skip 非空壳。

**核心思路**（与 Round 1 评审建议一致）：
- N7（morphology）= 被动查询分类 → CRAWL 预算
- **W4（本模块）= 主动话题预测 → 预载候选记忆**
- 两条道在 ContextAssembler 相遇：被动走 `memory` 块，主动走 `proactive` 块，各自 cap_chars 隔离

## 2. 新增文件

`crates/apeireth-companion/src/proactive_memory.rs`（841 行，含 30 测试）

### 2.1 三件套 API

| 组件 | 类型 | 职责 |
|------|------|------|
| `TopicCue` | input struct | 线索输入（最近 5 轮对话 + now + user_mood） |
| `predict_topic(cue) → TopicPrediction` | pure fn | 关键词(30+ 词表) + 时间锚(早/晚/夜) + 情绪锚(low/tired/excited 等) 三路触发 |
| `TopicPrediction::{top_topics, primary}` | method | 取 top-K / 主话题 |
| `MemoryCandidate` | struct | 候选记忆（content + timestamp + importance） |
| `PreloadChannel` trait | interface | 拉候选的统一口 |
| `KeywordChannel` | impl | 话题 → 关键词反查 → substring 命中（复用 TOPIC_KEYWORDS，0 新表） |
| `TimeChannel` | impl | timestamp desc 排序（兜底：与话题无关） |
| `ImportanceChannel` | impl | importance ≥ 8 主人惯例阈值（与 assemble.rs L1 对齐） |
| `CompositeChannel` + `default_composite_channel()` | impl | 三道合并 + HashSet 去重 + cap 截断 |
| `ProactiveBlock` | struct | ContextBlock 包装（name="proactive" + cap=max_chars） |
| `render_proactive_content(entries, max_chars)` | pure fn | 闭世界编号 + 反幻觉指令（仿 memory_injection 风格） |
| `build_proactive_block(cue, cands, channel, max_chars)` | 主入口 | 话题预测 → 多道拉候选 → 渲染 → 包装 |
| `recommend_proactive_cap(total_budget)` | helper | 总预算 1/4，钳位 [400, 2000]，< 400 返 0 |

### 2.2 lib.rs 集成

```rust
pub mod proactive_memory;
pub use proactive_memory::{
    build_proactive_block, default_composite_channel, predict_topic, recommend_proactive_cap,
    render_proactive_content, CompositeChannel, ImportanceChannel, KeywordChannel, MemoryCandidate,
    PreloadChannel, ProactiveBlock, TimeChannel, TopicCue, TopicHint, TopicPrediction,
};
```

## 3. 验收对照

### 3.1 任务要求 vs 交付

| 验收项 | 要求 | 实际 |
|--------|------|------|
| 预期话题分类器 | 多情境 | ✅ 8 个测试（考试/陪伴/早晨/晚间/深夜/组合/空/确定） |
| 预载检索道切换 | keyword + time + importance | ✅ 7 个测试（命中/不命中/top_k/排序/阈值/复合/空输入） |
| 注入预算不溢出 | 与 ContextAssembler 6000 char 协调 | ✅ 4 个测试（cap 推荐 3 + 向后兼容 1） |
| 旧 API 不破坏 | ContextAssembler 既有方法向后兼容 | ✅ `old_api_still_works_without_proactive` + `proactive_block_pushes_into_existing_assembler` |
| cargo test -p apeireth-companion --lib 全绿 | 是 | ✅ **576 passed; 0 failed** (546 基线 + 30 新增) |
| cargo check --workspace --all-targets 0 错 | 是 | ✅ 0 errors |
| 报告 reports/<taskId>-backend_engineer2-report.md | 是 | ✅ 本文件 |
| backlog.md W4 → ✅ | 是 | ✅ 已更新 |

### 3.2 关键设计决策（Ponytail 视角）

- **ladders 用得对**：复用 `TOPIC_KEYWORDS`（已有）+ `assemble.rs L1 重要性阈值 8`（已有），未引入新表/新依赖
- **预算协调最小化**：`recommend_proactive_cap` 一函数，4 行 clamp 逻辑
- **向后兼容**：ContextAssembler 0 改动，`ProactiveBlock` 只 push 一个 cap-limited block，`assemble_budgeted` 既有截断逻辑自然兜底
- **确定性**：`BTreeMap` + topic 名字典序次排序，`predict_topic` 同输入同输出
- **诚实标注**：`memory_injection.rs` 的"不主动说「我记得」" 风格延续，反幻觉指令闭世界编号
- **L1 阈值复用**：`ImportanceChannel { threshold: 8 }` 与 `assemble.rs` L1 `parse_importance(...) >= 8` 完全一致

### 3.3 与 N7 (morphology) 衔接

```
用户查询 ─→ morphology.classify(query) ──→ 检索模式档位 + CRAWL 预算
                                     │
                                     ▼
                              memory_graph.crawl(seeds, budget)
                                     │
                                     ▼
                              inject_memory (被动检索结果 → "memory" 块)

最近上下文+now+mood ─→ predict_topic(cue) ──→ 预期话题 (主动预测)
                                       │
                                       ▼
                              PreloadChannel.fetch(topics, cands) → candidates
                                       │
                                       ▼
                              render_proactive_content → ProactiveBlock ("proactive" 块)
```

两条道各自 cap_chars 隔离，在 ContextAssembler 汇合；总预算 6000 chars 时 proactive 推荐 1500，memory 3000，余 1500 给 state/graph/prefs/today/growth。

## 4. 已知边界（0 装标注，备升级路径）

1. **关键词匹配是 substring，非真正语义**：若需升级，可换 `apeireth-vector` 余弦相似度（已存在）
2. **TOPIC_KEYWORDS 词表 30+ 项是启发式常量**：A/B 测试后可用数据驱动（learn from user feedback）
3. **ProactiveBlock 尚未接到 `build_injection`**：当前是模块自包含，调用方按需挂入 `assemble.rs::build_injection` 即可（一行 push），不动现有管线
4. **无 fail-safe 退化**：若 `recent_episodes` 返回空，predict_topic 自然返空（不会 panic），ProactiveBlock 内容为空字符串，由 ContextAssembler 的 `filter(|(_, s)| !s.trim().is_empty())` 自动剔除

## 5. 提交

- 新文件：`crates/apeireth-companion/src/proactive_memory.rs`（841 行）
- 修改：`crates/apeireth-companion/src/lib.rs`（注册 + pub re-export）
- 修改：`docs/backlog.md`（W4 行 ✅）

— 后端工程师2 / W4