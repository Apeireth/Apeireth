# R179 Session-2 Handoff — mempalace 借鉴落地 + 4 层渐进 (2026-08-15)

## TL;DR
本 session 接手上一个 session 的 P1-9/10/11 三项, 全部完成 + 测试通过.

| # | 项 | 文件 | 测试 |
|---|---|---|---|
| 1 | P1-9 Dedup (wire + verify) | `crates/apeireth-memory/src/dedup.rs` (13604 bytes) + lib.rs 加 `pub mod dedup;` | 13 单元测试通过 |
| 2 | P1-10 Hallway (新建) | `crates/apeireth-memory/src/hallways.rs` (新, ~24 KB) + V2 migration + lib.rs 加 `pub mod hallways;` | 13 单元测试通过 |
| 3 | P1-11 4 层渐进 (新建) | `crates/apeireth-memory/src/lightmemo/progression.rs` (新, ~10 KB) + lightmemo/mod.rs 加 `pub mod progression;` | 11 单元测试通过 |

测试基线 (apeireth-memory 单 crate):
```
test result: ok. 263 passed; 0 failed; 0 ignored; 0 measured
```

测试基线 (workspace 全量):
```
22483 PASS / 0 fail / 1 unrelated pre-existing fail (docs/stage4/8-locked-unified missing, by R125 整合)
```

---

## P1-9 Episode Dedup (mempalace `dedup.py` 借鉴)

### 算法 (mempalace 原版 vs apeireth)
- mempalace: group by source_file, greedy longest-first, cosine distance threshold 0.15, keep longest
- apeireth: group by session_id, same greedy, same threshold, same longest-wins

### apeireth 守则
- **append-only**: 不真 DELETE, 只生成 `DedupReport` (deleted = marker, not physical delete)
- 默认 `dry_run = true` (跟 mempalace 一致)
- caller 自己决定怎么落地 (dedup_log 表, 或 index_episode 时 skip)

### API
```rust
use apeireth_memory::dedup::{dedup_session, default_threshold, DedupReport, DedupAction};
use apeireth_memory::semantic::HashEmbedder;

let embedder: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(64));
let report: DedupReport = dedup_session(&store, "session-1", default_threshold(), embedder)?;
println!("kept {} deleted {}", report.kept.len(), report.deleted.len());
```

### 13 测试覆盖
1. default_threshold = 0.15
2-5. cosine_distance: identical/orthogonal/opposite/different-len
6-13. dedup_session: empty / single / identical-pair / different / append-only / too-short / threshold / invalid

---

## P1-10 Hallway (mempalace `hallways.py` 借鉴)

### 概念映射
| mempalace | apeireth |
|---|---|
| wing | 项目/主题 (Note.tag[0]) |
| drawer | 一条 Note |
| entity | 一个 tag (排除 wing 自己) |
| hallway | wing 内两个 entity 的连接 |

