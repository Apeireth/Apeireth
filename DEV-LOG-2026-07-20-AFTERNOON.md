# DEV-LOG (after 主人 14:52 离开, "不计成本" 启动)

## 主人 14:27 关键指令
- "你来把关建造就行"
- "目标依然是聚集全人类的智慧来打造他"
- "打造他" — 中央 AI = "他" (像人, 多身份, 永恒)

## 主人 14:32 三件事
1. GitHub 网上好东西不要少
2. 不知道搜什么就问博查 AI
3. 底层代码要高效 nb, 不 Python 糊弄

## 主人 14:35 Rust 起手试探
"我在思考用rust写行不行, 你先继续"

## 主人 14:42 "哪个语言最高效?"
- 调研 DeltaMemory: Python 800ms vs Rust <50ms (16x)
- 调研 sinqua/agent-runtime-bench: C++ 100 并发 93 MiB
- 调研 Memory Safety: Prossimo / Microsoft 数据
- **结论**: 不计时间成本 → Rust 略胜 C++ (5-10% perf gap + 100% safety)

## 主人 14:45 "我们就rust起手, 开干"

## 主人 14:47 "多语言混合, 核心 Rust 或 C++, 够高效就行"

## 主人 14:48 "边写边搜论文, 联网查, GitHub钻研, 聚集全人类智慧"

## 主人 14:52 "最高深度, 最深刻优先, 不计成本和时间, 我离开一会儿, 你开干"

---

## 我做的 (主人不在时)

### Phase 3 完成 (background cron 跑)
- `relation.py` 9552 bytes — Relation Graph v0.1
- `relation_store.py` 11082 bytes — v0.2 SQLite 持久化
- `run_relation_demo.py` + `run_relation_store_demo.py`
- 中心节点 `ai_self` (主人 12:14 "像人是一切社会关系的总和")
- 8 node kinds + 7 edge kinds (AriGraph + Graphiti 借鉴)
- 跨层 ref (Episode/Note 引用)

### Phase 4 Rust scaffold 完成 (主人 14:47 + 14:52)
- **6 crates workspace**:
  - `apeireth-core` (9 modules: Episode / Note / Identity / Memory / Reconsolidate / Forget / WAL / Tier / RelationGraph)
  - `apeireth-ports` (Hexagonal Architecture: 7 traits)
  - `apeireth-adapters` (Sqlite / Qdrant / Tantivy / File WAL / OpenAI-compatible)
  - `apeireth-gateway` (Axum HTTP server)
  - `apeireth-py` (PyO3 binding — Python 调 Rust)
  - `apeireth-cli` (CLI + benchmarks)
- **46 files, 4395 lines** commit `5be6bc8`

### 调研真金白银
- **8 个新 arxiv 论文** (2603-2607 系列) — 主人 14:52 之前没读
  - 2603.07670: Memory for Autonomous LLM Agents (survey 2022-2026) ⭐⭐⭐⭐⭐
  - 2604.11544: RoMem - Temporal Knowledge Graph (主人 13:47 时效性)
  - 2607.00151: SmoothAgent - Long-horizon
  - 2605.18226: Context Memorization
  - 2605.30785: AdaCoM - Adaptive Context
  - 2602.11443: Filtered ANN Search
  - 2602.21600: AQR-HNSW
  - 2501.13956: Zep temporal KG (主人 12:47 调研过)

- **MemoryOS-Rust** (TelivANT, Apache-2.0) — 9-crate workspace + STM/MTM/LTM 直接借鉴
- **DeltaMemory** — WAL + CRC32 + salience decay formula
- **Qdrant / Tantivy / Graphiti / claude-mem** — 全部真调研 README 验证

### Rust install 状态
- ✅ rustup-init 12.8 MB 下载 (14:52)
- ✅ toolchain 装到 15:08
- ⏳ rustc 还在装 (CPU 2.6s, 没用 default profile)
- 我已经手动 uninstall + 重装 default profile, 还在跑

### 借鉴库汇总 (后续 L4-L5 用)
- DeltaMemory 真生产数据: Rust < 50ms vs Python 800ms
- sinqua bench: C++ 100 agent 93 MiB
- MemoryOS-Rust: 9-crate workspace + STM/MTM/LTM 范式

---

## git log (主人离开期间)

```
e0c84d7  research: sinqua C++ benchmark 93 MiB + Memory Safety in C++/Rust/Zig
5be6bc8  feat(rust): Phase 4 Rust substrate scaffold - 6 crates workspace + 9 modules + Hexagonal architecture
df95c97  feat(identity): Phase 3 Relation Graph v0.1 PoC + v0.2 SQLite 持久化
b906606  research: MemoryOS-Rust 9-crate workspace + STM/MTM/LTM + 4 方向疑问
69ae959  research: language decision - DeltaMemory 16x Rust gap + 3 选项
d597171  feat(memory): Phase 2.5 v0.2 SQLite+FTS5 + RESEARCH Rust 路线
8812412  demo: memory.demo.json
debc43b  feat(memory): Phase 2 Memory Layer v0.1
91b5231  feat(memory): background cron 跑出的 Phase 2 v0.1 PoC
413d7a5  feat(apeireth): AnySearch 集成
...
```

---

## 主人回来时下一步

1. **验证 cargo check** (Rust 装好后) → cargo build → cargo test
2. **Phase 3 Person 写完** (background cron 在做)
3. **Phase 4 CLI benchmark** (cargo run --release -- bench insert-episodes --count 10000)
4. **Phase 4 PyO3 binding test** (Python 调 Rust, 真 Episode 1000 insert 验证)
5. **决策**: 写 Questioning Engine (Pep / Funnel Question 借鉴) 还是 Reasoning Layer (MARS / Self-Harness)

---

_楚零 2026-07-20 15:12_
_主人 14:52 离开, "不计成本" 启动 Phase 4 Rust + Phase 3 Relation Graph 完成 + 8 个新 arxiv 论文调研_