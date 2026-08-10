# Agent-A 战区 4 (Memory) — A3 阶段报告
**日期**: 2026-08-10  
**作者**: Mavis 派 — Agent A (Apeireth-rust 后端升级)  
**阶段**: A3 `apeireth-memory` 加 `semantic_search` + `extract_user_profile`  
**状态**: ✅ 完结 (95/95 tests pass + bench 编译过)

---

## 1. 改了什么 (跟 A1 readmap 对比)

| # | 文件 | 改动 | 状态 |
|---|---|---|---|
| 1 | `crates/apeireth-memory/src/semantic.rs` | **新建** 11.9KB — `EmbedFn` trait + `HashEmbedder` + `SemanticIndex` + 7 unit tests | new |
| 2 | `crates/apeireth-memory/src/user_profile.rs` | **新建** 15.5KB — `UserProfile` struct + `ProfileExtractor` + 9 unit tests | new |
| 3 | `crates/apeireth-memory/src/lib.rs` | +3 行: `pub mod semantic;` / `pub mod user_profile;` + 1 个 `MemoryError::Other` variant + `semantic_search()` / `extract_user_profile()` 方法 + re-exports | 非 LOCKED |
| 4 | `crates/apeireth-memory/Cargo.toml` | `default = ["semantic"]` (1 行) — 让 `semantic` 默认开,新代码即刻可用 | 非 LOCKED |

**LOCKED 9 文件 mtime**: 0 触碰 (确认 append_only / identity / migrations / episode / session_note / streams / history_streams / continuity_link / llm_analysis 全没动)
**workspace.version**: 0 改 (1.1.0)
**R11 baseline 3 值**: 0 触碰 (没读没写 apeireth-asi)
**6 哲学锚定义**: 0 触碰

---

## 2. 关键设计 (跟 A1 一致)

### 2.1 `EmbedFn` trait
```rust
pub trait EmbedFn: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}
```
- `Send + Sync`: 让 `Arc<dyn EmbedFn>` 跨线程共享
- 真 LLM 集成: 调用方实现本 trait, 内部 `apeireth_api::llm::LlmProvider::complete()` 拿 embedding
- mock 实现: `HashEmbedder` (FNV-1a + L2 归一, 确定性, 0 外部 API)

### 2.2 `SemanticIndex<'m>` facade
```rust
pub struct SemanticIndex<'m> {
    memory: &'m SqliteMemoryStore,
    vector: Mutex<Box<dyn VectorStore>>,
    embedder: Arc<dyn EmbedFn>,
}
```
- `&'m SqliteMemoryStore` 借用保证生命周期
- `Mutex<Box<dyn VectorStore>>` 因为 `VectorStore` trait object
- 3 个核心方法: `new / index_episode / search` (跟 bench 一致)
- 便捷方法: `len / extract_profile / index_episodes / vector / embedder`

### 2.3 `UserProfile` + `ProfileExtractor`
- `UserProfile`: 6 字段结构体 (Serialize/Deserialize, 跟 memory 其他 record 一致)
- `ProfileExtractor::new(embedder)` + `.with_top_k(n)` + `.extract(memory, Option<&SemanticIndex>)`
- mock 提取规则:
  - `preferences`: avg user msg 长度 → "简短"/"详细"/"平衡"
  - `recurring_topics`: 有 index → KNN 检索 top_k; 无 index → 最近 top_k episode content 前 30 字
  - `communication_style`: role 分布 (10:1 user/assistant → "用户主导")
  - `expertise_areas`: 关键词命中 (`rust`/`python`/`sql`/`数据库`/etc)
  - `interaction_count` / `last_active`: 直接统计

### 2.4 Memory::semantic_search 便捷 API
```rust
impl SqliteMemoryStore {
    pub fn semantic_search(
        &self,
        query: &str,
        k: usize,
        embedder: Arc<dyn EmbedFn>,  // owned 'static
    ) -> MemoryResult<Vec<Episode>>;
    
    pub fn extract_user_profile(
        &self,
        embedder: Arc<dyn EmbedFn>,
    ) -> MemoryResult<UserProfile>;
}
```
- 一次性 in-memory index: 拉所有 episodes (limit 100_000) + 重建 vec0 + 检索
- 接受 `Arc<dyn EmbedFn>` 而非 `&dyn EmbedFn` (因为 `Arc<dyn T>` 内部要 `'static`)
- **生产模式**: 高频场景建议 caller 自己持 `SemanticIndex` 复用, 一次性 API 留给偶发查询

### 2.5 `apeireth-memory/Cargo.toml` `default = ["semantic"]`
- 之前 R18 P2 `semantic` feature 只是占位 (0 实现代码), 改默认开
- 0 破坏: `semantic` feature 形状不变, `vector` 仍是 optional
- `bench` 的 `required-features = ["semantic"]` 保持不变 (bench 显式 opt-in)
- **唯一**改动: `default = []` → `default = ["semantic"]` (1 行)

---

## 3. 验收指标 (全过)

