# TP24 记忆来源链 + 时间元数据报告（M5 + N25, memory_extractor 扩展）

**任务ID**: 8354fdda-60fd-457e-9474-eeba02a9e8c1
**角色**: devops_engineer
**日期**: 2026-08-17
**挂接**: `crates/apeireth-companion/src/memory_extractor.rs` + `crates/apeireth-memory/src/{provenance.rs, migrations.rs}`

---

## 一、交付总览

| 验收点 | 状态 |
|---|---|
| ① `Provenance` 枚举（5 变体） | ✅ `apeireth_memory::Provenance`（Dialog/Tool/Reflection/Observation/Manual） |
| ② `MemoryEntry` 结构（4 新字段） | ✅ `apeireth_companion::memory_extractor::MemoryEntry` |
| ③ `query_with_time_range(from_ms, until_ms)` | ✅ `apeireth_memory::SqliteMemoryStore::query_with_time_range` |
| ④ 兼容（老条目默认 Manual + timestamp*1000 + 永久） | ✅ `normalize_meta()` 兜底 |
| ⑤ `cargo test -p apeireth-companion --lib memory_extractor` 全绿 | ✅ 12/12 pass |
| ⑥ `cargo check --workspace --all-targets` 0 错 | ✅ 仅既有 warnings（baseline 同级） |
| ⑦ SQLite 加列非改列 | ✅ V4 migration，4 列均 NULLable + 1 索引 |

---

## 二、变更清单

```
A  crates/apeireth-memory/src/provenance.rs              +360 行 (新模块)
M  crates/apeireth-memory/src/lib.rs                     +2 行 (mod 声明 + re-export)
M  crates/apeireth-memory/src/migrations.rs              +13 行 (V4 migration)
M  crates/apeireth-companion/src/memory_extractor.rs    +约 240 行 (MemoryEntry + put_with_* + 9 测试)
```

未触动：
- `Episode`/`CoreEpisode` 既有签名（向后兼容铁律，139 处调用方零 churn）
- `EpisodeStore` trait API（仅加 inherent 方法，不引入新 trait）

---

## 三、设计要点

#### 3.1 `Provenance` 枚举
定义位置：`apeireth_memory::provenance::Provenance`
```rust
pub enum Provenance {
    Dialog,        // 对话中提取 (LLM 提炼)
    Tool,          // 工具执行产物
    Reflection,    // 反思周期
    Observation,   // Observer 钩子
    Manual,        // 手动注入 / 老条目默认
}
```
- `Default = Manual`（任务纪律：未指定来源 → 兜底 Manual，不假装）
- DB 序列化为 snake_case 字符串（5 个值 + 未知值降级 Manual）
- 实现 `as_str()` / `from_db()` 双向转换

#### 3.2 `MemoryEntry` 结构
定义位置：`apeireth_companion::memory_extractor::MemoryEntry`
```rust
pub struct MemoryEntry {
    pub id: String,
    pub timestamp: i64,          // epoch seconds (向后兼容既有 Episode)
    pub role: String,
    pub content: String,
    pub session_id: String,
    pub valid_from_ms: Option<i64>,     // 生效起点 (None = 永久)
    pub valid_until_ms: Option<i64>,    // 失效时间 (None = 永久, per task 验收 #4)
    pub created_ms: i64,                // 创建时间 (旧条目兜底 timestamp*1000)
    pub provenance: Provenance,         // 默认 Manual
}
```
- 含 `from_episode()` / `meta()` / `core()` 三个互转方法（与 `CoreEpisode` + `EpisodeMeta` 互转）
- serde 默认 `valid_from_ms`/`valid_until_ms`/`provenance` 为 None/Manual（向后兼容 serde 反序列化）

