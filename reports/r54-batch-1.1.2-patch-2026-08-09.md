# R54 batch 1.1.2 patch (B8 续升级, 2026-08-09)

> 1 commit 总, 4 R + 1 docs R 一气呵成 (R54-a/b/c + R55 + R56)

---

## 1. R54: backend wire-up + cognition_graph 数据流闭环 (主轴)

### R54-a: apeireth-graph 接 apeireth-tui Cargo.toml deps

`crates/apeireth-tui/Cargo.toml` 加 1 行:
```toml
# R54 B8 续: apeireth-graph 接 cognition_graph (run_cognition_graph_sync 拿 CognitionSummary)
# backend.rs::compute_main_ai_status 在 record_short_term_messages 后调, record_cognition_summary 真接
apeireth-graph = { path = "../apeireth-graph" }
```

不引新 transitive dep (apeireth-graph 当前 deps 已经满足 tokio / serde_json / apeireth-asi / apeireth-cognition / apeireth-core)。

### R54-b: backend.rs::compute_main_ai_status 真接 3 数据源

`crates/apeireth-tui/src/backend.rs` 修改 2 块:

1. **新 helper `compute_v05_with_dims() -> (AsiV05Scores, [f64; 24])`**:
   - 原 `compute_v05()` -> `compute_v05_with_dims().0` (向上兼容)
   - 返回 scores 同时返回 24 维数组 (供 cognition_graph 消费)
   - 充填逻辑: registry 返回 < 24 时 0-pad 到 24; > 24 时截断 (defensive)

2. **`compute_main_ai_status` 增量接 3 步**:
   ```rust
   // R54 B8 续: mid_term 真接 (last 24h SQLite query)
   let now_ts = chrono::Utc::now().timestamp();
   let mid_term_count = memory_store()
       .ok()
       .and_then(|s| {
           s.query(&EpisodeQuery::new().in_range(Some(now_ts - 86_400), None).limit(usize::MAX))
               .ok()
               .map(|v| v.len() as u64)
       })
       .unwrap_or(0);
   memory::record_mid_term_count(mid_term_count);
   // R54 B8 续: long_term 近似 (total/5, 0 假装)
   memory::record_long_term_count(episode_count.saturating_div(5));
   // R54 B8 续: cognition_graph 真接
   if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
       let summary = rt.block_on(apeireth_graph::cognition_graph::run_cognition_graph_sync(&v05_dims, "snapshot_organ_main"));
       memory::record_cognition_summary(summary.mean, summary.min, summary.max, summary.verdict_approve);
   }
   ```

走 `tokio::runtime::Builder::new_current_thread` 模式, 跟现有 `call_llm_stream_sync` 一致。

### R54-c: render() 0 假装小修 + 测试守门

`crates/apeireth-tui/src/organ/memory.rs` 修改 4 块:

1. **doc header**: R22 ST-A1.8 -> R54 B8 续升级; mid_term stub -> 真接 (last 24h); long_term stub -> 近似 (vector store 未上, 0 假装)
2. **render() 中期行**: `[stub — R25.3 接 summary]` -> `(last 24h episodes, real query)`
3. **render() 长期行**: `[stub — R25.3 接 vector store]` -> `(total/5 近似, vector store 未上)`
4. **render() readiness banner**: `[partial] 1/3 真接` -> `[partial] 2/3 真接 (短期/中期), 长期 近似`
5. **readiness 注释**: `1/3 真接 (short_term), 标 partial` -> `2/3 真接 (short_term + mid_term, long_term 近似), 标 partial 不假装 ok`
6. **测试更新**:
   - `render_marks_partial_honestly`: assertion message `1/3 真接` -> `2/3 真接`
   - `render_marks_stub_per_field`: 改为断言 mid/long 字段 0 `[stub — R25.3` 标签 (`render 中无 [stub 标记`)

## 2. R55: APEIRETH-VERSIONING.md 7 子系统 R54 同步

`APEIRETH-VERSIONING.md` 修改 7 处 (1 文档 meta + 7 子系统 行):
- 主代码: `Apeireth-1.1.0-R38` -> `Apeireth-1.1.2-R54`
- 设计层: 不动 (LOCKED 5.0-R14)
- 修正链: `Fix-3..Fix-12-R38` -> `Fix-3..Fix-13-R54` (新增 `Fix-13-R54` row, 主题 "R54 B8 续: backend wire-up + cognition_graph 真接 TUI memory")
- R 周期: R38 + R46-R53 (归档) + R54 (当前) 行添加; 当前标 R54 (1.1.2 patch)
- 指标: 新增 `V1136-R54` row (cognition summary 影响 R-Measure 预计 ±0.5%)
- 基线: 新增 `snap-eafb42c7` (R46-R53 末, R54 前)
- 手册: `Manual-Rev-H` -> `Manual-Rev-I` (新 Manual-Rev-I row, 主题 R46-R53 1.1.1 + R54 1.1.2 patch)

