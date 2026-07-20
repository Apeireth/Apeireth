# Apeireth Rust Substrate

> **主人 14:47 "多语言混合, 核心 Rust 或 C++, 够高效就行"**
> **主人 14:52 "最高深度, 最深刻优先, 不计成本"**
> **主人 14:27 "聚集全人类的智慧来打造他"**

---

## TL;DR

L0-L3 substrate 用 Rust 写 (借鉴 Qdrant / Tantivy / DeltaMemory / MemoryOS-Rust),
L4-L5 cognitive layer 留 Python (试错快 + 主人 13:47 按模块按步骤科学造)。

---

## 借鉴来源 (AnySearch 真抓 README 验证)

| 项目 | 借鉴什么 | License |
|------|---------|---------|
| **MemoryOS-Rust** (TelivANT) | 9-crate workspace + STM/MTM/LTM + Hexagonal Architecture | Apache-2.0 |
| **DeltaMemory** | WAL + CRC32 + salience decay formula | (blog post) |
| **Qdrant** (qdrant/qdrant) | HNSW vector index | Apache-2.0 |
| **Tantivy** (quickwit-oss/tantivy) | BM25 full-text + tokenizer | MIT |
| **Zep / Graphiti** (getzep/graphiti) | Episode provenance + temporal validity | Apache-2.0 |
| **claude-mem** (thedotmack) | 3-layer progressive disclosure | Apache-2.0 |
| **arXiv 2501.13956** (Zep paper) | Temporal KG architecture | paper |

---

## Workspace 架构 (借鉴 MemoryOS-Rust)

```
rust-substrate/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── apeireth-core/            # domain: Episode / Note / Identity / Memory / Reconsolidate / Forget / WAL / Tier / RelationGraph
│   ├── apeireth-ports/           # trait interfaces (Hexagonal)
│   ├── apeireth-adapters/        # Sqlite / Qdrant / Tantivy / File / OpenAI-compatible
│   ├── apeireth-gateway/         # HTTP / JSON-RPC server (Axum)
│   ├── apeireth-py/              # PyO3 binding (Python calls Rust)
│   └── apeireth-cli/             # CLI + benchmarks
```

---

## 模块 (apeireth-core)

| 模块 | 借鉴 | 主人原话 |
|------|------|---------|
| `episode.rs` | Graphiti | 主人 12:14 "中央 AI 是永恒身份" → 不可变 |
| `note.rs` | background cron memory.py | 主人 13:47 "记忆是我关心的" |
| `memory.rs` | MemoryOS-Rust | STM/MTM/LTM 三层 |
| `identity.rs` | 主人 12:54 "8 问" → IdentityCard | 主人 12:14 "多身份" |
| `reconsolidate.rs` | background cron 4 paths | 主人 13:47 关心 |
| `forget.rs` | PersistBench 97% sycophancy | 主人 13:47 防 sycophancy |
| `wal.rs` | DeltaMemory WAL + CRC32 | 主人 14:52 "24/7 不能崩" |
| `tier.rs` | MemoryOS-Rust tier_manager | 主人 12:14 "永恒身份" = LTM |
| `relation_graph.rs` | AriGraph + Graphiti | 主人 12:14 "社会关系总和" |

---

## Python ↔ Rust 桥 (PyO3)

```rust
// apeireth-py/src/lib.rs
#[pymodule]
fn apeireth_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEpisodeRepo>()?;
    m.add_function(wrap_pyfunction!(py_forget_sweep, m)?)?;
    m.add_function(wrap_pyfunction!(py_reconsolidate, m)?)?;
    Ok(())
}
```

Python 调:
```python
import apeireth_py
repo = apeireth_py.PyEpisodeRepo("data/apeireth.db")
ok = repo.append("master", "中央 AI 必须有 Memory", "ctx", "utterance", "hash", "stm")
print(repo.count())
```

---

## Gateway HTTP API (借鉴 MemoryOS-Rust gateway)

```
GET  /health
POST /episodes
GET  /episodes?tier=stm&limit=10
POST /notes/upsert
POST /notes/forget-sweep    {threshold}
POST /notes/reconsolidate   {IdentityCard}
```

---

## 状态

- ✅ Phase 4 scaffold 完成 (主人 14:52 离开期间)
- ⏳ Rust install 跑中 (PID 27564)
- ⏳ cargo check 验证
- ⏳ Phase 3 Python cognitive layer 同步推进

---

_楚零 2026-07-20 14:55_
_主人 14:47 + 14:52 拍板: 多语言混合 + 核心 Rust + 最高深度_