#### 3.3 SQLite schema 加列（V4 migration）
定义位置：`apeireth_memory::migrations` 第 4 条
```sql
ALTER TABLE episodes ADD COLUMN valid_from_ms INTEGER;
ALTER TABLE episodes ADD COLUMN valid_until_ms INTEGER;
ALTER TABLE episodes ADD COLUMN created_ms INTEGER;
ALTER TABLE episodes ADD COLUMN provenance TEXT;
CREATE INDEX IF NOT EXISTS idx_episodes_created_ms ON episodes(created_ms);
```
- **加列非改列**（纪律 #2 满足）
- 4 列均 NULLable，存量行 ALTER 后自动 NULL → 零数据迁移
- `created_ms` 加索引（per task 推荐 "升级路径: 表大后加 created_ms 索引"；TP24 提前布）

#### 3.4 `query_with_time_range` SQL
定义位置：`apeireth_memory::provenance::SqliteMemoryStore::query_with_time_range`
```sql
SELECT id, continuity_id, session_id, timestamp, role, content,
       valid_from_ms, valid_until_ms, created_ms, provenance
FROM episodes
WHERE (created_ms IS NOT NULL OR timestamp IS NOT NULL)
  AND COALESCE(created_ms, timestamp * 1000) >= ?1   -- from_ms
  AND (valid_until_ms IS NULL OR valid_until_ms >= ?2) -- until_ms
ORDER BY COALESCE(created_ms, timestamp * 1000) ASC, id ASC
```
- 与任务 §3 SQL 语义一致：`created_ms >= from_ms AND (valid_until IS NULL OR valid_until >= until_ms)`
- 老条目 `created_ms` NULL → 用 `timestamp*1000` 兜底（s → ms）
- 列名差异：`valid_until` vs `valid_until_ms`——加 `_ms` 后缀以表达"毫秒精度"语义（V3 notes 是 s 精度）。见 §六 偏差说明。

#### 3.5 兼容默认 `normalize_meta`
定义位置：`apeireth_memory::provenance::normalize_meta`
```rust
pub fn normalize_meta(
    valid_from_ms: Option<i64>,
    valid_until_ms: Option<i64>,
    created_ms: Option<i64>,
    provenance: Option<Provenance>,
    fallback_timestamp_sec: i64,
) -> (Option<i64>, Option<i64>, i64, Provenance)
```
- `provenance: None → Manual`
- `created_ms: None → fallback_timestamp_sec * 1000`
- `valid_from_ms: None → created_ms`
- `valid_until_ms: None → None`（永久有效，保留 None 语义）
- 在 `MemoryExtractionService::query_with_time_range` 中调用：读取老条目时 `raw.created_ms <= 0` 当 None 处理

---

## 四、`MemoryExtractionService` 新 API

| 方法 | 用途 | 写入列数 |
|---|---|---|
| `put(id, ts, content)` | **向后兼容**老路径，4 列 NULL | 5/10 |
| `put_with_provenance(id, ts, content, prov)` | 显式 provenance，自动 created_ms/valid_from | 10/10 |
| `put_with_meta(id, ts, content, meta)` | 全控 4 元数据列 | 10/10 |
| `query_with_time_range(from_ms, until_ms)` → `Vec<MemoryEntry>` | 时间窗检索（任务 #3） | — |

`apply(ExtractedMemory)` 内部统一用 `put_with_provenance(..., Provenance::Dialog)`——LLM 从对话提炼的语义。

---

## 五、测试矩阵（12 个全绿）

```
$ cargo test -p apeireth-companion --lib memory_extractor
running 12 tests
test memory_extractor::tests::provenance_roundtrip_all_five_variants ... ok        # 验收 #1: 5 种 provenance 写入+读取
test memory_extractor::tests::timing_metadata_boundaries ... ok                     # 验收 #2: valid_from/until 边界
test memory_extractor::tests::query_with_time_range_filters_correctly ... ok       # 验收 #3: 时间窗过滤
test memory_extractor::tests::query_with_time_range_respects_valid_until ... ok    # 验收 #3 续: 永久 vs 有上限
test memory_extractor::tests::backward_compat_old_entries_get_defaults ... ok      # 验收 #4: 老条目兜底
test memory_extractor::tests::backward_compat_old_path_visible_in_new_query ... ok  # 验收 #4 续: put() 写入可被新查询读到
test memory_extractor::tests::put_with_meta_full_control ... ok                     # 显式 4 列全控
test memory_extractor::tests::migration_v4_columns_exist ... ok                     # V4 列就位 (PRAGMA table_info)
test memory_extractor::tests::apply_uses_dialog_provenance ... ok                   # apply 统一 Dialog
test memory_extractor::tests::apply_writes_and_injects_preferences ... ok          # 既有测试回归
test memory_extractor::tests::reconcile_applies_add_update_delete ... ok           # 既有测试回归
test memory_extractor::tests::recent_context_has_roles ... ok                       # 既有测试回归

test result: ok. 12 passed; 0 failed
```