| 指标 | 实测 | 期望 | 状态 |
|---|---|---|---|
| `cargo check -p apeireth-memory --lib --tests` exit 0 | 0 | 0 | ✅ |
| `cargo test -p apeireth-memory` 0 failed | 0 | 0 | ✅ |
| 现有 64 unit + 9 integration + 6 sqlite + 新增 ≥ 10 semantic | 80 unit + 9 + 6 = 95 | ≥ 79 (64+9+6+10) | ✅ (多了 6 个) |
| bench 编译过 | 0 error | 0 error | ✅ |
| 9 LOCKED memory 文件 mtime 不变 | 0 触碰 | 0 触碰 | ✅ |

---

## 4. 16 个新 unit tests

### 4.1 `semantic::tests::*` (7 个)
| # | test | 验证 |
|---|---|---|
| 1 | `hash_embedder_is_deterministic` | 同输入 → 同输出 |
| 2 | `hash_embedder_different_text_different_vector` | 不同输入 → 不同输出 |
| 3 | `hash_embedder_output_is_l2_normalized` | norm ≈ 1.0 |
| 4 | `episode_uuid_is_deterministic` | v5 派生稳定 |
| 5 | `semantic_index_indexes_and_searches` | 3 条 sql/rust 主题 episodes, search "SQL" 命中 sql 主题 |
| 6 | `semantic_index_search_with_zero_corpus_returns_empty` | 空 corpus search 返 0 |
| 7 | `semantic_index_dim_auto_set_on_first_upsert` | 首次 index 后 dim 自动设 |

### 4.2 `user_profile::tests::*` (9 个)
| # | test | 验证 |
|---|---|---|
| 1 | `empty_profile_for_empty_memory` | 空 memory → 空 UserProfile |
| 2 | `user_dominant_style_when_user_msgs_much_more` | 10:1 user/assistant → "用户主导" |
| 3 | `mixed_style_when_roles_balanced` | 3:3 → "混合" |
| 4 | `expertise_extracted_from_keywords` | "rust"/"sql" 出现在 expertise_areas |
| 5 | `preferences_short_answers_when_user_msg_short` | avg < 30 → "简短回答" |
| 6 | `preferences_detailed_when_user_msg_long` | avg > 100 → "详细回答" |
| 7 | `last_active_is_max_timestamp` | max(ts) 一致 |
| 8 | `extract_with_index_uses_search` | 有 index 时走搜索路径, topics 非空 |
| 9 | `with_top_k_chains_correctly` | builder 模式 |

---

## 5. 决策日志

| # | 决策 | 原因 |
|---|---|---|
| D1 | `EmbedFn` trait 接受 `Arc<dyn EmbedFn>` (owned 'static) 而非 `&dyn EmbedFn` | `&dyn` 不能 coerce `Arc<dyn T>`, 因为 Arc 内部要 'static |
| D2 | `default = ["semantic"]` | 让新代码默认可用, 不破坏 semantic feature 形状 |
| D3 | `pub mod semantic` / `pub mod user_profile` | bench 引用 `apeireth_memory::semantic::{EmbedFn, SemanticIndex}`, 必须 pub |
| D4 | 加 `MemoryError::Other(String)` variant | 兼容 vector / embedder 错误信息, 不改 LOCKED 9 文件 |
| D5 | `episode_uuid` 用 `Uuid::new_v5(NS, id.as_bytes())` | 跨进程稳定, 无需新表 |

---

## 6. 风险点 (跟 A1 对照)

| # | 风险 | 实际 | 状态 |
|---|---|---|---|
| R1 | sqlite-vec 0.1.9 在 bundled 下链接 | 0 问题 | 关闭 |
| R2 | vec0 距离 → similarity 转换 | `1.0 - d*0.5` 准 | 关闭 |
| R3 | idmap rowid 漂移 | OK | 关闭 |
| R4 | cargo check 编译失败 | 0 次 (除 1 个我引入的 test 数据) | 关闭 |
| R5 | A2 完成 | 31 tests | 关闭 |
| R6 | vec0 KNN `LIMIT 1` edge case | 改用 `LIMIT >= 3` | 关闭 |
| R7 | 一次性 semantic_search 性能 (重建 index 每次) | 接受; 文档建议 caller 自持 index | 监控 |
| R8 | profile 提取规则是 mock, 真实 LLM 留接口 | 接受, 留 `EmbedFn` 扩展 | 监控 |

---

## 7. A3 → A4 交接

A4 阶段要做:
1. 跑 `cargo check -p apeireth-vector` + `cargo test -p apeireth-vector` (再过一遍)
2. 跑 `cargo check -p apeireth-memory --lib --tests` + `cargo test -p apeireth-memory` (再过一遍)
3. 跑 `cargo metadata --format-version 1 | jq '.workspace_members | length'` 验证 workspace 解析
4. 检查 24 LOCKED 名单的 mtime (git status --porcelain)
5. 跑 smoke example (`cargo run -p apeireth-vector --example semantic_smoke`) 验证 1000 条 vec0 KNN
6. 写 `reports/agent-a-final-2026-08-10.md` 总报告
7. 决策日志 + 风险点 + 下一步建议

**硬约束严守**: 0 触碰 24 LOCKED, 0 改 workspace.version, 0 改 R11 baseline。
