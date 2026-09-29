# research/b1 — B1 拓扑记忆实验确定性骨架 (Phase 0/1, 0 LLM)

协议: `_research_mem/ra/papers/b1/experiment-protocol-v1.md`。
本 crate 只实现 **Phase 0 (自检/dry-run)** 与 **Phase 1 (构图 + vendored Betti 环启发式分析)**,
不含任何 LLM 调用 (dry-run 打印 0 条调用清单, Phase 2-4 留 TODO 占位)。

## 构建 / 测试

```powershell
cd research/b1
cargo build
cargo test   # 18 个测试: 会话切分 / 截断 / vendored Betti 三角洞 / vendor 漂移检测(SHA-256)
```

## CLI 用法

```powershell
# Phase 0 自检 (0 LLM, 真实数据, 断言 去重=1033 / QA=1986 / 每会话唯一轮次之和=1033)
cargo run --release -- --mode preflight

# Phase 1 Betti (0 LLM, 需要嵌入缓存)
cargo run --release -- --mode betti --embed-cache .cache/embeddings.json

# dry-run: 打印 LLM 阶段调用清单 (骨架 = 0 条), 不花 API 钱
cargo run --release -- --mode betti --embed-cache .cache/embeddings.json --dry-run

# 参数 (默认值对齐协议)
#   --seed 42 --max-nodes 150 --threshold-ratio 0.05
```

输出:
- `research/logs/b1-preflight-<hash>.jsonl` / `research/logs/b1-betti-<hash>.jsonl` (JSONL, config_hash + log_event 风格)
- `research/metrics/b1-betti-<hash>.json` (β0 分布 / 洞数分布 / lifetime 直方图 / 每会话明细)

## 嵌入缓存格式 (骨架阶段)

fastembed 尚未接入 (协议 §7.3, `src/embed.rs` 留 TODO 占位)。骨架从 JSON 缓存读取
`dia_id → 384 维向量`, 统一 L2 归一化后构图:

```json
{
  "format": 1,
  "embed_model": "all-MiniLM-L6-v2",
  "dim": 384,
  "vectors": { "D1:1": [0.01, -0.02, ...], "...": [...] }
}
```

生成缓存 (接 fastembed 后): 对 1033 个唯一 dia_id 逐一嵌入, 写入
`research/b1/.cache/embeddings.json`。测试/离线构图可直接用 `InjectedEmbeddingProvider`
(代码级注入向量, 见 `src/embed.rs`)。

## vendor 漂移检测

`src/topo/{betti_hole_detector,kuramoto_resonance}.rs` 是
`crates/engine/memory/src/<file> @ 291442c9` 的**逐字节**副本 (文件头多一行 vendored 注释)。
`tests/vendor_drift.rs` 运行时读引擎源与副本做 SHA-256 比对, 不一致即 fail —— 保证「评的是引擎真实代码」。

## 诚实边界 (必读)

1. **环启发式 ≠ 持久同调** (协议 §8.1): `filtration_steps` 未使用、`max_dimension` 硬编码 2、
   β1 是 3/4-环枚举 + 启发式 birth/death; β2 恒为 0 (协议 §8.2)。
2. **`--max-nodes 150` 在 vendored 引擎下不可行 (重要)**:
   - `find_candidate_cycles` 的 4-环枚举以 `max_threshold = max_dist` 过滤, 而所有边 ≤ max_dist,
     故**每个 4-元组都成为候选环** (共 `C(n,4)` 个);
   - 4-环 `death_eps = max_diag.max(birth + 0.1)`, 故 lifetime ≥ 0.1; 而 L2 归一化后 max_dist ≤ 2.0,
     `min_persistence_threshold = 0.05·max_dist ≤ 0.1`, 故**每个 4-环都通过 lifetime 过滤**;
   - 结果: `analyze()` 会把 `C(n,4)` 个候选环全部物化成 `TopologicalVoidRing` (每个含多条 String/Vec
     堆分配)。n=150 ⇒ ~2×10⁷ 个环 ≈ 数十 GB 分配, debug/release 均无法在数分钟内完成。
   - 协议 §2.2 的「n=150 ⇒ ~2×10⁷ 次、可秒级」只算了枚举循环次数, 未计入 `~C(n,4)` 个结果的物化成本。
   - **实操建议**: 交互验证用 `--max-nodes 30..=40`; 若要按协议默认 150 跑真实数据,
     需先修引擎 (例如对 4-环增加真正的对角距离/边长阈值过滤, 而非 `max_dist`), 或接受巨额内存与时长。
     本骨架不改 vendored 代码 (字节等价 + 漂移检测), 该发现如实上报。
3. 会话切分按**全局首次出现**归属 10 个顶层会话 (与 `docs_from_sessions` 一致),
   自检「每会话唯一轮次之和 = 1033」通过。