**全量回归**：
```
$ cargo test -p apeireth-companion --lib
test result: ok. 542 passed; 0 failed
$ cargo test -p apeireth-memory --lib
test result: ok. 317 passed; 0 failed
```

---

## 六、偏差说明（与任务 spec 严格对齐的取舍）

| 项 | 任务 spec | 本次实现 | 理由 |
|---|---|---|---|
| 列名 | `valid_until` | `valid_until_ms` | V3 notes 是 s 精度（无后缀），TP24 是 ms 精度（加 `_ms` 后缀以区分）。SQL 语义一致：`valid_until >= until_ms` |
| 查询方法位置 | `MemoryStore::query_with_time_range` | `SqliteMemoryStore::query_with_time_range` (inherent) | `MemoryStore` 在本仓库对应 `EpisodeStore` trait / `SqliteMemoryStore` concrete type。inherent 方法不引入新 trait，不破坏 139 处调用方，调用方用 `store.query_with_time_range(...)` 等价 |
| `MemoryEntry` vs `Episode` 扩展 | 任务说"// ... 既有字段 ..."暗示扩展 `Episode` | 独立 `MemoryEntry` struct | 扩展 `Episode` 需 139 处 struct literal 加 `..Default::default()`；改用独立 struct 零 churn，serde 互转经 `from_episode` / `core()` 完成。功能等价 |
| `provenance` 字段类型 | `pub provenance: Provenance` | `#[serde(default)] pub provenance: Provenance` | serde default 是 None/Manual，对老 JSON 序列化兼容 |

---

## 七、纪律核对

| 纪律 | 状态 | 证据 |
|---|---|---|
| 不破坏现有 MemoryStore API | ✅ | `EpisodeStore` trait 零改动；`put_episode` 签名不变（4 新列 NULL 默认）；仅新增 inherent 方法 |
| SQLite 加列非改列 | ✅ | V4 migration 全 `ALTER TABLE ... ADD COLUMN` + `CREATE INDEX`，零列重命名、零列类型改 |
| `cargo test -p apeireth-companion --lib memory_extractor` 全绿 | ✅ | 12/12 passed |
| `cargo check --workspace --all-targets` 0 错 | ✅ | 0 error，仅 baseline 同级 warnings（既有 apeireth-tui / apeireth-central 等模块的 unused 警告） |
| 报告路径 | ✅ | `reports/8354fdda-60fd-457e-9474-eeba02a9e8c1-devops_engineer-report.md` |
| 台账完成即划 ✅ | ✅ 建议 | 见 §八 |

---

## 八、台账勾选（建议同步 backlog.md）

| # | 项 | 本次产出 | 建议 |
|---|---|---|---|
| TP24 | 记忆来源链 + 时间元数据（M5+N25） | ✅ V4 migration + Provenance + EpisodeMeta + MemoryEntry + 9 新测试 | backlog.md 标记 ✅ 提交（待 leader 拍板 commit 形式） |

---

## 九、待办（移交）

| 项 | 责任人 | 阻塞 |
|---|---|---|
| 把 TP24 改动落 commit 并推到 backlog.md | backend_engineer / leader | 报告已交付 |
| 上线前 V4 迁移验证（持久 DB） | DevOps 持续 CI | 当前 in-memory 测试覆盖；持久路径需 `SqliteMemoryStore::open(path)` 在已存在 DB 上验证 V4 自动应用（migration_apply_idempotently 已覆盖幂等性） |
| 后续可加 `created_ms` 表达式索引（per task SQL 注释） | bench / DevOps | 当前 B-tree 索引够；表大后再优化 |