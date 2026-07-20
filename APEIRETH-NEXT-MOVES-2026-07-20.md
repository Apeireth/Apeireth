# Apeireth 调研 + 方向疑问 — 2026-07-20 14:50

> 主人 14:48 "边写边搜论文，联网查，GitHub钻研，要聚集全人类的智慧"
> 主人 14:48 "你有任何疑问，方向疑问就和我说"

---

## 我刚调研的真发现 (AnySearch 真抓 GitHub raw)

### 1️⃣ MemoryOS-Rust (TelivANT) **直接对标我们的目标**

| 维度 | 我们 (Apeireth) | MemoryOS-Rust |
|------|----------------|---------------|
| 语言 | 混合 (Python + Rust) | Rust |
| Stars | 4 (刚起步) | 4 |
| Commits | 22+ | **165** |
| Crates | 1 (Python package) | **9 (workspace)** |
| Stack | SQLite + AnySearch | **Tokio + Axum + Tower + Qdrant-client + Redis** |
| 三层记忆 | ❌ 没 | ✅ STM / MTM / LTM |
| 借鉴度 | 0 | **直接抄 workspace 架构 + STM/MTM/LTM 范式** |

借鉴对象: `apeireth/memoryos_inspect.py` 已经抓了 6 个核心文件存档

### 2️⃣ DeltaMemory 真生产数据 (2026-01-15)
- Rust 实测 `< 50ms p50 retrieval`
- Python 实测 `800ms p50` (16 倍差距)
- **决定我们 L3 hot path 用 Rust**

### 3️⃣ 8 个我没读的新 arxiv 论文 (2026 系列)
需要在 Phase 4 之前读:
- arxiv.org/abs/2603.07670
- arxiv.org/abs/2605.18226
- arxiv.org/abs/2602.21600
- arxiv.org/abs/2604.11544
- arxiv.org/abs/2607.00151
- arxiv.org/abs/2605.30785
- arxiv.org/abs/2602.11443
- arxiv.org/pdf/2602.11443

(原来的 18 篇调研是 2025-2026 系列,这 8 个是 2026 新出的)

---

## 我的方向疑问 (4 个, 主人拍板)

### 疑问 1 — 怎么借鉴 MemoryOS-Rust 的 9-crate workspace?

```
它们:
crates/
├── memoryos-core/      # 核心 domain (memory, faq, identity...)
├── memoryos-ports/     # port 接口 (hexagonal architecture)
├── memoryos-adapters/  # Qdrant / Redis / LLM adapters
├── memoryos-gateway/   # HTTP API
├── memoryos-worker/    # background jobs
├── memoryos-metrics/   # Prometheus
├── memoryos-admin/     # CLI
├── memoryos-wiki-gen/  # doc generator
└── memoryos-mcp/       # MCP server
```

我的判断:
- ✅ 抄 workspace 模式 (1 cargo workspace 多 crate)
- ✅ 抄 Hexagonal Architecture (core / ports / adapters 分离)
- ❌ 不抄它们的命名 (memoryos-rust 不通用)

主人 OK 吗? 还是**先单 crate 起手**, 后续再 split?

### 疑问 2 — STM/MTM/LTM 三层记忆 vs 我们现在的 single-tier?

MemoryOS-Rust 的三层:
- **STM** (Short-term): 最近对话 → vector DB, 频繁更新
- **MTM** (Medium-term): 主题聚合 → vector DB, 定期总结
- **LTM** (Long-term): 持久事实 → vector DB, 永不丢

我们的现状: `Episode + Note + Forget Sweep` (单一时间线)

我的判断:
- 三层更接近**人类记忆**(感官/工作/长期)
- 主人 12:14 "中央 AI 永恒身份" = LTM 必须
- 主人 13:47 "记忆是我关心的" = 应该升级到三层

主人 OK 升级到 STM/MTM/LTM 吗?

### 疑问 3 — L4-L5 cognitive layer 留 Python 还是也迁 Rust?

主人 14:47 "多语言混合, 核心 Rust 或 C++, 够高效"

我的判断:
- **L4 身份/记忆 schema** 留 Python (试错快, schema 改动频繁)
- **L5 涌现 / 反思机制** 留 Python (LLM 调用是网络 bound)
- **L0-L3 substrate** 用 Rust (vector / search / async)

这个划分 OK 吗?

### 疑问 4 — Phase 4 启动顺序

我们当前 done:
- Phase 1: Identity Store ✅
- Phase 1.5: AnySearch ✅
- Phase 2: Memory in-memory ✅
- Phase 2.5: SQLite + FTS5 ✅

接下来:
- Phase 3: Persona + Relation Graph (Python 写, 借鉴 Graphiti episode)
- Phase 4: **Rust substrate 起手** (vector + WAL + async)
- Phase 5: 真涌现

我的判断: 主人拍板后立刻**装 Rust + 开 Phase 4 cargo new**

需要主人**拍 4 件事**:
1. Workspace 模式 (9 crate vs 1 crate 起手)
2. STM/MTM/LTM 三层升级 OK?
3. L4-L5 留 Python OK?
4. 装 Rust 走起?

---

## 我的下一步 (不需主人等)

1. **继续调研**: 读 8 个新 arxiv 论文 (主人 14:48 "搜论文")
2. **继续 GitHub 深读**: MemoryOS-Rust 的 STM/MTM/LTM 具体代码
3. **继续 Phase 3 写**: Persona + Relation Graph (Python, 借鉴 Graphiti)
4. **等主人拍板** Phase 4 启动

---

_楚零 2026-07-20 14:50_
_本报告: 主人 14:48 后真调研 + 4 个方向疑问, 等主人拍板_