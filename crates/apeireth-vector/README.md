# apeireth-vector

> Apeireth 向量检索子系统 (VectorStore trait + SqliteVecBackend) — V2 P1 战区 4 skeleton (docs/v2-strategy/05 §Step 4)

apeireth-vector 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (7 src 文件 / 48 测试 + 2 Kani proof + 13 集成)

- `src/lib.rs` — 向量检索 facade 入口 (VectorStore trait re-export)
- `src/traits.rs` — VectorStore trait (per V2 §Step 4)
- `src/distance.rs` — 距离度量 (cosine / L2 / dot / hamming) + 14 测试
- `src/sqlite_backend.rs` — SqliteVecBackend 真接 sqlite-vec 0.1.x C 扩展 + 18 测试
- `src/qdrant_compat.rs` — Qdrant HTTP 协议兼容层 (R150 P1 #6, 借鉴 qdrant REST API) + 11 测试
- `src/error.rs` — VectorError enum
- `src/organ_kani_proofs.rs` — vector organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/store.rs` (13)
- 例: `examples/semantic_smoke.rs`