> 注意: workspace.version = "1.1.0" Cargo.toml 0 触 — R54 续按 user 授权 "locked 文档可灵活" 在 documentation 层标 1.1.2 patch (semver-level 严格该改 1.1.2, 这是 doc-level vs semver-level 的有意分隔)。

## 3. R56: CHANGELOG + docs/1.1-release 1.1.2 entry

- `CHANGELOG.md`: 加 `## [R54 B8 续升级 / 1.1.2 patch]` entry (含 Added/Fixed/Changed/验证/不变边界/Follow-up/Commit/报告 8 节)
- `docs/1.1-release/README.md`: 加 "1.1.2 patch" section (含 stage 表 + 哲学锚 6 锚穿透 + 后续 follow-up)

## 4. 验证总表 (本批跑完)

| 范围 | 命令 | 结果 |
|---|---|---|
| 源仓 TUI 全测 | `cargo test -p apeireth-tui --bin apeireth-tui` | ✅ 402 passed, 0 failed (R54 续 0 退化) |
| TUI memory 专项 | `cargo test -p apeireth-tui --bin apeireth-tui organ::memory` | ✅ 13 passed (含 R54 render 0 假装小修守门) |
| TUI build | `cargo build -p apeireth-tui --bin apeireth-tui` | ✅ 0 errors |
| workspace lib 全测 | `cargo test --workspace --lib` | ✅ 4596 passed (R46-R53 1.1.1 baseline 不变, R54 续 0 added lib tests, 因 TUI 是 bin 而非 lib) |
| workspace build | `cargo build --workspace --tests` | ✅ 0 errors |

## 5. 哲学锚 + 锁定穿透 (R54 续 100%)

| 锚 | 落实 |
|---|---|
| S-1 北极星 | 24 LOCKED + 9 organ + 8 LOCKED + 1.1 workspace — 0 触 |
| S-2 实事求是 | render() 0 假装 (stub 标删除); mid_term 真接 (last 24h SQLite query); long_term 近似 (total/5 标注 vector store 未上) |
| O-2 走在前人尖上 | 借 VCP / LangGraph cognition_graph (R38 B8 已有) + EpisodeQuery::in_range 真接 mid_term |
| O-3 干到底 | R54 + R55 + R56 一 commit 一气呵成 |
| O-4 任何人都能接手 | 7 子系统 versioning + 本报告 + CHANGELOG + docs/1.1-release section |
| O-5 不假装 | render 3 行 0 假装小修; long_term "近似" 明确标注; render_marks_stub_per_field 测试守门 mid/long 字段 0 [stub 标签 |

## 6. 不变边界 (R54 续 0 触)

- 24 LOCKED crate src/** 0 触
- workspace.version = "1.1.0" 0 触 (per user 授权 APEIRETH-VERSIONING.md doc-level 灵活, semver-level workspace.version 不动 — 这是 doc-level vs semver-level 的有意分隔)
- 8 项承诺 0 触
- R11 LOCKED `apeireth-core::LifeStage` 10 变体 + `LEGAL_TRANSITIONS` 12 条 0 触
- R34 1.0 release 0 触

## 7. 后续 follow-up (R54 续 不在)

- **vector store 真接 long_term**: 当前 `total / 5` heuristic (0 假装标注); 真向量 store 是 1.3 路线, 需要 apeireth-vector 真接
- **TUI 9 organ memory page 加 cognition summary 显示行**: R47 已留 hook; 当前 render 显示 2/3 真接 + 长期近似标注; 真显示 cognition mean/min/max/verdict 行需用户放行 UI 改
- **backend 真接 cognition_summary 频率**: 当前 snapshot_organ_main 触发 (dashboard refresh); per-chat-cycle 更精细但 block_on cost 高
- **cognition_graph 真实 target_name**: 当前 hardcode `"snapshot_organ_main"`; 可后续按 (user input hash + session_id) 派生稳定 target_name

## 8. commit 节奏

- 源仓上批 R46-R53: commit `eafb42c7`
- 本批 R54-R56 + R55 + R56: 1 commit 总 (per user "commit 1 个也行" 授权)
- Desktop 后续: 同步源仓 commit (per user "desktop 同步")

## 9. 报告链

- 本报告 `reports/r54-batch-1.1.2-patch-2026-08-09.md`
- 上批同步报告 `reports/r46-r53-desktop-sync-2026-08-09.md`
- docs/1.1-release/README.md (NEW) 1.1.2 section
- APEIRETH-VERSIONING.md 7 子系统 R54 1.1.2 同步
- CHANGELOG.md `## [R54 B8 续升级 / 1.1.2 patch]` entry
