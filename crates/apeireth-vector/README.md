# apeireth-vector

Apeireth 向量检索子系统 skeleton (V2 P1 战区 4) — 对应 `docs/v2-strategy/05-EXECUTION-NOW.md` §Step 4。

## 设计

1. **`VectorStore` trait** — 唯一契约，业务方只依赖 trait，不绑死后端。
2. **`SqliteVecBackend`** — 默认实现：纯 `rusqlite` + BLOB-packed f32 + Rust 端 brute-force 余弦。
   - 表 `vec_items(id BLOB PK, dim INT, vec BLOB, metadata TEXT)`
   - 元数据表 `vec_meta(key, value)` 存维度
3. **与 `apeireth-memory` 共存** — 各自打开自己的 `.db` 文件，SQLite WAL 不互锁；类型层只通过 `Uuid` 标签对齐。

## 不做的事

- ❌ 不引入 `unsafe_code`
- ❌ 不引入 ORM
- ❌ 不触碰 `apeireth-memory` 任何现有表 / migration
- ❌ 不强行接 `sqlite-vec` C 扩展（0.1.10-alpha，对骨架不稳）

## 升级路径

ponytail ceiling（当前刻意简化）：
- 1000 条 / 256 维 brute-force 余弦 < 200ms 足够；
- 真上规模（>10w 条）时换 `sqlite-vec` C 扩展（vec0 虚拟表）或 `lancedb-rs`，
  **仅替换 backend 实现**，trait 调用方零改动。

## 用法

```bash
cargo run -p apeireth-vector --example semantic_smoke
```

输出：插入 1000 条 mock 向量 + 50 次 top-5 余弦检索 + 延迟统计 + top-1 类别命中率。