### 跟 mempalace 一致的关键设计
- **symmetric id**: sha1(wing::a::b)[..12] 排序后哈希, (A, B) == (B, A)
- **preserve L7 dynamics**: recompute 时保留 strength/stability/last_activated/access_count
  (per mempalace PR #1578 gemini-code-assist HIGH priority review)
- **tombstone delete**: 不物理删, 软删
- **min_count 默认 = 2**

### Schema (V2 migration, 新表 `hallways`)
```sql
CREATE TABLE hallways (
    id TEXT PRIMARY KEY,  -- symmetric sha1
    wing TEXT NOT NULL,
    entity_a TEXT NOT NULL,
    entity_b TEXT NOT NULL,
    co_occurrence_count INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    strength REAL NOT NULL DEFAULT 1.0,
    stability REAL NOT NULL DEFAULT 1.0,
    last_activated INTEGER,
    access_count INTEGER NOT NULL DEFAULT 0,
    tombstoned_at INTEGER
);
CREATE INDEX idx_hallways_wing ON hallways(wing);
CREATE INDEX idx_hallways_pair ON hallways(entity_a, entity_b);
CREATE INDEX idx_hallways_entity_a ON hallways(entity_a);
CREATE INDEX idx_hallways_entity_b ON hallways(entity_b);
```

注意: **不加 append-only trigger** (跟 6 流不同), 允许 recompute UPSERT 保留 dynamics.

### API
```rust
use apeireth_memory::hallways::{
    compute_hallways_for_wing, list_hallways, find_hallways_for_entity,
    delete_hallway, touch_hallway, default_min_count, Hallway,
};

// 1. 重算一个 wing
let created = compute_hallways_for_wing(&store, "memory-palace", default_min_count())?;

// 2. 列出某 wing 全部 (filter tombstoned)
let all = list_hallways(&store, Some("memory-palace"))?;

// 3. 查 entity 出现在哪些 hallway
let hws = find_hallways_for_entity(&store, "dedup")?;

// 4. 访问时 bump dynamics (同 mempalace)
touch_hallway(&store, &id)?;
```

### 13 测试覆盖
1-2. canonical_pair + make_id 对称性
3-7. compute_hallways: empty / single-entity / pair / min_count / cross-wing 不混
8. 持久化 + idempotent recompute
9. preserve dynamics (PR #1578 review 要求)
10. soft delete
11. find_hallways_for_entity
12-13. empty wing 报错 + label 格式

---

## P1-11 4 层渐进 (mempalace 4-layer closed-loop 借鉴)

### 概念映射
| mempalace | apeireth |
|---|---|
| L1 file (raw) | `l1_file.rs` (sqlite-backed FileEntry) |
| L2 vector | `l2_vector.rs` (cosine_search) |
| L3 tag (inverted index) | `l3_tag.rs` (forward/reverse map) |
| L4 LCM (long-context memory) | `l4_lcm.rs` (chunk + summarize callback) |
| 各自独立, 主动 fill | **progressive: 1 个 item 随生命周期升层** |

### 升级/降级规则 (跟 mempalace 不同的地方)
- L1 **永远保留** (raw persistence = append-only 安全网)
- L2 仅当有 embedding 时 promote
- L3 仅当有 tag 时 promote
- L4 仅当 content 长度 >= chunk_size 时 promote (短文本压缩丢信息, 不值)
- decay < threshold: 从 L4 → L3 → L2 cascade, **L1 永不降**

### API
```rust
use apeireth_memory::lightmemo::progression::{LayerProgression, Layer};

let mut prog = LayerProgression::new();
prog.touch_l1("item-1");
prog.touch_l2("item-1");  // 有 embedding
prog.touch_l3("item-1", &["memory".into(), "rust".into()]);
prog.touch_l4("item-1");  // 长文本 → LCM

assert!(prog.is_in("item-1", Layer::L4));

// 跨层降级 (忘记曲线)
let demoted = prog.decay_demote("item-1", 24.0);  // 24h 阈值

// 推荐层 (按 content 长度)
let top = LayerProgression::recommend_top_layer(content, &compressor);
```

### 11 测试覆盖
1. 新 item 只在 L1
2. touch_l2 蕴含 L1
3. 渐进升级 L1→L4
4. 升级不会退层
5. decay_demote 阈值守门
6. 久未访问的 item cascade 降级
7-8. count_per_layer / count_by_top_layer
9. remove
10. recommend_top_layer (按 content 长度)
11-12. Layer enum + promote_from

---

## 与 mempalace 4-layer 的本质差别 (1 段总结)

mempalace 是个**满铺架构** —— 1 个 entity 同时被写到所有 4 层, layer 间没有时间维度.
apeireth 是**渐进架构** —— 1 个 item 随生命周期逐层升级 (raw → embed → tag → compress).
- 一个 R178 才 put 的新 Note: 只在 L1 (无 embed / 无 tag / content 短)
- 被 dedup 系统触发了 embed: 升到 L2
- 用户给它打 tag: 升到 L3
- content 越长, dream system 把它 chunk 化: 升到 L4
- 一个月没人访问: 从 L4 降回 L3, 半年没人访问: 降到 L2, 但 L1 永远在.

这样:
1. L4 永远只是"高频长内容"的子集 (节省存储)
2. L1 是审计源头 (append-only 守则)
3. 升级/降级本身有 signals (decay / access_count), 不黑盒

---

## 后续 (按之前 plan)

| # | 项 | 估时 | 状态 |
|---|---|---|---|
| P0-5 | Embedder 身份校验 (semantic.rs 加 model_name + dimension + persistent mismatch warning) | 2 天 | 未动 |
| P0-6 | Normalize 版本 schema (semantic_persist 加 chunk_strategy_version) | 1 天 | **部分已做** — L4 chunk 已经有 version, semantic_persist 还未 |
| P0-3 | 拆 memory↔api 循环 | 1 周 | **需用户拍板**, 已准备方案 (见下) |
| P0-4 | 4 memory backend 真接 (S3/Redis/Postgres/Disk-LRU) | 4 周 | **需用户拍板**, 建议分 4 个独立 PR |
| P1-9/10/11 | mempalace 借鉴 (Dedup/走廊/4 层) | 2+3+3 天 | **已完成** ✅ |

---

## P0-3 拆循环方案 (待拍板)

### 现状 (据之前的代码审计)
- `apeireth-memory` ↔ `apeireth-api` 互相 import, 编译成单向依赖时还好,
  但 daemon 启动时 memory → api → memory 形成 init cycle, 需要 runtime hack 打破

### 提议方案 (二选一)
**方案 A: 抽 trait + interface crate (推荐)**
- 新建 `crates/apeireth-memory-iface` (轻量, 0 业务代码, 只有 trait + types)
- `apeireth-memory` 实现 iface
- `apeireth-api` 依赖 iface, 通过 dyn trait 调用 memory
- 优点: 编译期单向, runtime 还是双向 (但通过 trait 隔离)
- 工作量: 3-4 天

**方案 B: 全 async + message bus**
- 用 `apeireth-bus` 已经有的事件总线, memory 跟 api 通过 bus 通信
- 优点: 0 循环 (运行时解耦)
- 缺点: 现有代码是 sync, 大面积改 async 1 周+
- 工作量: 5-7 天

### 我的建议
方案 A — 改动最小, 不破坏现有 sync 代码, 0 性能损失.

---

## P0-4 4 backend 真接 — 分批方案 (待拍板)

之前估时 "各 1 周 × 4 = 4 周" 太激进. 实际:
- **S3**: 1 周 (aws-sdk-s3 crate 已存在, 加 trait impl + 测试)
- **Redis**: 1 周 (redis crate 已存在, 加 trait impl + 测试)
- **Postgres**: 1.5 周 (sqlx 已经用于 SQLite, 但 PostgreSQL dialect 需配置)
- **Disk-LRU**: 0.5 周 (在 file provider 之上加 LRU evict 即可)

### 我的建议
按 **Disk-LRU → Redis → S3 → Postgres** 顺序, 每周一个独立 PR.
第一个 Disk-LRU 最简单 (0.5 周) 当 PoC 跑通, 后 3 个按需并行.

---

## 范围遵守 (再次声明)
- ✅ 0 改 LOCKED crate 入口签名
- ✅ 0 改 workspace version
- ✅ 0 主动 commit-push
- ❌ 5 Provider SDK 真接 — 没碰 (你没 SDK)
- ❌ 桌宠 / Live2D — 没碰 (你说放最后)
- ❌ 前端 — 没碰
- ✅ 仅碰 apeireth-memory 1 个 crate (内 1 mod + 3 新 mod + V2 migration)
